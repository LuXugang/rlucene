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
use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::TextField;
use crate::core::index::concurrent_merge_scheduler::{
  AUTO_DETECT_MERGES_AND_THREADS, ConcurrentMergeScheduler,
};
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexWriter, IndexWriterHooks, IndexWriterHooksEnum};
use crate::core::index::index_writer_config::{IndexWriterConfig, OpenMode};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::info_stream::{InfoStream, InfoStreamEnum};
use crate::core::util::io_utils::IOUtils;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::test_index_writer::assert_no_unreferenced_files;
use crate::test::core::util::lucene_test_case::{
  is_night_mode, new_directory_shared, new_index_writer_config_with_analyzer,
  new_log_merge_policy_with_merge_factor, new_mock_directory, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

#[allow(dead_code)] // for quick search
struct TestConcurrentMergeScheduler;

#[test]
fn test_flush_exceptions() -> Result<()> {
  // TODO callStackContainsAnyOf未实现
  Ok(())
}

// Test that deletes committed after a merge started and
// before it finishes, are correctly merged back:
#[test]
fn test_delete_merging() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;

  let mut mp = LogMergePolicy::log_doc();
  // Force degenerate merging so we can get a mix of
  // merging of segments with and without deletes at the
  // start:
  mp.set_min_merge_docs(1000);
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwc.set_merge_policy(mp);
  iwc.set_merge_scheduler(ConcurrentMergeScheduler::new());
  let writer = IndexWriter::new(directory.clone(), iwc)?;
  TestUtil::reduce_open_files(&writer)?;

  for i in 0..10 {
    if cfg!(feature = "test_log_verbose") {
      println!("\nTEST: cycle");
    }
    for j in 0..100 {
      let mut doc = Document::new();
      doc.add(StringField::from_string(
        "id",
        (i * 100 + j).to_string(),
        Store::Yes,
      )?);
      writer.add_document(doc)?;
    }

    let mut del_id = i;
    while del_id < 100 * (1 + i) {
      if cfg!(feature = "test_log_verbose") {
        println!("TEST: del {del_id}");
      }
      writer.delete_documents_with_terms(vec![Term::from_text("id", del_id.to_string())])?;
      del_id += 10;
    }

    writer.commit()?;
  }

  writer.close()?;
  let reader = directory_reader::open(directory)?;
  // Verify that we did not lose any deletes...
  assert_eq!(450, reader.num_docs()?);
  reader.close()?;
  Ok(())
}

#[test]
fn test_no_extra_files() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;
  let mut field_types = std::collections::HashMap::new();

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwc.set_max_buffered_docs(2);
  iwc.set_merge_scheduler(ConcurrentMergeScheduler::new());
  let mut writer = IndexWriter::new(directory.clone(), iwc)?;

  for _iter in 0..7 {
    for _ in 0..21 {
      let mut doc = Document::new();
      doc.add(crate::test::core::util::lucene_test_case::new_text_field(
        &mut random,
        "content",
        "a b c",
        Store::No,
        &mut field_types,
      )?);
      writer.add_document(doc)?;
    }

    writer.close()?;
    assert_no_unreferenced_files(directory.clone(), "testNoExtraFiles")?;

    // Reopen
    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
    iwc.set_open_mode(OpenMode::Append);
    iwc.set_max_buffered_docs(2);
    iwc.set_merge_scheduler(ConcurrentMergeScheduler::new());
    writer = IndexWriter::new(directory.clone(), iwc)?;
  }

  writer.close()?;
  Ok(())
}

