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
use crate::core::analysis::analyzer::{Analyzer, AnalyzerStoredValue, TokenStreamComponents};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::string_field::StringField;
use crate::core::index::directory_reader::{self, DirectoryReader};
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::{IndexCommitWrapper, IndexWriter};
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::no_deletion_policy::NoDeletionPolicy;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::analysis::mock_fixed_length_payload_filter::MockFixedLengthPayloadFilter;
use crate::test_framework::core::analysis::mock_tokenizer::{MockTokenizer, WHITESPACE};
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::index::test_index_writer::{
  add_doc, add_doc_with_index, assert_no_unreferenced_files,
};
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer,
  new_log_merge_policy_with_merge_factor, new_mock_directory, new_searcher_with_reader, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::prelude::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

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
  let iwc1 = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
  let iwc2 = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
      reader.is_current()?,
      "reader should have still been current"
    );
  }

  writer.close()?;

  assert!(!reader.is_current()?, "reader should not be current now");

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
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
  // MemoryCodec, since it uses FST, is not necessarily
  // "additive", ie if you add up N small FSTs, then merge
  // them, the merged result can easily be larger than the
  // sum because the merged FST may use array encoding for
  // some arcs (which uses more space):

  let mut random = random();
  let dir = new_mock_directory(&mut random)?;
  let analyzer: Arc<dyn Analyzer> = if random.random_bool(0.5) {
    // no payloads
    Arc::new(CommitOnCloseDiskUsageNoPayloadAnalyzer::new(&mut random))
  } else {
    // fixed length payloads
    let length = random.random_range(0..200);
    Arc::new(CommitOnCloseDiskUsageFixedLengthPayloadAnalyzer::new(
      &mut random,
      length,
    ))
  };
  let mut iwc = new_index_writer_config_with_analyzer(
    &mut random,
    Box::new(analyzer.clone()) as Box<dyn Analyzer>,
  )?;
  iwc.set_max_buffered_docs(10);
  iwc.set_reader_pooling(false);
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let writer = IndexWriter::new(Arc::new(dir.clone()), iwc)?;
  let mut field_types = HashMap::new();
  for j in 0..30 {
    add_doc_with_index(&mut random, &writer, j, &mut field_types)?;
  }
  writer.close()?;
  dir.reset_max_used_size_in_bytes()?;

  dir.set_track_disk_usage(true);
  let start_disk_usage = dir.get_max_used_size_in_bytes();
  let mut iwc = new_index_writer_config_with_analyzer(
    &mut random,
    Box::new(analyzer.clone()) as Box<dyn Analyzer>,
  )?;
  iwc.set_open_mode(OpenMode::Append);
  iwc.set_max_buffered_docs(10);
  iwc.set_merge_scheduler(SerialMergeScheduler::new());
  iwc.set_reader_pooling(false);
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let writer = IndexWriter::new(Arc::new(dir.clone()), iwc)?;

  for j in 0..1470 {
    add_doc_with_index(&mut random, &writer, j, &mut field_types)?;
  }
  let mid_disk_usage = dir.get_max_used_size_in_bytes();
  dir.reset_max_used_size_in_bytes()?;
  writer.force_merge(1)?;
  writer.close()?;

  let reader = directory_reader::open(Arc::new(dir.clone()))?;
  reader.close()?;

  let end_disk_usage = dir.get_max_used_size_in_bytes();

  // Ending index is 50X as large as starting index; due
  // to 3X disk usage normally we allow 150X max
  // transient usage.  If something is wrong w/ deleter
  // and it doesn't delete intermediate segments then it
  // will exceed this 150X:
  assert!(
    mid_disk_usage < 150 * start_disk_usage,
    "writer used too much space while adding documents: mid={mid_disk_usage} start={start_disk_usage} end={end_disk_usage} max={}",
    start_disk_usage * 150
  );
  assert!(
    end_disk_usage < 150 * start_disk_usage,
    "writer used too much space after close: endDiskUsage={end_disk_usage} startDiskUsage={start_disk_usage} max={}",
    start_disk_usage * 150
  );

  Ok(())
}
#[test]
fn test_commit_on_close_force_merge() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_max_buffered_docs(10);
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let mut writer = IndexWriter::new(dir.clone(), iwc)?;
  for j in 0..17 {
    add_doc_with_index(&mut random, &writer, j, &mut field_types)?;
  }
  writer.close()?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_open_mode(OpenMode::Append);
  drop(writer);
  writer = IndexWriter::new(dir.clone(), iwc)?;
  writer.force_merge(1)?;

  let reader = directory_reader::open(dir.clone())?;
  assert!(
    (&reader).get_context()?.leaves()?.len() > 1,
    "Reader incorrectly sees one segment"
  );
  reader.close()?;

  writer.rollback()?;
  drop(writer);
  assert_no_unreferenced_files(dir.clone(), "aborted writer after forceMerge")?;

  let reader = directory_reader::open(dir.clone())?;
  assert!(
    (&reader).get_context()?.leaves()?.len() > 1,
    "Reader incorrectly sees one segment"
  );
  reader.close()?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_open_mode(OpenMode::Append);
  writer = IndexWriter::new(dir.clone(), iwc)?;
  writer.force_merge(1)?;
  writer.close()?;
  drop(writer);
  assert_no_unreferenced_files(dir.clone(), "aborted writer after forceMerge")?;

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(
    1,
    (&reader).get_context()?.leaves()?.len(),
    "Reader incorrectly sees more than one segment"
  );
  reader.close()?;

  Ok(())
}
#[test]
fn test_commit_thread_safety() -> Result<()> {
  const NUM_THREADS: usize = 5;
  const MAX_ITERATIONS: usize = 10;

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let writer = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);
  TestUtil::reduce_open_files(&writer.w)?;
  writer.commit(&mut random)?;
  let writer = Arc::new(writer);

  let failed = Arc::new(AtomicBool::new(false));
  let mut threads = Vec::new();

  for i in 0..NUM_THREADS {
    let dir = dir.clone();
    let writer = writer.clone();
    let failed = failed.clone();
    threads.push(thread::spawn(move || -> Result<()> {
      let mut thread_random = crate::test_framework::core::util::lucene_test_case::random();
      let mut reader = directory_reader::open(dir.clone())?;
      let mut iterations = 0;
      let mut count = 0;
      loop {
        if failed.load(Ordering::SeqCst) {
          break;
        }
        for _ in 0..10 {
          let s = format!("{}_{}", i, count);
          count += 1;
          let mut doc = Document::new();
          doc.add(StringField::from_string("f", s.clone(), Store::No)?);
          writer.add_document(&mut thread_random, doc)?;
          writer.commit(&mut thread_random)?;

          let reader2 = directory_reader::open_if_changed(&reader)?.unwrap();
          reader.close()?;
          reader = reader2;
          assert_eq!(1, reader.doc_freq(&Term::from_text("f", &s))?);
        }
        iterations += 1;
        if iterations >= MAX_ITERATIONS {
          break;
        }
      }
      reader.close()?;
      Ok(())
    }));
  }

  for thread in threads {
    match thread.join() {
      Ok(result) => {
        if result.is_err() {
          failed.store(true, Ordering::SeqCst);
        }
        result?;
      },
      Err(e) => {
        failed.store(true, Ordering::SeqCst);
        std::panic::resume_unwind(e);
      },
    }
  }

  assert!(!failed.load(Ordering::SeqCst));
  writer.close(&mut random)?;

  Ok(())
}
#[test]
fn test_force_commit() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
  let reader2 = directory_reader::open_if_changed(&reader)?.unwrap();
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
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_index_deletion_policy(NoDeletionPolicy);
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  let doc = Document::new();
  writer.add_document(doc.clone())?;

  // commit to "first"
  let mut commit_data = HashMap::new();
  commit_data.insert("tag".to_string(), "first".to_string());
  writer.set_live_commit_data(commit_data.clone());
  writer.commit()?;

  // commit to "second"
  writer.add_document(doc.clone())?;
  commit_data.insert("tag".to_string(), "second".to_string());
  writer.set_live_commit_data(commit_data.clone());
  writer.close()?;
  drop(writer);

  // open "first" with IndexWriter
  let commit = directory_reader::list_commits(dir.clone())?
    .into_iter()
    .find(|commit| {
      commit
        .get_user_data()
        .get("tag")
        .is_some_and(|tag| tag == "first")
    });

  assert!(commit.is_some());

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_index_deletion_policy(NoDeletionPolicy);
  let commit = commit.unwrap();
  let writer = IndexWriter::with_index_commit(
    dir.clone(),
    iwc,
    IndexCommitWrapper::new(Some(commit), None)?,
  )?;

  assert_eq!(1, writer.get_doc_stats()?.num_docs);

  // commit IndexWriter to "third"
  writer.add_document(doc)?;
  commit_data.insert("tag".to_string(), "third".to_string());
  writer.set_live_commit_data(commit_data);
  writer.close()?;

  // make sure "second" commit is still there
  let commit = directory_reader::list_commits(dir.clone())?
    .into_iter()
    .find(|commit| {
      commit
        .get_user_data()
        .get("tag")
        .is_some_and(|tag| tag == "second")
    });

  assert!(commit.is_some());

  Ok(())
}

