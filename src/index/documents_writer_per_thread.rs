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
use crate::document::numeric_doc_values_field::NumericDocValuesField;
use crate::index::buffered_updates::{MTBufferedUpdates, STBufferedUpdates};
use crate::index::documents_writer::FlushNotifications;
use crate::index::documents_writer_delete_queue::{DeleteSlice, DocumentsWriterDeleteQueue, Node};
use crate::index::field_infos::build::Builder;
use crate::index::field_infos::FieldInfos;
use crate::index::frozen_buffered_updates::FrozenBufferedUpdates;
use crate::index::index_writer::index_writer_util;
use crate::index::indexing_chain::{IndexingChain, ReservedField};
use crate::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::segment_info::SegmentInfo;
use crate::index::sorter::{DocMap, DocMapImpl};
use crate::search::query::Query;
use crate::store::directory::Directory;
use crate::store::tracking_directory_wrapper::TrackingDirectoryWrapper;
use crate::util::accountable::Accountable;
use crate::util::array_util::ArrayUtil;
use crate::util::bit_set::BitSet;
use crate::util::bits::Bits;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::info_stream::{InfoStream, InfoStreamLock};
use crate::util::io_consumer::IOConsumer;
use crate::util::StringHelper;
use std::cell::OnceCell;
use std::collections::HashSet;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

