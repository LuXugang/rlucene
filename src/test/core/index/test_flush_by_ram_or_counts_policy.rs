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
use crate::core::index::directory_reader;
use crate::core::index::documents_writer_flush_control::DocumentsWriterFlushControl;
use crate::core::index::flush_policy::FlushPolicyEnum;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexWriter, MAX_TERM_LENGTH};
use crate::core::index::index_writer_config::DISABLE_AUTO_FLUSH;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::Directory;
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::analysis::mock_analyzer::MockAnalyzer;
pub use crate::test::support::core::index::misc::MockDefaultFlushPolicy;
use crate::test::support::core::util::line_file_docs::LineFileDocs;
use crate::test::support::core::util::lucene_test_case::{
  at_least, is_night_mode, new_directory_shared, new_index_writer_config_with_analyzer, random,
  random_from_seed, rarely,
};
use crate::test::support::core::util::test_util::TestUtil;
use rand::RngExt;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;

#[allow(dead_code)] // for quick search
struct TestFlushByRamOrCountsPolicy;

#[test]
fn test_flush_by_ram() -> Result<()> {
  let mut random = random();
  let ram_buffer = (if is_night_mode() { 1.0 } else { 10.0 })
    + at_least(&mut random, 2) as f64
    + random.random::<f64>();
  run_flush_by_ram(
    1 + random.random_range(0..if is_night_mode() { 5 } else { 1 }),
    ram_buffer,
    false,
  )
}

#[test]
fn test_flush_by_ram_large_buffer() -> Result<()> {
  let mut random = random();
  // with a 256 mb ram buffer we should never stall
  run_flush_by_ram(
    1 + random.random_range(0..if is_night_mode() { 5 } else { 1 }),
    256.0,
    true,
  )
}

