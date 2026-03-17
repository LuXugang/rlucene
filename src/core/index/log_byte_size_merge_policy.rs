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
use crate::core::index::log_merge_policy::{LogMergePolicy, LogMergePolicyBase, size_bytes};
use crate::core::index::merge_policy::{
  DEFAULT_MAX_CFS_SEGMENT_SIZE, DEFAULT_NO_CFS_RATIO, MergeContext, MergePolicyBase,
};
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
/// This is a LogMergePolicy that measures size of a segment as the total byte size of the segment's files.
pub struct LogByteSizeMergePolicy;
impl LogByteSizeMergePolicy {
  /// Default minimum segment size. @see setMinMergeMB
  pub const DEFAULT_MIN_MERGE_MB: f64 = 1.6;

  /// Default maximum segment size. A segment of this size or larger will never be merged. @see
  /// setMaxMergeMB
  pub const DEFAULT_MAX_MERGE_MB: f64 = 2048.0;

  /// Default maximum segment size. A segment of this size or larger will never be merged during
  /// forceMerge. @see setMaxMergeMBForForceMerge
  pub const DEFAULT_MAX_MERGE_MB_FOR_FORCED_MERGE: f64 = i64::MAX as f64;
}
impl LogMergePolicyBase for LogByteSizeMergePolicy {
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
    size_bytes(info, merge_context, calibrate_size_by_deletes)
  }
}
impl LogMergePolicy<LogByteSizeMergePolicy> {
  pub fn log_bytes_size() -> Self {
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
      sub: LogByteSizeMergePolicy,
    };

    mp.min_merge_size = (LogByteSizeMergePolicy::DEFAULT_MIN_MERGE_MB * 1024.0 * 1024.0) as i64;
    mp.max_merge_size = (LogByteSizeMergePolicy::DEFAULT_MAX_MERGE_MB * 1024.0 * 1024.0) as i64;

    mp.max_merge_size_for_forced_merge =
      (LogByteSizeMergePolicy::DEFAULT_MAX_MERGE_MB_FOR_FORCED_MERGE * 1024.0 * 1024.0) as i64;

    mp
  }
  /// Determines the largest segment (measured by total byte size of the segment's files, in MB) that
  /// may be merged with other segments. Small values (e.g., less than 50 MB) are best for
  /// interactive indexing, as this limits the length of pauses while indexing to a few seconds.
  /// Larger values are best for batched indexing and speedier searches.
  ///
  /// Note that [`LogMergePolicy::set_max_merge_docs`] is also used to check whether a segment is too large for
  /// merging (it's either or).
  pub fn set_max_merge_mb(&mut self, mb: f64) {
    self.max_merge_size = (mb * 1024.0 * 1024.0) as i64;
  }

  /// Returns the largest segment (measured by total byte size of the segment's files, in MB) that
  /// may be merged with other segments.
  ///
  /// @see LogByteSizeMergePolicy::set_max_merge_mb
  pub fn get_max_merge_mb(&self) -> f64 {
    (self.max_merge_size as f64) / 1024.0 / 1024.0
  }

  /// Determines the largest segment (measured by total byte size of the segment's files, in MB) that
  /// may be merged with other segments during forceMerge. Setting it low will leave the index with
  /// more than 1 segment, even if [`IndexWriter::force_merge`] is called.
  pub fn set_max_merge_mb_for_forced_merge(&mut self, mb: f64) {
    self.max_merge_size_for_forced_merge = (mb * 1024.0 * 1024.0) as i64;
  }

  /// Returns the largest segment (measured by total byte size of the segment's files, in MB) that
  /// may be merged with other segments during forceMerge.
  ///
  /// @see LogByteSizeMergePolicy::set_max_merge_mb_for_forced_merge
  pub fn get_max_merge_mb_for_forced_merge(&self) -> f64 {
    (self.max_merge_size_for_forced_merge as f64) / 1024.0 / 1024.0
  }

  /// Sets the minimum size for the lowest level segments. Any segments below this size are
  /// candidates for full-flush merges and be merged more aggressively in order to avoid having a
  /// long tail of small segments. Large values of this parameter increase the merging cost during
  /// indexing if you flush small segments.
  pub fn set_min_merge_mb(&mut self, mb: f64) {
    self.min_merge_size = (mb * 1024.0 * 1024.0) as i64;
  }

  /// Get the minimum size for a segment to remain un-merged.
  ///
  /// @see LogByteSizeMergePolicy::set_min_merge_mb *
  pub fn get_min_merge_mb(&self) -> f64 {
    (self.min_merge_size as f64) / 1024.0 / 1024.0
  }
}
