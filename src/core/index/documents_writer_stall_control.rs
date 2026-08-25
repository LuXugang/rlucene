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
use parking_lot::{Condvar, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::ThreadId;
use std::time::Duration;
/// Controls the health status of [`DocumentsWriter`](crate::core::index::documents_writer::DocumentsWriter) sessions. This struct blocks
/// incoming indexing threads if flushing is significantly slower than indexing to ensure the
/// [`DocumentsWriter`](crate::core::index::documents_writer::DocumentsWriter)’s healthiness. If flushing is significantly slower than indexing, the net
/// memory used within an [`IndexWriter`](crate::core::index::index_writer::IndexWriter) session can increase quickly and exhaust available memory.
///
/// To prevent OOM errors and ensure [`IndexWriter`](crate::core::index::index_writer::IndexWriter)’s stability, this struct blocks incoming threads
/// from indexing once 2× the number of available [`DocumentsWriterPerThread`](crate::core::index::documents_writer_per_thread::DocumentsWriterPerThread)s in
/// [`DocumentsWriterPerThreadPool`](crate::core::index::documents_writer_per_thread_pool::DocumentsWriterPerThreadPool) is exceeded. Once flushing catches up and the number of flushing
/// DWPTs is equal to or lower than the number of active `DocumentsWriterPerThread`s, threads are
/// released and can continue indexing.
pub(crate) struct DocumentsWriterStallControl {
  inner: Mutex<State>,
  pausing: Condvar,
  stalled: AtomicBool,
}
pub(crate) struct State {
  // only with assert
  num_waiting: i32,
  // only with assert
  was_stalled: bool,
  // only with assert
  waiting: HashMap<ThreadId, bool>,
}

impl DocumentsWriterStallControl {
  pub(crate) fn new() -> Self {
    Self {
      inner: Mutex::new(State {
        num_waiting: 0,
        was_stalled: false,
        waiting: HashMap::new(),
      }),
      pausing: Condvar::new(),
      stalled: AtomicBool::new(false),
    }
  }
  /// Updates the stalled flag status.
  /// Sets the stalled flag to `true` if the number of flushing [`DocumentsWriterPerThread`](crate::core::index::documents_writer_per_thread::DocumentsWriterPerThread)s
  /// exceeds the number of active [`DocumentsWriterPerThread`](crate::core::index::documents_writer_per_thread::DocumentsWriterPerThread)s. Otherwise, resets the
  /// [`DocumentsWriterStallControl`] to healthy and releases all threads waiting on
  /// [`wait_if_stalled()`](Self::wait_if_stalled).
  pub(crate) fn update_stalled(&self, stalled: bool) {
    let mut st = self.inner.lock();
    let prev = self.stalled.load(Ordering::SeqCst);
    if prev != stalled {
      self.stalled.store(stalled, Ordering::SeqCst);
      if stalled {
        st.was_stalled = true;
      }
      self.pausing.notify_all();
    }
  }
  pub(crate) fn wait_if_stalled(&self) {
    if self.stalled.load(Ordering::SeqCst) {
      let mut st = self.inner.lock();
      if self.stalled.load(Ordering::SeqCst) {
        self.inc_waiters(&mut st);
        let _ = self.pausing.wait_for(&mut st, Duration::from_millis(1000));

        self.decr_waiters(&mut st);
      }
    }
  }
  pub(crate) fn any_stalled_threads(&self) -> bool {
    self.stalled.load(Ordering::SeqCst)
  }
  fn inc_waiters(&self, st: &mut State) {
    st.num_waiting += 1;
    debug_assert!(st.waiting.insert(thread::current().id(), true).is_none());
    debug_assert!(st.num_waiting > 0);
  }

  fn decr_waiters(&self, st: &mut State) {
    st.num_waiting -= 1;
    debug_assert!(st.waiting.remove(&thread::current().id()).is_some());
    debug_assert!(st.num_waiting >= 0);
  }
  #[cfg(test)]
  pub(crate) fn has_blocked(&self) -> bool {
    let st = self.inner.lock();
    st.num_waiting > 0
  }

  #[cfg(test)]
  pub(crate) fn get_num_waiting(&self) -> i32 {
    let st = self.inner.lock();
    st.num_waiting
  }

  #[cfg(test)]
  pub(crate) fn is_healthy(&self) -> bool {
    !self.stalled.load(Ordering::SeqCst)
  }

  #[cfg(test)]
  pub(crate) fn is_thread_queued(&self, tid: &ThreadId) -> bool {
    let st = self.inner.lock();
    st.waiting.contains_key(tid)
  }

  #[cfg(test)]
  pub(crate) fn was_stalled(&self) -> bool {
    let st = self.inner.lock();
    st.was_stalled
  }
}