fn run_flush_by_ram(num_threads: i32, max_ram_mb: f64, ensure_not_stalled: bool) -> Result<()> {
  let mut random = random();
  let num_documents_to_index = 10 + at_least(&mut random, 30);
  let num_docs = AtomicI32::new(num_documents_to_index);
  let dir = new_directory_shared(&mut random)?;
  let flush_policy = MockDefaultFlushPolicy::new();
  let mut analyzer = MockAnalyzer::new(&mut random);
  analyzer.set_max_token_length(TestUtil::next_int(&mut random, 1, MAX_TERM_LENGTH));

  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwc.set_flush_policy(flush_policy);
  iwc.set_ram_buffer_size_mb(max_ram_mb);
  iwc.set_max_buffered_docs(DISABLE_AUTO_FLUSH);
  let writer = IndexWriter::new(dir, iwc)?;
  let flush_policy = match writer.get_config().get_flush_policy() {
    FlushPolicyEnum::MockDefault(flush_policy) => flush_policy,
    _ => unreachable!("expected MockDefaultFlushPolicy"),
  };
  assert!(!flush_policy.base.flush_on_doc_count(writer.get_config()));
  assert!(flush_policy.base.flush_on_ram(writer.get_config()));
  let docs_writer = writer.get_docs_writer();
  let flush_control = &docs_writer.flush_control;
  assert_eq!(
    0,
    docs_writer.get_flushing_bytes(),
    " bytes must be 0 after init"
  );
  let seed = random.random();

  thread::scope(|scope| -> Result<()> {
    let mut handles = Vec::new();
    for _ in 0..num_threads {
      handles.push(scope.spawn(|| index_thread(seed, &num_docs, &writer, false)));
    }

    for handle in handles {
      handle.join().expect("thread panicked")?;
    }
    Ok(())
  })?;

  let max_ram_bytes = (writer.get_config().get_ram_buffer_size_mb() * 1024.0 * 1024.0) as i64;
  assert_eq!(
    0,
    docs_writer.get_flushing_bytes(),
    " all flushes must be due numThreads={}",
    num_threads
  );
  let doc_stats = writer.get_doc_stats()?;
  assert_eq!(num_documents_to_index, doc_stats.num_docs);
  assert_eq!(num_documents_to_index, doc_stats.max_doc);
  assert!(
    flush_policy.peak_bytes_without_flush.load(Ordering::SeqCst) <= max_ram_bytes,
    "peak bytes without flush exceeded watermark"
  );
  assert_active_bytes_after(flush_control)?;
  if flush_policy.has_marked_pending.load(Ordering::SeqCst) {
    assert!(max_ram_bytes < flush_control.get_peak_active_bytes());
  }
  if ensure_not_stalled {
    assert!(!flush_control.was_stalled());
  }
  writer.close()?;
  assert_eq!(0, flush_control.active_bytes(None));
  Ok(())
}
#[test]
fn test_flush_doc_count() -> Result<()> {
  let mut random = random();
  let num_threads = [2 + at_least(&mut random, 1), 1];
  for num_threads in num_threads {
    let mut analyzer = MockAnalyzer::new(&mut random);
    analyzer.set_max_token_length(TestUtil::next_int(&mut random, 1, MAX_TERM_LENGTH));
    let num_documents_to_index = 50 + at_least(&mut random, 30);
    let num_docs = AtomicI32::new(num_documents_to_index);
    let dir = new_directory_shared(&mut random)?;
    let flush_policy = MockDefaultFlushPolicy::new();
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
    iwc.set_flush_policy(flush_policy);

    iwc.set_max_buffered_docs(2 + at_least(&mut random, 10));
    iwc.set_ram_buffer_size_mb(DISABLE_AUTO_FLUSH as f64);
    let writer = IndexWriter::new(dir, iwc)?;
    let flush_policy = match writer.get_config().get_flush_policy() {
      FlushPolicyEnum::MockDefault(flush_policy) => flush_policy,
      _ => unreachable!("expected MockDefaultFlushPolicy"),
    };
    assert!(flush_policy.base.flush_on_doc_count(writer.get_config()));
    assert!(!flush_policy.base.flush_on_ram(writer.get_config()));
    let docs_writer = writer.get_docs_writer();
    let flush_control = &docs_writer.flush_control;
    assert_eq!(
      0,
      docs_writer.get_flushing_bytes(),
      " bytes must be 0 after init"
    );
    let seed = random.random();

    thread::scope(|scope| -> Result<()> {
      let mut handles = Vec::new();
      for _ in 0..num_threads {
        handles.push(scope.spawn(|| index_thread(seed, &num_docs, &writer, false)));
      }

      for handle in handles {
        handle.join().expect("thread panicked")?;
      }
      Ok(())
    })?;

    assert_eq!(
      0,
      docs_writer.get_flushing_bytes(),
      " all flushes must be due numThreads={}",
      num_threads
    );
    let doc_stats = writer.get_doc_stats()?;
    assert_eq!(num_documents_to_index, doc_stats.num_docs);
    assert_eq!(num_documents_to_index, doc_stats.max_doc);
    assert!(
      flush_policy
        .peak_doc_count_without_flush
        .load(Ordering::SeqCst)
        <= writer.get_config().get_max_buffered_docs() as i64,
      "peak bytes without flush exceeded watermark"
    );
    assert_active_bytes_after(flush_control)?;
    writer.close()?;
    assert_eq!(0, flush_control.active_bytes(None));
  }
  Ok(())
}

