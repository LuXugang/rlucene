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
use crate::index::approximate_priority_queue::IdentityId;
use crate::index::documents_writer_per_thread::DocumentsWriterPerThread;
use crate::index::documents_writer_per_thread_pool::DocumentsWriterPerThreadPool;
use crate::index::documents_writer_stall_control::DocumentsWriterStallControl;
use crate::index::flush_policy::FlushPolicy;
use crate::index::index_writer_config::iwc_util;
use crate::index::indexable_field::IndexableField;
use crate::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::search::query::Query;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
use crate::util::info_stream::{InfoStream, InfoStreamLock};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub(crate) struct DocumentsWriterFlushControl<D, IF, Q, F, FP, L>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,
    F: Fn() -> Result<DocumentsWriterPerThread<D, IF, Q>>,
    FP: FlushPolicy,
    L: LiveIndexWriterConfig,
{
    num_docs_since_stalled: i32,
    flush_deletes: AtomicBool,
    // only with assert
    peak_active_bytes: i64,
    // only with assert
    peak_flush_bytes: i64,
    // only with assert
    peak_net_bytes: i64,
    // only with assert
    peak_delta: i64,
    info_stream: InfoStreamLock,
    lock: Mutex<Inner<D, IF, Q, F, FP>>,
    config: Arc<L>,
}
pub(crate) struct Inner<D, IF, Q, F, FP>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,
    F: Fn() -> Result<DocumentsWriterPerThread<D, IF, Q>>,
    FP: FlushPolicy,
{
    // only with assert
    flush_by_ram_was_disabled: bool,
    // only with assert
    max_configured_ram_buffer: f64,
    hard_max_bytes_per_dwpt: i64,
    active_bytes: i64,
    flush_bytes: AtomicI64,
    num_pending: AtomicI32,
    full_flush: bool,
    // only for assertion that we don't get stale DWPTs from the pool
    full_flush_mark_done: bool,
    // The flushQueue is used to concurrently distribute DWPTs that are ready to be flushed ie. when a
    // full flush is in
    // progress. This might be triggered by a commit or NRT refresh. The trigger will only walk all
    // eligible DWPTs and
    // mark them as flushable putting them in the flushQueue ready for other threads (ie. indexing
    // threads) to help flushing
    flush_queue: VecDeque<DocumentsWriterPerThread<D, IF, Q>>,
    // only for safety reasons if a DWPT is close to the RAM limit
    blocked_flushes: VecDeque<DocumentsWriterPerThread<D, IF, Q>>,
    // flushingWriters holds all currently flushing writers. There might be writers in this list that
    // are also in the flushQueue which means that writers in the flushingWriters list are not
    // necessarily
    // already actively flushing. They are only in the state of flushing and might be picked up in the
    // future by
    // polling the flushQueue
    flushing_writers: Vec<String>,
    stall_control: DocumentsWriterStallControl,
    per_thread_pool: DocumentsWriterPerThreadPool<D, IF, Q, F>,
    flush_policy: FP,
    closed: bool,
    stall_start_ns: Instant,
}

