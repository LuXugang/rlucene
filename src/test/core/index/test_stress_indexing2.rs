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
use crate::core::document::document::Document;
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::document::fields::Fields as DocumentFields;
use crate::core::document::text_field;
use crate::core::index::directory_reader;
use crate::core::index::field_infos;
use crate::core::index::fields::Fields as IndexFields;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_bits;
use crate::core::index::multi_terms;
use crate::core::index::multi_terms::TermsType;
use crate::core::index::postings_enum::{ALL, FREQS, NONE, PostingsEnum};
use crate::core::index::standard_directory_reader::StandardDirectoryReader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::index::term_vectors::TermVectors;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::bits::Bits;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::iterator::IteratorExt;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_log_merge_policy_with_merge_factor_cfs, new_maybe_virus_checking_directory, random,
  random_from_seed,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::RngExt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

#[allow(dead_code)] // for quick search
struct TestStressIndexing2;

static MAX_FIELDS: i32 = 4;
static BIG_FIELD_SIZE: i32 = 10;
static SAME_FIELD_ORDER: bool = false;
static MERGE_FACTOR: i32 = 3;
static MAX_BUFFERED_DOCS: i32 = 3;
static SEED: i32 = 0;

#[test]
fn test_random_iw_reader() -> Result<()> {
  let mut r = random();
  let field_types = Mutex::new(HashMap::new());
  let dir = new_maybe_virus_checking_directory(&mut r)?;

  let dw = index_random_iw_reader(
    &mut r,
    &field_types,
    5,
    3,
    100,
    dir.clone(),
    SAME_FIELD_ORDER,
    MERGE_FACTOR,
    MAX_BUFFERED_DOCS,
    SEED,
  )?;
  let reader = directory_reader::open_with_writer_deletes(&dw.writer, true, false)?;
  dw.writer.commit()?;
  verify_equals(&mut r, &reader, dir, "id")?;
  reader.close()?;
  dw.writer.close()
}

#[test]
fn test_random() -> Result<()> {
  let mut r = random();
  let field_types = Mutex::new(HashMap::new());
  let dir1 = new_maybe_virus_checking_directory(&mut r)?;
  let dir2 = new_maybe_virus_checking_directory(&mut r)?;
  let do_reader_pooling = r.random_bool(0.5);
  let docs = index_random(
    &mut r,
    &field_types,
    5,
    3,
    100,
    dir1.clone(),
    do_reader_pooling,
    SAME_FIELD_ORDER,
    MERGE_FACTOR,
    MAX_BUFFERED_DOCS,
    SEED,
  )?;
  index_serial(&mut r, &docs, dir2.clone())?;

  verify_equals_dirs(&mut r, dir1, dir2, "id")
}

#[test]
fn test_multi_config() -> Result<()> {
  let mut r = random();
  let field_types = Mutex::new(HashMap::new());
  let num = at_least(&mut r, 3);
  let mut seed = SEED;
  for _ in 0..num {
    let same_field_order = r.random_bool(0.5);
    let merge_factor = r.random_range(0..3) + 2;
    let max_buffered_docs = r.random_range(0..3) + 2;
    let do_reader_pooling = r.random_bool(0.5);
    seed += 1;

    let n_threads = r.random_range(0..5) + 1;
    let iter = r.random_range(0..5) + 1;
    let range = r.random_range(0..20) + 1;
    let dir1 = new_directory_shared(&mut r)?;
    let dir2 = new_directory_shared(&mut r)?;
    let docs = index_random(
      &mut r,
      &field_types,
      n_threads,
      iter,
      range,
      dir1.clone(),
      do_reader_pooling,
      same_field_order,
      merge_factor,
      max_buffered_docs,
      seed,
    )?;
    index_serial(&mut r, &docs, dir2.clone())?;
    verify_equals_dirs(&mut r, dir1, dir2, "id")?;
  }
  Ok(())
}

