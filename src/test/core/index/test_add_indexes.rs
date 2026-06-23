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
use crate::core::document::int_point::IntPoint;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::{IndexWriterConfig, OpenMode};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::postings_enum::NONE;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::store::directory::{Directory, DirectoryEnum2};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_field, new_index_writer_config_with_analyzer,
  new_log_merge_policy_with_cfs, new_log_merge_policy_with_merge_factor,
  new_log_merge_policy_with_merge_factor_cfs, new_text_field, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestAddIndexes;

#[test]
fn test_simple_case() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let aux = new_directory_shared(&mut random)?;
  let aux2 = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Create);
  let mut writer = new_writer(dir.clone(), conf)?;
  add_docs(&mut random, &mut writer, 100, &mut field_types)?;
  assert_eq!(100, writer.get_doc_stats()?.max_doc);
  writer.close()?;
  drop(writer);
  TestUtil::check_index(dir.clone())?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Create);
  conf.set_merge_policy(new_log_merge_policy_with_cfs(&mut random, false)?);
  let mut writer = new_writer(aux.clone(), conf)?;
  add_docs(&mut random, &mut writer, 40, &mut field_types)?;
  assert_eq!(40, writer.get_doc_stats()?.max_doc);
  writer.close()?;
  drop(writer);

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Create);
  let mut writer = new_writer(aux2.clone(), conf)?;
  add_docs2(&mut random, &mut writer, 50, &mut field_types)?;
  assert_eq!(50, writer.get_doc_stats()?.max_doc);
  writer.close()?;
  drop(writer);

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Append);
  let writer = new_writer(dir.clone(), conf)?;
  assert_eq!(100, writer.get_doc_stats()?.max_doc);
  writer.add_indexes_from_dir(&[aux.clone(), aux2.clone()])?;
  assert_eq!(190, writer.get_doc_stats()?.max_doc);
  writer.close()?;
  TestUtil::check_index(dir.clone())?;
  drop(writer);

  verify_num_docs(aux.clone(), 40)?;
  verify_num_docs(dir.clone(), 190)?;

  let aux3 = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let mut writer = new_writer(aux3.clone(), conf)?;
  add_docs(&mut random, &mut writer, 40, &mut field_types)?;
  assert_eq!(40, writer.get_doc_stats()?.max_doc);
  writer.close()?;
  drop(writer);

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Append);
  let writer = new_writer(dir.clone(), conf)?;
  assert_eq!(190, writer.get_doc_stats()?.max_doc);
  writer.add_indexes_from_dir(std::slice::from_ref(&aux3))?;
  assert_eq!(230, writer.get_doc_stats()?.max_doc);
  writer.close()?;
  drop(writer);

  verify_num_docs(dir.clone(), 230)?;
  verify_term_docs(
    &mut random,
    dir.clone(),
    &Term::from_text("content", "aaa"),
    180,
  )?;
  verify_term_docs(
    &mut random,
    dir.clone(),
    &Term::from_text("content", "bbb"),
    50,
  )?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Append);
  let writer = new_writer(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;
  drop(writer);

  verify_num_docs(dir.clone(), 230)?;
  verify_term_docs(
    &mut random,
    dir.clone(),
    &Term::from_text("content", "aaa"),
    180,
  )?;
  verify_term_docs(
    &mut random,
    dir.clone(),
    &Term::from_text("content", "bbb"),
    50,
  )?;

  let aux4 = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let mut writer = new_writer(aux4.clone(), conf)?;
  add_docs2(&mut random, &mut writer, 1, &mut field_types)?;
  writer.close()?;
  drop(writer);

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Append);
  let writer = new_writer(dir.clone(), conf)?;
  assert_eq!(230, writer.get_doc_stats()?.max_doc);
  writer.add_indexes_from_dir(std::slice::from_ref(&aux4))?;
  assert_eq!(231, writer.get_doc_stats()?.max_doc);
  writer.close()?;
  drop(writer);

  verify_num_docs(dir.clone(), 231)?;
  verify_term_docs(
    &mut random,
    dir.clone(),
    &Term::from_text("content", "bbb"),
    51,
  )?;
  Ok(())
}
#[test]
fn test_with_pending_deletes() -> Result<()> {
  // main directory
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  // auxiliary directory
  let aux = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  set_up_dirs(&mut random, dir.clone(), aux.clone(), &mut field_types)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Append);
  let writer = new_writer(dir.clone(), conf)?;
  writer.add_indexes_from_dir(std::slice::from_ref(&aux))?;

  // Adds 10 docs, then replaces them with another 10
  // docs, so 10 pending deletes:
  for i in 0..20 {
    let mut doc = Document::new();
    doc.add(StringField::from_string(
      "id",
      (i % 10).to_string(),
      Store::No,
    )?);
    doc.add(new_text_field(
      &mut random,
      "content",
      format!("bbb {i}"),
      Store::No,
      &mut field_types,
    )?);
    doc.add(IntPoint::new("doc", [i])?);
    doc.add(IntPoint::new("doc2d", [i, i])?);
    doc.add(NumericDocValuesField::new("dv", i as i64));
    writer.update_document_with_term(Term::from_text("id", (i % 10).to_string()), doc)?;
  }
  // Deletes one of the 10 added docs, leaving 9:
  let q = PhraseQuery::from_terms_no_slop("content", &["bbb", "14"])?;
  writer.delete_documents_with_queries(vec![q.into()])?;

  writer.force_merge(1)?;
  writer.commit()?;

  verify_num_docs(dir.clone(), 1039)?;
  verify_term_docs(
    &mut random,
    dir.clone(),
    &Term::from_text("content", "aaa"),
    1030,
  )?;
  verify_term_docs(
    &mut random,
    dir.clone(),
    &Term::from_text("content", "bbb"),
    9,
  )?;

  writer.close()?;
  Ok(())
}
#[test]
fn test_with_pending_deletes2() -> Result<()> {
  // main directory
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  // auxiliary directory
  let aux = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  set_up_dirs(&mut random, dir.clone(), aux.clone(), &mut field_types)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Append);
  let writer = new_writer(dir.clone(), conf)?;

  // Adds 10 docs, then replaces them with another 10
  // docs, so 10 pending deletes:
  for i in 0..20 {
    let mut doc = Document::new();
    doc.add(StringField::from_string(
      "id",
      (i % 10).to_string(),
      Store::No,
    )?);
    doc.add(new_text_field(
      &mut random,
      "content",
      format!("bbb {i}"),
      Store::No,
      &mut field_types,
    )?);
    doc.add(IntPoint::new("doc", [i])?);
    doc.add(IntPoint::new("doc2d", [i, i])?);
    doc.add(NumericDocValuesField::new("dv", i as i64));
    writer.update_document_with_term(Term::from_text("id", (i % 10).to_string()), doc)?;
  }

  writer.add_indexes_from_dir(std::slice::from_ref(&aux))?;

  // Deletes one of the 10 added docs, leaving 9:
  let q = PhraseQuery::from_terms_no_slop("content", &["bbb", "14"])?;
  writer.delete_documents_with_queries(vec![q.into()])?;

  writer.force_merge(1)?;
  writer.commit()?;

  verify_num_docs(dir.clone(), 1039)?;
  verify_term_docs(
    &mut random,
    dir.clone(),
    &Term::from_text("content", "aaa"),
    1030,
  )?;
  verify_term_docs(
    &mut random,
    dir.clone(),
    &Term::from_text("content", "bbb"),
    9,
  )?;

  writer.close()?;
  Ok(())
}
#[test]
fn test_with_pending_deletes3() -> Result<()> {
  // main directory
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  // auxiliary directory
  let aux = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  set_up_dirs(&mut random, dir.clone(), aux.clone(), &mut field_types)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Append);
  let writer = new_writer(dir.clone(), conf)?;

  // Adds 10 docs, then replaces them with another 10
  // docs, so 10 pending deletes:
  for i in 0..20 {
    let mut doc = Document::new();
    doc.add(StringField::from_string(
      "id",
      (i % 10).to_string(),
      Store::No,
    )?);
    doc.add(new_text_field(
      &mut random,
      "content",
      format!("bbb {i}"),
      Store::No,
      &mut field_types,
    )?);
    doc.add(IntPoint::new("doc", [i])?);
    doc.add(IntPoint::new("doc2d", [i, i])?);
    doc.add(NumericDocValuesField::new("dv", i as i64));
    writer.update_document_with_term(Term::from_text("id", (i % 10).to_string()), doc)?;
  }

  // Deletes one of the 10 added docs, leaving 9:
  let q = PhraseQuery::from_terms_no_slop("content", &["bbb", "14"])?;
  writer.delete_documents_with_queries(vec![q.into()])?;

  writer.add_indexes_from_dir(std::slice::from_ref(&aux))?;

  writer.force_merge(1)?;
  writer.commit()?;

  verify_num_docs(dir.clone(), 1039)?;
  verify_term_docs(
    &mut random,
    dir.clone(),
    &Term::from_text("content", "aaa"),
    1030,
  )?;
  verify_term_docs(
    &mut random,
    dir.clone(),
    &Term::from_text("content", "bbb"),
    9,
  )?;

  writer.close()?;
  Ok(())
}
#[test]
fn test_add_self() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let aux = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let analyzer = MockAnalyzer::new(&mut random);
  let conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let mut writer = new_writer(dir.clone(), conf)?;
  add_docs(&mut random, &mut writer, 100, &mut field_types)?;
  assert_eq!(100, writer.get_doc_stats()?.max_doc);
  writer.close()?;
  drop(writer);

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Create);
  conf.set_max_buffered_docs(1000);
  conf.set_merge_policy(new_log_merge_policy_with_cfs(&mut random, false)?);
  let mut writer = new_writer(aux.clone(), conf)?;
  add_docs(&mut random, &mut writer, 40, &mut field_types)?;
  writer.close()?;
  drop(writer);

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Create);
  conf.set_max_buffered_docs(1000);
  conf.set_merge_policy(new_log_merge_policy_with_cfs(&mut random, false)?);
  let mut writer = new_writer(aux.clone(), conf)?;
  add_docs(&mut random, &mut writer, 100, &mut field_types)?;
  writer.close()?;
  drop(writer);

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Append);
  let writer2 = new_writer(dir.clone(), conf)?;

  let err = writer2.add_indexes_from_dir(&[aux.clone(), dir.clone()]);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));

  assert_eq!(100, writer2.get_doc_stats()?.max_doc);
  writer2.close()?;
  drop(writer2);

  verify_num_docs(dir.clone(), 100)?;
  Ok(())
}
// in all the remaining tests, make the doc count of the oldest segment
// in dir large so that it is never merged in addIndexes()
// case 1: no tail segments
#[test]
fn test_no_tail_segments() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let aux = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  set_up_dirs(&mut random, dir.clone(), aux.clone(), &mut field_types)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Append);
  conf.set_max_buffered_docs(10);
  conf.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 4)?);
  let mut writer = new_writer(dir.clone(), conf)?;
  add_docs(&mut random, &mut writer, 10, &mut field_types)?;

  writer.add_indexes_from_dir(std::slice::from_ref(&aux))?;
  assert_eq!(1040, writer.get_doc_stats()?.max_doc);
  assert_eq!(1000, writer.max_doc(0));
  writer.close()?;
  drop(writer);

  verify_num_docs(dir.clone(), 1040)?;
  Ok(())
}
#[test]
fn test_no_merge_after_copy() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let aux = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  set_up_dirs(&mut random, dir.clone(), aux.clone(), &mut field_types)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Append);
  conf.set_max_buffered_docs(10);
  conf.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 4)?);
  let writer_dir = Arc::new(DirectoryEnum2::A(dir.clone()));
  let writer = new_writer(writer_dir, conf)?;
  // TODO MockDirectoryWrapper 未实现
  let aux_copy = TestUtil::ram_copy_of(&mut random, aux.as_ref())?;
  writer.add_indexes_from_dir(&[
    Arc::new(DirectoryEnum2::A(aux.clone())),
    Arc::new(DirectoryEnum2::B(aux_copy)),
  ])?;
  assert_eq!(1060, writer.get_doc_stats()?.max_doc);
  assert_eq!(1000, writer.max_doc(0));
  writer.close()?;

  verify_num_docs(dir.clone(), 1060)?;
  Ok(())
}
#[test]
fn test_merge_after_copy() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let aux = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  set_up_dirs_with_id(
    &mut random,
    dir.clone(),
    aux.clone(),
    true,
    &mut field_types,
  )?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut dont_merge_config = new_index_writer_config_with_analyzer(&mut random, analyzer);
  dont_merge_config.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(aux.clone(), dont_merge_config)?;
  for i in 0..20 {
    writer.delete_documents_with_terms(vec![Term::from_text("id", i.to_string())])?;
  }
  writer.close()?;
  drop(writer);

  let reader = directory_reader::open(aux.clone())?;
  assert_eq!(10, reader.num_docs()?);
  reader.close()?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Append);
  conf.set_max_buffered_docs(4);
  conf.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 4)?);
  let writer_dir = Arc::new(DirectoryEnum2::A(dir.clone()));
  let writer = new_writer(writer_dir, conf)?;

  if cfg!(feature = "test_log_verbose") {
    println!("\nTEST: now addIndexes");
  }
  // TODO MockDirectoryWrapper 未实现
  let aux_copy = TestUtil::ram_copy_of(&mut random, aux.as_ref())?;
  writer.add_indexes_from_dir(&[
    Arc::new(DirectoryEnum2::A(aux.clone())),
    Arc::new(DirectoryEnum2::B(aux_copy)),
  ])?;
  assert_eq!(1020, writer.get_doc_stats()?.max_doc);
  assert_eq!(1000, writer.max_doc(0));
  writer.close()?;
  Ok(())
}