impl<D, IF, Q, F, FP, L> DocumentsWriterFlushControl<D, IF, Q, F, FP, L>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,
    F: Fn() -> Result<DocumentsWriterPerThread<D, IF, Q>>,
    FP: FlushPolicy,
    L: LiveIndexWriterConfig,
{
    pub fn active_bytes(&self) -> i64 {
        let guard = self.lock.lock();
        guard.active_bytes
    }

    pub(crate) fn flushing_bytes(&self) -> i64 {
        let guard = self.lock.lock();
        guard.flush_bytes.load(Ordering::SeqCst)
    }

    pub(crate) fn net_bytes(&self) -> i64 {
        let guard = self.lock.lock();
        guard.flush_bytes.load(Ordering::SeqCst) + guard.active_bytes
    }

    fn stall_limit_bytes(&self) -> i64 {
        let max_ram_mb = self.config.get_ram_buffer_size_mb();
        if max_ram_mb != iwc_util::DISABLE_AUTO_FLUSH as f64 {
            (2.0 * (max_ram_mb * 1024.0 * 1024.0)) as i64
        } else {
            i64::MAX
        }
    }
    fn assert_memory(&self, inner: &mut Inner<D, IF, Q, F, FP>) -> bool {
        let max_ram_mb = self.config.get_ram_buffer_size_mb();
        // We can only assert if we have always been flushing by RAM usage; otherwise the assert will
        // false trip if e.g. the
        // flush-by-doc-count * doc size was large enough to use far more RAM than the sudden change to
        // IWC's maxRAMBufferSizeMB:
        if max_ram_mb != iwc_util::DISABLE_AUTO_FLUSH as f64 && !inner.flush_by_ram_was_disabled {
            // for this assert we must be tolerant to ram buffer changes!
            inner.max_configured_ram_buffer = inner.max_configured_ram_buffer.max(max_ram_mb);
            let flush_bytes = inner.flush_bytes.load(Ordering::SeqCst);
            let active_bytes = inner.active_bytes;
            let num_pending = inner.num_pending.load(Ordering::SeqCst);

            let ram = flush_bytes + active_bytes;
            let ram_buffer_bytes = (inner.max_configured_ram_buffer * 1024.0 * 1024.0) as i64;
            // take peakDelta into account - worst case is that all flushing, pending and blocked DWPT had
            // maxMem and the last doc had the peakDelta

            // 2 * ramBufferBytes -> before we stall we need to cross the 2xRAM Buffer border this is
            // still a valid limit
            // (numPending + numFlushingDWPT() + numBlockedFlushes()) * peakDelta) -> those are the total
            // number of DWPT that are not active but not yet fully flushed
            // all of them could theoretically be taken out of the loop once they crossed the RAM buffer
            // and the last document was the peak delta
            // (numDocsSinceStalled * peakDelta) -> at any given time there could be n threads in flight
            // that crossed the stall control before we reached the limit and each of them could hold a
            // peak document
            let expected = (2 * ram_buffer_bytes)
                + ((num_pending as i64
                    + self.num_flushing_dwpt() as i64
                    + self.num_blocked_flushes() as i64)
                    * self.peak_delta)
                + (self.num_docs_since_stalled as i64 * self.peak_delta);
            // the expected ram consumption is an upper bound at this point and not really the expected
            // consumption
            if self.peak_delta < (ram_buffer_bytes >> 1) {
                /*
                 * if we are indexing with very low maxRamBuffer like 0.1MB memory can
                 * easily overflow if we check out some DWPT based on docCount and have
                 * several DWPT in flight indexing large documents (compared to the ram
                 * buffer). This means that those DWPT and their threads will not hit
                 * the stall control before asserting the memory which would in turn
                 * fail. To prevent this we only assert if the largest document seen
                 * is smaller than the 1/2 of the maxRamBufferMB
                 */
                debug_assert!(
                    ram <= expected,
                    "actual mem: {} byte, expected mem: {} byte, flush mem: {}, active mem: {}, pending DWPT: {}, flushing DWPT: {}, blocked DWPT: {}, peakDelta mem: {} bytes, ramBufferBytes={}, maxConfiguredRamBuffer={}",
                    ram,
                    expected,
                    flush_bytes,
                    active_bytes,
                    num_pending,
                    self.num_flushing_dwpt(),
                    self.num_blocked_flushes(),
                    self.peak_delta,
                    ram_buffer_bytes,
                    inner.max_configured_ram_buffer
                );
            }
        } else {
            inner.flush_by_ram_was_disabled = true;
        }
        true
    }
    pub(crate) fn num_flushing_dwpt(&self) -> usize {
        let guard = self.lock.lock();
        guard.flushing_writers.len()
    }
    /// Returns the number of flushes that are checked out but not yet available for flushing.
    /// This only applies during a full flush if a DWPT needs flushing but must not be flushed until the full flush has finished.
    pub(crate) fn num_blocked_flushes(&self) -> usize {
        let guard = self.lock.lock();
        guard.blocked_flushes.len()
    }
    // only for asserts
    fn update_peaks(&mut self, delta: i64) -> bool {
        let net = self.net_bytes();
        let guard = self.lock.lock();
        let active = guard.active_bytes;
        let flush = guard.flush_bytes.load(Ordering::SeqCst);

        self.peak_active_bytes = self.peak_active_bytes.max(active);
        self.peak_flush_bytes = self.peak_flush_bytes.max(flush);
        self.peak_net_bytes = self.peak_net_bytes.max(net);
        self.peak_delta = self.peak_delta.max(delta);

        true
    }
    /// Return the smallest number of bytes that we would like to make sure to not miss from the global RAM accounting.
    fn ram_buffer_granularity(&self) -> i64 {
        let mut ram_buffer_mb = self.config.get_ram_buffer_size_mb();
        if ram_buffer_mb == iwc_util::DISABLE_AUTO_FLUSH as f64 {
            ram_buffer_mb = self.config.get_ram_per_thread_hard_limit_mb() as f64;
        }
        // No more than ~0.1% of the RAM buffer size.
        let mut granularity = (ram_buffer_mb * 1024.0) as i64;
        // Or 16kB, so that with e.g. 64 active DWPTs, we'd never be missing more than 64*16kB = 1MB in
        // the global RAM buffer accounting.
        granularity = granularity.min(16 * 1024);
        granularity
    }
    fn checkout(
        &mut self,
        inner: &mut Inner<D, IF, Q, F, FP>,
        per_thread: DocumentsWriterPerThread<D, IF, Q>,
        mark_pending: bool,
    ) -> Result<Option<DocumentsWriterPerThread<D, IF, Q>>> {
        if inner.full_flush {
            if *per_thread.is_flush_pending() {
                self.checkout_and_block(per_thread, inner);
                match self.next_pending_flush(Some(inner)) {
                    (Some(dwpt), _, _) => return Ok(Some(dwpt)),
                    (None, full_flush, num_pending) => {
                        return self.try_get_next_pending_flush(
                            num_pending,
                            full_flush,
                            Some(inner),
                        )
                    },
                }
            }
        } else {
            if mark_pending {
                debug_assert!(!per_thread.is_flush_pending());
                self.set_flush_pending(&per_thread)?;
            }
            if *per_thread.is_flush_pending() {
                return Ok(Some(self.check_out_for_flush(per_thread, inner)));
            }
        }
        Ok(None)
    }
    fn update_stall_state(&self, inner: &mut Inner<D, IF, Q, F, FP>) -> bool {
        let limit = self.stall_limit_bytes();
        let active = inner.active_bytes;
        let flush = inner.flush_bytes.load(Ordering::SeqCst);
        let stall = (active + flush) > limit && active < limit && !inner.closed;

        let mut info_stream = self.info_stream.lock();
        if info_stream.enabled("DWFC") && stall != inner.stall_control.any_stalled_threads() {
            if stall {
                info_stream.message(
                        "DW",
                        &format!(
                            "now stalling flushes: netBytes: {:.1} MB flushBytes: {:.1} MB fullFlush: {}",
                            (self.net_bytes() as f64) / 1024.0 / 1024.0,
                            (self.flushing_bytes() as f64) / 1024.0 / 1024.0,
                            inner.full_flush
                        ),
                    );
                inner.stall_start_ns = Instant::now()
            } else {
                let elapsed = Instant::now()
                    .duration_since(inner.stall_start_ns)
                    .as_secs_f64()
                    * 1000.0;
                info_stream.message(
                        "DW",
                        &format!(
                            "done stalling flushes for {:.1} msec: netBytes: {:.1} MB flushBytes: {:.1} MB fullFlush: {}",
                            elapsed,
                            (self.net_bytes() as f64) / 1024.0 / 1024.0,
                            (self.flushing_bytes() as f64) / 1024.0 / 1024.0,
                            inner.full_flush
                        ),
                    );
            }
        }

        inner.stall_control.update_stalled(stall);
        stall
    }

    /// Sets flush pending state on the given [`DocumentsWriterPerThread`].
    /// The [`DocumentsWriterPerThread`] must have indexed at least on Document and must not be already pending.
    pub fn set_flush_pending(&self, per_thread: &DocumentsWriterPerThread<D, IF, Q>) -> Result<()> {
        let mut inner = self.lock.lock();
        debug_assert!(!per_thread.is_flush_pending());
        if per_thread.get_num_docs_in_ram() > 0 {
            per_thread.set_flush_pending()?;
            let bytes = per_thread.get_last_committed_bytes_used();
            inner.flush_bytes.fetch_add(bytes, Ordering::SeqCst);
            inner.active_bytes -= bytes;
            inner.num_pending.fetch_add(1, Ordering::SeqCst);
            assert!(self.assert_memory(&mut inner));
        }
        Ok(())
    }
    fn checkout_and_block(
        &self,
        per_thread: DocumentsWriterPerThread<D, IF, Q>,
        inner: &mut Inner<D, IF, Q, F, FP>,
    ) {
        let id = per_thread.id();
        debug_assert!(inner.per_thread_pool.is_registered(id));
        debug_assert!(
            per_thread.is_flush_pending(),
            "can not block non-pending threadstate"
        );
        debug_assert!(inner.full_flush, "can not block if fullFlush == false");

        inner.num_pending.fetch_sub(1, Ordering::SeqCst);
        let checked_out = inner.per_thread_pool.checkout(id);
        inner.blocked_flushes.push_back(per_thread);
        debug_assert!(checked_out);
    }
    fn check_out_for_flush(
        &self,
        per_thread: DocumentsWriterPerThread<D, IF, Q>,
        inner: &mut Inner<D, IF, Q, F, FP>,
    ) -> DocumentsWriterPerThread<D, IF, Q> {
        debug_assert!(per_thread.is_flush_pending());
        debug_assert!(inner.per_thread_pool.is_registered(per_thread.id()));

        let result = {
            self.add_flushing_dwpt(per_thread.id(), inner);
            inner.num_pending.fetch_sub(1, Ordering::SeqCst);
            let checked_out = inner.per_thread_pool.checkout(per_thread.id());
            debug_assert!(checked_out);
            per_thread
        };

        self.update_stall_state(inner);
        result
    }
    fn add_flushing_dwpt(&self, per_thread_id: &str, inner: &mut Inner<D, IF, Q, F, FP>) {
        let id = per_thread_id.to_string();
        debug_assert!(
            !inner.flushing_writers.contains(&id),
            "DWPT is already flushing"
        );
        inner.flushing_writers.push(id);
    }
    pub fn next_pending_flush(
        &mut self,
        inner: Option<&mut Inner<D, IF, Q, F, FP>>,
    ) -> (Option<DocumentsWriterPerThread<D, IF, Q>>, bool, i32) {
        let inner = if let Some(inner) = inner {
            inner
        } else {
            &mut *self.lock.lock()
        };
        if let Some(dwpt) = inner.flush_queue.pop_front() {
            // update stall state before returning
            self.update_stall_state(inner);
            return (
                Some(dwpt),
                inner.full_flush,
                inner.num_pending.load(Ordering::SeqCst),
            );
        }
        (
            None,
            inner.full_flush,
            inner.num_pending.load(Ordering::SeqCst),
        )
    }
    pub fn try_get_next_pending_flush(
        &self,
        num_pending: i32,
        full_flush: bool,
        inner: Option<&mut Inner<D, IF, Q, F, FP>>,
    ) -> Result<Option<DocumentsWriterPerThread<D, IF, Q>>> {
        let inner = if let Some(inner) = inner {
            inner
        } else {
            &mut *self.lock.lock()
        };
        if num_pending > 0 && !full_flush {
            match inner.per_thread_pool.get_flush_pending_dwpt()? {
                Some(dwpt) => {
                    return Ok(Some(self.check_out_for_flush(dwpt, inner)));
                },
                None => return Ok(None),
            }
        }
        Ok(None)
    }
}
