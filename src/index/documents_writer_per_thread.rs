/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::analysis::token_stream::TokenStream;
use crate::document::numeric_doc_values_field::NumericDocValuesField;
use crate::index::buffered_updates::{MTBufferedUpdates, STBufferedUpdates};
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
    pub(crate) directory: TrackingDirectoryWrapper<D>,
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
    pub fn get_num_docs_in_ram(&self) -> i32 {
        self.num_docs_in_ram
    }
    pub(crate) fn prepare_flush(&mut self) -> Result<FrozenBufferedUpdates<Q>> {
        debug_assert!(self.num_docs_in_ram > 0);

        let global_updates = self
            .delete_queue
            .freeze_global_buffer(&mut self.delete_slice)?;
        let delete_slice = self.delete_slice.as_mut().unwrap();

        if !delete_slice.is_empty() {
            delete_slice.apply(&mut self.pending_updates, self.num_docs_in_ram)?;
            debug_assert!(delete_slice.is_empty());
            delete_slice.reset();
        }
        match global_updates {
            Some(global_updates) => Ok(global_updates),
            None => Err(LuceneError::illegal_state("global_updates is None"))?,
        }
    }
    pub(crate) fn pending_files_to_delete(&self) -> &HashSet<String> {
        &self.files_to_delete
    }
    fn sort_live_docs(live_docs: &impl Bits, sort_map: &DocMapImpl) -> FixedBitSet {
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
    live_docs: FixedBitSet,
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
        live_docs: FixedBitSet,
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