struct DocsAndWriter {
  docs: HashMap<String, Document>,
  writer: Arc<IndexWriter<DirEnum>>,
}
#[allow(clippy::too_many_arguments)]
fn index_random_iw_reader<R>(
  random: &mut R,
  field_types: &Mutex<HashMap<String, FieldType>>,
  n_threads: i32,
  iterations: i32,
  range: i32,
  dir: Arc<DirEnum>,
  same_field_order: bool,
  merge_factor: i32,
  max_buffered_docs: i32,
  seed: i32,
) -> Result<DocsAndWriter>
where
  R: rand::Rng + ?Sized,
{
  let mut docs = HashMap::new();
  let analyzer = MockAnalyzer::new(&mut *random);
  let mut config = new_index_writer_config_with_analyzer(&mut *random, analyzer)?;
  config.set_open_mode(OpenMode::Create);
  config.set_ram_buffer_size_mb(0.1);
  config.set_max_buffered_docs(max_buffered_docs);
  config.set_merge_policy(new_log_merge_policy_with_merge_factor_cfs(
    &mut *random,
    false,
    merge_factor,
  )?);
  let w = IndexWriter::new(dir.clone(), config)?;
  w.commit()?;

  let thread_results = thread::scope(|scope| {
    let mut handles = Vec::new();
    for i in 0..n_threads {
      let w = &w;
      handles.push(scope.spawn(move || {
        IndexingThread::new(1000000 * i, range, iterations, same_field_order, seed)
          .run(w, field_types)
      }));
    }

    let mut results = Vec::new();
    for handle in handles {
      results.push(handle.join());
    }
    results
  });

  for thread_result in thread_results {
    match thread_result {
      Ok(Ok(thread_docs)) => docs.extend(thread_docs),
      Ok(Err(err)) => return Err(err),
      Err(_) => return Err(LuceneError::illegal_state("thread hit exception")),
    }
  }

  TestUtil::check_index(random, dir)?;
  Ok(DocsAndWriter { docs, writer: w })
}
#[allow(clippy::too_many_arguments)]
fn index_random<R>(
  random: &mut R,
  field_types: &Mutex<HashMap<String, FieldType>>,
  n_threads: i32,
  iterations: i32,
  range: i32,
  dir: Arc<DirEnum>,
  do_reader_pooling: bool,
  same_field_order: bool,
  merge_factor: i32,
  max_buffered_docs: i32,
  seed: i32,
) -> Result<HashMap<String, Document>>
where
  R: rand::Rng + ?Sized,
{
  let mut docs = HashMap::new();
  let analyzer = MockAnalyzer::new(&mut *random);
  let mut config = new_index_writer_config_with_analyzer(&mut *random, analyzer)?;
  config.set_open_mode(OpenMode::Create);
  config.set_ram_buffer_size_mb(0.1);
  config.set_max_buffered_docs(max_buffered_docs);
  config.set_reader_pooling(do_reader_pooling);
  config.set_merge_policy(new_log_merge_policy_with_merge_factor_cfs(
    &mut *random,
    false,
    merge_factor,
  )?);
  let w = IndexWriter::new(dir.clone(), config)?;

  let thread_results = thread::scope(|scope| {
    let mut handles = Vec::new();
    for i in 0..n_threads {
      let w = &w;
      handles.push(scope.spawn(move || {
        IndexingThread::new(1000000 * i, range, iterations, same_field_order, seed)
          .run(w, field_types)
      }));
    }

    let mut results = Vec::new();
    for handle in handles {
      results.push(handle.join());
    }
    results
  });

  w.close()?;

  for thread_result in thread_results {
    match thread_result {
      Ok(Ok(thread_docs)) => docs.extend(thread_docs),
      Ok(Err(err)) => return Err(err),
      Err(_) => return Err(LuceneError::illegal_state("thread hit exception")),
    }
  }

  TestUtil::check_index(random, dir)?;
  Ok(docs)
}

fn index_serial<R>(
  random: &mut R,
  docs: &HashMap<String, Document>,
  dir: Arc<DirEnum>,
) -> Result<()>
where
  R: rand::Rng + ?Sized,
{
  let analyzer = MockAnalyzer::new(random);
  let mut config = new_index_writer_config_with_analyzer(random, analyzer)?;
  config.set_merge_policy(new_log_merge_policy(random)?);
  let w = IndexWriter::new(dir, config)?;

  for d in docs.values() {
    let mut fields: Vec<DocumentFields> = d.get_fields().to_vec();
    fields.sort_by(|a, b| a.name().cmp(b.name()));

    let mut d1 = Document::new();
    for field in fields {
      d1.add(field);
    }
    w.add_document(d1)?;
  }

  w.close()
}