#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let num_threads = 1 + random.random_range(0..8);
  let num_documents_to_index = 50 + at_least(&mut random, 70);
  let num_docs = AtomicI32::new(num_documents_to_index);
  let dir = new_directory_shared(&mut random)?;
  let mut analyzer = MockAnalyzer::new(&mut random);
  analyzer.set_max_token_length(TestUtil::next_int(&mut random, 1, MAX_TERM_LENGTH));
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let flush_policy = MockDefaultFlushPolicy::new();
  iwc.set_flush_policy(flush_policy);

  let writer = IndexWriter::new(dir, iwc)?;
  let flush_policy = match writer.get_config().get_flush_policy() {
    FlushPolicyEnum::MockDefault(flush_policy) => flush_policy,
    _ => unreachable!("expected MockDefaultFlushPolicy"),
  };
  let docs_writer = writer.get_docs_writer();
  let flush_control = &docs_writer.flush_control;

  assert_eq!(
    0,
    docs_writer.get_flushing_bytes(),
    " bytes must be 0 after init"
  );
  let seed = random.random();

  thread::scope(|scope| -> Result<()> {
    let mut handles = Vec::new();
    for _ in 0..num_threads {
      handles.push(scope.spawn(|| index_thread(seed, &num_docs, &writer, true)));
    }

    for handle in handles {
      handle.join().expect("thread panicked")?;
    }
    Ok(())
  })?;

  assert_eq!(
    0,
    docs_writer.get_flushing_bytes(),
    " all flushes must be due"
  );
  let doc_stats = writer.get_doc_stats()?;
  assert_eq!(num_documents_to_index, doc_stats.num_docs);
  assert_eq!(num_documents_to_index, doc_stats.max_doc);
  if flush_policy.base.flush_on_ram(writer.get_config())
    && !flush_policy.base.flush_on_doc_count(writer.get_config())
  {
    let max_ram_bytes = (writer.get_config().get_ram_buffer_size_mb() * 1024.0 * 1024.0) as i64;
    assert!(
      flush_policy.peak_bytes_without_flush.load(Ordering::SeqCst) <= max_ram_bytes,
      "peak bytes without flush exceeded watermark"
    );
    if flush_policy.has_marked_pending.load(Ordering::SeqCst) {
      assert!(
        max_ram_bytes <= flush_control.get_peak_active_bytes(),
        "max: {} {}",
        max_ram_bytes,
        flush_control.get_peak_active_bytes()
      );
    }
  }
  assert_active_bytes_after(flush_control)?;
  writer.commit()?;
  assert_eq!(0, flush_control.active_bytes(None));
  let reader = directory_reader::open_from_writer(&writer)?;
  assert_eq!(num_documents_to_index, reader.num_docs()?);
  assert_eq!(num_documents_to_index, reader.max_doc()?);
  if !flush_policy.base.flush_on_ram(writer.get_config()) {
    assert!(
      !flush_control.was_stalled(),
      "never stall if we don't flush on RAM"
    );
    assert!(
      !flush_control.has_blocked(),
      "never block if we don't flush on RAM"
    );
  }
  reader.close()?;
  writer.close()?;
  Ok(())
}

#[test]
fn test_stall_control() -> Result<()> {
  // TODO IMPORTANT MockDirectoryWrapper未实现
  Ok(())
}

fn index_thread<D>(
  seed: u64,
  pending_docs: &AtomicI32,
  writer: &IndexWriter<D>,
  do_random_commit: bool,
) -> Result<()>
where
  D: Directory + 'static,
{
  let mut random = random_from_seed(seed);
  let mut docs = LineFileDocs::new(&mut random)?;
  loop {
    let remaining = pending_docs.fetch_sub(1, Ordering::SeqCst) - 1;
    if remaining < 0 {
      break;
    }
    let doc = docs.next_doc()?;
    writer.add_document(doc)?;
    if do_random_commit && rarely(&mut random) {
      writer.commit()?;
    }
  }
  writer.commit()?;
  Ok(())
}

fn assert_active_bytes_after<D>(flush_control: &DocumentsWriterFlushControl<D>) -> Result<()>
where
  D: Directory,
{
  let mut bytes_used = 0;
  for (_id, next) in flush_control.per_thread_pool.iterator() {
    if !next.state.is_flush_pending() {
      bytes_used += next.dwpt.lock().ram_bytes_used()?;
    }
  }
  assert_eq!(bytes_used, flush_control.active_bytes(None));
  Ok(())
}