#[test]
fn test_no_wait_close() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  // Force excessive merging:
  iwc
    .set_max_buffered_docs(2)
    .set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 100)?)
    .set_commit_on_close(false)
    .set_merge_scheduler(ConcurrentMergeScheduler::new());

  let mut writer = IndexWriter::new(directory.clone(), iwc)?;

  let num_iters = if is_night_mode() { 10 } else { 3 };
  for iter in 0..num_iters {
    for j in 0..201 {
      let mut doc = Document::new();
      doc.add(StringField::from_string(
        "id",
        (iter * 201 + j).to_string(),
        Store::Yes,
      )?);
      doc.add(KnnFloatVectorField::new(
        "knn",
        vec![random.random::<f32>(), random.random::<f32>()],
      )?);
      writer.add_document(doc)?;
    }

    let mut del_id = iter * 201;
    for _ in 0..20 {
      writer.delete_documents_with_terms(vec![Term::from_text("id", del_id.to_string())])?;
      del_id += 5;
    }

    // Force a bunch of merge threads to kick off so we
    // stress out aborting them on close:
    match writer.get_config_mut().get_merge_policy_mut() {
      crate::core::index::merge_policy::MergePolicyEnum::LogDoc(mp) => mp.set_merge_factor(3)?,
      crate::core::index::merge_policy::MergePolicyEnum::LogBytesSize(mp) => {
        mp.set_merge_factor(3)?
      },
      _ => {},
    }
    let mut doc = Document::new();
    doc.add(StringField::from_string(
      "id",
      format!("extra-{iter}"),
      Store::Yes,
    )?);
    doc.add(KnnFloatVectorField::new(
      "knn",
      vec![random.random::<f32>(), random.random::<f32>()],
    )?);
    writer.add_document(doc)?;

    let commit_result = writer.commit();
    let close_result = writer.close();
    commit_result?;
    close_result?;

    let reader = directory_reader::open(directory.clone())?;
    assert_eq!((1 + iter) * 182, reader.num_docs()?);
    reader.close()?;

    // Reopen
    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
    iwc.set_open_mode(OpenMode::Append);
    iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 100)?);
    // Force excessive merging:
    iwc.set_max_buffered_docs(2);
    iwc.set_commit_on_close(false);
    iwc.set_merge_scheduler(ConcurrentMergeScheduler::new());
    writer = IndexWriter::new(directory.clone(), iwc)?;
  }
  writer.close()?;

  Ok(())
}

// LUCENE-4544
#[test]
fn test_max_merge_count() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_small_merges_do_not_get_threads() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_intra_merge_thread_pool_is_limited_by_max_threads() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_total_bytes_size() -> Result<()> {
  // TrackingCMS未实现
  Ok(())
}

#[test]
fn test_invalid_max_merge_count_and_threads() -> Result<()> {
  let cms = ConcurrentMergeScheduler::new();
  assert!(matches!(
    cms
      .set_max_merges_and_threads(AUTO_DETECT_MERGES_AND_THREADS, 3)
      .unwrap_err(),
    LuceneError::IllegalArgument(_)
  ));
  assert!(matches!(
    cms
      .set_max_merges_and_threads(3, AUTO_DETECT_MERGES_AND_THREADS)
      .unwrap_err(),
    LuceneError::IllegalArgument(_)
  ));
  Ok(())
}

#[test]
fn test_live_max_merge_count() -> Result<()> {
  // TODO doMerge不能覆写
  Ok(())
}

// LUCENE-6063
#[test]
fn test_maybe_stall_called() -> Result<()> {
  // TODO maybeStall不能覆写
  Ok(())
}

// LUCENE-6094
#[test]
fn test_hang_during_rollback() -> Result<()> {
  // TODO doMerge不能覆写
  Ok(())
}

// LUCENE-10118 : Verify the basic log output from MergeThreads
#[test]
fn test_merge_thread_messages() -> Result<()> {
  // TODO getMergeThread不能覆写
  Ok(())
}

#[test]
fn test_dynamic_defaults() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let cms = ConcurrentMergeScheduler::new();
  assert_eq!(AUTO_DETECT_MERGES_AND_THREADS, cms.get_max_merge_count());
  assert_eq!(AUTO_DETECT_MERGES_AND_THREADS, cms.get_max_thread_count());
  iwc.set_merge_scheduler(cms.clone());
  iwc.set_max_buffered_docs(2);
  let mut lmp = LogMergePolicy::log_doc();
  lmp.set_merge_factor(2)?;
  iwc.set_merge_policy(lmp);

  let writer = IndexWriter::new(dir, iwc)?;
  writer.add_document(Document::new())?;
  writer.add_document(Document::new())?;
  // flush

  writer.add_document(Document::new())?;
  writer.add_document(Document::new())?;
  // flush + merge

  // CMS should have now set true values:
  assert_ne!(AUTO_DETECT_MERGES_AND_THREADS, cms.get_max_merge_count());
  assert_ne!(AUTO_DETECT_MERGES_AND_THREADS, cms.get_max_thread_count());
  writer.close()?;
  Ok(())
}

#[test]
fn test_reset_to_auto_default() -> Result<()> {
  let cms = ConcurrentMergeScheduler::new();
  assert_eq!(AUTO_DETECT_MERGES_AND_THREADS, cms.get_max_merge_count());
  assert_eq!(AUTO_DETECT_MERGES_AND_THREADS, cms.get_max_thread_count());
  cms.set_max_merges_and_threads(4, 3)?;
  assert_eq!(4, cms.get_max_merge_count());
  assert_eq!(3, cms.get_max_thread_count());

  assert!(matches!(
    cms
      .set_max_merges_and_threads(AUTO_DETECT_MERGES_AND_THREADS, 4)
      .unwrap_err(),
    LuceneError::IllegalArgument(_)
  ));

  assert!(matches!(
    cms
      .set_max_merges_and_threads(4, AUTO_DETECT_MERGES_AND_THREADS)
      .unwrap_err(),
    LuceneError::IllegalArgument(_)
  ));

  cms.set_max_merges_and_threads(
    AUTO_DETECT_MERGES_AND_THREADS,
    AUTO_DETECT_MERGES_AND_THREADS,
  )?;
  assert_eq!(AUTO_DETECT_MERGES_AND_THREADS, cms.get_max_merge_count());
  assert_eq!(AUTO_DETECT_MERGES_AND_THREADS, cms.get_max_thread_count());
  Ok(())
}

