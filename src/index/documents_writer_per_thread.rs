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
    pub segment_updates: Option<FrozenBufferedUpdates<Q, InfoStreamLock>>,
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