pub(crate) struct DocumentsWriterPerThread<D, P, T, O, TS, L, Q>
where
    D: Directory,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    TS: TokenStream,
    L: LiveIndexWriterConfig,
    Q: Query,
{
    // wrap with Option for std::mem::take()
    pub(crate) directory: Option<TrackingDirectoryWrapper<D>>,
    indexing_chain: IndexingChain<D, O, P, T, TS, L>,
    pending_updates: MTBufferedUpdates<Q>,
    segment_info: SegmentInfo<D>,
    aborted: bool,
    flush_pending: OnceCell<bool>,
    last_committed_bytes_used: AtomicI64,
    has_flushed: OnceCell<bool>,
    field_infos: Builder,
    info_stream: InfoStreamLock,
    num_docs_in_ram: i32,
    pub(crate) delete_queue: Arc<DocumentsWriterDeleteQueue<Q>>,
    delete_slice: Option<DeleteSlice<Q>>,
    pending_num_docs: AtomicI64,
    index_writer_config: L,
    enable_test_points: bool,
    delete_doc_ids: Vec<i32>,
    num_deleted_doc_ids: usize,
    index_major_version_created: i32,
    parent_field: ReservedField<NumericDocValuesField>,
    files_to_delete: HashSet<String>,
    aborting_exception: Option<LuceneError>,
}
impl<D, P, T, O, TS, L, Q> DocumentsWriterPerThread<D, P, T, O, TS, L, Q>
where
    D: Directory,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    TS: TokenStream,
    L: LiveIndexWriterConfig,
    Q: Query,
{
    fn on_aborting_exception(&mut self, throwable: LuceneError) {
        debug_assert!(
            self.aborting_exception.is_none(),
            "aborting exception has already been set"
        );
        self.aborting_exception = Some(throwable);
    }
    pub(crate) fn abort(&mut self) -> Result<()> {
        self.aborted = true;
        self.pending_num_docs
            .fetch_add(-(self.num_docs_in_ram as i64), Ordering::SeqCst);

        {
            let mut info_stream = self.info_stream.lock();
            if info_stream.enabled("DWPT") {
                info_stream.message("DWPT", "now abort");
            }
        }

        let abort_result = (|| {
            self.indexing_chain.abort()?;
            Ok(())
        })();
        self.pending_updates.clear();

        {
            let mut info_stream = self.info_stream.lock();
            if info_stream.enabled("DWPT") {
                info_stream.message("DWPT", "done abort");
            }
        }
        abort_result
    }
    pub(crate) fn test_point(&self, message: &str) {
        if self.enable_test_points {
            let mut info_stream = self.info_stream.lock();
            debug_assert!(info_stream.enabled("TP"));
            info_stream.message("TP", message);
        }
    }
    /// Anything that will add N docs to the index should reserve first to make sure it's allowed.
    fn reserve_one_doc(&self) -> Result<()> {
        let new_count = self
            .pending_num_docs
            .fetch_add(1, Ordering::SeqCst)
            .wrapping_add(1);

        let max = index_writer_util::ACTUAL_MAX_DOCS as i64;
        if new_count > max {
            self.pending_num_docs.fetch_sub(1, Ordering::SeqCst);
            return Err(LuceneError::illegal_argument(format!(
                "number of documents in the index cannot exceed {max}"
            )));
        }
        Ok(())
    }
    fn finish_documents(
        &mut self,
        delete_node: Option<Arc<Node<Q>>>,
        doc_id_up_to: i32,
    ) -> Result<i64> {
        // here we actually finish the document in two steps 1. push the delete into
        // the queue and update our slice. 2. increment the DWPT private document
        // id.
        //
        //the updated slice we get from 1. holds all the deletes that have occurred
        //since we updated the slice the last time.
        //
        // Apply delTerm only after all indexing has
        //succeeded, but apply it only to docs prior to when
        //this batch started:
        let delete_slice = self.delete_slice.as_mut().unwrap();
        let seq_no: i64 = if let Some(node) = delete_node {
            let seq = self
                .delete_queue
                .add_with_slice(node.clone(), delete_slice)?;
            debug_assert!(
                delete_slice.is_tail(&node),
                "expected the delete term as the tail item"
            );
            delete_slice.apply(&mut self.pending_updates, doc_id_up_to)?;
            seq
        } else {
            let mut seq = self.delete_queue.update_slice(delete_slice)?;
            if seq < 0 {
                seq = -seq;
                delete_slice.apply(&mut self.pending_updates, doc_id_up_to)?;
            } else {
                delete_slice.reset();
            }
            seq
        };

        Ok(seq_no)
    }
    // This method marks the last N docs as deleted. This is used
    // in the case of a non-aborting exception. There are several cases
    // where we fail a document ie. due to an exception during analysis
    // that causes the doc to be rejected but won't cause the DWPT to be
    // stale nor the entire IW to abort and shutdown. In such a case
    // we only mark these docs as deleted and turn it into a livedocs
    // during flush
    fn delete_last_docs(&mut self, doc_count: i32) -> Result<()> {
        let from = self.num_docs_in_ram - doc_count;
        let to = self.num_docs_in_ram;
        let new_len = self.num_deleted_doc_ids + (to - from) as usize;
        ArrayUtil::grow_i32(&mut self.delete_doc_ids, new_len)?;

        for doc_id in from..to {
            self.delete_doc_ids[self.num_docs_in_ram as usize] = doc_id;
            self.num_deleted_doc_ids += 1;
        }
        self.num_deleted_doc_ids = self.delete_doc_ids.len();
        // NOTE: we do not trigger flush here.  This is
        // potentially a RAM leak, if you have an app that tries
        // to add docs but every single doc always hits a
        // non-aborting exception.  Allowing a flush here gets
        // very messy because we are only invoked when handling
        // exceptions so to do this properly, while handling an
        // exception we'd have to go off and flush new deletes
        // which is risky (likely would hit some other
        // confounding exception).
        Ok(())
    }
    /// Returns the number of RAM resident documents in this [`DocumentsWriterPerThread`]
    pub fn get_num_docs_in_ram(&self) -> i32 {
        self.num_docs_in_ram
    }
    /// Prepares this DWPT for flushing. This method will freeze and return the [`DocumentsWriterDeleteQueue`]’s global buffer and apply all pending deletes to this DWPT.
    pub(crate) fn prepare_flush(&mut self) -> Result<FrozenBufferedUpdates<Q>> {
        debug_assert!(self.num_docs_in_ram > 0);

        let global_updates = self
            .delete_queue
            .freeze_global_buffer(&mut self.delete_slice)?;
        // deleteSlice can possibly be null if we have hit non-aborting exceptions during indexing and never succeeded adding a document
        if let Some(delete_slice) = self.delete_slice.as_mut() {
            // apply all deletes before we flush and release the delete slice
            delete_slice.apply(&mut self.pending_updates, self.num_docs_in_ram)?;
            debug_assert!(delete_slice.is_empty());
            delete_slice.reset();
        }
        match global_updates {
            Some(global_updates) => Ok(global_updates),
            None => Err(LuceneError::illegal_state("global_updates is None"))?,
        }
    }
    fn maybe_abort<FN>(&mut self, location: &str, flush_notifications: &mut FN) -> Result<()>
    where
        FN: FlushNotifications,
    {
        match self.aborting_exception {
            Some(_) if !self.aborted => {
                // if we are not already aborted, we can abort
                let result = self.abort();
                flush_notifications
                    .on_tragic_event(self.aborting_exception.take().unwrap(), location);
                result
            },
            _ => Ok(()),
        }
    }
    pub(crate) fn pending_files_to_delete(&self) -> &HashSet<String> {
        &self.files_to_delete
    }
    fn sort_live_docs(live_docs: &impl Bits, sort_map: &impl DocMap) -> FixedBitSet {
        let live_docs_len = live_docs.length();
        let mut sorted_live_docs = FixedBitSet::new(live_docs_len);
        sorted_live_docs.set_with_range(0, live_docs_len);

        for i in 0..live_docs_len {
            if !live_docs.get(i) {
                sorted_live_docs.clear_with_index(sort_map.old_to_new(i));
            }
        }
        sorted_live_docs
    }
    /// Seals the `SegmentInfo` for the new flushed segment and persists the deleted documents [`FixedBitSet`].
    pub(crate) fn seal_flushed_segment<FN, DM>(
        &mut self,
        flushed_segment: &mut FlushedSegment<D, Q>,
        sort_map: Option<Rc<DM>>,
        flush_notifications: &mut FN,
    ) -> Result<()>
    where
        FN: FlushNotifications,
        DM: DocMap,
    {
        // let mut new_segment = &mut flushed_segment.segment_info;
        //
        // // set diagnostics
        // index_writer_util::set_diagnostics(&mut new_segment.info, index_writer_util::SOURCE_FLUSH);
        //
        // // prepare IOContext
        // let info = &new_segment.info;
        // let context = IOContext::with_flush(FlushInfo::new(
        //     info.max_doc()?,
        //     new_segment.size_in_bytes()?,
        // ))?;
        //
        // let mut success = false;
        // let result: Result<()> = (|| {
        //     // compound file if needed
        //     if self.index_writer_config.get_use_compound_file() {
        //         let original_files = info.files()?.lock().clone();
        //         let mut dir = TrackingDirectoryWrapper::new(Arc::new(Mutex::new(self.directory.take().unwrap())));
        //         let info = new_segment.info.clone();
        //         index_writer_util::create_compound_file(
        //             &self.info_stream,
        //             &mut dir,
        //             &*info,
        //             &context,
        //             IOConsumerImpl::new(flush_notifications),
        //         )?;
        //         let dir = match Arc::try_unwrap(dir.base.delegate) {
        //             Ok(mutex) => {
        //                 mutex.into_inner()
        //             }
        //             Err(_) => return Err(LuceneError::illegal_state("TrackingDirectoryWrapper was not uniquely owned")),
        //         };
        //         self.directory = Some(dir);
        //         self.files_to_delete.extend(original_files);
        //         new_segment.info.set_use_compound_file(true);
        //     }
        //
        //     // Have codec write SegmentInfo.  Must do this after
        //     // creating CFS so that 1) .si isn't slurped into CFS,
        //     // and 2) .si reflects useCompoundFile=true change
        //     // above:
        //         LATEST_CODEC
        //         .segment_info_format()
        //         .write(self.directory.as_mut().unwrap(), &mut new_segment.info, &context)?;
        //
        //     // TODO: ideally we would freeze newSegment here!!
        //     // because any changes after writing the .si will be
        //     // lost...
        //
        //     // Must write deleted docs after the CFS so we don't
        //     // slurp the del file into CFS:
        //     if let Some(live_docs) = &flushed_segment.live_docs {
        //         let del_count = flushed_segment.del_count;
        //         debug_assert!(del_count > 0);
        //
        //         if self.info_stream.lock().enabled("DWPT") {
        //             self.info_stream.lock().message(
        //                 "DWPT",
        //                 &format!(
        //                     "flush: write {} deletes gen={}",
        //                     del_count,
        //                     new_segment.get_del_gen()
        //                 ),
        //             );
        //         }
        //         match sort_map{
        //             Some(map)  => {
        //                 LATEST_CODEC.live_docs_format().write_live_docs(
        //                     &Self::sort_live_docs(live_docs, &*map),
        //                     self.directory.as_mut().unwrap(),
        //                     new_segment,
        //                     del_count,
        //                     &context,
        //                 )?;
        //             },
        //             None => {LATEST_CODEC.live_docs_format().write_live_docs(
        //                 live_docs,
        //                 self.directory.as_mut().unwrap(),
        //                 new_segment,
        //                 del_count,
        //                 &context,
        //             )?;},
        //         }
        //
        //         new_segment.set_del_count(del_count)?;
        //         new_segment.advance_del_gen();
        //     }
        //
        //     success = true;
        //     Ok(())
        // })();
        //
        // if result.is_err() && !success {
        //     if self.info_stream.lock().enabled("DWPT") {
        //         self.info_stream.lock().message(
        //             "DWPT",
        //             &format!(
        //                 "hit exception creating compound file for newly flushed segment {}",
        //                 new_segment.info.name
        //             ),
        //         );
        //     }
        // }

        // result
        Ok(())
    }

    /// Returns true iff this DWPT is marked as flush pending
    pub(crate) fn is_flush_pending(&self) -> &bool {
        self.flush_pending.get().unwrap_or(&false)
    }
    pub(crate) fn is_queue_advanced(&self) -> bool {
        self.delete_queue.is_advanced()
    }
    /// Sets this DWPT as flush pending. This can only be set once.
    pub(crate) fn set_flush_pending(&self) -> Result<()> {
        if self.flush_pending.set(true).is_err() {
            return Err(LuceneError::illegal_state("flush_pending has been set"));
        }
        Ok(())
    }
    pub(crate) fn get_last_committed_bytes_used(&self) -> i64 {
        self.last_committed_bytes_used.load(Ordering::SeqCst)
    }
}
impl<D, P, T, O, TS, L, Q> Accountable for DocumentsWriterPerThread<D, P, T, O, TS, L, Q>
where
    D: Directory,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    TS: TokenStream,
    L: LiveIndexWriterConfig,
    Q: Query,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }

    fn get_child_resources<A>(&self) -> Vec<A>
    where
        A: Accountable,
    {
        todo!()
    }
}
impl<D, P, T, O, TS, L, Q> Display for DocumentsWriterPerThread<D, P, T, O, TS, L, Q>
where
    D: Directory,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    TS: TokenStream,
    L: LiveIndexWriterConfig,
    Q: Query,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DocumentsWriterPerThread [pendingDeletes={}, segment={}, aborted={}, numDocsInRAM={}, deleteQueue={}, {} deleted docIds]",
            self.pending_updates,
            self.segment_info.name,
            self.aborted,
            self.num_docs_in_ram,
            self.delete_queue,
            self.num_deleted_doc_ids,
        )
    }
}

