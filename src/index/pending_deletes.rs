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
use crate::codecs::live_docs_format::LiveDocsFormat;
use crate::index::index_reader::IndexReader;
use crate::index::leaf_reader::LeafReader;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::segment_reader::SegmentReader;
use crate::store::directory::Directory;
use crate::util::either_enums::EitherBitSet;
use crate::util::error::lucene_error::Result;
use crate::util::fixed_bit_set::FixedBitSet;

pub(crate) struct PendingDeletes<L>
where
    L: LeafReader,
{
    // SegmentInfo#id
    pub(crate) info_id: String,
    live_docs: Option<EitherBitSet<L::Bits, FixedBitSet>>,
    writeable_live_docs: Option<FixedBitSet>,
    pub(crate) pending_delete_count: i32,
    live_docs_initialized: bool,
}
impl<L> PendingDeletes<L>
where
    L: LeafReader,
{
    pub(crate) fn from_reader<D, LF>(
        reader: &SegmentReader<LF>,
        info: &SegmentCommitInfo<D>,
    ) -> Result<Self>
    where
        D: Directory,
        LF: LiveDocsFormat<Bits = L::Bits>,
    {
        let mut v = Self::with(
            info.info.get_id_str(),
            Some(EitherBitSet::F(reader.get_live_docs()?)),
            true,
        );
        v.pending_delete_count = reader.num_deleted_docs() - info.get_del_count();
        Ok(v)
    }
    pub(crate) fn new<D>(info: &SegmentCommitInfo<D>) -> Self
    where
        D: Directory,
    {
        PendingDeletes::with(info.info.get_id_str(), None, !info.has_deletions())
        // if we don't have deletions we can mark it as initialized since we might receive deletes on a
        // segment
        // without having a reader opened on it ie. after a merge when we apply the deletes that IW
        // received while merging.
        // For segments that were published we enforce a reader in the
        // BufferedUpdatesStream.SegmentState ctor
    }

    pub(crate) fn with(
        info_id: String,
        live_docs: Option<EitherBitSet<L::Bits, FixedBitSet>>,
        live_docs_initialized: bool,
    ) -> Self {
        PendingDeletes {
            info_id,
            live_docs,
            writeable_live_docs: None,
            pending_delete_count: 0,
            live_docs_initialized,
        }
    }
    pub(crate) fn get_mutable_bits(&mut self, max_doc: i32) -> &FixedBitSet {
        // // if we pull mutable bits but we haven't been initialized something is completely off.
        // // this means we receive deletes without having the bitset that is on-disk ready to be cloned
        // assert!(
        //     self.live_docs_initialized,
        //     "can't delete if liveDocs are not initialized"
        // );
        // if self.writeable_live_docs.is_none() {
        //     // Copy on write: this means we've cloned a
        //     // SegmentReader sharing the current liveDocs
        //     // instance; must now make a private clone so we can
        //     // change it:
        //     let mut fb = if let Some(ref bits) = self.live_docs {
        //         bits.clone()
        //     } else {
        //         let mut new_bits = FixedBitSet::new(max_doc);
        //         new_bits.set_with_range(0, max_doc);
        //         new_bits
        //     };
        //     self.writeable_live_docs = Some(fb.clone());
        //     self.live_docs = Some(fb);
        // }
        // self.writeable_live_docs.as_ref().unwrap()
        todo!()
    }
}