#[test]
fn test_spinning_defaults() -> Result<()> {
  let cms = ConcurrentMergeScheduler::new();
  cms.set_default_max_merges_and_threads(true);
  assert_eq!(1, cms.get_max_thread_count());
  assert_eq!(6, cms.get_max_merge_count());
  Ok(())
}

#[test]
fn test_auto_io_throttle_getter() -> Result<()> {
  let cms = ConcurrentMergeScheduler::new();
  assert!(!cms.get_auto_io_throttle());
  cms.enable_auto_io_throttle()?;
  assert!(cms.get_auto_io_throttle());
  cms.disable_auto_io_throttle()?;
  assert!(!cms.get_auto_io_throttle());
  Ok(())
}

#[test]
fn test_non_spinning_defaults() -> Result<()> {
  let cms = ConcurrentMergeScheduler::new();
  cms.set_default_max_merges_and_threads(false);
  let thread_count = cms.get_max_thread_count();
  assert!(thread_count >= 1);
  // assert!(thread_count <= 4);
  assert_eq!(5 + thread_count, cms.get_max_merge_count());
  Ok(())
}

// LUCENE-6197
#[test]
fn test_no_stall_merge_threads() -> Result<()> {
  let mut random = random();
  let dir = Arc::new(new_mock_directory(&mut random)?);

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwc.set_merge_policy(NoMergePolicy::default());
  iwc.set_max_buffered_docs(2);
  iwc.set_use_compound_file(true); // reduce open files
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  let num_docs = if is_night_mode() { 1000 } else { 100 };
  for i in 0..num_docs {
    let mut doc = Document::new();
    doc.add(StringField::from_string(
      "field",
      i.to_string(),
      Store::Yes,
    )?);
    writer.add_document(doc)?;
  }
  writer.close()?;

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let failed = Arc::new(AtomicBool::new(false));
  let cms = ConcurrentMergeScheduler::new();
  cms.set_stall_on_merge_thread(failed.clone());
  cms.enable_auto_io_throttle()?;
  cms.set_max_merges_and_threads(2, 1)?;
  iwc.set_merge_scheduler(cms);
  iwc.set_max_buffered_docs(2);

  let writer = IndexWriter::new(dir, iwc)?;
  writer.force_merge(1)?;
  writer.close()?;

  assert!(!failed.load(Ordering::SeqCst));
  Ok(())
}

/*
 * This test tries to produce 2 merges running concurrently with 2 segments per merge. While these
 * merges run we kick off a forceMerge that puts a pending merge in the queue but waits for things to happen.
 * While we do this we reduce maxMergeCount to 1. If concurrency in CMS is not right the forceMerge will wait forever
 * since none of the currently running merges picks up the pending merge. This test fails every time.
 */
