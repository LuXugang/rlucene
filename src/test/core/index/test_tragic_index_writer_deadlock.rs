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
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use rand::rngs::StdRng;

use crate::core::document::document::Document;
use crate::core::index::concurrent_merge_scheduler::{
  ConcurrentMergeScheduler, ConcurrentMergeSchedulerHook,
};
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexWriter, IndexWriterHooksEnum};
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::merge_scheduler::MergeSchedulerEnum;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::index::suppressing_concurrent_merge_scheduler::SuppressingConcurrentMergeScheduler;
use crate::test_framework::core::index::test_concurrent_merge_scheduler::CountDownLatch;
use crate::test_framework::core::index::test_tragic_index_writer_deadlock::{
  StalledMergesConcurrentMergeScheduler, TragicIndexWriter,
};
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, new_mock_directory, random,
};
#[allow(dead_code)] // for quick search
struct TestTragicIndexWriterDeadlock;

#[test]
fn test_deadlock_exc_nrt_reader_commit() -> Result<()> {
  let mut random = random();
  let dir = Arc::new(new_mock_directory(&mut random)?);
  let mut iwc = new_index_writer_config(&mut random)?;
  if matches!(iwc.get_merge_scheduler(), MergeSchedulerEnum::Concurrent(_)) {
    iwc.set_merge_scheduler(ConcurrentMergeScheduler::with_hook(
      ConcurrentMergeSchedulerHook::Suppressing(SuppressingConcurrentMergeScheduler::all()),
    ));
  }
  let w = IndexWriter::new(dir.clone(), iwc)?;
  let starting_gun = CountDownLatch::new(1);
  let done = Arc::new(AtomicBool::new(false));
  let commit_thread = {
    let starting_gun = starting_gun.clone();
    let done = done.clone();
    let w = w.clone();
    thread::spawn(move || {
      let result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
        starting_gun.wait();
        while !done.load(Ordering::SeqCst) {
          w.add_document(Document::new())?;
          w.commit()?;
        }
        Ok(())
      }));
      if !matches!(result, Ok(Ok(()))) {
        done.store(true, Ordering::SeqCst);
      }
      // println!("commit exc:");
      // Inspect `result` here for the captured error or panic.
    })
  };
  let r0 = directory_reader::open_from_writer(&w)?;
  let nrt_thread = {
    let starting_gun = starting_gun.clone();
    let done = done.clone();
    thread::spawn(move || {
      let result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
        let mut r = r0;
        let body_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
          starting_gun.wait();
          while !done.load(Ordering::SeqCst) {
            if let Some(r2) = directory_reader::open_if_changed(&r)? {
              let old_reader = std::mem::replace(&mut r, r2);
              old_reader.dec_ref()?;
            }
          }
          Ok(())
        }));
        let close_result = r.close();
        close_result?;
        match body_result {
          Ok(result) => result,
          Err(payload) => resume_unwind(payload),
        }
      }));
      if !matches!(result, Ok(Ok(()))) {
        done.store(true, Ordering::SeqCst);
      }
      // println!("nrt exc:");
      // Inspect `result` here for the captured error or panic.
    })
  };
  dir.set_random_io_exception_rate(0.1);
  starting_gun.count_down();
  let _ = commit_thread.join();
  let _ = nrt_thread.join();
  dir.set_random_io_exception_rate(0.0);
  w.close()?;
  dir.close()?;
  Ok(())
}

// LUCENE-7570
#[test]
fn test_deadlock_stalled_merges() -> Result<()> {
  let mut random = random();
  do_test_deadlock_stalled_merges(false, &mut random)
}

#[test]
fn test_deadlock_stalled_full_flush_merges() -> Result<()> {
  let mut random = random();
  do_test_deadlock_stalled_merges(true, &mut random)
}

fn do_test_deadlock_stalled_merges(merge_on_flush: bool, random: &mut StdRng) -> Result<()> {
  let dir = new_directory_shared(random)?;
  let mut iwc = IndexWriterConfig::new()?;
  iwc.set_max_full_flush_merge_wait_millis(if merge_on_flush { 1000 } else { 0 });

  // So we merge every 2 segments:
  let mut mp = LogMergePolicy::log_doc();
  mp.set_merge_factor(2)?;
  iwc.set_merge_policy(mp);
  let done = CountDownLatch::new(1);
  let cms = ConcurrentMergeScheduler::with_hook(ConcurrentMergeSchedulerHook::StalledMerges(
    StalledMergesConcurrentMergeScheduler::new(done),
  ));

  // So we stall once the 2nd merge wants to run:
  cms.set_max_merges_and_threads(1, 1)?;
  iwc.set_merge_scheduler(cms);

  // So we write a segment every 2 indexed docs:
  iwc.set_max_buffered_docs(2);

  let w = IndexWriter::with_hooks(
    dir.clone(),
    iwc,
    Some(IndexWriterHooksEnum::TestTragicIndexWriterDeadlock(
      TragicIndexWriter,
    )),
  )?;

  w.add_document(Document::new())?;
  w.add_document(Document::new())?;
  // w writes first segment
  w.add_document(Document::new())?;
  w.add_document(Document::new())?;
  // w writes second segment, and kicks off merge, that takes forever (done.await)
  w.add_document(Document::new())?;
  w.add_document(Document::new())?;
  // w writes third segment
  w.add_document(Document::new())?;
  let error = w.commit().expect_err("commit should fail after a tragedy");
  assert!(matches!(error, LuceneError::IllegalState(_)), "{error}");
  assert!(
    error
      .to_string()
      .starts_with("this writer hit an unrecoverable error"),
    "{error}"
  );
  // w writes fourth segment, and commit flushes and kicks off merge that stalls
  w.close()?;
  dir.close()?;
  Ok(())
}