#[test]
fn test_more_merges() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let aux = new_directory_shared(&mut random)?;
  let aux2 = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  set_up_dirs_with_id(
    &mut random,
    dir.clone(),
    aux.clone(),
    true,
    &mut field_types,
  )?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Create);
  conf.set_max_buffered_docs(100);
  conf.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let writer = new_writer(aux2.clone(), conf)?;
  writer.add_indexes_from_dir(std::slice::from_ref(&aux))?;
  assert_eq!(30, writer.get_doc_stats()?.max_doc);
  assert_eq!(3, writer.get_segment_count());
  writer.close()?;
  drop(writer);

  let analyzer = MockAnalyzer::new(&mut random);
  let mut dont_merge_config = new_index_writer_config_with_analyzer(&mut random, analyzer);
  dont_merge_config.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(aux.clone(), dont_merge_config)?;
  for i in 0..27 {
    writer.delete_documents_with_terms(vec![Term::from_text("id", i.to_string())])?;
  }
  writer.close()?;
  drop(writer);

  let reader = directory_reader::open(aux.clone())?;
  assert_eq!(3, reader.num_docs()?);
  reader.close()?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut dont_merge_config = new_index_writer_config_with_analyzer(&mut random, analyzer);
  dont_merge_config.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(aux2.clone(), dont_merge_config)?;
  for i in 0..8 {
    writer.delete_documents_with_terms(vec![Term::from_text("id", i.to_string())])?;
  }
  writer.close()?;
  drop(writer);

  let reader = directory_reader::open(aux2.clone())?;
  assert_eq!(22, reader.num_docs()?);
  reader.close()?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
  conf.set_open_mode(OpenMode::Append);
  conf.set_max_buffered_docs(6);
  conf.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 4)?);
  let writer = new_writer(dir.clone(), conf)?;

  writer.add_indexes_from_dir(&[aux.clone(), aux2.clone()])?;
  assert_eq!(1040, writer.get_doc_stats()?.max_doc);
  assert_eq!(1000, writer.max_doc(0));
  writer.close()?;
  Ok(())
}
fn new_writer<D>(dir: Arc<D>, mut conf: IndexWriterConfig) -> Result<IndexWriter<D>>
where
  D: Directory + 'static,
{
  conf.set_merge_policy(LogMergePolicy::log_doc());
  IndexWriter::new(dir, conf)
}
fn add_docs<D, R>(
  random: &mut R,
  writer: &mut IndexWriter<D>,
  num_docs: i32,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  for i in 0..num_docs {
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      "content",
      "aaa",
      Store::No,
      field_types,
    )?);
    doc.add(IntPoint::new("doc", [i])?);
    doc.add(IntPoint::new("doc2d", [i, i])?);
    doc.add(NumericDocValuesField::new("dv", i as i64));
    writer.add_document(doc)?;
  }
  Ok(())
}

