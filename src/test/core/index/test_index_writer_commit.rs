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
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader::{self, DirectoryReader};
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::test_index_writer::{
  add_doc, add_doc_with_index, assert_no_unreferenced_files,
};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config_with_analyzer,
  new_log_merge_policy_with_merge_factor, new_searcher_with_reader, random,
};
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
struct TestIndexWriterCommit;
/*
 * Simple test for "commit on close": open writer then
 * add a bunch of docs, making sure reader does not see
 * these docs until writer is closed.
 */
#[test]
fn test_commit_on_close() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mock = MockAnalyzer::new(&mut random);
  let iwc1 = new_index_writer_config_with_analyzer(&mut random, mock);
  {
    let writer = IndexWriter::new(dir.clone(), iwc1)?;

    for _ in 0..14 {
      add_doc(&mut random, &writer, &mut field_types)?;
    }

    writer.close()?;
  }

  let search_term = Term::from_text("content", "aaa");

  {
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    let hits = searcher.search(TermQuery::new(search_term.clone()), 1000)?;
    assert_eq!(14, hits.score_docs.len(), "first number of hits");
  }

  let reader = directory_reader::open(dir.clone())?;

  let mock = MockAnalyzer::new(&mut random);
  let iwc2 = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), iwc2)?;

  for _ in 0..3 {
    for _ in 0..11 {
      add_doc(&mut random, &writer, &mut field_types)?;
    }

    let r = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(r)?;
    let hits = searcher.search(TermQuery::new(search_term.clone()), 1000)?;
    assert_eq!(
      14,
      hits.score_docs.len(),
      "reader incorrectly sees changes from writer"
    );

    assert!(
      reader.is_current(&writer)?,
      "reader should have still been current"
    );
  }

  writer.close()?;

  assert!(
    !reader.is_current(&writer)?,
    "reader should not be current now"
  );

  {
    let r = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(r)?;
    let hits = searcher.search(TermQuery::new(search_term.clone()), 1000)?;
    assert_eq!(
      47,
      hits.score_docs.len(),
      "reader did not see changes after writer was closed"
    );
  }

  Ok(())
}
#[test]
fn test_commit_on_close_abort() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_max_buffered_docs(10);
  let mut writer = IndexWriter::new(dir.clone(), iwc)?;
  for _ in 0..14 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }
  writer.close()?;

  let search_term = Term::from_text("content", "aaa");
  let reader = directory_reader::open(dir.clone())?;
  let searcher = new_searcher_with_reader(reader)?;
  let hits = searcher.search(TermQuery::new(search_term.clone()), 1000)?;
  assert_eq!(14, hits.score_docs.len(), "first number of hits");

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_open_mode(OpenMode::Append);
  iwc.set_max_buffered_docs(10);
  drop(writer);
  writer = IndexWriter::new(dir.clone(), iwc)?;
  for _ in 0..17 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }
  writer.delete_documents_with_terms(vec![search_term.clone()])?;

  let reader = directory_reader::open(dir.clone())?;
  let searcher = new_searcher_with_reader(reader)?;
  let hits = searcher.search(TermQuery::new(search_term.clone()), 1000)?;
  assert_eq!(
    14,
    hits.score_docs.len(),
    "reader incorrectly sees changes from writer"
  );

  writer.rollback()?;
  drop(writer);
  assert_no_unreferenced_files(dir.clone(), "unreferenced files remain after rollback()")?;

  let reader = directory_reader::open(dir.clone())?;
  let searcher = new_searcher_with_reader(reader)?;
  let hits = searcher.search(TermQuery::new(search_term.clone()), 1000)?;
  assert_eq!(14, hits.score_docs.len(), "saw changes after writer.abort");

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_open_mode(OpenMode::Append);
  iwc.set_max_buffered_docs(10);
  writer = IndexWriter::new(dir.clone(), iwc)?;

  for _ in 0..12 {
    for _ in 0..17 {
      add_doc(&mut random, &writer, &mut field_types)?;
    }
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    let hits = searcher.search(TermQuery::new(search_term.clone()), 1000)?;
    assert_eq!(
      14,
      hits.score_docs.len(),
      "reader incorrectly sees changes from writer"
    );
  }

  writer.close()?;
  let reader = directory_reader::open(dir.clone())?;
  let searcher = new_searcher_with_reader(reader)?;
  let hits = searcher.search(TermQuery::new(search_term), 1000)?;
  assert_eq!(218, hits.score_docs.len(), "didn't see changes after close");

  Ok(())
}

