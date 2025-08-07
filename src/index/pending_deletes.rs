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
use crate::util::bits::Bits;
use crate::util::either_enums::EitherBits;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::FixedBitSet;
use std::fmt;
/// This class handles accounting and applying pending deletes for live segment readers
pub(crate) struct PendingDeletes<L>
where
    L: LeafReader,
{
    // SegmentInfo#id
    pub(crate) info_id: String,
    live_docs: Option<EitherBits<L::Bits, FixedBitSet>>,
    writeable_live_docs: bool,
    pub(crate) pending_delete_count: i32,
    live_docs_initialized: bool,
    max_doc: i32,
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
            Some(EitherBits::F(reader.get_live_docs()?)),
            true,
            info.info.max_doc()?,
        );
        v.pending_delete_count = reader.num_deleted_docs()? - info.get_del_count();
        Ok(v)
    }
    pub(crate) fn new<D>(info: &SegmentCommitInfo<D>) -> Result<Self>
    where
        D: Directory,
    {
        Ok(PendingDeletes::with(
            info.info.get_id_str(),
            None,
            !info.has_deletions(),
            info.info.max_doc()?,
        ))
        // if we don't have deletions we can mark it as initialized since we might receive deletes on a
        // segment
        // without having a reader opened on it ie. after a merge when we apply the deletes that IW
        // received while merging.
        // For segments that were published we enforce a reader in the
        // BufferedUpdatesStream.SegmentState ctor
    }

    pub(crate) fn with(
        info_id: String,
        live_docs: Option<EitherBits<L::Bits, FixedBitSet>>,
        live_docs_initialized: bool,
        max_doc: i32,
    ) -> Self {
        PendingDeletes {
            info_id,
            live_docs,
            writeable_live_docs: false,
            pending_delete_count: 0,
            live_docs_initialized,
            max_doc,
        }
    }
    pub(crate) fn get_mutable_bits(&mut self) -> Result<&mut FixedBitSet> {
        // if we pull mutable bits but we haven't been initialized something is completely off.
        // this means we receive deletes without having the bitset that is on-disk ready to be cloned
        assert!(
            self.live_docs_initialized,
            "can't delete if liveDocs are not initialized"
        );
        if !self.writeable_live_docs {
            self.live_docs = if self.live_docs.is_some() {
                Some(EitherBits::S(self.live_docs.take().unwrap().copy_of()))
            } else {
                let mut v = FixedBitSet::new(self.max_doc);
                v.set_with_range(0, self.max_doc);
                Some(EitherBits::S(v))
            };
        }
        match self.live_docs.as_mut().unwrap() {
            EitherBits::S(bs) => Ok(bs),
            EitherBits::F(_) => Err(LuceneError::illegal_state(
                "live_docs should be FixedBitSet ",
            )),
        }
    }
    /// Marks a document as deleted in this segment and return true if a document got actually deleted or if the document was already deleted.
    pub(crate) fn delete(&mut self, doc_id: i32) -> Result<bool> {
        debug_assert!(self.max_doc > 0);

        let mutable_bits = self.get_mutable_bits()?;
        debug_assert!(mutable_bits.length() > 0);

        debug_assert!(
            (0..mutable_bits.length()).contains(&doc_id),
            "out of bounds: docID={} liveDocsLength={} seg={} maxDoc={}",
            doc_id,
            mutable_bits.length(),
            self.info_id,
            self.max_doc
        );

        let did_delete = mutable_bits.get_and_clear(doc_id);
        if did_delete {
            self.pending_delete_count += 1;
        }
        Ok(did_delete)
    }
    /// Returns a snapshot of the current live docs.
    pub(crate) fn get_live_docs(&mut self) -> Option<&EitherBits<L::Bits, FixedBitSet>> {
        // Prevent modifications to the returned live docs
        self.writeable_live_docs = false;
        self.live_docs.as_ref()
    }

    /// Returns a snapshot of the hard live docs.
    pub(crate) fn get_hard_live_docs(&mut self) -> Option<&EitherBits<L::Bits, FixedBitSet>> {
        self.get_live_docs()
    }

    /// Returns the number of pending deletes that are not yet flushed to disk.
    pub(crate) fn num_pending_deletes(&self) -> i32 {
        self.pending_delete_count
    }

    fn assert_check_live_docs(
        &self,
        bits: &impl Bits,
        expected_length: i32,
        expected_delete_count: i32,
    ) -> bool {
        debug_assert_eq!(
            bits.length(),
            expected_length,
            "length: {} != expected: {}",
            bits.length(),
            expected_length
        );

        let mut deleted = 0;
        for i in 0..bits.length() {
            if !bits.get(i) {
                deleted += 1;
            }
        }

        debug_assert_eq!(
            deleted, expected_delete_count,
            "deleted: {deleted} != expected: {expected_delete_count}"
        );

        true
    }

    /// Resets the pending docs
    pub(crate) fn drop_changes(&mut self) {
        self.pending_delete_count = 0;
    }
}
impl<L> fmt::Display for PendingDeletes<L>
where
    L: LeafReader,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PendingDeletes(seg={} numPendingDeletes={} writeable={})",
            self.info_id, self.pending_delete_count, self.writeable_live_docs
        )
    }
}
