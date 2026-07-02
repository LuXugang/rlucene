/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::test_framework::core::util::lucene_test_case::DEFAULT_LINE_DOCS_FILE;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, NaiveDateTime};
use flate2::read::GzDecoder;
use rand::prelude::StdRng;
use rand::{Rng, RngExt, SeedableRng};

use crate::core::document::document::Document;
use crate::core::document::field::{Field, FieldBase, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::int_field::IntField;
use crate::core::document::int_point::IntPoint;
use crate::core::document::keyword_field::KeywordField;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::index_options::IndexOptions;
use crate::core::util::error::lucene_error::{LuceneError, Result};

const BUFFER_SIZE: usize = 1 << 16;
const SEP: char = '\t';

pub const DATE_FIELD_VALUE_TO_LOCALDATETIME: fn(&str) -> Result<NaiveDateTime> =
  date_field_value_to_local_date_time;

/// Minimal port of benchmark's LineDocSource + DocMaker, so tests can enum docs from a line file
/// created by benchmark's WriteLineDoc task.
pub struct LineFileDocs {
  reader: Option<Box<dyn BufRead + Send>>,
  id: i32,
  path: String,
  random: StdRng,
  thread_docs: Option<DocState>,
}

impl LineFileDocs {
  pub fn new<R>(random: &mut R) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    Self::with_path(random, DEFAULT_LINE_DOCS_FILE)
  }

  pub fn with_path<R>(random: &mut R, path: &str) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    let mut line_file_docs = Self {
      reader: None,
      id: 0,
      path: path.to_string(),
      random: StdRng::seed_from_u64(random.random()),
      thread_docs: None,
    };
    line_file_docs.open()?;
    Ok(line_file_docs)
  }

  pub fn close(&mut self) {
    self.reader = None;
    self.thread_docs = None;
  }

  fn random_seek_pos(&mut self, size: u64) -> u64 {
    if size <= 3 {
      0
    } else {
      self.random.random::<u64>() % (size / 3)
    }
  }

  fn open(&mut self) -> Result<()> {
    let file = resolve_line_file_path(&self.path)?;
    let size = file.metadata()?.len();
    let need_skip;

    let is: Box<dyn Read + Send> = if self.path.ends_with(".gz") {
      need_skip = true;
      Box::new(file)
    } else {
      let seek_to = self.random_seek_pos(size);
      let mut file = file;
      file.seek(SeekFrom::Start(seek_to))?;
      if seek_to > 0 {
        let mut b = [0u8; 1];
        loop {
          if file.read(&mut b)? == 0 || b[0] == b'\r' || b[0] == b'\n' {
            break;
          }
        }
      }
      need_skip = false;
      Box::new(file)
    };

    let is: Box<dyn Read + Send> = if need_skip {
      let v = seek_file_path(&self.path)?;
      let seek_file = resolve_line_file_path(&v)?;
      let mut skip_points = Vec::new();
      skip_points.push(0);

      let reader = BufReader::new(seek_file);
      for line in reader.lines() {
        skip_points.push(line?.trim().parse::<u64>()?);
      }

      let seek_to = skip_points[self.random.random_range(0..skip_points.len())];
      let mut is = is;
      let mut skipped = 0;
      let mut buffer = [0u8; BUFFER_SIZE];
      while skipped < seek_to {
        let left = (seek_to - skipped) as usize;
        let count = is.read(&mut buffer[..left.min(BUFFER_SIZE)])?;
        if count == 0 {
          break;
        }
        skipped += count as u64;
      }
      Box::new(GzDecoder::new(is))
    } else {
      is
    };

    self.reader = Some(Box::new(BufReader::with_capacity(BUFFER_SIZE, is)));
    Ok(())
  }

  pub fn reset(&mut self) -> Result<()> {
    self.reader = None;
    self.open()?;
    self.id = 0;
    Ok(())
  }

  /// Note: Document instance is re-used per-thread in Java. This Rust port keeps the same DocState
  /// shape and refreshes the stored Document from the current field values.
  pub fn next_doc(&mut self) -> Result<Document> {
    let mut line = String::new();
    {
      let reader = self
        .reader
        .as_mut()
        .ok_or_else(|| LuceneError::illegal_state("reader is closed"))?;
      if reader.read_line(&mut line)? == 0 {
        self.reader = None;
        self.open()?;
        line.clear();
        self
          .reader
          .as_mut()
          .ok_or_else(|| LuceneError::illegal_state("reader is closed"))?
          .read_line(&mut line)?;
      }
    }
    while line.ends_with('\n') || line.ends_with('\r') {
      line.pop();
    }

    if self.thread_docs.is_none() {
      self.thread_docs = Some(DocState::new()?);
    }
    let doc_state = self.thread_docs.as_mut().unwrap();

    let spot = line.find(SEP).ok_or_else(|| {
      LuceneError::illegal_argument(format!("line: [{}] is in an invalid format !", line))
    })?;
    let spot2 = line[spot + 1..]
      .find(SEP)
      .map(|pos| spot + 1 + pos)
      .ok_or_else(|| {
        LuceneError::illegal_argument(format!("line: [{}] is in an invalid format !", line))
      })?;

    doc_state.body.set_string_value(&line[spot2 + 1..])?;
    let title = &line[..spot];
    doc_state.title.set_string_value(title)?;
    doc_state.title_tokenized.set_string_value(title)?;
    doc_state.date.set_string_value(&line[spot + 1..spot2])?;
    let i = self.id;
    self.id += 1;
    doc_state.id.set_string_value(i.to_string())?;
    doc_state.id_num.set_int_value(i)?;
    doc_state
      .page_views
      .set_long_value(self.random.random_range(0..10_000))?;
    doc_state.set_doc();

    if self.random.random_range(0..5) == 4 {
      let mut doc = Document::new();
      for field in doc_state.doc.get_fields() {
        doc.add(field.clone());
      }

      if self.random.random_range(0..3) == 1 {
        let x = self.random.random_range(0..4);
        doc.add(IntPoint::new(
          format!("docLength{}", x),
          [line.chars().count() as i32],
        )?);
      }

      if self.random.random_range(0..3) == 1 {
        let x = self.random.random_range(0..4);
        doc.add(IntPoint::new(
          format!("docTitleLength{}", x),
          [title.chars().count() as i32],
        )?);
      }

      if self.random.random_range(0..3) == 1 {
        let x = self.random.random_range(0..4);
        doc.add(NumericDocValuesField::new(
          format!("docLength{}", x),
          line.chars().count() as i64,
        ));
      }

      // TODO: more random sparse fields here too
    }

    Ok(doc_state.doc.clone())
  }
}

