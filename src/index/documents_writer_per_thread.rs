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
use crate::index::buffered_updates::STBufferedUpdates;
use crate::index::documents_writer_delete_queue::{DeleteSlice, DocumentsWriterDeleteQueue};
use crate::index::field_infos::build::Builder;
use crate::index::field_infos::FieldInfos;
use crate::index::frozen_buffered_updates::FrozenBufferedUpdates;
use crate::index::indexing_chain::{IndexingChain, ReservedField};
use crate::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::segment_info::SegmentInfo;
use crate::index::sorter::DocMapImpl;
use crate::search::query::Query;
use crate::store::directory::Directory;
use crate::store::tracking_directory_wrapper::TrackingDirectoryWrapper;
use crate::util::error::lucene_error::Result;
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::info_stream::InfoStreamLock;
use crate::util::StringHelper;
use std::cell::OnceCell;
use std::rc::Rc;
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, Mutex};

pub struct DocumentsWriterPerThread<D, P, T, O, TS, L, Q>
where
    D: Directory,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    TS: TokenStream,
    L: LiveIndexWriterConfig,
    Q: Query,
{
    pub directory: TrackingDirectoryWrapper<D>,
    pub indexing_chain: IndexingChain<D, O, P, T, TS, L>,
    pub pending_updates: STBufferedUpdates<Q>,
    pub segment_info: SegmentInfo<D>,
    pub aborted: bool,
    pub flush_pending: OnceCell<bool>,
    pub last_committed_bytes_used: AtomicI64,
    pub has_flushed: OnceCell<bool>,
    pub field_infos: Builder,
    pub info_stream: InfoStreamLock,
    pub num_docs_in_ram: i32,
    pub delete_queue: Arc<Mutex<DocumentsWriterDeleteQueue<Q>>>,
    pub delete_slice: DeleteSlice<Q>,
    pub pending_num_docs: AtomicI64,
    pub index_writer_config: L,
    pub enable_test_points: bool,
    pub delete_doc_ids: Vec<i32>,
    pub num_deleted_doc_ids: usize,
    pub index_major_version_created: i32,
    pub parent_field: ReservedField<NumericDocValuesField>,
}

pub(crate) struct FlushedSegment<D, Q>
where
    D: Directory,
    Q: Query,
{
    pub segment_info: SegmentCommitInfo<D>,
    pub field_infos: FieldInfos,
    pub segment_updates: Option<FrozenBufferedUpdates<Q>>,
    pub live_docs: FixedBitSet,
    pub sort_map: Option<Rc<DocMapImpl>>,
    pub del_count: i32,
}
impl<D, Q> FlushedSegment<D, Q>
where
    D: Directory,
    Q: Query,
{
    pub fn new(
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
