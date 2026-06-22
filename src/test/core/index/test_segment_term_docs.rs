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
use crate::core::document::field::Store::No;
use crate::core::document::field_type::FieldType;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::postings_enum::{FREQS, PostingsEnum};
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_reader::SegmentReader;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::LATEST;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::doc_helper::{DocHelper, TEXT_FIELD_2_KEY};
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_io_context, new_text_field,
  random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestSegmentTermDocs;

fn set_up<R>(random: &mut R) -> Result<(Arc<DirEnum>, Document, SegmentCommitInfo<DirEnum>)>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let mut document = Document::new();
  DocHelper::setup_doc(&mut document);
  let info = DocHelper::write_doc(random, dir.clone(), document.clone())?;
  Ok((dir, document, info))
}
#[test]
fn test_term_docs() -> Result<()> {
  let mut random = random();
  let (_dir, _doc, info) = set_up(&mut random)?;

  let reader = SegmentReader::new(&info, LATEST.major, &new_io_context(&mut random)?)?;
  assert!(reader.max_doc()? >= 0);

  let terms = reader.terms(TEXT_FIELD_2_KEY)?.expect("terms should exist");
  let mut terms_enum = terms.iterator()?;
  terms_enum.seek_ceil(&BytesRef::from_string("field"))?;

  let mut term_docs = TestUtil::docs(&mut random, &mut terms_enum, None, FREQS as i32)?;

  if term_docs.next_doc()? != NO_MORE_DOCS {
    let doc_id = term_docs.doc_id();
    assert_eq!(doc_id, 0);
    let freq = term_docs.freq()?;
    assert_eq!(freq, 3);
  }

  reader.close()?;
  Ok(())
}
#[test]
fn test_bad_seek() -> Result<()> {
  let mut random = random();
  let (_dir, _doc, info) = set_up(&mut random)?;

  {
    let reader = Arc::new(SegmentReader::new(
      &info,
      LATEST.major,
      &new_io_context(&mut random)?,
    )?);
    assert!(reader.max_doc()? >= 0);
    let multi_readers = MultiReader::with_leaf_reader(vec![reader.clone()])?;

    let term_docs = TestUtil::docs_with_reader(
      &mut random,
      &multi_readers,
      "textField2",
      &BytesRef::from_string("bad"),
      None,
      0,
    )?;
    assert!(term_docs.is_none());

    reader.close()?;
  }

  {
    let reader = Arc::new(SegmentReader::new(
      &info,
      LATEST.major,
      &new_io_context(&mut random)?,
    )?);
    assert!(reader.max_doc()? >= 0);
    let multi_readers = MultiReader::with_leaf_reader(vec![reader.clone()])?;
    let term_docs = TestUtil::docs_with_reader(
      &mut random,
      &multi_readers,
      "junk",
      &BytesRef::from_string("bad"),
      None,
      0,
    )?;
    assert!(term_docs.is_none());

    reader.close()?;
  }

  Ok(())
}
#[test]
fn test_skip_to() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();
  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let mut writer = IndexWriter::new(dir.clone(), iwc)?;

  let ta = Term::from_text("content", "aaa");
  for _ in 0..10 {
    add_doc(
      &mut random,
      &mut writer,
      "aaa aaa aaa aaa",
      &mut field_types,
    )?;
  }

  let tb = Term::from_text("content", "bbb");
  for _ in 0..16 {
    add_doc(
      &mut random,
      &mut writer,
      "bbb bbb bbb bbb",
      &mut field_types,
    )?;
  }

  let tc = Term::from_text("content", "ccc");
  for _ in 0..50 {
    add_doc(
      &mut random,
      &mut writer,
      "ccc ccc ccc ccc",
      &mut field_types,
    )?;
  }
  writer.force_merge(1)?;
  writer.close()?;

  let reader = Arc::new(directory_reader::open(dir.clone())?);
  let mut tdocs = TestUtil::docs_with_reader(
    &mut random,
    &reader,
    ta.field(),
    &BytesRef::from_string(&ta.text()?),
    None,
    FREQS as i32,
  )?
  .expect("tdocs should exist");

  assert_ne!(tdocs.next_doc()?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 0);
  assert_eq!(tdocs.freq()?, 4);

  assert_ne!(tdocs.next_doc()?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 1);
  assert_eq!(tdocs.freq()?, 4);

  assert_ne!(tdocs.advance(2)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 2);

  assert_ne!(tdocs.advance(4)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 4);

  assert_ne!(tdocs.advance(9)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 9);

  assert_eq!(tdocs.advance(10)?, NO_MORE_DOCS);

  let mut tdocs = TestUtil::docs_with_reader(
    &mut random,
    &reader,
    ta.field(),
    &BytesRef::from_string(&ta.text()?),
    None,
    0,
  )?
  .expect("tdocs should exist");

  assert_ne!(tdocs.advance(0)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 0);

  assert_ne!(tdocs.advance(4)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 4);

  assert_ne!(tdocs.advance(9)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 9);

  assert_eq!(tdocs.advance(10)?, NO_MORE_DOCS);

  // bbb ----------------------------------------------------------

  let mut tdocs = TestUtil::docs_with_reader(
    &mut random,
    &reader,
    tb.field(),
    &BytesRef::from_string(&tb.text()?),
    None,
    FREQS as i32,
  )?
  .expect("tdocs should exist");

  assert_ne!(tdocs.next_doc()?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 10);
  assert_eq!(tdocs.freq()?, 4);

  assert_ne!(tdocs.next_doc()?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 11);
  assert_eq!(tdocs.freq()?, 4);

  assert_ne!(tdocs.advance(12)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 12);

  assert_ne!(tdocs.advance(15)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 15);

  assert_ne!(tdocs.advance(24)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 24);

  assert_ne!(tdocs.advance(25)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 25);

  assert_eq!(tdocs.advance(26)?, NO_MORE_DOCS);

  // without next
  let mut tdocs = TestUtil::docs_with_reader(
    &mut random,
    &reader,
    tb.field(),
    &BytesRef::from_string(&tb.text()?),
    None,
    FREQS as i32,
  )?
  .expect("tdocs should exist");

  assert_ne!(tdocs.advance(5)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 10);

  assert_ne!(tdocs.advance(15)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 15);

  assert_ne!(tdocs.advance(24)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 24);

  assert_ne!(tdocs.advance(25)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 25);

  assert_eq!(tdocs.advance(26)?, NO_MORE_DOCS);

  // ccc ----------------------------------------------------------

  let mut tdocs = TestUtil::docs_with_reader(
    &mut random,
    &reader,
    tc.field(),
    &BytesRef::from_string(&tc.text()?),
    None,
    FREQS as i32,
  )?
  .expect("tdocs should exist");

  assert_ne!(tdocs.next_doc()?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 26);
  assert_eq!(tdocs.freq()?, 4);

  assert_ne!(tdocs.next_doc()?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 27);
  assert_eq!(tdocs.freq()?, 4);

  assert_ne!(tdocs.advance(28)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 28);

  assert_ne!(tdocs.advance(40)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 40);

  assert_ne!(tdocs.advance(57)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 57);

  assert_ne!(tdocs.advance(74)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 74);

  assert_ne!(tdocs.advance(75)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 75);

  assert_eq!(tdocs.advance(76)?, NO_MORE_DOCS);

  // without next
  let mut tdocs = TestUtil::docs_with_reader(
    &mut random,
    &reader,
    tc.field(),
    &BytesRef::from_string(&tc.text()?),
    None,
    0,
  )?
  .expect("tdocs should exist");

  assert_ne!(tdocs.advance(5)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 26);

  assert_ne!(tdocs.advance(40)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 40);

  assert_ne!(tdocs.advance(57)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 57);

  assert_ne!(tdocs.advance(74)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 74);

  assert_ne!(tdocs.advance(75)?, NO_MORE_DOCS);
  assert_eq!(tdocs.doc_id(), 75);

  assert_eq!(tdocs.advance(76)?, NO_MORE_DOCS);
  reader.close()?;
  Ok(())
}
fn add_doc<D, R>(
  random: &mut R,
  writer: &mut IndexWriter<D>,
  value: &str,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory,
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(new_text_field(random, "content", value, No, field_types)?);
  writer.add_document(doc)?;
  Ok(())
}
