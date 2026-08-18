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
use crate::core::document::field::Store;
use crate::core::document::field_type::FieldType;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::parallel_leaf_reader::ParallelLeafReader;
use crate::core::index::postings_enum::NONE;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::terms::Terms;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::util::lucene_test_case::{
  get_only_leaf_reader, is_light_mode, new_directory_shared, new_index_writer_config_with_analyzer,
  new_text_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

#[allow(dead_code)] // for quick search
struct TestParallelTermEnum;

type TestLeafReader = DefaultLeafReader<DirEnum>;

static LIGHT_DIRS: LazyLock<(Arc<DirEnum>, Arc<DirEnum>)> = LazyLock::new(|| {
  let mut random = random();
  build_set_up_dirs(&mut random).expect("failed to initialize TestParallelTermEnum")
});

fn set_up<R>(random: &mut R) -> Result<(TestLeafReader, TestLeafReader, Arc<DirEnum>, Arc<DirEnum>)>
where
  R: Rng + ?Sized,
{
  let (rd1, rd2) = if is_light_mode() {
    (LIGHT_DIRS.0.clone(), LIGHT_DIRS.1.clone())
  } else {
    build_set_up_dirs(random)?
  };
  let ir1 = get_only_leaf_reader(directory_reader::open(rd1.clone())?)?;
  let ir2 = get_only_leaf_reader(directory_reader::open(rd2.clone())?)?;
  Ok((ir1, ir2, rd1, rd2))
}

fn build_set_up_dirs<R>(random: &mut R) -> Result<(Arc<DirEnum>, Arc<DirEnum>)>
where
  R: Rng + ?Sized,
{
  let rd1 = new_directory_shared(random)?;
  let analyzer = MockAnalyzer::new(random);
  let iw1 = IndexWriter::new(
    rd1.clone(),
    new_index_writer_config_with_analyzer(random, analyzer)?,
  )?;

  let mut field_types = HashMap::<String, FieldType>::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "field1",
    "the quick brown fox jumps",
    Store::Yes,
    &mut field_types,
  )?);
  doc.add(new_text_field(
    random,
    "field2",
    "the quick brown fox jumps",
    Store::Yes,
    &mut field_types,
  )?);
  iw1.add_document(doc)?;
  iw1.close()?;

  let rd2 = new_directory_shared(random)?;
  let analyzer = MockAnalyzer::new(random);
  let iw2 = IndexWriter::new(
    rd2.clone(),
    new_index_writer_config_with_analyzer(random, analyzer)?,
  )?;

  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "field1",
    "the fox jumps over the lazy dog",
    Store::Yes,
    &mut field_types,
  )?);
  doc.add(new_text_field(
    random,
    "field3",
    "the fox jumps over the lazy dog",
    Store::Yes,
    &mut field_types,
  )?);
  iw2.add_document(doc)?;
  iw2.close()?;

  Ok((rd1, rd2))
}

fn check_terms<T, R>(random: &mut R, terms: Option<T>, terms_list: &[&str]) -> Result<()>
where
  T: Terms,
  R: Rng + ?Sized,
{
  let terms = terms.expect("terms");
  let mut terms_enum = terms.iterator()?;

  for expected in terms_list {
    let term = terms_enum.next()?.expect("term");
    assert_eq!(*expected, term.utf8_to_string()?);
    let mut postings = TestUtil::docs(random, &mut terms_enum, None, NONE as i32)?;
    assert_ne!(NO_MORE_DOCS, postings.next_doc()?);
    assert_eq!(0, postings.doc_id());
    assert_eq!(NO_MORE_DOCS, postings.next_doc()?);
  }
  assert!(terms_enum.next()?.is_none());
  Ok(())
}

#[test]
fn test1() -> Result<()> {
  let mut random = random();
  let (ir1, ir2, rd1, rd2) = set_up(&mut random)?;
  let pr = ParallelLeafReader::new(vec![ir1.clone(), ir2.clone()])?;

  assert_eq!(3, pr.get_field_infos()?.size());

  check_terms(
    &mut random,
    pr.terms("field1")?,
    &["brown", "fox", "jumps", "quick", "the"],
  )?;
  check_terms(
    &mut random,
    pr.terms("field2")?,
    &["brown", "fox", "jumps", "quick", "the"],
  )?;
  check_terms(
    &mut random,
    pr.terms("field3")?,
    &["dog", "fox", "jumps", "lazy", "over", "the"],
  )?;

  ir1.close()?;
  ir2.close()?;
  rd1.close()?;
  rd2.close()
}
