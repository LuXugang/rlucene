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
use crate::core::document::field_type::FieldType;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::index::multi_terms::get_term_postings_enum;
use crate::core::index::postings_enum::{FREQS, PostingsEnum};
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, new_field, new_index_writer_config_with_analyzer,
  new_log_merge_policy_with_merge_factor, random,
};
use crate::test::support::core::util::test_util::TestUtil;
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
pub struct TestOmitPositions;

#[test]
fn test_basic() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut field_to_type = HashMap::new();

  let mut doc = Document::new();

  let mut ft = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  ft.set_index_options(IndexOptions::DocsAndFreqs)?;

  let f = new_field(
    &mut random,
    "foo",
    "this is a test test",
    &ft,
    &mut field_to_type,
  )?;
  doc.add(f);

  for _ in 0..100 {
    w.add_document(&mut random, doc.clone())?;
  }

  let reader = w.get_reader(&mut random)?;
  w.close(&mut random)?;

  assert!(get_term_postings_enum(&reader, "foo", &BytesRef::from_string("test"),)?.is_some());

  let mut de = TestUtil::docs_with_reader(
    &mut random,
    &reader,
    "foo",
    &BytesRef::from_string("test"),
    None,
    FREQS as i32,
  )?
  .unwrap();

  while de.next_doc()? != NO_MORE_DOCS {
    assert_eq!(2, de.freq()?);
  }
  Ok(())
}
#[test]
fn test_positions() -> Result<()> {
  let mut random = random();
  let ram = new_directory_shared(&mut random)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let writer = IndexWriter::new(ram.clone(), iwc)?;

  let mut field_to_type = HashMap::new();

  let mut d = Document::new();

  let mut ft = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  ft.set_index_options(IndexOptions::Docs)?;
  let f1 = new_field(
    &mut random,
    "f1",
    "This field has docs only",
    &ft,
    &mut field_to_type,
  )?;
  d.add(f1);

  let mut ft2 = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  ft2.set_index_options(IndexOptions::DocsAndFreqs)?;
  let f2 = new_field(
    &mut random,
    "f2",
    "This field has docs and freqs",
    &ft2,
    &mut field_to_type,
  )?;
  d.add(f2);

  let mut ft3 = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  ft3.set_index_options(IndexOptions::DocsAndFreqsAndPositions)?;
  let f3 = new_field(
    &mut random,
    "f3",
    "This field has docs and freqs and positions",
    &ft3,
    &mut field_to_type,
  )?;
  d.add(f3);

  writer.add_document(d)?;
  writer.force_merge(1)?;
  writer.close()?;

  let reader = directory_reader::open(ram.clone())?;
  let leaf = get_only_leaf_reader(&reader)?;

  let fi = leaf.get_field_infos()?;

  assert_eq!(
    IndexOptions::Docs,
    *fi.field_info_by_name("f1").unwrap().get_index_options()
  );
  assert_eq!(
    IndexOptions::DocsAndFreqs,
    *fi.field_info_by_name("f2").unwrap().get_index_options()
  );
  assert_eq!(
    IndexOptions::DocsAndFreqsAndPositions,
    *fi.field_info_by_name("f3").unwrap().get_index_options()
  );

  Ok(())
}
fn assert_no_prx(dir: &DirEnum) -> Result<()> {
  let files = dir.list_all()?;
  for file in files {
    assert!(!file.ends_with(".prx"));
    assert!(!file.ends_with(".pos"));
  }
  Ok(())
}
#[test]
fn test_no_prx_file() -> Result<()> {
  let mut random = random();
  let ram = new_directory_shared(&mut random)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwc.set_max_buffered_docs(3);
  let mut log_merge_policy = new_log_merge_policy_with_merge_factor(&mut random, 2)?;
  log_merge_policy.get_base_mut().set_no_cfs_ratio(0.0)?;
  iwc.set_merge_policy(log_merge_policy);

  let writer = IndexWriter::new(ram.clone(), iwc)?;

  let mut field_to_type = HashMap::new();

  let mut d = Document::new();

  let mut ft = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  ft.set_index_options(IndexOptions::DocsAndFreqs)?;

  let f1 = new_field(
    &mut random,
    "f1",
    "This field has term freqs",
    &ft,
    &mut field_to_type,
  )?;

  d.add(f1);

  for _ in 0..30 {
    writer.add_document(d.clone())?;
  }

  writer.commit()?;

  assert_no_prx(&ram)?;

  writer.close()?;
  Ok(())
}