#[test]
fn test_change_max_merge_county_while_force_merge() -> Result<()> {
  let mut random = random();
  let num_iters = if is_night_mode() { 100 } else { 10 };
  for _ in 0..num_iters {
    let mut mp = LogMergePolicy::log_doc();
    mp.set_merge_factor(2)?;
    let force_merge_waits = Arc::new(CountDownLatch::new(1));
    let merge_threads_start_after_wait = Arc::new(CountDownLatch::new(1));
    let merge_threads_arrived = Arc::new(CountDownLatch::new(2));
    let stream = ChangeMaxMergeCountInfoStream {
      force_merge_waits: force_merge_waits.clone(),
      merge_threads_start_after_wait: merge_threads_start_after_wait.clone(),
      merge_threads_arrived: merge_threads_arrived.clone(),
    };

    let dir = new_directory_shared(&mut random)?;
    let cms = ConcurrentMergeScheduler::new();
    let writer_result = (|| -> Result<_> {
      let mut iwc = IndexWriterConfig::new()?;
      iwc.set_merge_scheduler(cms.clone());
      iwc.set_merge_policy(mp);
      iwc.set_info_stream(InfoStreamEnum::Custom(Box::new(stream)));
      IndexWriter::with_hooks(
        dir.clone(),
        iwc,
        Some(IndexWriterHooksEnum::custom(TestPointsIndexWriterHooks)),
      )
    })();
    let writer = match writer_result {
      Ok(writer) => writer,
      Err(err) => {
        let dir_close_result = match Arc::try_unwrap(dir) {
          Ok(mut dir) => dir.close(),
          Err(_) => Err(LuceneError::illegal_state(
            "directory still has outstanding references",
          )),
        };
        return IOUtils::use_or_suppress_result(Err(err), dir_close_result);
      },
    };

    let body_result = (|| -> Result<()> {
      cms.set_max_merges_and_threads(2, 2)?;

      let force_merge_thread = {
        let _release_merge_threads = CountDownOnDrop::new(merge_threads_start_after_wait.clone());
        for _ in 0..4 {
          let mut document = Document::new();
          document.add(TextField::from_string(
            "foo",
            "the quick brown fox jumps over the lazy dog",
            Store::Yes,
          )?);
          document.add(TextField::from_string(
            "bar",
            TestUtil::random_realistic_unicode_string_with_len(&mut random, 20),
            Store::Yes,
          )?);
          writer.add_document(document)?;
          writer.flush()?;
        }
        let segment_infos = writer.clone_segment_infos()?;
        assert_eq!(4, writer.get_segment_count(), "{}", segment_infos);
        merge_threads_arrived.wait();
        let writer_ref = writer.clone();
        let force_merge_thread = thread::spawn(move || writer_ref.force_merge(1));
        force_merge_waits.wait();
        cms.set_max_merges_and_threads(1, 1)?;
        force_merge_thread
      };

      while !force_merge_thread.is_finished() {
        thread::sleep(Duration::from_millis(10));
        if cms.merge_thread_count() == 0 && writer.has_pending_merges()? {
          return Err(LuceneError::illegal_state(
            "writer has pending merges but no CMS threads are running",
          ));
        }
      }
      match force_merge_thread.join() {
        Ok(result) => result?,
        Err(payload) => {
          return Err(LuceneError::tragedy_from_panic(
            "panic while force merging",
            payload.as_ref(),
          ));
        },
      }
      assert_eq!(1, writer.get_segment_count());
      Ok(())
    })();

    let close_result = writer.close();
    drop(writer);
    let dir_close_result = match Arc::try_unwrap(dir) {
      Ok(mut dir) => dir.close(),
      Err(_) => Err(LuceneError::illegal_state(
        "directory still has outstanding references",
      )),
    };
    let close_result = IOUtils::use_or_suppress_result(close_result, dir_close_result);
    IOUtils::use_or_suppress_result(body_result, close_result)?;
  }
  Ok(())
}

struct CountDownLatch {
  count: Mutex<usize>,
  condvar: Condvar,
}

impl CountDownLatch {
  fn new(count: usize) -> Self {
    Self {
      count: Mutex::new(count),
      condvar: Condvar::new(),
    }
  }

  fn count_down(&self) {
    let mut count = self.count.lock().expect("latch mutex poisoned");
    if *count > 0 {
      *count -= 1;
      if *count == 0 {
        self.condvar.notify_all();
      }
    }
  }

  fn wait(&self) {
    let mut count = self.count.lock().expect("latch mutex poisoned");
    while *count > 0 {
      count = self.condvar.wait(count).expect("latch mutex poisoned");
    }
  }
}

struct CountDownOnDrop {
  latch: Arc<CountDownLatch>,
}

impl CountDownOnDrop {
  fn new(latch: Arc<CountDownLatch>) -> Self {
    Self { latch }
  }
}

impl Drop for CountDownOnDrop {
  fn drop(&mut self) {
    self.latch.count_down();
  }
}

struct ChangeMaxMergeCountInfoStream {
  force_merge_waits: Arc<CountDownLatch>,
  merge_threads_start_after_wait: Arc<CountDownLatch>,
  merge_threads_arrived: Arc<CountDownLatch>,
}

impl CloseableRef for ChangeMaxMergeCountInfoStream {
  fn close(&self) -> Result<()> {
    Ok(())
  }
}

impl InfoStream for ChangeMaxMergeCountInfoStream {
  fn message(&self, component: &str, message: &str) -> Result<()> {
    if component == "TP" {
      if message == "mergeMiddleStart" {
        self.merge_threads_arrived.count_down();
        self.merge_threads_start_after_wait.wait();
      } else if message == "forceMergeBeforeWait" {
        self.force_merge_waits.count_down();
      }
    }
    Ok(())
  }

  fn is_enabled(&self, component: &str) -> bool {
    component == "TP"
  }
}

struct TestPointsIndexWriterHooks;

impl IndexWriterHooks for TestPointsIndexWriterHooks {
  fn is_enable_test_points(&self) -> bool {
    true
  }
}