fn add_docs2<D, R>(
  random: &mut R,
  writer: &mut IndexWriter<D>,
  num_docs: i32,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  for i in 0..num_docs {
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      "content",
      "bbb",
      Store::No,
      field_types,
    )?);
    doc.add(IntPoint::new("doc", [i])?);
    doc.add(IntPoint::new("doc2d", [i, i])?);
    doc.add(NumericDocValuesField::new("dv", i as i64));
    writer.add_document(doc)?;
  }
  Ok(())
}

fn verify_num_docs<D>(dir: Arc<D>, num_docs: i32) -> Result<()>
where
  D: Directory + 'static,
{
  let reader = directory_reader::open(dir)?;
  assert_eq!(num_docs, reader.max_doc()?);
  assert_eq!(num_docs, reader.num_docs()?);
  reader.close()?;
  Ok(())
}

fn verify_term_docs<R, D>(random: &mut R, dir: Arc<D>, term: &Term, num_docs: i32) -> Result<()>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  let reader = directory_reader::open(dir)?;
  let mut postings_enum = TestUtil::docs_with_reader(
    random,
    &reader,
    term.field(),
    term.bytes(),
    None,
    NONE as i32,
  )?
  .unwrap();

  let mut count = 0;
  while postings_enum.next_doc()? != NO_MORE_DOCS {
    count += 1;
  }

  assert_eq!(num_docs, count);
  reader.close()?;
  Ok(())
}
fn add_docs_with_id<R, D>(
  random: &mut R,
  writer: &IndexWriter<D>,
  num_docs: i32,
  doc_start: i32,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  R: Rng + ?Sized,
  D: Directory + 'static,
{
  for i in 0..num_docs {
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      "content",
      "aaa",
      Store::No,
      field_to_type,
    )?);
    doc.add(new_text_field(
      random,
      "id",
      (doc_start + i).to_string(),
      Store::Yes,
      field_to_type,
    )?);
    doc.add(IntPoint::new("doc", vec![i])?);
    doc.add(IntPoint::new("doc2d", vec![i, i])?);
    doc.add(NumericDocValuesField::new("dv", i as i64));
    writer.add_document(doc)?;
  }
  Ok(())
}

