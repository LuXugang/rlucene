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
use crate::core::index::flush_policy::FlushPolicy;
use crate::core::index::index_writer_config::DISABLE_AUTO_FLUSH;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use parking_lot::MutexGuard;
/// Default flushing implementation that writes new segments based on RAM usage and document count,
/// depending on the `IndexWriter`'s
/// [`IndexWriterConfig`](crate::core::index::index_writer_config::IndexWriterConfig).
/// It also applies pending deletes based on the number of buffered delete terms.
///
/// All [`IndexWriterConfig`](crate::core::index::index_writer_config::IndexWriterConfig) settings
/// are used to mark [`IndexWriter`](crate::core::index::index_writer::IndexWriter) per-thread
/// indexing buffers as flush-pending with respect to their live updates.
///
/// If
/// [`IndexWriterConfig::set_ram_buffer_size_mb`](crate::core::index::index_writer_config::IndexWriterConfig::set_ram_buffer_size_mb)
/// is enabled, the largest per-thread indexing buffer is marked as pending **iff** global active
/// RAM consumption is `>=` the configured maximum RAM buffer.
pub struct FlushByRamOrCountsPolicy;
impl Default for FlushByRamOrCountsPolicy {
  fn default() -> Self {
    Self::new()
  }
}

impl FlushByRamOrCountsPolicy {
  pub(crate) fn new() -> Self {
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
      self.find_largest_non_pending_writer_for_thread(control, per_thread)?;
    if let Some(largest_non_pending_writer) = largest_non_pending_writer {
      control.set_flush_pending(&largest_non_pending_writer.state, Some(inner), config)?;
    }
    Ok(())
  }
  /// Returns `true` if this [`FlushByRamOrCountsPolicy`] flushes on
  /// [`LiveIndexWriterConfig::get_max_buffered_docs`], otherwise `false`.
  pub(crate) fn flush_on_doc_count<L>(&self, index_writer_config: &L) -> bool
  where
    L: LiveIndexWriterConfig,
  {
    index_writer_config.get_max_buffered_docs() != DISABLE_AUTO_FLUSH
  }

  /// Returns `true` if this [`FlushByRamOrCountsPolicy`] flushes on
  /// [`LiveIndexWriterConfig::get_ram_buffer_size_mb`], otherwise `false`.
  pub(crate) fn flush_on_ram<L>(&self, index_writer_config: &L) -> bool
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
      control.set_flush_pending(&pt.state, Some(inner), config)?;
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