impl Drop for LineFileDocs {
  fn drop(&mut self) {
    self.close();
  }
}

pub fn date_field_value_to_local_date_time(s: &str) -> Result<NaiveDateTime> {
  if s.len() == 10
    && s.as_bytes()[0..4].iter().all(u8::is_ascii_digit)
    && s.as_bytes()[4] == b'-'
    && s.as_bytes()[5..7].iter().all(u8::is_ascii_digit)
    && s.as_bytes()[7] == b'-'
    && s.as_bytes()[8..10].iter().all(u8::is_ascii_digit)
  {
    Ok(
      NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|err| LuceneError::illegal_argument(err.to_string()))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| LuceneError::illegal_argument(format!("invalid date: {}", s)))?,
    )
  } else {
    let lower = s.to_ascii_lowercase();
    let mut normalized = String::with_capacity(lower.len());
    let mut make_upper = true;
    for ch in lower.chars() {
      if make_upper && ch.is_ascii_alphabetic() {
        normalized.push(ch.to_ascii_uppercase());
        make_upper = false;
      } else {
        normalized.push(ch);
        make_upper = ch == '-' || ch == ' ';
      }
    }
    let date = NaiveDateTime::parse_from_str(&normalized, "%d-%b-%Y %H:%M:%S%.f")
      .map_err(|err| LuceneError::illegal_argument(err.to_string()))?;
    Ok(date)
  }
}

struct DocState {
  doc: Document,
  title_tokenized: Field,
  title: KeywordField,
  body: Field,
  id: StringField,
  id_num: IntField,
  date: StringField,
  page_views: NumericDocValuesField,
}

impl DocState {
  fn new() -> Result<Self> {
    let title = KeywordField::from_string("title", "", Store::No)?;

    let mut ft = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
    ft.set_index_options(IndexOptions::DocsAndFreqsAndPositions)?;
    ft.set_store_term_vectors(true)?;
    ft.set_store_term_vector_offsets(true)?;
    ft.set_store_term_vector_positions(true)?;

    let title_tokenized = Field::from_string("titleTokenized", "", ft.clone())?;
    let body = Field::from_string("body", "", ft)?;
    let id = StringField::from_string("docid", "", Store::Yes)?;
    let id_num = IntField::new("docid_int", 0, Store::No)?;
    let date = StringField::from_string("date", "", Store::Yes)?;
    let page_views = NumericDocValuesField::new("page_views", 0);

    let mut doc_state = Self {
      doc: Document::new(),
      title_tokenized,
      title,
      body,
      id,
      id_num,
      date,
      page_views,
    };
    doc_state.set_doc();
    Ok(doc_state)
  }

  fn set_doc(&mut self) {
    let mut doc = Document::new();
    doc.add(self.title.clone());
    doc.add(self.title_tokenized.clone());
    doc.add(self.body.clone());
    doc.add(self.id.clone());
    doc.add(self.id_num.clone());
    doc.add(self.date.clone());
    doc.add(self.page_views.clone());
    self.doc = doc;
  }
}

fn seek_file_path(path: &str) -> Result<String> {
  let index = path.rfind('.').ok_or_else(|| {
    LuceneError::illegal_argument(format!(
      "could not determine extension for path \"{}\"",
      path
    ))
  })?;
  Ok(format!("{}.seek", &path[..index]))
}

fn resolve_line_file_path(path: &str) -> Result<File> {
  let path = Path::new(path);

  let resolved_path = if path.is_absolute() {
    path.to_path_buf()
  } else {
    let source_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file!());
    let source_dir = source_file_path.parent().ok_or_else(|| {
      LuceneError::illegal_state(format!(
        "could not determine LineFileDocs source directory from {}",
        source_file_path.display()
      ))
    })?;
    let candidates = [
      source_dir.join(path),
      PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join(path),
    ];
    candidates
      .into_iter()
      .find(|candidate| candidate.exists())
      .unwrap_or_else(|| source_dir.join(path))
  };

  File::open(&resolved_path)
    .map_err(|err| LuceneError::io_with_path(resolved_path.display().to_string(), err))
}