fn verify_equals<D, R>(
  random: &mut R,
  r1: &StandardDirectoryReader<D>,
  dir2: Arc<D>,
  id_field: &str,
) -> Result<()>
where
  D: Directory + 'static,
  R: rand::Rng + ?Sized,
{
  let r2 = directory_reader::open(dir2)?;
  verify_equals_readers(random, r1, &r2, id_field)?;
  r2.close()
}

fn verify_equals_dirs<D, R>(
  random: &mut R,
  dir1: Arc<D>,
  dir2: Arc<D>,
  id_field: &str,
) -> Result<()>
where
  D: Directory + 'static,
  R: rand::Rng + ?Sized,
{
  let r1 = directory_reader::open(dir1)?;
  let r2 = directory_reader::open(dir2)?;
  verify_equals_readers(random, &r1, &r2, id_field)?;
  r1.close()?;
  r2.close()
}

fn next_non_deleted_doc<P, B>(it: &mut P, live_docs: Option<&B>) -> Result<i32>
where
  P: PostingsEnum,
  B: Bits,
{
  let mut doc = it.next_doc()?;
  while doc != NO_MORE_DOCS
    && live_docs.is_some_and(|live_docs| !live_docs.get(doc as usize).unwrap())
  {
    doc = it.next_doc()?;
  }
  Ok(doc)
}

