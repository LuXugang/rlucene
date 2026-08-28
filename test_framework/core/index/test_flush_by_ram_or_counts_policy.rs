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
use crate::core::index::documents_writer_per_thread_pool::DwptWrapper;
use crate::core::index::flush_by_ram_or_counts_policy::FlushByRamOrCountsPolicy;
use crate::core::index::flush_policy::FlushPolicy;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::store::directory::Directory;
use parking_lot::MutexGuard;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

#[allow(dead_code)]
pub struct MockDefaultFlushPolicy {
  pub peak_bytes_without_flush: AtomicI64,
  pub peak_doc_count_without_flush: AtomicI64,
  pub has_marked_pending: AtomicBool,
  pub base: FlushByRamOrCountsPolicy,
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
  ) -> crate::core::util::error::lucene_error::Result<()>
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
        .find_largest_non_pending_writer_for_thread(control, inner, dwpt)?;
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
fn find_dwpt<D>(writers: &[Arc<DwptWrapper<D>>], state_id: &str) -> Option<Arc<DwptWrapper<D>>>
where
  D: Directory,
{
  writers
    .iter()
    .find(|dwpt| dwpt.state.id == state_id)
    .cloned()
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
