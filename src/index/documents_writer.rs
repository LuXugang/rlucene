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
use crate::index::doc_values_update::DocValuesUpdate;
use crate::index::documents_writer_delete_queue::{DocumentsWriterDeleteQueue, Node};
use crate::index::documents_writer_flush_control::DocumentsWriterFlushControl;
use crate::index::documents_writer_flush_queue::{DocumentsWriterFlushQueue, FlushTicket};
use crate::index::documents_writer_per_thread::DocumentsWriterPerThread;
use crate::index::documents_writer_per_thread_pool::DocumentsWriterPerThreadPool;
use crate::index::field_infos::build::Builder;
use crate::index::field_infos::FieldNumbers;
use crate::index::indexable_field::IndexableField;
use crate::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::index::lockable_concurrent_approximate_priority_queue::Lock;
use crate::index::segment_info::SegmentInfo;
use crate::index::term::Term;
use crate::search::query::Query;
use crate::store::directory::Directory;
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use crate::util::info_stream::{InfoStream, InfoStreamLock};
use crate::util::io_consumer::IOConsumer;
use crate::util::supplier::Supplier;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;
/// This struct accepts multiple added documents and directly writes segment files.
///
/// Each added document is passed to the indexing chain, which processes the document into
/// the different codec formats. Some formats write bytes to files immediately (e.g. stored fields
/// and term vectors), while others are buffered by the indexing chain and written only on flush.
///
/// Once we have used our allowed RAM buffer, or the number of added docs is large enough (in the
/// case we are flushing by doc count instead of RAM usage), we create a real segment and flush it to
/// the `Directory`.
///
/// Threads:
///
/// Multiple threads are allowed into `add_document` at once. There is an initial synchronized call
/// to [`DocumentsWriterFlushControl::obtain_and_lock()`] which allocates a DWPT for this indexing
/// thread. The same thread will not necessarily get the same DWPT over time. Then `update_documents` is
/// called on that DWPT without synchronization (most of the “heavy lifting” is in this call). Once a
/// DWPT fills up enough RAM or holds enough documents in memory, the DWPT is checked out for flush and
/// all changes are written to the directory. Each DWPT corresponds to one segment being written.
///
/// When `flush` is called by `IndexWriter`, we check out all DWPTs associated with the
/// current [`DocumentsWriterDeleteQueue`] out of the [`DocumentsWriterPerThreadPool`] and
/// write them to disk. The flush process can piggyback on incoming indexing threads or even block
/// them from adding documents if flushing can’t keep up with new documents being added. Unless the
/// stall control kicks in to block indexing threads, flushes happen concurrently with indexing requests.
///
/// Exceptions:
///
/// Because this struct directly updates in-memory posting lists, and flushes stored fields and
/// term vectors directly to files in the directory, there are limited times when an
/// exception can corrupt this state. For example, a disk full while flushing stored fields can leave
/// the file in a corrupt state. Or an OOM exception while appending to the in-memory posting lists
/// can corrupt that posting list. We call such errors “aborting exceptions.” In these cases we
/// must call `abort()` to discard all docs added since the last flush.
///
/// All other exceptions (“non-aborting exceptions”) can still partially update the index
/// structures. These updates are consistent but represent only a part of the document seen up
/// until the exception was hit. When this happens, we immediately mark the document as deleted so
/// that the document is always atomically (“all or none”) added to the index.
pub(crate) struct DocumentsWriter<D, IF, Q, L, FN>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,
    L: LiveIndexWriterConfig,
    FN: FlushNotifications,
{
    pending_num_docs: Arc<AtomicI64>,
    flush_notifications: FN,
    closed: AtomicBool,
    info_stream: InfoStreamLock,
    config: Arc<L>,
    num_docs_in_ram: AtomicI32,
    ticket_queue: DocumentsWriterFlushQueue<D, Q>,
    // we preserve changes during a full flush since IW might not check out before
    // we release all changes. NRT Readers otherwise suddenly return true from
    // isCurrent while there are actually changes currently committed. See also
    // #anyChanges() & #flushAllThreads
    pending_changes_in_current_full_flush: AtomicBool,
    per_thread_pool: DocumentsWriterPerThreadPool<D, IF, Q>,
    pub(crate) lock: Mutex<Inner<Q>>,
    flush_control: DocumentsWriterFlushControl<D, IF, Q, L>,
    index_created_version_major: i32,
    directory: Arc<Mutex<D>>,
    directory_orig: Arc<Mutex<D>>,
    enable_test_points: bool,
    global_field_number_map: Arc<Mutex<FieldNumbers>>,
}
pub(crate) struct Inner<Q>
where
    Q: Query,
{
    pub(crate) delete_queue: Arc<DocumentsWriterDeleteQueue<Q>>,
    current_full_flush_del_queue: Option<Arc<DocumentsWriterDeleteQueue<Q>>>,
}
impl<D, IF, Q, L, FN> DocumentsWriter<D, IF, Q, L, FN>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,
    L: LiveIndexWriterConfig,
    FN: FlushNotifications,
{
    pub fn delete_queries(&self, queries: Vec<Q>) -> Result<i64> {
        self.apply_delete_or_update(|upd| {
            upd.add_delete_query(queries.into_iter().map(Arc::new).collect())
        })
    }

    pub fn delete_terms(&self, terms: Vec<Term>) -> Result<i64> {
        self.apply_delete_or_update(|upd| upd.add_delete_term(terms))
    }

    pub fn update_doc_values(&self, updates: Vec<DocValuesUpdate>) -> Result<i64> {
        self.apply_delete_or_update(|upd| upd.add_doc_values_updates(updates))
    }
    pub fn apply_delete_or_update<F>(&self, func: F) -> Result<i64>
    where
        F: FnOnce(&DocumentsWriterDeleteQueue<Q>) -> Result<i64>,
    {
        // Check the applyAllDeletes flag first. This helps exit early most of the time without checking
        // isFullFlush(), which takes a lock and introduces contention on small documents that are quick
        // to index.
        let inner = self.lock.lock();
        let mut seq_no = func(&*inner.delete_queue)?;
        self.flush_control
            .do_on_delete(self.config.get_flush_policy());
        if self.apply_all_deletes(Some(&inner))? {
            seq_no = -seq_no;
        }
        Ok(seq_no)
    }
    /// If buffered deletes are using too much heap, resolve them and write disk and return true.
    fn apply_all_deletes(&self, inner: Option<&Inner<Q>>) -> Result<bool> {
        // Check the applyAllDeletes flag first. This helps exit early most of the time without checking
        // isFullFlush(), which takes a lock and introduces contention on small documents that are quick
        // to index.
        let delete_queue = {
            match inner {
                Some(inner) => inner.delete_queue.clone(),
                None => self.lock.lock().delete_queue.clone(),
            }
        };
        if self.flush_control.get_apply_all_deletes()
            && !self.flush_control.is_full_flush()
            // never apply deletes during full flush this breaks happens before relationship.
            && delete_queue.is_open()
            // if it's closed then it's already fully applied and we have a new delete queue
            && self.flush_control.get_and_reset_apply_all_deletes()
        {
            let supplier = SupplierImpl::new(delete_queue);
            if self.ticket_queue.add_ticket(supplier)?.is_some() {
                self.flush_notifications.on_deletes_applied(); // apply deletes event forces a purge
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(crate) fn purge_flush_tickets<C>(&mut self, forced: bool, mut consumer: C) -> Result<()>
    where
        C: IOConsumer<FlushTicket<D, Q>>,
    {
        if forced {
            self.ticket_queue.force_purge(&mut consumer)
        } else {
            self.ticket_queue.try_purge(&mut consumer)
        }
    }
    /// Returns how many docs are currently buffered in RAM.
    pub(crate) fn get_num_docs(&self) -> i32 {
        self.num_docs_in_ram.load(Ordering::SeqCst)
    }

    fn ensure_open(&self) -> Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            Err(LuceneError::already_closed(
                "this DocumentsWriter is closed",
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn abort(&self) {
        // TODO
    }

    pub(crate) fn flush_one_dwpt(&mut self) -> Result<bool> {
        {
            let mut info_stream = self.info_stream.lock();
            if info_stream.enabled("DW") {
                info_stream.message("DW", "startFlushOneDWPT");
            }
        }

        if !self.maybe_flush()? {
            // if let Some(dwpt) = self.flush_control.checkout_largest_non_pending_writer() {
            //     self.do_flush(dwpt)?;
            //     return Ok(true);
            // }
            // return Ok(false);
            unimplemented!()
        }
        Ok(true)
    }

    /// Locks all currently active DWPT and aborts them.
    /// The returned Closeable should be closed once the locks for the aborted DWPTs can be released.
    fn lock_and_abort_all(&self) {
        // TODO
    }
    /// Returns how many documents were aborted.
    fn abort_documents_writer_per_thread(
        &self,
        mut per_thread: DocumentsWriterPerThread<D, IF, Q>,
    ) -> Result<()> {
        debug_assert!(self.lock.is_locked());

        let num = per_thread.get_num_docs_in_ram();
        self.subtract_flushed_num_docs(num);

        per_thread.abort()?;
        self.flush_control.do_on_abort(&per_thread);

        Ok(())
    }
    /// returns the maximum sequence number for all previously completed operations
    pub(crate) fn get_max_completed_sequence_number(&self) -> i64 {
        let inner = self.lock.lock();
        inner.delete_queue.get_max_completed_seq_no()
    }
    fn any_changes(&self) -> bool {
        // changes are either in a DWPT or in the deleteQueue.
        // yet if we currently flush deletes and / or dwpt there
        // could be a window where all changes are in the ticket queue
        // before they are published to the IW. ie we need to check if the
        // ticket queue has any tickets.
        let num_docs = self.num_docs_in_ram.load(Ordering::SeqCst) != 0;
        let deletions = self.any_deletions();
        let tickets = self.ticket_queue.has_tickets();
        let pending_full = self
            .pending_changes_in_current_full_flush
            .load(Ordering::SeqCst);

        let any = num_docs || deletions || tickets || pending_full;

        let mut info = self.info_stream.lock();
        if info.enabled("DW") && any {
            info.message(
                "DW",
                &format!(
                    "anyChanges? numDocsInRam={num_docs} deletes={deletions} hasTickets={tickets} pendingChangesInFullFlush={pending_full}"
                ),
            );
        }

        any
    }
    pub(crate) fn get_buffered_delete_terms_size(&self) -> Result<i32> {
        let delete_queue = self.lock.lock().delete_queue.clone();
        delete_queue.get_buffered_updates_terms_size()
    }
    pub(crate) fn any_deletions(&self) -> bool {
        let delete_queue = self.lock.lock().delete_queue.clone();
        delete_queue.any_changes()
    }
    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        // TODO
    }
    fn pre_update(&mut self) -> Result<bool> {
        self.ensure_open()?;
        let mut has_events = false;

        while self.flush_control.any_stalled_threads()
            || (self.config.get_check_pending_flush_on_update()
                && self.flush_control.num_queued_flushes() > 0)
        {
            // Help out flushing any queued DWPTs so we can un-stall:
            // Try pickup pending threads here if possible
            // no need to loop over the next pending flushes... doFlush will take care of this
            has_events |= self.maybe_flush()?;
            self.flush_control.wait_if_stalled();
        }

        Ok(has_events)
    }

    fn post_update(
        &self,
        flushing_dwpt: Option<DocumentsWriterPerThread<D, IF, Q>>,
        mut has_events: bool,
    ) -> Result<bool> {
        has_events |= self.apply_all_deletes(None)?;
        if let Some(dwpt) = flushing_dwpt {
            self.do_flush(dwpt)?;
            has_events = true;
        } else if self.config.get_check_pending_flush_on_update() {
            has_events |= self.maybe_flush()?;
        }
        Ok(has_events)
    }
    fn update_documents<I, J>(&mut self, docs: I, del_node: &Node<Q>) -> Result<i64>
    where
        I: IntoIterator<Item = J>,
        J: IntoIterator<Item = IF>,
    {
        let has_events = self.pre_update()?;

        let delete_queue = self.lock.lock().delete_queue.clone();
        let dwpt_factory = SupplierImpl2::new(
            self.index_created_version_major,
            self.directory_orig.clone(),
            self.directory.clone(),
            self.config.clone(),
            delete_queue.clone(),
            self.global_field_number_map.clone(),
            self.pending_num_docs.clone(),
            self.enable_test_points,
        );
        let dwpt = self
            .flush_control
            .obtain_and_lock(&delete_queue, dwpt_factory)?;
        let mut flushing_dwpt_opt = None;
        let mut seq_no = 0;
        let result = (|| {
            // This must happen after we've pulled the DWPT because IW.close
            // waits for all DWPT to be released:
            self.ensure_open()?;
            let result: Result<()> = {
                // TODO
                seq_no = 0;
                Ok(())
            };
            if dwpt.is_aborted() {
                self.flush_control.do_on_abort(&dwpt);
            }
            // TODO: 这段代码有点问题 回头用测试才能调试
            let dwpt_id = dwpt.id().to_string();
            let dwpt_lock_state = dwpt.state.clone();
            let dwpt_delete_queue = dwpt.delete_queue.clone();
            let dwpt_abort = dwpt.aborted.clone();
            let dpwt_pending_state = dwpt.flush_pending.clone();
            let dwpt = if result.is_ok() {
                let (new_dwpt, old_dwpt) = self
                    .flush_control
                    .do_after_document(dwpt, self.config.get_flush_policy())?;
                debug_assert!(!(new_dwpt.is_some() && old_dwpt.is_some()));
                flushing_dwpt_opt = new_dwpt;
                old_dwpt
            } else {
                Option::from(dwpt)
            };
            {
                // If a flush is occurring, we don't want to allow this dwpt to be reused
                // If it is aborted, we shouldn't allow it to be reused
                // If the deleteQueue is advanced, this means the maximum seqNo has been set and it cannot be
                // reused
                let _ = self.flush_control.lock.lock();
                if *dpwt_pending_state.get().unwrap_or(&false)
                    || dwpt_abort.load(Ordering::SeqCst)
                    || dwpt_delete_queue.is_advanced()
                {
                    dwpt_lock_state.unlock();
                    if dwpt.is_some() {
                        debug_assert!(flushing_dwpt_opt.is_none());
                        flushing_dwpt_opt = dwpt;
                    }
                } else {
                    debug_assert!(dwpt.is_some());
                    debug_assert!(dwpt.as_ref().unwrap().id() == dwpt_id);
                    self.per_thread_pool
                        .mark_as_free_and_unlock(dwpt.unwrap())?;
                }
            }
            result
        })();
        if result.is_err() {
            return Err(result.unwrap_err());
        }
        if self.post_update(flushing_dwpt_opt, has_events)? {
            seq_no = -seq_no;
        }
        Ok(seq_no)
    }

    fn maybe_flush(&self) -> Result<bool> {
        let flushing_dwpt = match self.flush_control.next_pending_flush(None) {
            (Some(dwpt), _, _) => Some(dwpt),
            (None, full_flush, num_pending) => {
                self.flush_control
                    .try_get_next_pending_flush(num_pending, full_flush, None)?
            },
        };

        if let Some(flushing_dwpt) = flushing_dwpt {
            self.do_flush(flushing_dwpt)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn do_flush(&self, mut flushing_dwpt: DocumentsWriterPerThread<D, IF, Q>) -> Result<()> {
        loop {
            assert!(!flushing_dwpt.has_flushed(),);
            {
                let current_full_flush_del_queue =
                    self.lock.lock().current_full_flush_del_queue.clone();
                debug_assert!(
                    current_full_flush_del_queue.is_none()
                        || Arc::ptr_eq(
                            &flushing_dwpt.delete_queue,
                            current_full_flush_del_queue.as_ref().unwrap()
                        )
                );
            }

            // Since with DWPT the flush process is concurrent and several DWPT
            // could flush at the same time we must maintain the order of the
            // flushes before we can apply the flushed segment and the frozen global
            // deletes it is buffering. The reason for this is that the global
            // deletes mark a certain point in time where we took a DWPT out of
            // rotation and freeze the global deletes.
            //
            // Example: A flush 'A' starts and freezes the global deletes, then
            // flush 'B' starts and freezes all deletes occurred since 'A' has
            // started. if 'B' finishes before 'A' we need to wait until 'A' is done
            // otherwise the deletes frozen by 'B' are not applied to 'A' and we
            // might miss to deletes documents in 'A'.
            let mut has_ticket = None;
            let result = (|| {
                debug_assert!(self.assert_ticket_queue_modification(&flushing_dwpt.delete_queue));
                let supplier = SupplierImpl1 {
                    dwpt: &mut flushing_dwpt,
                };
                let ticket = self.ticket_queue.add_ticket(supplier)?;
                match ticket {
                    Some(ticket) => {
                        has_ticket = Some(ticket);
                        let flushing_docs_in_ram = flushing_dwpt.get_num_docs_in_ram();
                        let result = (|| {
                            let v =
                                flushing_dwpt.flush(&self.flush_notifications, &*self.config)?;
                            match v {
                                Some(new_segment) => {
                                    self.ticket_queue.add_segment(ticket, new_segment);
                                    Ok(())
                                },
                                None => {
                                    Err(LuceneError::illegal_state("flush_segment returned None"))
                                },
                            }
                        })();
                        self.subtract_flushed_num_docs(flushing_docs_in_ram);
                        if !flushing_dwpt.pending_files_to_delete().is_empty() {
                            let files = flushing_dwpt.pending_files_to_delete();
                            self.flush_notifications.delete_unused_files(files.clone());
                        }
                        if result.is_err() {
                            self.flush_notifications
                                .flush_failed(flushing_dwpt.get_segment_info())
                        }
                        result
                    },
                    None => Err(LuceneError::illegal_state("ticket returned None")),
                }
            })();
            if result.is_err() && has_ticket.is_some() {
                // In the case of a failure make sure we are making progress and
                // apply all the deletes since the segment flush failed since the flush
                // ticket could hold global deletes see FlushTicket#canPublish()
                let flush_ticket = &mut self.ticket_queue.inner.lock().queue[has_ticket.unwrap()];
                self.ticket_queue.mark_ticket_failed(flush_ticket);
            }
            //Now we are done and try to flush the ticket queue if the head of the
            // queue has already finished the flush.
            if self.ticket_queue.get_ticket_count() as usize >= self.per_thread_pool.size() {
                // This means there is a backlog: the one
                // thread in innerPurge can't keep up with all
                // other threads flushing segments.  In this case
                // we forcefully stall the producers.
                self.flush_notifications.on_ticket_backlog();
            }

            self.flush_control.do_after_flush(flushing_dwpt);
            let v = match self.flush_control.next_pending_flush(None) {
                (Some(dwpt), _, _) => Some(dwpt),
                (None, full_flush, num_pending) => {
                    self.flush_control
                        .try_get_next_pending_flush(num_pending, full_flush, None)?
                },
            };

            match v {
                Some(next_dwpt) => {
                    flushing_dwpt = next_dwpt;
                    continue;
                },
                None => break,
            }
        }

        self.flush_notifications.after_segments_flushed()?;
        Ok(())
    }
    pub(crate) fn get_next_sequence_number(&self) -> i64 {
        let delete_queue = self.lock.lock().delete_queue.clone();
        delete_queue.get_next_sequence_number(None)
    }

    pub(crate) fn reset_delete_queue(
        &self,
        inner: &mut Inner<Q>,
        max_num_pending_ops: i64,
    ) -> Result<i64> {
        let new_queue = inner.delete_queue.advance_queue(max_num_pending_ops)?;
        debug_assert!(inner.delete_queue.is_advanced());
        debug_assert!(!new_queue.is_advanced());
        debug_assert!(
            inner.delete_queue.get_last_sequence_number() <= new_queue.get_last_sequence_number()
        );
        debug_assert!(
            inner.delete_queue.get_max_seq_no() <= new_queue.get_last_sequence_number(),
            "max_seq_no: {} vs. {}",
            inner.delete_queue.get_max_seq_no(),
            new_queue.get_last_sequence_number()
        );
        let old_max_seq_no = inner.delete_queue.get_max_seq_no();
        inner.delete_queue = Arc::new(new_queue);
        Ok(old_max_seq_no)
    }

    pub(crate) fn subtract_flushed_num_docs(&self, num_flushed: i32) {
        let mut old_value = self.num_docs_in_ram.load(Ordering::SeqCst);
        loop {
            let new_value = old_value - num_flushed;
            if self
                .num_docs_in_ram
                .compare_exchange(old_value, new_value, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
            old_value = self.num_docs_in_ram.load(Ordering::SeqCst);
        }
        debug_assert!(self.num_docs_in_ram.load(Ordering::SeqCst) >= 0);
    }

    fn set_flushing_delete_queue(
        &self,
        session: Option<Arc<DocumentsWriterDeleteQueue<Q>>>,
    ) -> bool {
        let mut inner = self.lock.lock();
        debug_assert!(
            inner
                .current_full_flush_del_queue
                .as_ref()
                .is_none_or(|q| !q.is_open()),
            "Can not replace a full flush queue if the queue is not closed"
        );
        inner.current_full_flush_del_queue = session;
        true
    }
    fn assert_ticket_queue_modification(
        &self,
        delete_queue: &Arc<DocumentsWriterDeleteQueue<Q>>,
    ) -> bool {
        let inner = self.lock.lock();
        debug_assert!(
            inner
                .current_full_flush_del_queue
                .as_ref()
                .is_none_or(|q| Arc::ptr_eq(q, delete_queue)),
            "only modifications from the current flushing queue are permitted while doing a full flush"
        );
        true
    }

    // FlushAllThreads is synced by IW fullFlushLock. Flushing all threads is a
    // two stage operation; the caller must ensure (in try/finally) that finishFlush
    // is called after this method, to release the flush lock in DWFlushControl
    fn flush_all_threads(&self) -> Result<i64> {
        {
            let mut info_stream = self.info_stream.lock();
            if info_stream.enabled("DW") {
                info_stream.message("DW", "startFullFlush");
            }
        }

        let (flushing_delete_queue, seq_no) = {
            let inner = self.lock.lock();
            let pending = self.any_changes();
            self.pending_changes_in_current_full_flush
                .store(pending, Ordering::SeqCst);
            let fq = inner.delete_queue.clone();
            // Cutover to a new delete queue.  This must be synced on the flush control
            // otherwise a new DWPT could sneak into the loop with an already flushing
            // delete queue
            let sn = self.flush_control.mark_for_full_flush(self)?;
            debug_assert!(self.set_flushing_delete_queue(Some(Arc::clone(&fq))));
            (fq, sn)
        };
        debug_assert!({
            let current_full_flush_del_queue =
                self.lock.lock().current_full_flush_del_queue.clone();
            current_full_flush_del_queue.is_some()
                && !Arc::ptr_eq(
                    &flushing_delete_queue,
                    &current_full_flush_del_queue.unwrap(),
                )
        });

        let mut anything_flushed = false;
        anything_flushed |= self.maybe_flush()?;
        // If a concurrent flush is still in flight wait for it
        self.flush_control.wait_for_flush();
        if !anything_flushed && flushing_delete_queue.any_changes() {
            {
                let mut info_stream = self.info_stream.lock();
                if info_stream.enabled("DW") {
                    let v = thread::current();
                    let name = v.name().unwrap_or("<unnamed>");
                    info_stream
                        .message("DW", &format!("{name}: flush naked frozen global deletes"));
                }
            }

            debug_assert!(self.assert_ticket_queue_modification(&flushing_delete_queue));
            let supplier = SupplierImpl::new(flushing_delete_queue.clone());
            self.ticket_queue.add_ticket(supplier)?;
        }
        // we can't assert that we don't have any tickets in the queue since we might add a
        // DocumentsWriterDeleteQueue
        // concurrently if we have very small ram buffers this happens quite frequently
        debug_assert!(!flushing_delete_queue.any_changes());
        {
            let inner = self.lock.lock();
            debug_assert!(Arc::ptr_eq(
                &flushing_delete_queue,
                inner.current_full_flush_del_queue.as_ref().unwrap()
            ));
            // all DWPT have been processed and this queue has been fully flushed to the
            // ticket-queue
            flushing_delete_queue.close()?;
        }
        Ok(if anything_flushed { -seq_no } else { seq_no })
    }
    pub(crate) fn finish_full_flush(&mut self, success: bool) -> Result<()> {
        let mut info_stream = self.info_stream.lock();
        if info_stream.enabled("DW") {
            let thread_name = thread::current().name().unwrap_or("<unnamed>").to_string();
            info_stream.message(
                "DW",
                &format!("{thread_name} finishFullFlush success={success}"),
            );
        }
        debug_assert!(self.set_flushing_delete_queue(None));

        {
            let delete_queue = &self.lock.lock().delete_queue;
            if success {
                self.flush_control.finish_full_flush(delete_queue);
            } else {
                // TODO
                // self.flush_control.abort_full_flushes(delete_queue)?;
            }
        }
        self.pending_changes_in_current_full_flush
            .store(false, Ordering::SeqCst);
        // make sure we do execute this since we block applying deletes during full
        // flush
        self.apply_all_deletes(None)?;

        Ok(())
    }

    /// Returns the number of bytes currently being flushed
    /// This is a subset of the value returned by ramBytesUsed()
    pub(crate) fn get_flushing_bytes(&self) -> i64 {
        self.flush_control.get_flushing_bytes()
    }
}
impl<D, IF, Q, L, FN> Accountable for DocumentsWriter<D, IF, Q, L, FN>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,
    L: LiveIndexWriterConfig,
    FN: FlushNotifications,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        let inner = self.lock.lock();
        Ok(self
            .flush_control
            .get_delete_bytes_used(&*inner.delete_queue)?
            + self.flush_control.net_bytes())
    }
}

pub(crate) trait FlushNotifications {
    /// Called when files were written to disk that are not used anymore.
    /// It's the implementation's responsibility to clean these files up.
    fn delete_unused_files<I>(&self, files: I)
    where
        I: IntoIterator<Item = String>;

    /// Called when a segment failed to flush.
    fn flush_failed<D>(&self, info: &SegmentInfo<D>)
    where
        D: Directory;

    /// Called after one or more segments were flushed to disk.
    fn after_segments_flushed(&self) -> Result<()>;

    /// Should be called if a flush or an indexing operation caused
    /// a tragic / unrecoverable event.
    fn on_tragic_event(&self, event: LuceneError, message: &str);

    /// Called once deletes have been applied either after a flush or on a deletes call.
    fn on_deletes_applied(&self);

    /// Called once the DocumentsWriter ticket queue has a backlog. This means there is an inner
    /// thread that tries to publish flushed segments but can't keep up with the other threads
    /// flushing new segments. This likely requires other thread to forcefully purge the buffer to
    /// help publishing. This can't be done in-place since we might hold index writer locks when this
    /// is called. The caller must ensure that the purge happens without an index writer lock being
    /// held.
    fn on_ticket_backlog(&self);
}

struct SupplierImpl<Q>
where
    Q: Query,
{
    delete_queue: Arc<DocumentsWriterDeleteQueue<Q>>,
}
impl<Q> SupplierImpl<Q>
where
    Q: Query,
{
    pub(crate) fn new(delete_queue: Arc<DocumentsWriterDeleteQueue<Q>>) -> Self {
        SupplierImpl { delete_queue }
    }
}
impl<D, Q> Supplier<Option<FlushTicket<D, Q>>> for SupplierImpl<Q>
where
    D: Directory,
    Q: Query,
{
    fn get(&mut self) -> Result<Option<FlushTicket<D, Q>>> {
        // it's maybeFreezeGlobalBuffer(DocumentsWriterDeleteQueue deleteQueue)'s logic in Java Lucene
        if let Some(frozen_updates) = self.delete_queue.maybe_freeze_global_buffer()? {
            Ok(Some(FlushTicket::new(frozen_updates, false)))
        } else {
            Ok(None)
        }
    }
}

struct SupplierImpl1<'a, D, IF, Q>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,
{
    dwpt: &'a mut DocumentsWriterPerThread<D, IF, Q>,
}
impl<'a, D, IF, Q> SupplierImpl1<'a, D, IF, Q>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,
{
    pub(crate) fn new(dwpt: &'a mut DocumentsWriterPerThread<D, IF, Q>) -> Self {
        SupplierImpl1 { dwpt }
    }
}
impl<'a, D, IF, Q> Supplier<Option<FlushTicket<D, Q>>> for SupplierImpl1<'a, D, IF, Q>
where
    D: Directory,
    IF: IndexableField,
    Q: Query,
{
    fn get(&mut self) -> Result<Option<FlushTicket<D, Q>>> {
        let frozen_buffered_updates = self.dwpt.prepare_flush()?;
        Ok(Some(FlushTicket::new(frozen_buffered_updates, false)))
    }
}

struct SupplierImpl2<D, Q, L>
where
    D: Directory,
    Q: Query,
    L: LiveIndexWriterConfig,
{
    index_major_version_created: i32,
    directory_orig: Arc<Mutex<D>>,
    directory: Arc<Mutex<D>>,
    config: Arc<L>,
    delete_queue: Arc<DocumentsWriterDeleteQueue<Q>>,
    pending_num_docs: Arc<AtomicI64>,
    enable_test_points: bool,
    field_numbers: Arc<Mutex<FieldNumbers>>,
}
impl<D, Q, L> SupplierImpl2<D, Q, L>
where
    D: Directory,
    Q: Query,
    L: LiveIndexWriterConfig,
{
    pub(crate) fn new(
        index_major_version_created: i32,
        directory_orig: Arc<Mutex<D>>,
        directory: Arc<Mutex<D>>,
        config: Arc<L>,
        delete_queue: Arc<DocumentsWriterDeleteQueue<Q>>,
        field_numbers: Arc<Mutex<FieldNumbers>>,
        pending_num_docs: Arc<AtomicI64>,
        enable_test_points: bool,
    ) -> Self {
        SupplierImpl2 {
            index_major_version_created,
            directory_orig,
            directory,
            config,
            delete_queue,
            pending_num_docs,
            enable_test_points,
            field_numbers,
        }
    }
}
impl<D, Q, L, IF> Supplier<DocumentsWriterPerThread<D, IF, Q>> for SupplierImpl2<D, Q, L>
where
    D: Directory,
    Q: Query,
    L: LiveIndexWriterConfig,
    IF: IndexableField,
{
    fn get(&mut self) -> Result<DocumentsWriterPerThread<D, IF, Q>> {
        let infos = Builder::new(self.field_numbers.clone());
        let dwpt = DocumentsWriterPerThread::new(
            self.index_major_version_created,
            "",
            self.directory_orig.clone(),
            self.directory.clone(),
            &*self.config,
            self.delete_queue.clone(),
            infos,
            self.pending_num_docs.clone(),
            self.enable_test_points,
        )?;
        Ok(dwpt)
    }
}