fn verify_equals_readers<D, R>(
  random: &mut R,
  r1: &StandardDirectoryReader<D>,
  r2: &StandardDirectoryReader<D>,
  id_field: &str,
) -> Result<()>
where
  D: Directory,
  R: rand::Rng + ?Sized,
{
  assert_eq!(r1.num_docs()?, r2.num_docs()?);
  let has_deletes = !(r1.max_doc()? == r2.max_doc()? && r1.num_docs()? == r1.max_doc()?);

  let mut r2r1 = vec![0; r2.max_doc()? as usize];
  let mut indexed_fields1: Vec<_> = field_infos::get_indexed_fields(r1)?.into_iter().collect();
  if indexed_fields1.is_empty() {
    assert!(field_infos::get_indexed_fields(r2)?.is_empty());
    return Ok(());
  }
  indexed_fields1.sort();

  let Some(terms1) = multi_terms::get_terms(r1, id_field)? else {
    assert!(multi_terms::get_terms(r2, id_field)?.is_none());
    return Ok(());
  };
  let mut terms_enum = terms1.iterator()?;

  let live_docs1 = multi_bits::get_live_docs(r1)?;
  let live_docs2 = multi_bits::get_live_docs(r2)?;
  let terms2 = multi_terms::get_terms(r2, id_field)?;
  if terms2.is_none() {
    let mut docs = None;
    while terms_enum.next()?.is_some() {
      docs = Some(TestUtil::docs(
        &mut *random,
        &mut terms_enum,
        docs,
        NONE as i32,
      )?);
      if next_non_deleted_doc(docs.as_mut().unwrap(), live_docs1.as_ref())? != NO_MORE_DOCS {
        unreachable!("r1 is not empty but r2 is");
      }
    }
    return Ok(());
  }
  let terms2 = terms2.unwrap();
  let mut terms_enum2 = terms2.iterator()?;
  let mut stored_fields1 = r1.stored_fields()?;
  let mut stored_fields2 = r2.stored_fields()?;
  let mut term_vectors1 = r1.term_vectors()?;
  let mut term_vectors2 = r2.term_vectors()?;
  let mut term_docs1 = None;
  let mut term_docs2 = None;

  while let Some(term) = terms_enum.next()?.map(|term| term.into_owned()) {
    term_docs1 = Some(TestUtil::docs(
      &mut *random,
      &mut terms_enum,
      term_docs1,
      NONE as i32,
    )?);
    term_docs2 = if terms_enum2.seek_exact(&term)? {
      Some(TestUtil::docs(
        &mut *random,
        &mut terms_enum2,
        term_docs2,
        NONE as i32,
      )?)
    } else {
      None
    };

    if next_non_deleted_doc(term_docs1.as_mut().unwrap(), live_docs1.as_ref())? == NO_MORE_DOCS {
      assert!(
        term_docs2.is_none()
          || next_non_deleted_doc(term_docs2.as_mut().unwrap(), live_docs2.as_ref())?
            == NO_MORE_DOCS
      );
      continue;
    }

    let id1 = term_docs1.as_ref().unwrap().doc_id();
    assert_eq!(
      NO_MORE_DOCS,
      next_non_deleted_doc(term_docs1.as_mut().unwrap(), live_docs1.as_ref())?
    );

    let term_docs2 = term_docs2.as_mut().expect("term must exist in r2");
    assert_ne!(
      NO_MORE_DOCS,
      next_non_deleted_doc(term_docs2, live_docs2.as_ref())?
    );
    let id2 = term_docs2.doc_id();
    assert_eq!(
      NO_MORE_DOCS,
      next_non_deleted_doc(term_docs2, live_docs2.as_ref())?
    );

    r2r1[id2 as usize] = id1;
    verify_equals_document(
      &stored_fields1.document(id1)?,
      &stored_fields2.document(id2)?,
    )?;
    verify_equals_fields(
      &mut *random,
      term_vectors1.get(id1)?,
      term_vectors2.get(id2)?,
    )?;
  }

  let mut fields1_enum = indexed_fields1.into_iter();
  let mut indexed_fields2: Vec<_> = field_infos::get_indexed_fields(r2)?.into_iter().collect();
  indexed_fields2.sort();
  let mut fields2_enum = indexed_fields2.into_iter();
  let mut field1 = None;
  let mut field2 = None;
  let mut terms_enum1: Option<<TermsType<&StandardDirectoryReader<D>> as Terms>::TermsEnum> = None;
  let mut terms_enum2: Option<<TermsType<&StandardDirectoryReader<D>> as Terms>::TermsEnum> = None;
  let mut docs1 = None;
  let mut docs2 = None;
  loop {
    let mut term1 = None;
    let mut term2 = None;

    let mut info1 = Vec::new();
    loop {
      info1.clear();
      if terms_enum1.is_none() {
        let Some(next_field1) = fields1_enum.next() else {
          break;
        };
        field1 = Some(next_field1);
        let terms = multi_terms::get_terms(r1, field1.as_ref().unwrap())?;
        if terms.is_none() {
          continue;
        }
        terms_enum1 = Some(terms.unwrap().iterator()?);
      }
      let terms_enum1_ref = terms_enum1.as_mut().unwrap();
      term1 = terms_enum1_ref.next()?.map(|term| term.into_owned());
      if term1.is_none() {
        terms_enum1 = None;
        continue;
      }

      docs1 = Some(TestUtil::docs(
        &mut *random,
        terms_enum1.as_mut().unwrap(),
        docs1,
        FREQS as i32,
      )?);
      let docs1_ref = docs1.as_mut().unwrap();
      while docs1_ref.next_doc()? != NO_MORE_DOCS {
        let d = docs1_ref.doc_id();
        if live_docs1
          .as_ref()
          .is_some_and(|live_docs| !live_docs.get(d as usize).unwrap())
        {
          continue;
        }
        let f = docs1_ref.freq()?;
        info1.push(((d as i64) << 32) | i64::from(f));
      }
      if !info1.is_empty() {
        break;
      }
    }

    let mut info2 = Vec::new();
    loop {
      info2.clear();
      if terms_enum2.is_none() {
        let Some(next_field2) = fields2_enum.next() else {
          break;
        };
        field2 = Some(next_field2);
        let terms = multi_terms::get_terms(r2, field2.as_ref().unwrap())?;
        if terms.is_none() {
          continue;
        }
        terms_enum2 = Some(terms.unwrap().iterator()?);
      }
      let terms_enum2_ref = terms_enum2.as_mut().unwrap();
      term2 = terms_enum2_ref.next()?.map(|term| term.into_owned());
      if term2.is_none() {
        terms_enum2 = None;
        continue;
      }

      docs2 = Some(TestUtil::docs(
        &mut *random,
        terms_enum2.as_mut().unwrap(),
        docs2,
        FREQS as i32,
      )?);
      let docs2_ref = docs2.as_mut().unwrap();
      while docs2_ref.next_doc()? != NO_MORE_DOCS {
        let doc_id = docs2_ref.doc_id();
        if live_docs2
          .as_ref()
          .is_some_and(|live_docs| !live_docs.get(doc_id as usize).unwrap())
        {
          continue;
        }
        let d = r2r1[doc_id as usize];
        let f = docs2_ref.freq()?;
        info2.push(((d as i64) << 32) | i64::from(f));
      }
      if !info2.is_empty() {
        break;
      }
    }

    assert_eq!(info1.len(), info2.len());
    if info1.is_empty() {
      break;
    }

    assert_eq!(field1, field2);
    assert_eq!(term1, term2);

    if !has_deletes {
      let doc_freq1 = terms_enum1.as_mut().unwrap().doc_freq()?;
      let doc_freq2 = terms_enum2.as_mut().unwrap().doc_freq()?;
      assert_eq!(doc_freq1, doc_freq2);
    }

    assert_eq!(term1, term2);
    info2.sort();
    for (left, right) in info1.iter().zip(info2.iter()) {
      assert_eq!(
        left,
        right,
        "field={} term={}",
        field1.as_ref().unwrap(),
        term1.as_ref().unwrap().utf8_to_string()?
      );
    }
  }
  Ok(())
}

