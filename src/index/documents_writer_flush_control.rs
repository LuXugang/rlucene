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
use crate::index::documents_writer::DocumentsWriter;
use crate::index::documents_writer_delete_queue::DocumentsWriterDeleteQueue;
use crate::index::documents_writer_per_thread::DocumentsWriterPerThread;
use crate::index::documents_writer_per_thread_pool::DocumentsWriterPerThreadPool;
use crate::index::documents_writer_stall_control::DocumentsWriterStallControl;
use crate::index::flush_policy::FlushPolicy;
use crate::index::index_writer_config::iwc_util;
use crate::index::indexable_field::IndexableField;
use crate::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::index::lockable_concurrent_approximate_priority_queue::Lock;
use crate::search::query::Query;
use crate::store::directory::Directory;
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::info_stream::{InfoStream, InfoStreamLock};
use crate::util::supplier::Supplier;
use parking_lot::{Condvar, Mutex};
use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub(crate) struct DocumentsWriterFlushControl<D, IF, Q, L>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,
    L: LiveIndexWriterConfig,
{
    flush_deletes: AtomicBool,
    info_stream: InfoStreamLock,
    lock: Mutex<Inner<D, IF, Q>>,
    config: Arc<L>,
    stall_control: DocumentsWriterStallControl,
    pausing: Condvar,
}
pub(crate) struct Inner<D, IF, Q>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,
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
    per_thread_pool: DocumentsWriterPerThreadPool<D, IF, Q>,
    closed: bool,
    stall_start_ns: Instant,
    // only with assert
    peak_active_bytes: i64,
    // only with assert
    peak_flush_bytes: i64,
    // only with assert
    peak_net_bytes: i64,
    // only with assert
    peak_delta: i64,
    num_docs_since_stalled: i32,
}