fn set_up_dirs<R, D1, D2>(
  random: &mut R,
  dir: Arc<D1>,
  aux: Arc<D2>,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  R: Rng + ?Sized,
  D1: Directory + 'static,
  D2: Directory + 'static,
{
  set_up_dirs_with_id(random, dir, aux, false, field_types)
}

fn set_up_dirs_with_id<R, D1, D2>(
  random: &mut R,
  dir: Arc<D1>,
  aux: Arc<D2>,
  with_id: bool,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  R: Rng + ?Sized,
  D1: Directory + 'static,
  D2: Directory + 'static,
{
  let analyzer = MockAnalyzer::new(random);
  let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
  conf.set_open_mode(OpenMode::Create);
  conf.set_max_buffered_docs(1000);
  let mut writer = new_writer(dir.clone(), conf)?;

  if with_id {
    add_docs_with_id(random, &writer, 1000, 0, field_types)?;
  } else {
    add_docs(random, &mut writer, 1000, field_types)?;
  }
  assert_eq!(1000, writer.get_doc_stats()?.max_doc);
  assert_eq!(1, writer.get_segment_count());
  writer.close()?;
  drop(writer);

  let analyzer = MockAnalyzer::new(random);
  let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
  conf.set_open_mode(OpenMode::Create);
  conf.set_max_buffered_docs(1000);
  conf.set_merge_policy(new_log_merge_policy_with_merge_factor_cfs(
    random, false, 10,
  )?);

  let mut writer = new_writer(aux.clone(), conf)?;

  for i in 0..3 {
    if with_id {
      add_docs_with_id(random, &writer, 10, 10 * i, field_types)?;
    } else {
      add_docs(random, &mut writer, 10, field_types)?;
    }
    writer.close()?;
    drop(writer);

    let analyzer = MockAnalyzer::new(random);
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_open_mode(OpenMode::Append);
    conf.set_max_buffered_docs(1000);
    conf.set_merge_policy(new_log_merge_policy_with_merge_factor_cfs(
      random, false, 10,
    )?);
    writer = new_writer(aux.clone(), conf)?;
  }

  assert_eq!(30, writer.get_doc_stats()?.max_doc);
  assert_eq!(3, writer.get_segment_count());
  writer.close()?;
  Ok(())
}
#[test]
fn test_hang_on_close() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let mut lmp = LogMergePolicy::log_bytes_size();
  lmp.get_base_mut().set_no_cfs_ratio(0.0)?;
  lmp.set_merge_factor(100)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_max_buffered_docs(5);
  iwc.set_merge_policy(lmp);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let mut doc = Document::new();
  let mut custom_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
  let mut field_types = HashMap::new();
  custom_type.set_store_term_vectors(true)?;
  custom_type.set_store_term_vector_positions(true)?;
  custom_type.set_store_term_vector_offsets(true)?;
  doc.add(new_field(
    &mut random,
    "content",
    "aaa bbb ccc ddd eee fff ggg hhh iii",
    &custom_type,
    &mut field_types,
  )?);
  for _ in 0..60 {
    writer.add_document(doc.clone())?;
  }

  let mut doc2 = Document::new();
  let mut custom_type2 = FieldType::new();
  custom_type2.set_stored(true)?;
  doc2.add(new_field(
    &mut random,
    "content",
    "aaa bbb ccc ddd eee fff ggg hhh iii",
    &custom_type2,
    &mut field_types,
  )?);
  doc2.add(new_field(
    &mut random,
    "content",
    "aaa bbb ccc ddd eee fff ggg hhh iii",
    &custom_type2,
    &mut field_types,
  )?);
  doc2.add(new_field(
    &mut random,
    "content",
    "aaa bbb ccc ddd eee fff ggg hhh iii",
    &custom_type2,
    &mut field_types,
  )?);
  doc2.add(new_field(
    &mut random,
    "content",
    "aaa bbb ccc ddd eee fff ggg hhh iii",
    &custom_type2,
    &mut field_types,
  )?);

  for _ in 0..10 {
    writer.add_document(doc2.clone())?;
  }
  writer.close()?;
  drop(writer);

  let dir2 = new_directory_shared(&mut random)?;
  let mut lmp = LogMergePolicy::log_bytes_size();
  lmp.set_min_merge_mb(0.0001);
  lmp.get_base_mut().set_no_cfs_ratio(0.0)?;
  lmp.set_merge_factor(4)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_merge_scheduler(SerialMergeScheduler::new());
  iwc.set_merge_policy(lmp);
  let writer = IndexWriter::new(dir2.clone(), iwc)?;
  writer.add_indexes_from_dir(std::slice::from_ref(&dir))?;
  writer.close()?;
  Ok(())
}
fn add_doc<D, R>(
  random: &mut R,
  writer: &mut IndexWriter<D>,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "content",
    "aaa",
    Store::No,
    field_types,
  )?);
  writer.add_document(doc)?;
  Ok(())
}
#[test]
fn test_add_indexes_with_concurrent_merges() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_indexes_with_partial_merge_failures() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_indexes_with_null_merge_spec() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_indexes_with_empty_merge_spec() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_indexes_with_empty_readers() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_cascading_merges_triggered() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_indexes_hitting_max_docs_limit() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_indexes_with_threads() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_indexes_with_close() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_indexes_with_rollback() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_existing_deletes() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_simple_case_custom_codec() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}
#[test]
fn test_non_cfs_leftovers() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_index_missing_codec() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_field_names_changed() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_empty() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_fake_all_deleted() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_locks_block() -> Result<()> {
  let mut random = random();

  let src = new_directory_shared(&mut random)?;
  let w1 = RandomIndexWriter::new(&mut random, src.clone());
  w1.add_document(&mut random, Document::new())?;
  w1.commit(&mut random)?;

  let dest = new_directory_shared(&mut random)?;
  let a = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, a);
  let w2 = RandomIndexWriter::with_config(&mut random, dest.clone(), iwc);

  let err = w2.add_indexes_from_dir(&mut random, std::slice::from_ref(&src));
  assert!(matches!(err, Err(LuceneError::LockObtainFailed(_))));

  w1.close(&mut random)?;
  w2.close(&mut random)?;
  Ok(())
}

