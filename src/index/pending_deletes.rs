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
use crate::codecs::{Codec, get_default_code};
use crate::index::codec_reader::CodecReader;
use crate::index::doc_values_field_updates::{DocValuesFieldIteratorEnum, MergedIterator};
use crate::index::field_info::FieldInfo;
use crate::index::index_reader::IndexReader;
use crate::index::leaf_reader::LeafReader;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::segment_reader::SegmentReader;
use crate::store::IOContext;
use crate::store::directory::Directory;
use crate::store::tracking_directory_wrapper::TrackingDirectoryWrapper;
use crate::util::IOUtils;
use crate::util::bits::{Bits, Either2Bits};
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::{FixedBit, FixedBitSet};
use std::fmt;
use std::sync::Arc;

/// This class handles accounting and applying pending deletes for live segment readers
pub(crate) struct PendingDeletes<D>
where
    D: Directory,
{
    // SegmentInfo#id
    pub(crate) info_id: String,
    live_docs: Option<DocBits<D>>,
    writeable_live_docs: bool,
    pub(crate) pending_delete_count: i32,
    pub(crate) live_docs_initialized: bool,
    max_doc: i32,
}
impl<D> PendingDeletes<D>
where
    D: Directory,
{
    pub(crate) fn from_reader(
        reader: &SegmentReader<D>,
        info: &SegmentCommitInfo<D>,
    ) -> Result<Self> {
        let mut v = Self::with(
            info.info.get_id_str(),
            reader.get_live_docs()?.map(Either2Bits::A),
            true,
            info.info.max_doc()?,
        );
        v.pending_delete_count = reader.num_deleted_docs()? - info.get_del_count();
        Ok(v)
    }
    pub(crate) fn new(info: &SegmentCommitInfo<D>) -> Result<Self> {
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
        live_docs: Option<DocBits<D>>,
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
                Some(Either2Bits::B(Either2Bits::B(
                    self.live_docs.take().unwrap().copy_of(),
                )))
            } else {
                let mut v = FixedBitSet::new(self.max_doc);
                v.set_with_range(0, self.max_doc);
                Some(Either2Bits::B(Either2Bits::B(v)))
            };
        }
        match self.live_docs.as_mut().unwrap() {
            Either2Bits::B(bs) => match bs {
                Either2Bits::A(_) => Err(LuceneError::illegal_state(
                    "live_docs should be FixedBitSet ",
                )),
                Either2Bits::B(v) => Ok(v),
            },
            Either2Bits::A(_) => Err(LuceneError::illegal_state(
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
    pub(crate) fn get_live_docs(&mut self) -> Option<LiveDocsBits<D>> {
        // Prevent modifications to the returned live docs
        self.writeable_live_docs = false;
        match self.live_docs.take() {
            Some(Either2Bits::A(bits)) => {
                self.live_docs = Some(Either2Bits::A(bits.clone()));
                Some(Either2Bits::A(bits))
            },
            Some(Either2Bits::B(bits)) => match bits {
                Either2Bits::A(fixed_bits) => {
                    self.live_docs = Some(Either2Bits::B(Either2Bits::A(fixed_bits.clone())));
                    Some(Either2Bits::B(fixed_bits))
                },
                Either2Bits::B(fixed_bit_set) => {
                    let v = Arc::new(fixed_bit_set.to_read_only_bits());
                    self.live_docs = Some(Either2Bits::B(Either2Bits::A(v.clone())));
                    Some(Either2Bits::B(v))
                },
            },
            None => None,
        }
    }

    /// Returns a snapshot of the hard live docs.
    pub(crate) fn get_hard_live_docs(&mut self) -> Option<LiveDocsBits<D>> {
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
    /// Writes the live docs to disk and returns `true` if any new docs were written.
    pub(crate) fn write_live_docs(
        &mut self,
        dir: Arc<D>,
        info: &mut SegmentCommitInfo<D>,
    ) -> Result<bool> {
        if self.pending_delete_count == 0 {
            return Ok(false);
        }

        let live_docs = match self.live_docs.as_ref() {
            Some(ld) => ld,
            None => return Err(LuceneError::illegal_state("liveDocs must be initialized")),
        };
        debug_assert!(info.info.max_doc()? == self.max_doc);
        // We have new deletes
        debug_assert_eq!(
            live_docs.length(),
            info.info.max_doc()?,
            "liveDocs.length must match maxDoc"
        );
        // Do this so we can delete any created files on
        // exception; this saves all codecs from having to do
        // it:
        let tracking_dir = TrackingDirectoryWrapper::new(dir);
        // We can write directly to the actual name (vs to a
        // .tmp & renaming it) because the file is not live
        // until segments file is written:
        let write_res = (|| -> Result<()> {
            let codec = get_default_code();
            codec.live_docs_format().write_live_docs(
                live_docs,
                &tracking_dir,
                info,
                self.pending_delete_count,
                &IOContext::default_io_context()?,
            )?;
            Ok(())
        })();

        if let Err(err) = write_res {
            // Advance only the nextWriteDelGen so that a 2nd
            // attempt to write will write to a new file
            info.advance_next_write_del_gen();
            // Delete any partially created file(s):
            for file in tracking_dir.get_created_files() {
                IOUtils::delete_files_ignoring_exceptions(&*tracking_dir.base.delegate, &[&file]);
            }
            return Err(err);
        }
        // If we hit an exc in the line above (eg disk full)
        // then info's delGen remains pointing to the previous
        // (successfully written) del docs:
        info.advance_del_gen();
        let new_del_count = info.get_del_count() + self.pending_delete_count;
        info.set_del_count(new_del_count)?;
        self.drop_changes();

        Ok(true)
    }
    pub(crate) fn is_fully_deleted<F>(
        &self,
        _reader_io_supplier: F,
        info: &SegmentCommitInfo<D>,
    ) -> Result<bool>
    where
        F: Fn() -> Arc<SegmentReader<D>>,
    {
        debug_assert!(info.info.max_doc()? == self.max_doc);
        Ok(self.get_del_count(info) == info.info.max_doc()?)
    }

    pub(crate) fn on_doc_values_update(
        &self,
        _info: &FieldInfo,
        _iterator: Option<MergedIterator<DocValuesFieldIteratorEnum>>,
    ) {
    }

    /// Returns true if the given reader needs to be refreshed to see the latest deletes
    pub(crate) fn needs_refresh(
        &mut self,
        reader: &SegmentReader<D>,
        info: &SegmentCommitInfo<D>,
    ) -> Result<bool> {
        let same_live_docs = match (reader.get_live_docs()?, self.get_live_docs()) {
            (None, None) => true,
            (Some(reader_bits), Some(Either2Bits::A(current_bits))) => {
                Arc::ptr_eq(&reader_bits, &current_bits)
            },
            _ => false,
        };
        Ok(!same_live_docs || reader.num_deleted_docs()? != self.get_del_count(info))
    }

    /// Returns the number of deleted docs in the segment.
    pub(crate) fn get_del_count(&self, info: &SegmentCommitInfo<D>) -> i32 {
        info.get_del_count() + info.get_soft_del_count() + self.num_pending_deletes()
    }
    /// Returns the number of live documents in this segment
    pub(crate) fn num_docs(&self, info: &SegmentCommitInfo<D>) -> Result<i32> {
        debug_assert!(info.info.max_doc()? == self.max_doc);
        let max_doc = info.info.max_doc()?;
        Ok(max_doc - self.get_del_count(info))
    }

    // Call only from assert!
    pub(crate) fn verify_doc_counts(
        &mut self,
        reader: &impl CodecReader,
        info: &SegmentCommitInfo<D>,
    ) -> Result<bool> {
        debug_assert!(info.info.max_doc()? == self.max_doc);
        let max_doc = info.info.max_doc()?;
        let mut count = 0;
        match self.get_live_docs() {
            Some(bits) => {
                for doc_id in 0..max_doc {
                    if bits.get(doc_id) {
                        count += 1;
                    }
                }
            },
            _ => {
                count = max_doc;
            },
        }

        debug_assert_eq!(
            self.num_docs(info)?,
            count,
            "info.maxDoc={} info.getDelCount={} info.getSoftDelCount={} pendingDeletes={} count={} numDocs={}",
            max_doc,
            info.get_del_count(),
            info.get_soft_del_count(),
            self.num_pending_deletes(),
            count,
            self.num_docs(info)?
        );

        debug_assert_eq!(
            reader.num_docs()?,
            self.num_docs(info)?,
            "reader.numDocs={} numDocs={}",
            reader.num_docs()?,
            self.num_docs(info)?
        );

        debug_assert!(
            reader.num_deleted_docs()? <= max_doc,
            "delCount={} info.maxDoc={} pendingDeleteCount={} info.getDelCount={}",
            reader.num_deleted_docs()?,
            max_doc,
            self.num_pending_deletes(),
            info.get_del_count()
        );
        Ok(true)
    }
    /// Returns `true` if this `PendingDeletes` must be initialized before [`delete`](Self::delete);
    /// otherwise it is ready to accept deletes.
    /// A `PendingDeletes` can be initialized by providing it a reader via [`on_new_reader`](Self::on_new_reader).
    pub(crate) fn must_init_on_delete(&self) -> bool {
        false
    }
}
pub(crate) type LiveDocsBits<D> =
    Either2Bits<Arc<<SegmentReader<D> as LeafReader>::Bits>, Arc<FixedBit>>;
pub(crate) type DocBits<D> = Either2Bits<
    Arc<<SegmentReader<D> as LeafReader>::Bits>,
    Either2Bits<Arc<FixedBit>, FixedBitSet>,
>;
impl<D> fmt::Display for PendingDeletes<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}(seg={} numPendingDeletes={} writeable={})",
            std::any::type_name::<Self>(),
            self.info_id,
            self.pending_delete_count,
            self.writeable_live_docs
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::codecs::field_infos_format::FieldInfosFormat;
    use crate::codecs::live_docs_format::LiveDocsFormat;
    use crate::codecs::{Codec, get_default_code};
    use crate::index::field_infos::FieldInfos;
    use crate::index::pending_deletes::PendingDeletes;
    use crate::index::segment_commit_info::SegmentCommitInfo;
    use crate::index::segment_info::SegmentInfo;
    use crate::index::segment_reader::SegmentReader;
    use crate::store::IOContext;
    use crate::store::directory::Directory;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{new_directory, random};
    use crate::util::bits::Bits;
    use crate::util::error::lucene_error::Result;
    use crate::util::{LATEST, StringHelper};
    use parking_lot::Mutex;
    use rand::Rng;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[allow(dead_code)] // for quick search
    struct TestPendingDeletes;

    fn new_pending_deletes<D>(commit_info: &SegmentCommitInfo<D>) -> Result<PendingDeletes<D>>
    where
        D: Directory,
    {
        PendingDeletes::new(commit_info)
    }
    #[test]
    fn test_delete_doc() -> Result<()> {
        // TODO: ByteBuffersDirectory 没有实现
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);
        let si = SegmentInfo::new(
            dir.clone(),
            Some((*LATEST).clone()),
            Some((*LATEST).clone()),
            "test",
            10,
            false,
            false,
            HashMap::new(),
            StringHelper::random_id(),
            HashMap::new(),
            None,
        )?;
        let commit_info =
            SegmentCommitInfo::new(si, 0, 0, -1, -1, -1, Some(StringHelper::random_id()))?;

        let mut deletes = new_pending_deletes(&commit_info)?;
        assert!(deletes.get_live_docs().is_none());

        let doc_to_delete = random.random_range(0..=7);
        assert!(deletes.delete(doc_to_delete)?);
        let mut live_docs = deletes.get_live_docs().unwrap();
        assert_eq!(deletes.num_pending_deletes(), 1);

        assert!(!live_docs.get(doc_to_delete));
        assert!(!deletes.delete(doc_to_delete)?);

        assert!(live_docs.get(8));
        assert!(deletes.delete(8)?);
        assert!(live_docs.get(8));
        assert_eq!(deletes.num_pending_deletes(), 2);

        assert!(live_docs.get(9));
        assert!(deletes.delete(9)?);
        assert!(live_docs.get(9));

        live_docs = deletes.get_live_docs().unwrap();
        assert!(!live_docs.get(8));
        assert!(!live_docs.get(9));
        assert!(!live_docs.get(doc_to_delete));
        assert_eq!(deletes.num_pending_deletes(), 3);
        Ok(())
    }
    #[test]
    fn test_write_live_docs() -> Result<()> {
        // TODO: ByteBuffersDirectory 没有实现
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);
        let si = SegmentInfo::new(
            dir.clone(),
            Some((*LATEST).clone()),
            Some((*LATEST).clone()),
            "test",
            6,
            false,
            false,
            HashMap::new(),
            StringHelper::random_id(),
            HashMap::new(),
            None,
        )?;
        let mut commit_info =
            SegmentCommitInfo::new(si, 0, 0, -1, -1, -1, Some(StringHelper::random_id()))?;

        let mut deletes = new_pending_deletes(&commit_info)?;
        assert!(!deletes.write_live_docs(dir.clone(), &mut commit_info)?);
        assert_eq!(dir.list_all()?.len(), 0);

        let second_doc_deletes: bool = random.random_bool(0.5);
        deletes.delete(5)?;
        if second_doc_deletes {
            let _ = deletes.get_live_docs();
            deletes.delete(2)?;
        }

        assert_eq!(commit_info.get_del_gen(), -1);
        assert_eq!(commit_info.get_del_count(), 0);

        let expected_pending = if second_doc_deletes { 2 } else { 1 };
        assert_eq!(deletes.num_pending_deletes(), expected_pending);

        assert!(deletes.write_live_docs(dir.clone(), &mut commit_info)?);
        assert_eq!(dir.list_all()?.len(), 1);

        let codec = get_default_code();
        let live_docs = codec.live_docs_format().read_live_docs(
            &*dir,
            &commit_info,
            &IOContext::default_io_context()?,
        )?;
        assert!(!live_docs.get(5));
        if second_doc_deletes {
            assert!(!live_docs.get(2));
        } else {
            assert!(live_docs.get(2));
        }
        for doc in &[0, 1, 3, 4] {
            assert!(live_docs.get(*doc));
        }

        assert_eq!(deletes.num_pending_deletes(), 0);
        assert_eq!(commit_info.get_del_count(), expected_pending);
        assert_eq!(commit_info.get_del_gen(), 1);

        deletes.delete(0)?;
        assert!(deletes.write_live_docs(dir.clone(), &mut commit_info)?);
        assert_eq!(dir.list_all()?.len(), 2);

        let live_docs = codec.live_docs_format().read_live_docs(
            &*dir,
            &commit_info,
            &IOContext::default_io_context()?,
        )?;
        assert!(!live_docs.get(5));
        if second_doc_deletes {
            assert!(!live_docs.get(2));
        } else {
            assert!(live_docs.get(2));
        }
        assert!(!live_docs.get(0));
        for doc in &[1, 3, 4] {
            assert!(live_docs.get(*doc));
        }

        assert_eq!(deletes.num_pending_deletes(), 0);
        let expected_total = expected_pending + 1;
        assert_eq!(commit_info.get_del_count(), expected_total);
        assert_eq!(commit_info.get_del_gen(), 2);

        Ok(())
    }
    #[test]
    fn test_is_fully_deleted() -> Result<()> {
        // TODO: ByteBuffersDirectory 没有实现
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);
        let si = SegmentInfo::new(
            dir.clone(),
            Some((*LATEST).clone()),
            Some((*LATEST).clone()),
            "test",
            3,
            false,
            false,
            HashMap::new(),
            StringHelper::random_id(),
            HashMap::new(),
            None,
        )?;
        let mut commit_info =
            SegmentCommitInfo::new(si, 0, 0, -1, -1, -1, Some(StringHelper::random_id()))?;

        let codec = get_default_code();
        let field_infos = FieldInfos::new(Vec::new())?;
        codec.field_infos_format().write(
            &*dir,
            &commit_info.info,
            "",
            &field_infos,
            &IOContext::default_io_context()?,
        )?;

        let mut deletes = new_pending_deletes(&commit_info)?;

        for i in 0..3 {
            assert!(deletes.delete(i)?);
            if random.random_bool(0.5) {
                assert!(deletes.write_live_docs(dir.clone(), &mut commit_info)?);
            }
            let io_context = IOContext::default_io_context()?;

            assert_eq!(
                i == 2,
                deletes.is_fully_deleted(
                    || {
                        let sr = SegmentReader::new(&commit_info.clone(), 0, &io_context)
                            .expect("should not failed here");
                        Arc::new(sr)
                    },
                    &commit_info
                )?
            );
        }

        Ok(())
    }
}
