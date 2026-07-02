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
use crate::core::document::field::{FieldBase, Store};
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::TextField;
use crate::core::index::BytesRef;
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::multi_terms;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::util::access::SharedAccessVec;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::analysis::mock_tokenizer::WHITESPACE;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, random,
};
use rand::{Rng, RngExt};
use std::collections::HashSet;
#[allow(dead_code)] // for quick search
pub struct TestIndexWriterUnicode;

const UTF8_DATA: &[(&str, &str)] = &[
  ("ab\u{fffd}cd", "ab\u{fffd}cd"),
  ("\u{fffd}abcd", "\u{fffd}abcd"),
  ("\u{fffd}", "\u{fffd}"),
  ("ab\u{fffd}\u{fffd}cd", "ab\u{fffd}\u{fffd}cd"),
  ("\u{fffd}\u{fffd}abcd", "\u{fffd}\u{fffd}abcd"),
  ("\u{fffd}\u{fffd}", "\u{fffd}\u{fffd}"),
  ("ab\u{fffd}cd", "ab\u{fffd}cd"),
  ("\u{fffd}abcd", "\u{fffd}abcd"),
  ("\u{fffd}", "\u{fffd}"),
  ("ab\u{fffd}\u{fffd}cd", "ab\u{fffd}\u{fffd}cd"),
  ("\u{fffd}\u{fffd}abcd", "\u{fffd}\u{fffd}abcd"),
  ("\u{fffd}\u{fffd}", "\u{fffd}\u{fffd}"),
  ("ab\u{fffd}\u{fffd}cd", "ab\u{fffd}\u{fffd}cd"),
  ("\u{fffd}\u{fffd}abcd", "\u{fffd}\u{fffd}abcd"),
  ("\u{fffd}\u{fffd}", "\u{fffd}\u{fffd}"),
  (
    "ab\u{fffd}\u{10517}\u{fffd}cd",
    "ab\u{fffd}\u{10517}\u{fffd}cd",
  ),
  (
    "\u{fffd}\u{10517}\u{fffd}abcd",
    "\u{fffd}\u{10517}\u{fffd}abcd",
  ),
  ("\u{fffd}\u{10517}\u{fffd}", "\u{fffd}\u{10517}\u{fffd}"),
];

fn next_int<R>(random: &mut R, lim: i32) -> i32
where
  R: Rng + ?Sized,
{
  random.random_range(0..lim)
}

fn next_int_between<R>(random: &mut R, start: i32, end: i32) -> i32
where
  R: Rng + ?Sized,
{
  start + next_int(random, end - start)
}

fn fill_unicode<R>(
  random: &mut R,
  buffer: &mut [char],
  expected: &mut [char],
  offset: usize,
  count: usize,
) -> bool
where
  R: Rng + ?Sized,
{
  let len = offset + count;
  let has_illegal = false;

  for i in offset..len {
    let t = next_int(random, 5);
    let ch = if t <= 1 {
      char::from_u32(next_int(random, 0x80) as u32).unwrap()
    } else if t == 2 {
      char::from_u32(next_int_between(random, 0x80, 0x800) as u32).unwrap()
    } else if t == 3 {
      char::from_u32(next_int_between(random, 0x800, 0xd800) as u32).unwrap()
    } else {
      char::from_u32(next_int_between(random, 0xe000, 0x10000) as u32).unwrap()
    };
    buffer[i] = ch;
    expected[i] = ch;
  }

  has_illegal
}

fn get_int<R>(random: &mut R, start: i32, end: i32) -> i32
where
  R: Rng + ?Sized,
{
  start + random.random_range(0..=end - start)
}

fn as_unicode_char(c: char) -> String {
  format!("U+{:x}", c as u32)
}

fn term_desc(s: &str) -> String {
  let mut chars = s.chars();
  let first = chars.next().expect("term should not be empty");
  match chars.next() {
    Some(second) => format!("{},{}", as_unicode_char(first), as_unicode_char(second)),
    None => as_unicode_char(first),
  }
}

fn check_terms_order<T>(terms: &T, all_terms: &HashSet<String>, is_top: bool) -> Result<()>
where
  T: Terms,
{
  let mut terms_enum = terms.iterator()?;
  let mut last = BytesRef::new();
  let mut seen_terms = HashSet::new();

  while let Some(term) = terms_enum.next()? {
    assert!(last < *term);
    last = BytesRef::deep_copy_of(&term);

    let s = term.utf8_to_string()?;
    assert!(
      all_terms.contains(&s),
      "term {} was not added to index (count={})",
      term_desc(&s),
      all_terms.len()
    );
    seen_terms.insert(s);
  }

  if is_top {
    assert_eq!(all_terms, &seen_terms);
  }

  for term in &seen_terms {
    let tr = BytesRef::from_string(term);
    assert_eq!(
      SeekStatus::Found,
      terms_enum.seek_ceil(&tr)?,
      "seek failed for term={}",
      term_desc(term)
    );
  }
  Ok(())
}