#[test]
fn test_illegal_index_sort_change1() -> Result<()> {
  let mut random = random();

  let dir1 = new_directory_shared(&mut random)?;
  let a = MockAnalyzer::new(&mut random);
  let mut iwc1 = new_index_writer_config_with_analyzer(&mut random, a);
  iwc1.set_index_sort(Sort::with_fields(vec![SortField::new(
    Some("foo"),
    SortFieldType::Int,
  )?])?)?;
  let w1 = RandomIndexWriter::with_config(&mut random, dir1.clone(), iwc1);
  w1.add_document(&mut random, Document::new())?;
  w1.commit(&mut random)?;
  w1.add_document(&mut random, Document::new())?;
  w1.commit(&mut random)?;
  w1.force_merge(&mut random, 1)?;
  w1.close(&mut random)?;
  drop(w1);

  let dir2 = new_directory_shared(&mut random)?;
  let a = MockAnalyzer::new(&mut random);
  let mut iwc2 = new_index_writer_config_with_analyzer(&mut random, a);
  iwc2.set_index_sort(Sort::with_fields(vec![SortField::new(
    Some("foo"),
    SortFieldType::String,
  )?])?)?;
  let w2 = RandomIndexWriter::with_config(&mut random, dir2.clone(), iwc2);

  let err = w2.add_indexes_from_dir(&mut random, std::slice::from_ref(&dir1));
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert_eq!(
    "cannot change index sort from <int: \"foo\"> to <string: \"foo\">",
    err.unwrap_err().to_string()
  );

  w2.close(&mut random)?;
  Ok(())
}

