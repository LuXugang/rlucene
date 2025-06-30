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
use crate::index::buffered_updates::STBufferedUpdates;
use crate::index::field_infos::FieldInfos;
use crate::index::frozen_buffered_updates::FrozenBufferedUpdates;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::sorter::DocMapImpl;
use crate::search::query::Query;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::info_stream::InfoStreamLock;
use crate::util::StringHelper;
use std::rc::Rc;

pub(crate) struct DocumentsWriterPerThread;

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