impl<D, IF, Q, L> DocumentsWriterFlushControl<D, IF, Q, L>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,
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
    fn assert_memory(&self, inner: &mut Inner<D, IF, Q>) -> bool {
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
                    * inner.peak_delta)
                + (inner.num_docs_since_stalled as i64 * inner.peak_delta);
            // the expected ram consumption is an upper bound at this point and not really the expected
            // consumption
            if inner.peak_delta < (ram_buffer_bytes >> 1) {
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
                    inner.peak_delta,
                    ram_buffer_bytes,
                    inner.max_configured_ram_buffer
                );
            }
        } else {
            inner.flush_by_ram_was_disabled = true;
        }
        true
    }

    // only for asserts
    fn update_peaks(&self, delta: i64, inner: &mut Inner<D, IF, Q>) -> bool {
        let net = self.net_bytes();
        let active = inner.active_bytes;
        let flush = inner.flush_bytes.load(Ordering::SeqCst);

        inner.peak_active_bytes = inner.peak_active_bytes.max(active);
        inner.peak_flush_bytes = inner.peak_flush_bytes.max(flush);
        inner.peak_net_bytes = inner.peak_net_bytes.max(net);
        inner.peak_delta = inner.peak_delta.max(delta);

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
    pub(crate) fn do_after_document<FP>(
        &mut self,
        mut per_thread: DocumentsWriterPerThread<D, IF, Q>,
        flush_policy: FP,
    ) -> Result<Option<DocumentsWriterPerThread<D, IF, Q>>>
    where
        FP: FlushPolicy,
    {
        let delta = per_thread.get_commit_last_bytes_used_delta()?;
        // in order to prevent contention in the case of many threads indexing small documents
        // we skip ram accounting unless the DWPT accumulated enough ram to be worthwhile
        if self.config.get_max_buffered_docs() == iwc_util::DISABLE_AUTO_FLUSH
            && delta < self.ram_buffer_granularity()
        {
            // Skip accounting for now, we'll come back to it later when the delta is bigger
            return Ok(None);
        }
        let mut inner = self.lock.lock();
        let result = (|| {
            // we need to commit this under lock but calculate it outside of the lock to minimize the time
            // this lock is held
            // per document. The reason we update this under lock is that we mark DWPTs as pending without
            // acquiring it's
            // lock in #setFlushPending and this also reads the committed bytes and modifies the
            // flush/activeBytes.
            // In the future we can clean this up to be more intuitive.
            per_thread.commit_last_bytes_used(delta)?;
            // We need to differentiate here if we are pending since setFlushPending
            // moves the perThread memory to the flushBytes and we could be set to
            // pending during a delete
            if *per_thread.is_flush_pending() {
                inner.flush_bytes.fetch_add(delta, Ordering::SeqCst);
                self.update_peaks(delta, &mut inner);
            } else {
                inner.active_bytes += delta;
                self.update_peaks(delta, &mut inner);
                flush_policy.on_change(Some(&per_thread));
                if !per_thread.is_flush_pending()
                    && per_thread.ram_bytes_used()? > inner.hard_max_bytes_per_dwpt
                {
                    // Safety check to prevent a single DWPT exceeding its RAM limit. This
                    // is super important since we can not address more than 2048 MB per DWPT
                    self.set_flush_pending(&mut per_thread)?;
                }
            }
            self.checkout(&mut inner, per_thread, false)
        })();

        let stall = self.update_stall_state(&mut inner);
        debug_assert!(
            self.assert_num_docs_since_stalled(stall, &mut inner) && self.assert_memory(&mut inner)
        );

        result
    }
    fn checkout(
        &self,
        inner: &mut Inner<D, IF, Q>,
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
    fn assert_num_docs_since_stalled(&self, stalled: bool, inner: &mut Inner<D, IF, Q>) -> bool {
        //  updates the number of documents "finished" while we are in a stalled state.
        //  this is important for asserting memory upper bounds since it corresponds
        //  to the number of threads that are in-flight and crossed the stall control
        //  check before we actually stalled.
        //  see #assertMemory()
        if stalled {
            inner.num_docs_since_stalled += 1;
        } else {
            inner.num_docs_since_stalled = 0;
        }
        true
    }
    pub(crate) fn do_after_flush(&self, dwpt: DocumentsWriterPerThread<D, IF, Q>) {
        let mut inner = self.lock.lock();
        let id = dwpt.id().to_string();
        debug_assert!(inner.flushing_writers.contains(&id),);
        if let Some(pos) = inner.flushing_writers.iter().position(|w| *w == id) {
            inner.flushing_writers.remove(pos);
        }
        inner
            .flush_bytes
            .fetch_sub(dwpt.get_last_committed_bytes_used(), Ordering::SeqCst);

        debug_assert!(self.assert_memory(&mut inner));

        let _ = self.update_stall_state(&mut inner);
        self.pausing.notify_all();
    }
    fn update_stall_state(&self, inner: &mut Inner<D, IF, Q>) -> bool {
        let limit = self.stall_limit_bytes();
        let active = inner.active_bytes;
        let flush = inner.flush_bytes.load(Ordering::SeqCst);
        let stall = (active + flush) > limit && active < limit && !inner.closed;

        let mut info_stream = self.info_stream.lock();
        if info_stream.enabled("DWFC") && stall != self.stall_control.any_stalled_threads() {
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

        self.stall_control.update_stalled(stall);
        stall
    }

    pub fn wait_for_flush(&self) {
        let mut inner = self.lock.lock();
        while !inner.flushing_writers.is_empty() {
            self.pausing.wait(&mut inner);
        }
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
    pub fn do_on_abort(&self, per_thread: &DocumentsWriterPerThread<D, IF, Q>) {
        let mut inner = self.lock.lock();
        {
            debug_assert!(inner.per_thread_pool.is_registered(per_thread.id()));
            let bytes = per_thread.get_last_committed_bytes_used();
            if *per_thread.is_flush_pending() {
                inner.flush_bytes.fetch_sub(bytes, Ordering::SeqCst);
            } else {
                inner.active_bytes -= bytes;
            };
            debug_assert!(self.assert_memory(&mut inner));
            // Take it out of the loop this DWPT is stale
        };

        let _ = self.update_stall_state(&mut inner);
        let checked_out = inner.per_thread_pool.checkout(per_thread.id());
        debug_assert!(checked_out);
    }
    /// To be called only by the owner of this object's monitor lock
    fn checkout_and_block(
        &self,
        per_thread: DocumentsWriterPerThread<D, IF, Q>,
        inner: &mut Inner<D, IF, Q>,
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
        inner: &mut Inner<D, IF, Q>,
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
    fn add_flushing_dwpt(&self, per_thread_id: &str, inner: &mut Inner<D, IF, Q>) {
        let id = per_thread_id.to_string();
        debug_assert!(
            !inner.flushing_writers.contains(&id),
            "DWPT is already flushing"
        );
        inner.flushing_writers.push(id);
    }
    pub fn next_pending_flush(
        &self,
        inner: Option<&mut Inner<D, IF, Q>>,
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
        _num_pending: i32,
        _full_flush: bool,
        _inner: Option<&mut Inner<D, IF, Q>>,
    ) -> Result<Option<DocumentsWriterPerThread<D, IF, Q>>> {
        // TODO:
        Ok(None)
    }
    pub fn close(&self) {
        let mut inner = self.lock.lock();
        inner.closed = true;
    }

    /// Returns heap bytes currently consumed by buffered deletes/updates that would be freed if we pushed all deletes.
    /// This does not include bytes consumed by already pushed delete/update packets.
    pub(crate) fn get_delete_bytes_used(
        &self,
        delete_queue: &DocumentsWriterDeleteQueue<Q>,
    ) -> Result<i64> {
        delete_queue.ram_bytes_used()
    }

    pub(crate) fn num_flushing_dwpt(&self) -> usize {
        let inner = self.lock.lock();
        inner.flushing_writers.len()
    }
    pub fn get_and_reset_apply_all_deletes(&self) -> bool {
        self.flush_deletes
            .swap(false, std::sync::atomic::Ordering::SeqCst)
    }
    /// Check whether deletes need to be applied. This can be used as a pre-flight check before calling
    /// [`getAndResetApplyAllDeletes()`](Self::get_and_reset_apply_all_deletes) to make sure that a single thread applies deletes.
    pub fn get_apply_all_deletes(&self) -> bool {
        self.flush_deletes.load(Ordering::SeqCst)
    }

    pub fn set_apply_all_deletes(&self) {
        self.flush_deletes.store(true, Ordering::SeqCst);
    }

    pub fn obtain_and_lock<S>(
        &self,
        delete_queue: &Arc<DocumentsWriterDeleteQueue<Q>>,
        dwpt_factory: S,
    ) -> Result<DocumentsWriterPerThread<D, IF, Q>>
    where
        S: Supplier<DocumentsWriterPerThread<D, IF, Q>>,
    {
        loop {
            let inner = self.lock.lock();
            if inner.closed {
                return Err(LuceneError::already_closed("flush control is closed"));
            }

            let per_thread = inner.per_thread_pool.get_and_lock(&dwpt_factory)?;
            if Arc::ptr_eq(&per_thread.delete_queue, delete_queue) {
                // simply return the DWPT even in a flush all case since we already hold the lock and the
                // DWPT is not stale
                // since it has the current delete queue associated with it. This means we have established
                // a happens-before
                // relationship and all docs indexed into this DWPT are guaranteed to not be flushed with
                // the currently
                // progress full flush.
                return Ok(per_thread);
            }

            debug_assert!(
                    inner.full_flush && !inner.full_flush_mark_done,
                    "found a stale DWPT but full flush mark phase is already done fullFlush: {} markDone: {}",
                    inner.full_flush,
                    inner.full_flush_mark_done
                );
            inner.per_thread_pool.mark_as_free_and_unlock(per_thread)?;
        }
    }
    pub(crate) fn mark_for_full_flush(
        &mut self,
        documents_writer: &mut DocumentsWriter<Q>,
    ) -> Result<i64> {
        let flushing_queue;
        let seq_no = {
            let mut inner = self.lock.lock();
            debug_assert!(
                !inner.full_flush,
                "called mark_for_full_flush while already in full flush"
            );
            debug_assert!(
                !inner.full_flush_mark_done,
                "fullFlushMarkDone is already true"
            );

            inner.full_flush = true;
            flushing_queue = documents_writer.delete_queue.clone();
            // Set a new delete queue - all subsequent DWPT will use this queue until
            // we do another full flush
            inner.per_thread_pool.lock_new_writers();
            // no new thread-states while we do a flush otherwise the seqNo
            // accounting might be off

            let size = inner.per_thread_pool.size();
            // Insert a gap in seqNo of current active thread count, in the worst case each of those
            // threads now have one operation in flight.  It's fine
            // if we have some sequence numbers that were never assigned:
            let seq_no = documents_writer.reset_delete_queue(size);
            inner.per_thread_pool.unlock_new_writers();
            seq_no
        };

        let mut full_flush_buffer = Vec::new();
        let dwpts = {
            let inner = self.lock.lock();
            inner
                .per_thread_pool
                .filter_and_lock(|_, gen| gen == flushing_queue.generation)?
        };

        for mut next in dwpts {
            if next.get_num_docs_in_ram() > 0 {
                let flushing_dwpt = {
                    if !next.is_flush_pending() {
                        self.set_flush_pending(&mut next)?;
                    }
                    let mut inner = self.lock.lock();
                    next.unlock();
                    self.check_out_for_flush(next, &mut inner)
                };
                full_flush_buffer.push(flushing_dwpt);
            } else {
                next.unlock();
                let checked_out = self.lock.lock().per_thread_pool.checkout(next.id());
                debug_assert!(checked_out);
            }
        }

        {
            // make sure we move all DWPT that are where concurrently marked as
            // pending and moved to blocked are moved over to the flushQueue. There is
            // a chance that this happens since we marking DWPT for full flush without
            // blocking indexing
            let mut inner = self.lock.lock();
            self.prune_blocked_queue(&flushing_queue, &mut inner);
            debug_assert!(self.assert_blocked_flushes(&documents_writer.delete_queue));
            inner.flush_queue.extend(full_flush_buffer);
            self.update_stall_state(&mut inner);
            inner.full_flush_mark_done = true;
        }

        debug_assert!(self.assert_active_delete_queue(&documents_writer.delete_queue));
        debug_assert!(flushing_queue.get_last_sequence_number() <= flushing_queue.get_max_seq_no());

        Ok(seq_no)
    }
    pub fn assert_active_delete_queue(&self, queue: &Arc<DocumentsWriterDeleteQueue<Q>>) -> bool {
        let inner = self.lock.lock();
        for next in inner.per_thread_pool.inner.lock().dwpts.values() {
            debug_assert!(next.gen == queue.generation);
        }
        true
    }

    /// Prunes the blockedQueue by removing all DWPTs that are associated with the given flush queue.
    fn prune_blocked_queue(
        &self,
        flushing_queue: &Arc<DocumentsWriterDeleteQueue<Q>>,
        inner: &mut Inner<D, IF, Q>,
    ) {
        let mut idxs = Vec::new();
        for (i, dwpt) in inner.blocked_flushes.iter().enumerate() {
            if Arc::ptr_eq(&dwpt.delete_queue, flushing_queue) {
                idxs.push(i);
            }
        }

        for &i in idxs.iter().rev() {
            let dwpt = inner
                .blocked_flushes
                .remove(i)
                .expect("should never fail to remove blocked DWPT under lock");
            self.add_flushing_dwpt(dwpt.id(), inner);
            inner.flush_queue.push_back(dwpt);
        }
    }
    fn finish_full_flush(&self, documents_writer: &DocumentsWriter<Q>) {
        let mut inner = self.lock.lock();
        debug_assert!(inner.full_flush);
        debug_assert!(inner.flush_queue.is_empty());
        debug_assert!(
            inner.flushing_writers.is_empty(),
            "flushing_writers must be empty"
        );

        if !inner.blocked_flushes.is_empty() {
            debug_assert!(self.assert_blocked_flushes(&documents_writer.delete_queue),);
            self.prune_blocked_queue(&documents_writer.delete_queue, &mut inner);
            debug_assert!(
                inner.blocked_flushes.is_empty(),
                "blocked_flushes must be empty after pruning"
            );
        }

        inner.full_flush_mark_done = false;
        inner.full_flush = false;
        let _ = self.update_stall_state(&mut inner);
    }
    pub(crate) fn assert_blocked_flushes(
        &self,
        flushing_queue: &Arc<DocumentsWriterDeleteQueue<Q>>,
    ) -> bool {
        let inner = self.lock.lock();
        for blocked in inner.blocked_flushes.iter() {
            debug_assert!(Arc::ptr_eq(&blocked.delete_queue, flushing_queue),);
        }
        true
    }

    /// Returns `true` if a full flush is currently running
    pub fn is_full_flush(&self) -> bool {
        let inner = self.lock.lock();
        inner.full_flush
    }

    /// Returns the number of flushes that are already checked out but not yet actively flushing
    pub fn num_queued_flushes(&self) -> usize {
        let inner = self.lock.lock();
        inner.flush_queue.len()
    }

    /// Returns the number of flushes that are checked out but not yet available for flushing.
    /// This only applies during a full flush if a DWPT needs flushing but must not be flushed
    /// until the full flush has finished.
    pub fn num_blocked_flushes(&self) -> i32 {
        let inner = self.lock.lock();
        inner.blocked_flushes.len() as i32
    }

    /// This method will block if too many DWPT are currently flushing and no checked out DWPT are available
    pub fn wait_if_stalled(&self) {
        self.stall_control.wait_if_stalled();
    }

    /// Returns `true` iff stalled.
    pub fn any_stalled_threads(&self) -> bool {
        self.stall_control.any_stalled_threads()
    }

    pub(crate) fn peak_active_bytes(&self) -> i64 {
        let inner = self.lock.lock();
        inner.peak_active_bytes
    }

    pub(crate) fn peak_net_bytes(&self) -> i64 {
        let inner = self.lock.lock();
        inner.peak_net_bytes
    }
}
impl<D, IF, Q, L> Drop for DocumentsWriterFlushControl<D, IF, Q, L>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,

    L: LiveIndexWriterConfig,
{
    fn drop(&mut self) {
        self.close()
    }
}
impl<D, IF, Q, L> fmt::Display for DocumentsWriterFlushControl<D, IF, Q, L>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,

    L: LiveIndexWriterConfig,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.lock.lock();
        let active = inner.active_bytes;
        let flush = inner.flush_bytes.load(Ordering::SeqCst);
        write!(
            f,
            "DocumentsWriterFlushControl [activeBytes={active}, flushBytes={flush}]"
        )
    }
}
