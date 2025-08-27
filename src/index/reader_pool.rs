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
use crate::index::field_infos::FieldNumbers;
use crate::index::readers_and_updates::ReadersAndUpdates;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::store::directory::Directory;
use crate::store::lock_validating_directory_wrapper::LockValidatingDirectoryWrapper;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::info_stream::InfoStreamMT;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Holds shared SegmentReader instances.
/// IndexWriter uses SegmentReaders for
/// 1) applying deletes/DV updates,
/// 2) doing merges,
/// 3) handing out a real-time reader.
/// This pool reuses instances of the SegmentReaders in all these places
/// if it is in "near real-time mode" (getReader() has been called on this instance).
pub(crate) struct ReaderPool<D>
where
    D: Directory,
{
    directory: Arc<LockValidatingDirectoryWrapper<D>>,
    original_directory: Arc<D>,
    field_numbers: Arc<FieldNumbers>,
    info_stream: InfoStreamMT,
    soft_deletes_field: Option<String>,
    // This is a "write once" variable (like the organic dye
    // on a DVD-R that may or may not be heated by a laser and
    // then cooled to permanently record the event): it's
    // false, by default until {@link #enableReaderPooling()}
    // is called for the first time,
    // at which point it's switched to true and never changes
    // back to false.  Once this is true, we hold open and
    // reuse SegmentReader instances internally for applying
    // deletes, doing merges, and reopening near real-time
    // readers.
    // in practice this should be called once the readers are likely
    // to be needed and reused ie if IndexWriter#getReader is called.
    pool_readers: AtomicBool,
    inner: Mutex<Inner<D>>,
}
pub(crate) struct Inner<D>
where
    D: Directory,
{
    reader_map: HashMap<String, ReadersAndUpdates<D>>,
    closed: bool,
}

impl<D> ReaderPool<D>
where
    D: Directory,
{
    /// Asserts this info still exists in IW's segment infos
    pub(crate) fn assert_info_is_live(&self, _info: &SegmentCommitInfo<D>) -> bool {
        todo!()
    }
    /// Drops reader for the given SegmentCommitInfo if it's pooled
    pub(crate) fn drop(&self, info: &SegmentCommitInfo<D>) -> Result<bool> {
        let mut inner = self.inner.lock();
        if let Some(rld) = inner.reader_map.remove(&info.info.get_id_str()) {
            debug_assert_eq!(info.info.get_id_str(), rld.get_info_id(None));
            rld.drop_readers()?;
            return Ok(true);
        }
        Ok(false)
    }
    /// Returns the sum of the ram used by all the buffered readers and updates in MB
    pub(crate) fn ram_bytes_used(&self) -> i64 {
        let inner = self.inner.lock();
        let mut bytes: i64 = 0;
        for rld in inner.reader_map.values() {
            bytes += rld
                .ram_bytes_used
                .load(std::sync::atomic::Ordering::Relaxed);
        }
        bytes
    }
    /// Returns true iff any of the buffered readers and updates has at least one pending delete
    pub(crate) fn any_deletions(
        &self,
        infos: &HashMap<String, SegmentCommitInfo<D>>,
    ) -> Result<bool> {
        let inner = self.inner.lock();
        for rld in inner.reader_map.values() {
            let info = match infos.get(&rld.get_info_id(None)) {
                Some(info) => info,
                None => return Err(LuceneError::illegal_state("SegmentCommitInfo missing")),
            };
            if rld.get_del_count(info) > 0 {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// Enables reader pooling for this pool. This should be called once the readers in this pool are
    /// shared with an outside resource like an NRT reader. Once reader pooling is enabled a `ReadersAndUpdates`
    /// will be kept around in the reader pool on calling `release(ReadersAndUpdates, boolean)` until the
    /// segment get dropped via calls to `drop(SegmentCommitInfo)` or `dropAll()` or `close()`. Reader pooling
    /// is disabled upon construction but can't be disabled again once it's enabled.
    pub(crate) fn enable_reader_pooling(&self) {
        self.pool_readers
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub(crate) fn is_reader_pooling_enabled(&self) -> bool {
        self.pool_readers.load(std::sync::atomic::Ordering::Relaxed)
    }
    pub(crate) fn release(
        &self,
        rld: ReadersAndUpdates<D>,
        assert_info_live: bool,
        info: &mut SegmentCommitInfo<D>,
    ) -> Result<bool> {
        // let inner = self.inner.lock();
        // let mut changed = false;
        //
        // // Matches incRef in get:
        // rld.dec_ref();
        // let info_id = rld.get_info_id(None);
        // if rld.ref_count() == 0 {
        //     // This happens if the segment was just merged away,
        //     // while a buffered deletes packet was still applying deletes/updates to it.
        //     debug_assert!(
        //         !inner.reader_map.contains_key(&info_id),
        //         "seg={} has refCount 0 but still unexpectedly exists in the reader pool",
        //         info_id
        //     );
        // } else {
        //     // Pool still holds a ref:
        //     debug_assert!(
        //         rld.ref_count() > 0,
        //         "refCount={} reader={:?}",
        //         rld.ref_count(),
        //         info_id
        //     );
        //
        //     if !self.is_reader_pooling_enabled()
        //         && rld.ref_count() == 1
        //         && inner.reader_map.contains_key(&info_id)
        //     {
        //         // This is the last ref to this RLD, and we're not
        //         // pooling, so remove it:
        //         if rld.write_live_docs(self.directory.clone(),info)? {
        //             // Make sure we only write del docs for a live segment:
        //             debug_assert!(
        //                 !assert_info_live || self.assert_info_is_live(rld.info()),
        //                 "assertInfoIsLive failed for {:?}",
        //                 info_id
        //             );
        //             // Must checkpoint because we just created new _X_N.del and field updates files;
        //             // don't call IW.checkpoint because that also increments SIS.version,
        //             // which we do not want to do here.
        //             changed = true;
        //         }
        //         if rld.write_field_updates(
        //             &self.directory,
        //             &self.field_numbers,
        //             (self.completed_del_gen_supplier)(),
        //             &self.info_stream,
        //         )? {
        //             changed = true;
        //         }
        //         if rld.get_num_dv_updates() == 0 {
        //             rld.drop_readers()?;
        //             inner.reader_map.remove(&info_id);
        //         } else {
        //             // We are forced to pool this segment until its deletes fully apply
        //             // (no delGen gaps)
        //         }
        //     }
        // }
        //
        // Ok(changed)
        todo!()
    }
}
