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
use crate::core::index::documents_writer_flush_control::{DocumentsWriterFlushControl, Inner};

use crate::core::index::documents_writer_per_thread::DocumentsWriterPerThread;
#[cfg(test)]
use crate::core::index::documents_writer_per_thread_pool::DwptWrapper;
use crate::core::index::flush_policy::FlushPolicy;
use crate::core::index::index_writer_config::DISABLE_AUTO_FLUSH;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use parking_lot::MutexGuard;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

/// Default [`FlushPolicy`] implementation that flushes new segments based on RAM usage and
/// document count, depending on the `IndexWriter`'s [`IndexWriterConfig`](crate::core::index::index_writer_config::IndexWriterConfig).
/// It also applies pending deletes based on the number of buffered delete terms.
///
/// All [`IndexWriterConfig`](crate::core::index::index_writer_config::IndexWriterConfig) settings are used to mark [`DocumentsWriterPerThread`] as
/// flush-pending during indexing with respect to their live updates.
///
/// If [`IndexWriterConfig::set_ram_buffer_size_mb`](crate::core::index::index_writer_config::IndexWriterConfig::set_ram_buffer_size_mb) is enabled, the largest RAM-consuming
/// [`DocumentsWriterPerThread`] will be marked as pending **iff** the global active RAM consumption
/// is `>=` the configured max RAM buffer.
pub struct FlushByRamOrCountsPolicy;
impl Default for FlushByRamOrCountsPolicy {
  fn default() -> Self {
    Self::new()
  }
}

impl FlushByRamOrCountsPolicy {
  pub fn new() -> Self {
    Self {}
  }
}

impl FlushByRamOrCountsPolicy {
  fn flush_deletes<D>(&self, control: &DocumentsWriterFlushControl<D>) -> Result<()>
  where
    D: Directory,
  {
    control.set_apply_all_deletes();

    Ok(())
  }
  fn flush_active_bytes<D, L>(
    &self,
    control: &DocumentsWriterFlushControl<D>,
    per_thread: &DocumentsWriterPerThread<D>,
    inner: &mut Inner<D>,
    config: &L,
  ) -> Result<()>
  where
    D: Directory,
    L: LiveIndexWriterConfig,
  {
    self.mark_largest_writer_pending(control, per_thread, inner, config)?;
    Ok(())
  }
  /// Marks the most ram consuming active [`DocumentsWriterPerThread`] flush pending
  pub(crate) fn mark_largest_writer_pending<D, L>(
    &self,
    control: &DocumentsWriterFlushControl<D>,
    per_thread: &DocumentsWriterPerThread<D>,
    inner: &mut Inner<D>,
    config: &L,
  ) -> Result<()>
  where
    D: Directory,
    L: LiveIndexWriterConfig,
  {
    let largest_non_pending_writer =
      self.find_largest_non_pending_writer_for_thread(control, per_thread);
    if let Some(largest_non_pendingwriter) = largest_non_pending_writer {
      // If the found instance is itself, then use the `per_thread` parameter; otherwise, it may cause a deadlock.
      let v = if Arc::ptr_eq(&largest_non_pendingwriter.state, &per_thread.state) {
        per_thread
      } else {
        &*largest_non_pendingwriter.dwpt.lock()
      };
      control.set_flush_pending(v, Some(inner), config)?;
    }
    Ok(())
  }
  /// Returns `true` if this [`FlushPolicy`](crate::core::index::flush_policy::FlushPolicy) flushes on
  /// [`LiveIndexWriterConfig::get_max_buffered_docs`], otherwise `false`.
  fn flush_on_doc_count<L>(&self, index_writer_config: &L) -> bool
  where
    L: LiveIndexWriterConfig,
  {
    index_writer_config.get_max_buffered_docs() != DISABLE_AUTO_FLUSH
  }

