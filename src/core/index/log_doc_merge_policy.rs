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
use crate::core::index::log_merge_policy::{LogMergePolicy, LogMergePolicyBase, size_docs};
use crate::core::index::merge_policy::{
    DEFAULT_MAX_CFS_SEGMENT_SIZE, DEFAULT_NO_CFS_RATIO, MergeContext, MergePolicyBase,
};
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;

/// This is a [`LogMergePolicy`] that measures size of a segment as the number of documents (not
/// taking deletions into account).
pub struct LogDocMergePolicy;

impl LogDocMergePolicy {
    /// Default minimum segment size. @see setMinMergeDocs
    pub const DEFAULT_MIN_MERGE_DOCS: i32 = 1000;
}

impl LogMergePolicyBase for LogDocMergePolicy {
    fn size<D, MC>(
        &self,
        info: &SegmentCommitInfo<D>,
        merge_context: &MC,
        calibrate_size_by_deletes: bool,
    ) -> Result<i64>
    where
        D: Directory,
        MC: MergeContext<D>,
    {
        size_docs(info, merge_context, calibrate_size_by_deletes)
    }
}

impl LogMergePolicy<LogDocMergePolicy> {
    /// Sole constructor, setting all settings to their defaults.
    pub fn log_doc() -> Self {
        let base = MergePolicyBase::new(DEFAULT_NO_CFS_RATIO, DEFAULT_MAX_CFS_SEGMENT_SIZE);
        let mut mp = LogMergePolicy {
            merge_factor: Self::DEFAULT_MERGE_FACTOR,
            min_merge_size: 0,
            max_merge_size: 0,
            max_merge_size_for_forced_merge: i64::MAX,
            max_merge_docs: Self::DEFAULT_MAX_MERGE_DOCS,
            calibrate_size_by_deletes: true,
            target_search_concurrency: 1,
            base,
            sub: LogDocMergePolicy,
        };

        mp.min_merge_size = LogDocMergePolicy::DEFAULT_MIN_MERGE_DOCS as i64;

        // maxMergeSize(ForForcedMerge) are never used by LogDocMergePolicy; set
        // it to i64::MAX to disable it
        mp.max_merge_size = i64::MAX;
        mp.max_merge_size_for_forced_merge = i64::MAX;

        mp
    }

    /// Sets the minimum size for the lowest level segments. Any segments below this size are
    /// candidates for full-flush merges and merged more aggressively in order to avoid having a long
    /// tail of small segments. Large values of this parameter increase the merging cost during
    /// indexing if you flush small segments.
    pub fn set_min_merge_docs(&mut self, min_merge_docs: i32) {
        self.min_merge_size = min_merge_docs as i64;
    }

    /// Get the minimum size for a segment to remain un-merged.
    ///
    /// @see LogMergePolicy::<LogDocMergePolicy>::set_min_merge_docs *
    pub fn get_min_merge_docs(&self) -> i32 {
        self.min_merge_size as i32
    }
}