#[test]
fn test_zero_commits() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, mock)?,
  )?;
  match directory_reader::list_commits(dir.clone()) {
    Ok(_) => panic!("expected IndexNotFound"),
    Err(err) => assert!(matches!(err, LuceneError::IndexNotFound(_))),
  }

  // No changes still should generate a commit, because it's a new index.
  writer.close()?;
  assert_eq!(
    1,
    directory_reader::list_commits(dir.clone())?.len(),
    "expected 1 commits!"
  );
  Ok(())
}
#[test]
fn test_prepare_commit() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
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

  let reader3 = directory_reader::open_if_changed(&reader)?.unwrap();
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
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
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

  let reader3 = directory_reader::open_if_changed(&reader)?;
  assert!(reader3.is_none());
  assert_eq!(0, reader.num_docs()?);
  assert_eq!(0, reader2.num_docs()?);
  reader.close()?;
  reader2.close()?;

  let mock = MockAnalyzer::new(&mut random);
  drop(writer);
  writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, mock)?,
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
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  writer.prepare_commit()?;
  writer.commit()?;
  writer.close()?;

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(0, reader.num_docs()?);

  Ok(())
}

#[test]
fn test_commit_user_data() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_max_buffered_docs(2);
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  for _ in 0..17 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }
  writer.close()?;
  drop(writer);

  let r = directory_reader::open(dir.clone())?;
  // commit(Map) never called for this index
  assert_eq!(0, r.get_index_commit()?.get_user_data().len());
  r.close()?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_max_buffered_docs(2);
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  for _ in 0..17 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }
  let mut data = HashMap::new();
  data.insert("label".to_string(), "test1".to_string());
  writer.set_live_commit_data(data);
  writer.close()?;
  drop(writer);

  let r = directory_reader::open(dir.clone())?;
  assert_eq!(
    Some("test1"),
    r.get_index_commit()?
      .get_user_data()
      .get("label")
      .map(String::as_str)
  );
  r.close()?;

  let mock = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, mock)?,
  )?;
  writer.force_merge(1)?;
  writer.close()?;

  Ok(())
}