fn verify_equals_document(d1: &Document, d2: &Document) -> Result<()> {
  let mut ff1: Vec<_> = d1.get_fields().iter().collect();
  let mut ff2: Vec<_> = d2.get_fields().iter().collect();

  ff1.sort_by(|a, b| a.name().cmp(b.name()));
  ff2.sort_by(|a, b| a.name().cmp(b.name()));

  assert_eq!(ff1.len(), ff2.len(), "{} : {}", d1, d2);
  for (f1, f2) in ff1.iter().zip(ff2.iter()) {
    if f1.binary_value()?.is_some() {
      assert!(f2.binary_value()?.is_some());
    } else {
      assert_eq!(
        f1.string_value()?.map(|value| value.into_owned()),
        f2.string_value()?.map(|value| value.into_owned()),
        "{} : {}",
        d1,
        d2
      );
    }
  }
  Ok(())
}

fn verify_equals_fields<F1, F2, R>(random: &mut R, d1: Option<F1>, d2: Option<F2>) -> Result<()>
where
  F1: IndexFields,
  F2: IndexFields,
  R: rand::Rng + ?Sized,
{
  if d1.is_none() {
    assert!(d2.is_none_or(|fields| fields.size().unwrap() == 0));
    return Ok(());
  }
  let d1 = d1.unwrap();
  let d2 = d2.expect("term vectors must exist in d2");

  let mut fields_enum2 = d2.iterator()?;
  let mut fields_enum1 = d1.iterator()?;
  let mut d_enum1 = None;
  let mut d_enum2 = None;
  while fields_enum1.has_next()? {
    let field1 = fields_enum1
      .next()?
      .ok_or_else(|| LuceneError::illegal_state("Fields.iterator().has_next returned true"))?;
    let field2 = fields_enum2.next()?.expect("field missing in d2");
    assert_eq!(field1, field2);

    let terms1 = d1.terms(field1)?.expect("terms missing in d1");
    let mut terms_enum1 = terms1.iterator()?;

    let terms2 = d2.terms(field2)?.expect("terms missing in d2");
    let mut terms_enum2 = terms2.iterator()?;

    while let Some(term1) = terms_enum1.next()?.map(|term| term.into_owned()) {
      let term2 = terms_enum2
        .next()?
        .expect("term missing in d2")
        .into_owned();
      assert_eq!(term1, term2);
      assert_eq!(
        terms_enum1.total_term_freq()?,
        terms_enum2.total_term_freq()?
      );

      if terms1.has_positions() {
        assert!(terms2.has_positions());
        let mut dp_enum1 = terms_enum1.postings_with_flags(None, ALL as i32)?;
        let mut dp_enum2 = terms_enum2.postings_with_flags(None, ALL as i32)?;
        let doc_id1 = dp_enum1.next_doc()?;
        dp_enum2.next_doc()?;
        assert_ne!(NO_MORE_DOCS, doc_id1);

        let freq1 = dp_enum1.freq()?;
        let freq2 = dp_enum2.freq()?;
        assert_eq!(freq1, freq2);

        for _ in 0..freq1 {
          assert_eq!(dp_enum1.next_position()?, dp_enum2.next_position()?);
          if terms1.has_offsets() {
            assert!(terms2.has_offsets());
            assert_eq!(dp_enum1.start_offset()?, dp_enum2.start_offset()?);
            assert_eq!(dp_enum1.end_offset()?, dp_enum2.end_offset()?);
          }
        }
        assert_eq!(NO_MORE_DOCS, dp_enum1.next_doc()?);
        assert_eq!(NO_MORE_DOCS, dp_enum2.next_doc()?);
      } else {
        d_enum1 = Some(TestUtil::docs(
          &mut *random,
          &mut terms_enum1,
          d_enum1,
          FREQS as i32,
        )?);
        d_enum2 = Some(TestUtil::docs(
          &mut *random,
          &mut terms_enum2,
          d_enum2,
          FREQS as i32,
        )?);
        let d_enum1 = d_enum1.as_mut().unwrap();
        let d_enum2 = d_enum2.as_mut().unwrap();
        let doc_id1 = d_enum1.next_doc()?;
        d_enum2.next_doc()?;
        assert_ne!(NO_MORE_DOCS, doc_id1);
        let freq1 = d_enum1.freq()?;
        let freq2 = d_enum2.freq()?;
        assert_eq!(freq1, freq2);
        assert_eq!(NO_MORE_DOCS, d_enum1.next_doc()?);
        assert_eq!(NO_MORE_DOCS, d_enum2.next_doc()?);
      }
    }
    assert!(terms_enum2.next()?.is_none());
  }
  assert!(fields_enum2.next()?.is_none());
  Ok(())
}