pub(crate) struct FlushedSegment<D, Q>
where
    D: Directory,
    Q: Query,
{
    segment_info: SegmentCommitInfo<D>,
    field_infos: FieldInfos,
    segment_updates: Option<FrozenBufferedUpdates<Q>>,
    live_docs: Option<FixedBitSet>,
    sort_map: Option<Rc<DocMapImpl>>,
    del_count: i32,
}
impl<D, Q> FlushedSegment<D, Q>
where
    D: Directory,
    Q: Query,
{
    fn new(
        info_stream: InfoStreamLock,
        segment_info: SegmentCommitInfo<D>,
        field_infos: FieldInfos,
        mut segment_updates: Option<STBufferedUpdates<Q>>,
        live_docs: Option<FixedBitSet>,
        del_count: i32,
        sort_map: Option<Rc<DocMapImpl>>,
    ) -> Result<Self> {
        let segment_updates = match segment_updates {
            Some(ref mut upd) if upd.any() => Some(FrozenBufferedUpdates::new(
                info_stream,
                upd,
                Option::from(StringHelper::id_to_string(Some(segment_info.info.get_id()))),
            )?),
            _ => None,
        };

        Ok(FlushedSegment {
            segment_info,
            field_infos,
            segment_updates,
            live_docs,
            del_count,
            sort_map,
        })
    }
}

pub struct IOConsumerImpl<'a, FN>
where
    FN: FlushNotifications,
{
    flush_notifications: &'a mut FN,
}
impl<'a, FN> IOConsumerImpl<'a, FN>
where
    FN: FlushNotifications,
{
    pub fn new(flush_notifications: &'a mut FN) -> Self {
        IOConsumerImpl {
            flush_notifications,
        }
    }
}
impl<FN> IOConsumer for IOConsumerImpl<'_, FN>
where
    FN: FlushNotifications,
{
    type V = HashSet<String>;

    fn accept(&mut self, input: Self::V) -> Result<()> {
        self.flush_notifications.delete_unused_files(input);
        Ok(())
    }
}