#[test]
fn test_random_unicode_strings() -> Result<()> {
  let mut random = random();
  let mut buffer = ['\0'; 20];
  let mut expected = ['\0'; 20];

  let num = at_least(&mut random, 10000);
  for _ in 0..num {
    let has_illegal = fill_unicode(&mut random, &mut buffer, &mut expected, 0, 20);
    let s: String = buffer.iter().collect();
    let utf8: BytesRef<Vec<u8>> = BytesRef::from_string(&s);
    if !has_illegal {
      let bytes = s.as_bytes();
      assert_eq!(bytes.len(), utf8.length);
      utf8.bytes.access(|utf8_bytes| {
        for i in 0..bytes.len() {
          assert_eq!(bytes[i], utf8_bytes[i]);
        }
      });
    }

    let utf16: String = utf8.utf8_to_string()?;
    assert_eq!(utf16.chars().count(), 20);
    for (actual, expected) in utf16.chars().zip(expected.iter()) {
      assert_eq!(actual, *expected);
    }
  }
  Ok(())
}

#[test]
fn test_all_unicode_chars() -> Result<()> {
  for ch in 0..=0x0010ffff {
    let Some(c) = char::from_u32(ch) else {
      continue;
    };

    let s1 = c.to_string();
    let utf8: BytesRef<Vec<u8>> = BytesRef::from_string(&s1);
    let s2 = utf8.utf8_to_string()?;
    assert_eq!(s1, s2, "codepoint {}", ch);

    let bytes = s1.as_bytes();
    assert_eq!(utf8.length, bytes.len());
    utf8.bytes.access(|utf8_bytes| {
      for j in 0..utf8.length {
        assert_eq!(utf8_bytes[j], bytes[j]);
      }
    });
  }
  Ok(())
}

#[test]
fn test_embedded_ffff() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let writer = crate::core::index::index_writer::IndexWriter::new(dir.clone(), config)?;

  let mut doc = Document::new();
  doc.add(TextField::from_string("field", "a a\u{ffff}b", Store::No)?);
  writer.add_document(doc)?;

  let mut doc = Document::new();
  doc.add(TextField::from_string("field", "a", Store::No)?);
  writer.add_document(doc)?;

  let reader = directory_reader::open_from_writer(&writer)?;
  assert_eq!(1, reader.doc_freq(&Term::from_text("field", "a\u{ffff}b"))?);
  reader.close()?;
  writer.close()?;
  Ok(())
}

#[test]
fn test_invalid_utf16() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::with_automaton(&mut random, WHITESPACE.clone(), false);
  let config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let writer = crate::core::index::index_writer::IndexWriter::new(dir.clone(), config)?;

  let mut doc = Document::new();
  for (i, (input, _expected)) in UTF8_DATA.iter().enumerate() {
    doc.add(TextField::from_string(
      format!("f{}", i),
      *input,
      Store::Yes,
    )?);
  }
  writer.add_document(doc)?;
  writer.close()?;

  let ir = directory_reader::open(dir)?;
  let mut stored_fields = ir.stored_fields()?;
  let doc2 = stored_fields.document(0)?;
  for (i, (_input, expected)) in UTF8_DATA.iter().enumerate() {
    assert_eq!(
      1,
      ir.doc_freq(&Term::from_text(format!("f{}", i), *expected))?,
      "field {} was not indexed correctly",
      i
    );
    assert_eq!(
      *expected,
      doc2
        .get_field(&format!("f{}", i))
        .expect("field should exist")
        .string_value()?
        .expect("field should be stored")
        .as_ref(),
      "field {} is incorrect",
      i
    );
  }
  ir.close()?;
  Ok(())
}

#[test]
fn test_term_utf16_sort_order() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir)?;

  let mut f = StringField::from_string("f", "", Store::No)?;
  let mut all_terms = HashSet::new();

  let num = at_least(&mut random, 200);
  for i in 0..num {
    let s = if random.random_bool(0.5) {
      if random.random_bool(0.5) {
        char::from_u32(get_int(&mut random, 0xe000, 0xffff) as u32)
          .unwrap()
          .to_string()
      } else {
        char::from_u32(get_int(&mut random, 0, 0xd7ff) as u32)
          .unwrap()
          .to_string()
      }
    } else {
      char::from_u32(get_int(&mut random, 0x10000, 0x10ffff) as u32)
        .unwrap()
        .to_string()
    };
    all_terms.insert(s.clone());
    f.set_string_value(s)?;

    let mut doc = Document::new();
    doc.add(f.clone());
    writer.add_document(&mut random, doc)?;

    if (1 + i) % 42 == 0 {
      writer.commit(&mut random)?;
    }
  }

  let r = writer.get_reader(&mut random)?;
  let top_reader_context = get_context(&r)?;
  for ctx in top_reader_context.leaves()? {
    if let Some(terms) = ctx.reader().terms("f")? {
      check_terms_order(&terms, &all_terms, false)?;
    }
  }
  let terms = multi_terms::get_terms(&r, "f")?.expect("terms should exist");
  check_terms_order(&terms, &all_terms, true)?;

  r.close()?;

  writer.force_merge(&mut random, 1)?;

  let r = writer.get_reader(&mut random)?;
  let terms = multi_terms::get_terms(&r, "f")?.expect("terms should exist");
  check_terms_order(&terms, &all_terms, true)?;
  r.close()?;

  writer.close(&mut random)?;
  Ok(())
}