struct IndexingThread {
  base: i32,
  range: i32,
  iterations: i32,
  same_field_order: bool,
  seed: i32,
  docs: HashMap<String, Document>,
  buffer: String,
}

impl IndexingThread {
  fn new(base: i32, range: i32, iterations: i32, same_field_order: bool, seed: i32) -> Self {
    Self {
      base,
      range,
      iterations,
      same_field_order,
      seed,
      docs: HashMap::new(),
      buffer: String::with_capacity(100),
    }
  }

  fn next_int<R>(r: &mut R, lim: i32) -> i32
  where
    R: rand::Rng + ?Sized,
  {
    r.random_range(0..lim)
  }

  fn next_int_range<R>(r: &mut R, start: i32, end: i32) -> i32
  where
    R: rand::Rng + ?Sized,
  {
    start + r.random_range(0..end - start)
  }

  fn add_utf8_token<R>(&mut self, r: &mut R)
  where
    R: rand::Rng + ?Sized,
  {
    let len = Self::next_int(r, 20);
    for _ in 0..len {
      let t = Self::next_int(r, 5);
      let codepoint = match t {
        0 => Self::next_int_range(r, 0x10000, 0x10ffff),
        1 => Self::next_int(r, 0x80),
        2 => Self::next_int_range(r, 0x80, 0x800),
        3 => Self::next_int_range(r, 0x800, 0xd800),
        _ => Self::next_int_range(r, 0xe000, 0xffff),
      } as u32;
      if let Some(ch) = char::from_u32(codepoint) {
        self.buffer.push(ch);
      }
    }
    self.buffer.push(' ');
  }

  fn get_string<R>(&mut self, r: &mut R, mut n_tokens: i32) -> String
  where
    R: rand::Rng + ?Sized,
  {
    if n_tokens == 0 {
      n_tokens = Self::next_int(r, 4) + 1;
    }

    if r.random_bool(0.5) {
      return self.get_utf8_string(r, n_tokens);
    }

    let mut s = String::with_capacity((n_tokens * 2) as usize);
    for _ in 0..n_tokens {
      s.push((b'A' + Self::next_int(r, 10) as u8) as char);
      s.push(' ');
    }
    s
  }

  fn get_utf8_string<R>(&mut self, r: &mut R, n_tokens: i32) -> String
  where
    R: rand::Rng + ?Sized,
  {
    self.buffer.clear();
    for _ in 0..n_tokens {
      self.add_utf8_token(r);
    }
    self.buffer.clone()
  }

  fn get_id_string<R>(&self, r: &mut R) -> String
  where
    R: rand::Rng + ?Sized,
  {
    (self.base + Self::next_int(r, self.range)).to_string()
  }