#[test]
fn test_illegal_index_sort_change2() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_indexes_dv_update_same_segment_name() -> Result<()> {
  let mut random = random();

  let dir1 = new_directory_shared(&mut random)?;
  let a = MockAnalyzer::new(&mut random);
  let iwc1 = new_index_writer_config_with_analyzer(&mut random, a);
  let w1 = IndexWriter::new(dir1.clone(), iwc1)?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  doc.add(StringField::from_string("version", "1", Store::Yes)?);
  doc.add(NumericDocValuesField::new("soft_delete", 1));
  w1.add_document(doc)?;
  w1.flush()?;

  w1.update_doc_values(
    Term::from_text("id", "1"),
    vec![NumericDocValuesField::new("soft_delete", 1).into()],
  )?;
  w1.commit()?;
  w1.close()?;
  drop(w1);
  let a = MockAnalyzer::new(&mut random);
  let iwc2 = new_index_writer_config_with_analyzer(&mut random, a);
  let dir2 = new_directory_shared(&mut random)?;
  let w2 = IndexWriter::new(dir2.clone(), iwc2)?;
  w2.add_indexes_from_dir(std::slice::from_ref(&dir1))?;
  w2.commit()?;
  w2.close()?;
  drop(w2);

  if cfg!(feature = "test_log_verbose") {
    println!("\nTEST: now open w3");
  }

  let a = MockAnalyzer::new(&mut random);
  let iwc3 = new_index_writer_config_with_analyzer(&mut random, a);
  let w3 = IndexWriter::new(dir2.clone(), iwc3)?;
  w3.close()?;
  drop(w3);
  let a = MockAnalyzer::new(&mut random);
  let iwc3 = new_index_writer_config_with_analyzer(&mut random, a);
  let w3 = IndexWriter::new(dir2.clone(), iwc3)?;
  w3.close()?;

  Ok(())
}

#[test]
fn test_add_indexes_dv_update_new_segment_name() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_indices_with_soft_deletes() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_add_indices_with_blocks() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_set_diagnostics() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_illegal_parent_doc_change() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}

#[test]
fn test_illegal_non_parent_field() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}
