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
use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::analysis::token_stream::TokenStream;
use crate::index::approximate_priority_queue::IdentityId;
use crate::index::documents_writer_per_thread::DocumentsWriterPerThread;
use crate::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::index::lockable_concurrent_approximate_priority_queue::{
    Lock, LockableConcurrentApproximatePriorityQueue,
};
use crate::search::query::Query;
use crate::store::directory::Directory;
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::{LuceneError, Result};
use parking_lot::{Condvar, Mutex};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
/// [`DocumentsWriterPerThreadPool`] controls [`DocumentsWriterPerThread`] instances and their thread assignments during indexing.
/// Each [`DocumentsWriterPerThread`] is obtained from the pool and exclusively used for indexing a single document or list of documents by the obtaining thread.
/// Each indexing thread must obtain such a [`DocumentsWriterPerThread`] to make progress. Depending on the [`DocumentsWriterPerThreadPool`] implementation, [`DocumentsWriterPerThread`]
/// assignments might differ from document to document.
///
/// Once a [`DocumentsWriterPerThread`] is selected for flush, it will be checked out of the thread pool and won’t be reused for indexing. See [`checkout`](DocumentsWriterPerThreadPool::checkout)
pub(crate) struct DocumentsWriterPerThreadPool<D, P, T, O, TS, L, Q, F>
where
    D: Directory,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    TS: TokenStream,
    L: LiveIndexWriterConfig,
    Q: Query,
    F: Fn() -> DocumentsWriterPerThread<D, P, T, O, TS, L, Q>,
{
    inner: Mutex<State>,
    free_list:
        LockableConcurrentApproximatePriorityQueue<DocumentsWriterPerThread<D, P, T, O, TS, L, Q>>,
    dwpt_factory: F,
    pausing: Condvar,
    closed: AtomicBool,
}
pub(crate) struct State {
    dwpts: HashSet<String>,
    taken_writer_permits: i32,
}
impl<D, P, T, O, TS, L, Q, F> DocumentsWriterPerThreadPool<D, P, T, O, TS, L, Q, F>
where
    D: Directory,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    TS: TokenStream,
    L: LiveIndexWriterConfig,
    Q: Query,
    F: Fn() -> DocumentsWriterPerThread<D, P, T, O, TS, L, Q>,
{
    pub fn new(dwpt_factory: F) -> Result<Self> {
        let inner = Mutex::new(State {
            dwpts: HashSet::new(),
            taken_writer_permits: 0,
        });
        Ok(Self {
            inner,
            free_list: LockableConcurrentApproximatePriorityQueue::new()?,
            dwpt_factory,
            pausing: Condvar::new(),
            closed: AtomicBool::new(false),
        })
    }
    /// Returns the active number of [`DocumentsWriterPerThread`] instances.
    pub(crate) fn size(&self) -> usize {
        let inner = self.inner.lock();
        inner.dwpts.len()
    }

    pub(crate) fn lock_new_writers(&mut self) {
        // this is similar to a semaphore - we need to acquire all permits ie. takenWriterPermits must
        // be == 0
        // any call to lockNewWriters() must be followed by unlockNewWriters() otherwise we will
        // deadlock at some
        // point
        let mut inner = self.inner.lock();
        debug_assert!(inner.taken_writer_permits >= 0);
        inner.taken_writer_permits += 1;
    }
    pub(crate) fn unlock_new_writers(&self) {
        let mut inner = self.inner.lock();
        debug_assert!(inner.taken_writer_permits > 0);
        inner.taken_writer_permits -= 1;

        if inner.taken_writer_permits == 0 {
            self.pausing.notify_all();
        }
    }

    /// Returns a new already locked [`DocumentsWriterPerThread`]
    pub(crate) fn new_writer(&self) -> Result<DocumentsWriterPerThread<D, P, T, O, TS, L, Q>> {
        let mut inner = self.inner.lock();
        debug_assert!(inner.taken_writer_permits >= 0);
        while inner.taken_writer_permits > 0 {
            self.pausing.wait(&mut inner);
        }
        // we must check if we are closed since this might happen while we are waiting for the writer
        // permit
        // and if we miss that we might release a new DWPT even though the pool is closed. Yet, that
        // wouldn't be the
        // end of the world it's violating the contract that we don't release any new DWPT after this
        // pool is closed
        self.ensure_open()?;
        let dwpt = (self.dwpt_factory)();
        dwpt.lock()?;
        inner.dwpts.insert(dwpt.id().to_string());
        Ok(dwpt)
    }
    /// This method is used by `DocumentsWriter`/`FlushControl` to obtain a DWPT to do an indexing
    /// operation (add/updateDocument).
    pub(crate) fn get_and_lock(&self) -> Result<DocumentsWriterPerThread<D, P, T, O, TS, L, Q>> {
        self.ensure_open()?;

        if let Some(dwpt) = self.free_list.lock_and_poll() {
            return Ok(dwpt);
        }
        // newWriter() adds the DWPT to the `dwpts` set as a side-effect. However it is not added to
        // `freeList` at this point, it will be added later on once DocumentsWriter has indexed a
        // document into this DWPT and then gives it back to the pool by calling
        // #marksAsFreeAndUnlock.
        self.new_writer()
    }

    fn ensure_open(&self) -> Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(LuceneError::already_closed("DWPTPool is already closed"));
        }
        Ok(())
    }

    pub(crate) fn contains(&self, state: &DocumentsWriterPerThread<D, P, T, O, TS, L, Q>) -> bool {
        let inner = self.inner.lock();
        inner.dwpts.contains(state.id())
    }
    pub(crate) fn mark_as_free_and_unlock(
        &self,
        state: DocumentsWriterPerThread<D, P, T, O, TS, L, Q>,
    ) -> Result<()> {
        let ram_bytes_used = state.ram_bytes_used()?;

        debug_assert!(
            !state.is_flush_pending() && !state.is_aborted() && !state.is_queue_advanced(),
            "DWPT has pending flush: {}, aborted={}, queueAdvanced={}",
            state.is_flush_pending(),
            state.is_aborted(),
            state.is_queue_advanced()
        );

        debug_assert!(
            self.contains(&state),
            "Tried to add a DWPT back to the pool but the pool doesn't know about this DWPT"
        );

        self.free_list.add_and_unlock(state, ram_bytes_used);
        Ok(())
    }
    /// Filters all `DocumentsWriterPerThread`s that the given predicate applies to and that can be checked out of the pool via [`checkout`](Self::checkout).
    /// All returned DWPTs are already locked, and [`is_registered`](Self::is_registered) will return `true` for each one.
    pub(crate) fn filter_and_lock<F1>(&self, predicate: F1) -> Result<Vec<String>>
    where
        F1: Fn(&str) -> bool,
    {
        let mut list = Vec::new();
        let inner = self.inner.lock();
        for id in inner.dwpts.iter() {
            if predicate(id) {
                self.free_list.lock(id)?;
                if self.is_registered(id) {
                    list.push(id.clone());
                } else {
                    self.free_list.unlock(id)?
                }
            }
        }
        Ok(list)
    }
    /// Removes the given DWPT from the pool unless it has already been removed.
    ///
    /// # Returns
    ///
    /// `true` if the DWPT was removed; `false` otherwise.
    pub(crate) fn checkout(&mut self, per_thread: &str) -> bool {
        let mut inner = self.inner.lock();

        if inner.dwpts.remove(per_thread) {
            self.free_list.remove(per_thread);
            true
        } else {
            debug_assert!(!self.free_list.contains(per_thread));
            false
        }
    }
    ///  Returns `true` if this DWPT is still part of the pool
    pub(crate) fn is_registered(&self, per_thread: &str) -> bool {
        let inner = self.inner.lock();
        inner.dwpts.contains(per_thread)
    }
    pub fn close(&mut self) {
        self.closed.store(true, Ordering::SeqCst);
    }
}