  /// Returns `true` if this [`FlushPolicy`](crate::core::index::flush_policy::FlushPolicy) flushes on
  /// [`LiveIndexWriterConfig::get_ram_buffer_size_mb`], otherwise `false`.
  fn flush_on_ram<L>(&self, index_writer_config: &L) -> bool
  where
    L: LiveIndexWriterConfig,
  {
    index_writer_config.get_ram_buffer_size_mb() != DISABLE_AUTO_FLUSH as f64
  }
}
impl FlushPolicy for FlushByRamOrCountsPolicy {
  fn on_change<D, L>(
    &self,
    control: &DocumentsWriterFlushControl<D>,
    inner: &mut Inner<D>,
    per_thread: Option<&MutexGuard<'_, DocumentsWriterPerThread<D>>>,
    config: &L,
  ) -> Result<()>
  where
    D: Directory,
    L: LiveIndexWriterConfig,
  {
    let index_writer_config = config;
    if let Some(pt) = per_thread
      && self.flush_on_doc_count(index_writer_config)
      && pt.get_num_docs_in_ram() >= index_writer_config.get_max_buffered_docs()
    {
      // Flush this state by num docs
      control.set_flush_pending(pt, Some(inner), config)?;
      return Ok(());
    }

    if self.flush_on_ram(index_writer_config) {
      let limit = (index_writer_config.get_ram_buffer_size_mb() * 1024.0 * 1024.0) as i64;
      let active_ram = control.active_bytes(Some(inner));
      let deletes_ram = control.get_delete_bytes_used()?;

      if deletes_ram >= limit
        && active_ram >= limit
        && let Some(pt) = per_thread
      {
        self.flush_deletes(control)?;
        self.flush_active_bytes(control, pt, inner, config)?;
        return Ok(());
      }

      if deletes_ram >= limit {
        self.flush_deletes(control)?;
      } else if active_ram + deletes_ram >= limit
        && let Some(pt) = per_thread
      {
        self.flush_active_bytes(control, pt, inner, config)?;
      }
    }
    Ok(())
  }
}
#[cfg(test)]
pub mod tests {
  use super::*;
  use crate::core::index::directory_reader;
  use crate::core::index::flush_policy::FlushPolicyEnum;
  use crate::core::index::index_reader::IndexReader;
  use crate::core::index::index_writer::{IndexWriter, IndexWriterBase, MAX_TERM_LENGTH};
  use crate::core::index::two_phase_commit::TwoPhaseCommit;
  use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
  use crate::test::core::util::line_file_docs::LineFileDocs;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    at_least, is_night_mode, new_directory_shared, new_index_writer_config_with_analyzer, random,
    random_from_seed, rarely,
  };
  use crate::test::core::util::test_util::TestUtil;
  use rand::RngExt;
  use std::sync::Arc;
  use std::sync::atomic::AtomicI32;
  use std::thread;

  #[allow(dead_code)] // for quick search
  struct TestFlushByRamOrCountsPolicy;

  // TODO: memory calculation not implement
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

  // TODO: memory calculation not implement
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

    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
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
    assert_active_bytes_after(flush_control);
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
      let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
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
      assert_active_bytes_after(flush_control);
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
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
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
    assert_active_bytes_after(flush_control);
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

  #[allow(dead_code)]
  pub struct MockDefaultFlushPolicy {
    peak_bytes_without_flush: AtomicI64,
    peak_doc_count_without_flush: AtomicI64,
    has_marked_pending: AtomicBool,
    base: FlushByRamOrCountsPolicy,
  }

  impl MockDefaultFlushPolicy {
    pub fn new() -> Self {
      Self {
        peak_bytes_without_flush: AtomicI64::new(i32::MIN as i64),
        peak_doc_count_without_flush: AtomicI64::new(i32::MIN as i64),
        has_marked_pending: AtomicBool::new(false),
        base: FlushByRamOrCountsPolicy::new(),
      }
    }
  }

  impl Default for MockDefaultFlushPolicy {
    fn default() -> Self {
      Self::new()
    }
  }

  impl FlushPolicy for MockDefaultFlushPolicy {
    fn on_change<D, L>(
      &self,
      control: &DocumentsWriterFlushControl<D>,
      inner: &mut Inner<D>,
      per_thread: Option<&MutexGuard<'_, DocumentsWriterPerThread<D>>>,
      config: &L,
    ) -> Result<()>
    where
      D: Directory,
      L: LiveIndexWriterConfig,
    {
      let Some(dwpt) = per_thread else {
        unreachable!("");
      };

      let mut pending = Vec::new();
      let mut not_pending = Vec::new();
      find_pending(control, &mut pending, &mut not_pending);

      let flush_current = dwpt.is_flush_pending();
      let active_bytes = control.active_bytes(Some(inner));
      let to_flush = if flush_current {
        find_dwpt(&pending, &dwpt.state.id)
      } else if self.base.flush_on_doc_count(config)
        && dwpt.get_num_docs_in_ram() >= config.get_max_buffered_docs()
      {
        find_dwpt(&not_pending, &dwpt.state.id)
      } else if self.base.flush_on_ram(config)
        && active_bytes >= (config.get_ram_buffer_size_mb() * 1024.0 * 1024.0) as i64
      {
        let to_flush = self
          .base
          .find_largest_non_pending_writer_for_thread(control, dwpt);
        if let Some(to_flush) = to_flush {
          assert!(!to_flush.state.is_flush_pending());
          Some(to_flush)
        } else {
          None
        }
      } else {
        None
      };

      self.base.on_change(control, inner, Some(dwpt), config)?;

      if let Some(to_flush) = to_flush {
        let list = if flush_current {
          &mut pending
        } else {
          &mut not_pending
        };
        let pos = list
          .iter()
          .position(|dwpt| Arc::ptr_eq(dwpt, &to_flush) || dwpt.state.id == to_flush.state.id)
          .expect("expected DWPT in pending snapshot");
        list.remove(pos);
        assert!(to_flush.state.is_flush_pending());
        self.has_marked_pending.store(true, Ordering::SeqCst);
      } else {
        self
          .peak_bytes_without_flush
          .fetch_max(active_bytes, Ordering::SeqCst);
        self
          .peak_doc_count_without_flush
          .fetch_max(dwpt.get_num_docs_in_ram() as i64, Ordering::SeqCst);
      }

      for per_thread in not_pending {
        assert!(!per_thread.state.is_flush_pending());
      }

      Ok(())
    }
  }

  fn find_pending<D>(
    flush_control: &DocumentsWriterFlushControl<D>,
    pending: &mut Vec<Arc<DwptWrapper<D>>>,
    not_pending: &mut Vec<Arc<DwptWrapper<D>>>,
  ) where
    D: Directory,
  {
    for (_id, next) in flush_control.per_thread_pool.iterator() {
      if next.state.is_flush_pending() {
        pending.push(next);
      } else {
        not_pending.push(next);
      }
    }
  }

  fn find_dwpt<D>(writers: &[Arc<DwptWrapper<D>>], state_id: &str) -> Option<Arc<DwptWrapper<D>>>
  where
    D: Directory,
  {
    writers
      .iter()
      .find(|dwpt| dwpt.state.id == state_id)
      .cloned()
  }

  fn index_thread<D, B>(
    seed: u64,
    pending_docs: &AtomicI32,
    writer: &IndexWriter<D, B>,
    do_random_commit: bool,
  ) -> Result<()>
  where
    D: Directory,
    B: IndexWriterBase,
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

  fn assert_active_bytes_after<D>(flush_control: &DocumentsWriterFlushControl<D>)
  where
    D: Directory,
  {
    let mut _bytes_used = 0;
    for (_id, next) in flush_control.per_thread_pool.iterator() {
      if !next.state.is_flush_pending() {
        _bytes_used += next.state.get_last_committed_bytes_used();
      }
    }
    // TODO: memory calculation not implement
    // assert_eq!(bytes_used, flush_control.active_bytes(None));
  }
}
