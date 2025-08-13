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
use crate::index::doc_values_field_updates::DocValuesFieldUpdatesEnum;
use crate::index::leaf_reader::LeafReader;
use crate::index::pending_deletes::PendingDeletes;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::segment_reader::SegmentReader;
use crate::index::sorter::DocMapImpl;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};

pub(crate) struct ReadersAndUpdates<L, LF>
where
    L: LeafReader,
    LF: LiveDocsFormat,
{
    // Tracks how many consumers are using this instance:
    ref_count: AtomicI32, // starts at 1
    // the major version this index was created with
    index_created_version_major: i32,
    // Only set if there are doc values updates against this segment, and the index is sorted:
    sort_map: Option<Rc<DocMapImpl>>,
    ram_bytes_used: AtomicI64,
    inner: Mutex<Inner<L, LF>>,
}

pub(crate) struct Inner<L, LF>
where
    L: LeafReader,
    LF: LiveDocsFormat,
{
    // Set once (None, and then maybe set, and never set again):
    reader: Option<SegmentReader<LF>>,
    // How many further deletions we've done against
    // liveDocs vs when we loaded it or last wrote it:
    pending_deletes: PendingDeletes<L>,
    // Indicates whether this segment is currently being merged. While a segment
    // is merging, all field updates are also registered in the
    // mergingDVUpdates map. Also, calls to writeFieldUpdates merge the
    // updates with mergingDVUpdates.
    // That way, when the segment is done merging, IndexWriter can apply the
    // updates on the merged segment too.
    is_merging: bool,
    // Holds resolved (to docIDs) doc values updates that have not yet been
    // written to the index
    pending_dv_updates: HashMap<String, Vec<DocValuesFieldUpdatesEnum>>,
    // Holds resolved (to docIDs) doc values updates that were resolved while
    // this segment was being merged; at the end of the merge we carry over
    // these updates (remapping their docIDs) to the newly merged segment
    merging_dv_updates: HashMap<String, Vec<DocValuesFieldUpdatesEnum>>,
}

impl<L, LF> ReadersAndUpdates<L, LF>
where
    L: LeafReader,
    LF: LiveDocsFormat,
{
    pub(crate) fn new(
        index_created_version_major: i32,
        pending_deletes: PendingDeletes<L>,
    ) -> Self {
        let inner = Mutex::new(Inner {
            reader: None,
            pending_deletes,
            is_merging: false,
            pending_dv_updates: HashMap::new(),
            merging_dv_updates: HashMap::new(),
        });
        Self {
            ref_count: AtomicI32::new(1),
            index_created_version_major,
            sort_map: None,
            ram_bytes_used: AtomicI64::new(0),
            inner,
        }
    }
    pub fn inc_ref(&self) {
        let rc = self.ref_count.fetch_add(1, Ordering::SeqCst) + 1;
        debug_assert!(rc > 1);
    }

    pub fn dec_ref(&self) {
        let rc = self.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
        debug_assert!(rc >= 0);
    }

    pub fn ref_count(&self) -> i32 {
        let rc = self.ref_count.load(Ordering::SeqCst);
        debug_assert!(rc >= 0);
        rc
    }
    pub(crate) fn get_del_count<D>(&self, info: &SegmentCommitInfo<D>) -> i32
    where
        D: Directory,
    {
        self.inner.lock().pending_deletes.get_del_count(info)
    }

    fn assert_no_dup_gen(
        &self,
        field_updates: &[DocValuesFieldUpdatesEnum],
        update: &DocValuesFieldUpdatesEnum,
    ) -> bool {
        let dup = field_updates
            .iter()
            .any(|old_update| old_update.del_gen() == update.del_gen());
        debug_assert!(!dup, "duplicate delGen={}", update.del_gen());
        true
    }
    /// Adds a new resolved (meaning it maps docIDs to new values) doc values packet.
    /// We buffer these in RAM and write to disk when too much RAM is used or when a merge needs to kick off, or a commit/refresh.
    pub fn add_dv_update(&self, update: DocValuesFieldUpdatesEnum) -> Result<()> {
        Ok(())
    }

    pub(crate) fn get_num_dv_updates(&self) -> i64 {
        let inner = self.inner.lock();
        inner
            .pending_dv_updates
            .values()
            .map(|v| v.len() as i64)
            .sum()
    }
}