#[test]
fn test_prepare_commit_then_close() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, mock)?,
  )?;
  writer.add_document(Document::new())?;

  writer.prepare_commit()?;
  let err = writer.close();
  assert!(matches!(err, Err(LuceneError::IllegalState(_))));
  writer.commit()?;
  writer.close()?;

  let r = directory_reader::open(dir.clone())?;
  assert_eq!(1, r.max_doc()?);
  r.close()?;

  Ok(())
}

#[test]
fn test_commit_data_is_live() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, mock)?,
  )?;
  writer.add_document(Document::new())?;

  let mut commit_data = HashMap::new();
  commit_data.insert("foo".to_string(), "bar".to_string());

  // make sure "foo" / "bar" doesn't take
  writer.set_live_commit_data(commit_data.clone());
  {
    let mut inner = writer.inner.lock();
    let commit_data = inner.commit_user_data.as_mut().unwrap();
    commit_data.clear();
    commit_data.insert("boo".to_string(), "baz".to_string());
  }

  // this finally does the commit, and should burn "boo" / "baz"
  writer.close()?;

  let commits = directory_reader::list_commits(dir.clone())?;
  assert_eq!(1, commits.len());

  let data = commits[0].get_user_data();
  assert_eq!(1, data.len());
  assert_eq!(Some("baz"), data.get("boo").map(String::as_str));

  Ok(())
}

struct CommitOnCloseDiskUsageNoPayloadAnalyzer {
  random: Mutex<StdRng>,
  stored_value: AnalyzerStoredValue,
}

impl CommitOnCloseDiskUsageNoPayloadAnalyzer {
  fn new<R>(random: &mut R) -> Self
  where
    R: Rng + ?Sized,
  {
    Self {
      random: Mutex::new(StdRng::seed_from_u64(random.random())),
      stored_value: AnalyzerStoredValue::new(),
    }
  }

  fn next_random(&self) -> StdRng {
    StdRng::seed_from_u64(self.random.lock().expect("random mutex poisoned").random())
  }
}

impl Analyzer for CommitOnCloseDiskUsageNoPayloadAnalyzer {
  fn create_components(&self, _field_name: &str) -> Result<TokenStreamComponents> {
    Ok(TokenStreamComponents::new(
      Box::new(MockTokenizer::with_default_max_token_length(
        self.next_random(),
        WHITESPACE.clone(),
        true,
      )) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(CommitOnCloseDiskUsageNoPayloadAnalyzer);

struct CommitOnCloseDiskUsageFixedLengthPayloadAnalyzer {
  random: Mutex<StdRng>,
  length: usize,
  stored_value: AnalyzerStoredValue,
}

impl CommitOnCloseDiskUsageFixedLengthPayloadAnalyzer {
  fn new<R>(random: &mut R, length: usize) -> Self
  where
    R: Rng + ?Sized,
  {
    Self {
      random: Mutex::new(StdRng::seed_from_u64(random.random())),
      length,
      stored_value: AnalyzerStoredValue::new(),
    }
  }

  fn next_random(&self) -> StdRng {
    StdRng::seed_from_u64(self.random.lock().expect("random mutex poisoned").random())
  }
}

impl Analyzer for CommitOnCloseDiskUsageFixedLengthPayloadAnalyzer {
  fn create_components(&self, _field_name: &str) -> Result<TokenStreamComponents> {
    let tokenizer =
      MockTokenizer::with_default_max_token_length(self.next_random(), WHITESPACE.clone(), true);
    let filter = MockFixedLengthPayloadFilter::new(tokenizer, self.next_random(), self.length);
    Ok(TokenStreamComponents::new(
      Box::new(filter) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(CommitOnCloseDiskUsageFixedLengthPayloadAnalyzer);