#[test]
fn test_commit_on_close_disk_usage() -> Result<()> {
  Ok(())
}
#[test]
fn test_commit_on_close_force_merge() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_max_buffered_docs(10);
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let mut writer = IndexWriter::new(dir.clone(), iwc)?;
  for j in 0..17 {
    add_doc_with_index(&mut random, &writer, j, &mut field_types)?;
  }
  writer.close()?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_open_mode(OpenMode::Append);
  drop(writer);
  writer = IndexWriter::new(dir.clone(), iwc)?;
  writer.force_merge(1)?;

  let reader = directory_reader::open(dir.clone())?;
  assert!(
    get_context(&reader)?.leaves()?.len() > 1,
    "Reader incorrectly sees one segment"
  );
  reader.close()?;

  writer.rollback()?;
  drop(writer);
  assert_no_unreferenced_files(dir.clone(), "aborted writer after forceMerge")?;

  let reader = directory_reader::open(dir.clone())?;
  assert!(
    get_context(&reader)?.leaves()?.len() > 1,
    "Reader incorrectly sees one segment"
  );
  reader.close()?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_open_mode(OpenMode::Append);
  writer = IndexWriter::new(dir.clone(), iwc)?;
  writer.force_merge(1)?;
  writer.close()?;
  drop(writer);
  assert_no_unreferenced_files(dir.clone(), "aborted writer after forceMerge")?;

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(
    1,
    get_context(&reader)?.leaves()?.len(),
    "Reader incorrectly sees more than one segment"
  );
  reader.close()?;

  Ok(())
}
#[test]
fn test_commit_thread_safety() -> Result<()> {
  // TODO: 多线程未实现
  Ok(())
}
#[test]
fn test_force_commit() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_max_buffered_docs(2);
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 5)?);
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  writer.commit()?;

  for _ in 0..23 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(0, reader.num_docs()?);
  writer.commit()?;
  // TODO IMPORTANT: openIfChanged 未实现
  let reader2 = directory_reader::open_from_writer(&writer)?;
  assert_eq!(0, reader.num_docs()?);
  assert_eq!(23, reader2.num_docs()?);
  reader.close()?;

  for _ in 0..17 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }
  assert_eq!(23, reader2.num_docs()?);
  reader2.close()?;
  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(23, reader.num_docs()?);
  reader.close()?;
  writer.commit()?;

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(40, reader.num_docs()?);
  reader.close()?;
  writer.close()?;

  Ok(())
}
#[test]
fn test_future_commit() -> Result<()> {
  // TODO: ReaderCommit未实现
  Ok(())
}

#[test]
fn test_zero_commits() -> Result<()> {
  // TODO: ReaderCommit未实现
  Ok(())
}
#[test]
fn test_prepare_commit() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_max_buffered_docs(2);
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 5)?);
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  writer.commit()?;

  for _ in 0..23 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(0, reader.num_docs()?);

  writer.prepare_commit()?;

  let reader2 = directory_reader::open(dir.clone())?;
  assert_eq!(0, reader2.num_docs()?);

  writer.commit()?;

  // TODO IMPORTANT: openIfChanged 未实现
  let reader3 = directory_reader::open_from_writer(&writer)?;
  assert_eq!(0, reader.num_docs()?);
  assert_eq!(0, reader2.num_docs()?);
  assert_eq!(23, reader3.num_docs()?);
  reader.close()?;
  reader2.close()?;

  for _ in 0..17 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }

  assert_eq!(23, reader3.num_docs()?);
  reader3.close()?;
  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(23, reader.num_docs()?);
  reader.close()?;

  writer.prepare_commit()?;

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(23, reader.num_docs()?);
  reader.close()?;

  writer.commit()?;
  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(40, reader.num_docs()?);
  reader.close()?;
  writer.close()?;

  Ok(())
}

#[test]
fn test_prepare_commit_rollback() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_max_buffered_docs(2);
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 5)?);
  let mut writer = IndexWriter::new(dir.clone(), iwc)?;
  writer.commit()?;

  for _ in 0..23 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(0, reader.num_docs()?);

  writer.prepare_commit()?;

  let reader2 = directory_reader::open(dir.clone())?;
  assert_eq!(0, reader2.num_docs()?);

  writer.rollback()?;

  // TODO IMPORTANT: openIfChanged 未实现
  let reader3 = directory_reader::open(dir.clone())?;
  assert_eq!(0, reader3.num_docs()?);
  reader3.close()?;
  assert_eq!(0, reader.num_docs()?);
  assert_eq!(0, reader2.num_docs()?);
  reader.close()?;
  reader2.close()?;

  let mock = MockAnalyzer::new(&mut random);
  drop(writer);
  writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, mock),
  )?;
  for _ in 0..17 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(0, reader.num_docs()?);
  reader.close()?;

  writer.prepare_commit()?;

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(0, reader.num_docs()?);
  reader.close()?;

  writer.commit()?;
  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(17, reader.num_docs()?);
  reader.close()?;
  writer.close()?;

  Ok(())
}
#[test]
fn test_prepare_commit_no_changes() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  writer.prepare_commit()?;
  writer.commit()?;
  writer.close()?;

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(0, reader.num_docs()?);

  Ok(())
}