  fn index_doc<R>(
    &mut self,
    r: &mut R,
    w: &IndexWriter<DirEnum>,
    field_types: &Mutex<HashMap<String, FieldType>>,
  ) -> Result<()>
  where
    R: rand::Rng + ?Sized,
  {
    let mut d = Document::new();

    let mut custom_type1 = FieldType::from_ref(&*text_field::TYPE_STORED)?;
    custom_type1.set_tokenized(false)?;
    custom_type1.set_omit_norms(true)?;

    let mut fields = Vec::new();
    let id_string = self.get_id_string(r);
    fields.push(Field::from_string("id", id_string.clone(), custom_type1)?.into());

    let n_fields = Self::next_int(r, MAX_FIELDS);
    for _ in 0..n_fields {
      let field_name = format!("f{}", Self::next_int(r, 100));
      let field_type = {
        let mut field_types = field_types.lock().unwrap();
        if let Some(field_type) = field_types.get(&field_name) {
          field_type.clone()
        } else {
          let mut ft = FieldType::new();
          match Self::next_int(r, 4) {
            0 => {},
            1 => ft.set_store_term_vectors(true)?,
            2 => {
              ft.set_store_term_vectors(true)?;
              ft.set_store_term_vector_positions(true)?;
            },
            _ => {
              ft.set_store_term_vectors(true)?;
              ft.set_store_term_vector_offsets(true)?;
            },
          }
          match Self::next_int(r, 4) {
            0 => {
              ft.set_stored(true)?;
              ft.set_omit_norms(true)?;
              ft.set_index_options(IndexOptions::DocsAndFreqsAndPositions)?;
            },
            1 => {
              ft.set_index_options(IndexOptions::DocsAndFreqsAndPositions)?;
              ft.set_tokenized(true)?;
            },
            2 => {
              ft.set_stored(true)?;
              ft.set_store_term_vectors(false)?;
              ft.set_store_term_vector_offsets(false)?;
              ft.set_store_term_vector_positions(false)?;
            },
            _ => {
              ft.set_stored(true)?;
              ft.set_index_options(IndexOptions::DocsAndFreqsAndPositions)?;
              ft.set_tokenized(true)?;
            },
          }
          ft.freeze();
          field_types.insert(field_name.clone(), ft.clone());
          ft
        }
      };
      let mut n_tokens = Self::next_int(r, 3);
      n_tokens = if n_tokens < 2 {
        n_tokens
      } else {
        BIG_FIELD_SIZE
      };
      let value = self.get_string(r, n_tokens);
      fields.push(Field::from_string(field_name, value, field_type)?.into());
    }

    if self.same_field_order {
      fields.sort_by(|a: &DocumentFields, b| a.name().cmp(b.name()));
    } else {
      let pos = Self::next_int(r, fields.len() as i32) as usize;
      fields.swap(pos, 0);
    }

    for field in fields {
      d.add(field);
    }
    w.update_document_with_term(Some(Term::from_text("id", id_string.clone())), d.clone())?;
    self.docs.insert(id_string, d);
    Ok(())
  }

  fn delete_doc<R>(&mut self, r: &mut R, w: &IndexWriter<DirEnum>) -> Result<()>
  where
    R: rand::Rng + ?Sized,
  {
    let id_string = self.get_id_string(r);
    w.delete_documents_with_terms(vec![Term::from_text("id", id_string.clone())])?;
    self.docs.remove(&id_string);
    Ok(())
  }

  fn delete_by_query<R>(&mut self, r: &mut R, w: &IndexWriter<DirEnum>) -> Result<()>
  where
    R: rand::Rng + ?Sized,
  {
    let id_string = self.get_id_string(r);
    w.delete_documents_with_queries(vec![
      TermQuery::new(Term::from_text("id", id_string.clone())).into(),
    ])?;
    self.docs.remove(&id_string);
    Ok(())
  }

  fn run(
    mut self,
    w: &IndexWriter<DirEnum>,
    field_types: &Mutex<HashMap<String, FieldType>>,
  ) -> Result<HashMap<String, Document>> {
    let mut r = random_from_seed((self.base + self.range + self.seed) as u64);
    for _ in 0..self.iterations {
      let what = Self::next_int(&mut r, 100);
      if what < 5 {
        self.delete_doc(&mut r, w)?;
      } else if what < 10 {
        self.delete_by_query(&mut r, w)?;
      } else {
        self.index_doc(&mut r, w, field_types)?;
      }
    }
    Ok(self.docs)
  }
}
