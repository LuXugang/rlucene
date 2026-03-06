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
use crate::core::index::index_writer::Inner;
use crate::core::index::merge_policy::{
    DEFAULT_MAX_CFS_SEGMENT_SIZE, DEFAULT_NO_CFS_RATIO, MergeContext, MergePolicy, MergePolicyBase,
    MergeSpecification, MergeSpecificationNoReader, OneMerge,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::tiered_merge_policy::SegmentDocAndID;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

pub struct LogMergePolicy {
    /// How many segments to merge at a time.
    pub(crate) merge_factor: i32,
    /// Any segments whose size is smaller than this value will be candidates for full-flush merges and
    /// merged more aggressively.
    pub(crate) min_merge_size: i64,
    /// If the size of a segment exceeds this value then it will never be merged.
    pub(crate) max_merge_size: i64,
    /// Although the core MPs set it explicitly, we must default in case someone
    /// out there wrote his own LMP ...
    /// If the size of a segment exceeds this value then it will never be merged during
    /// [`IndexWriter::force_merge`].
    pub(crate) max_merge_size_for_forced_merge: i64,
    /// If a segment has more than this many documents then it will never be merged.
    pub(crate) max_merge_docs: i32,
    /// If true, we pro-rate a segment's size by the percentage of non-deleted documents.
    pub(crate) calibrate_size_by_deletes: bool,
    /// Target search concurrency. This merge policy will avoid creating segments that have more than
    /// `maxDoc / targetSearchConcurrency` documents.
    pub(crate) target_search_concurrency: i32,
    pub(crate) base: MergePolicyBase,
}

impl LogMergePolicy {
    /// Defines the allowed range of log(size) for each level. A level is computed by taking the max
    /// segment log size, minus LEVEL_LOG_SPAN, and finding all segments falling within that range.
    pub const LEVEL_LOG_SPAN: f64 = 0.75;
    /// Default merge factor, which is how many segments are merged at a time
    pub const DEFAULT_MERGE_FACTOR: i32 = 10;
    /// Default maximum segment size. A segment of this size or larger will never be merged. @see
    /// setMaxMergeDocs
    pub const DEFAULT_MAX_MERGE_DOCS: i32 = i32::MAX;
    /// Default noCFSRatio. If a merge's size is `>= 10%` of the index, then we disable compound
    /// file for it.
    ///
    /// @see MergePolicy#setNoCFSRatio
    pub const DEFAULT_NO_CFS_RATIO: f64 = 0.1;
    /// Sole constructor. (For invocation by subclass constructors, typically implicit.)
    pub fn new() -> Self {
        let base = MergePolicyBase::new(DEFAULT_NO_CFS_RATIO, DEFAULT_MAX_CFS_SEGMENT_SIZE);
        Self {
            merge_factor: Self::DEFAULT_MERGE_FACTOR,
            min_merge_size: 0,
            max_merge_size: 0,
            max_merge_size_for_forced_merge: i64::MAX,
            max_merge_docs: Self::DEFAULT_MAX_MERGE_DOCS,
            calibrate_size_by_deletes: true,
            target_search_concurrency: 1,
            base,
        }
    }
    /// Returns the number of segments that are merged at once and also controls the total number of
    /// segments allowed to accumulate in the index.
    pub fn get_merge_factor(&self) -> i32 {
        self.merge_factor
    }
    /// Determines how often segment indices are merged by addDocument(). With smaller values, less RAM
    /// is used while indexing, and searches are faster, but indexing speed is slower. With larger
    /// values, more RAM is used during indexing, and while searches is slower, indexing is faster.
    /// Thus larger values (`> 10`) are best for batch index creation, and smaller values (`< 10`)
    /// for indices that are interactively maintained.
    pub fn set_merge_factor(&mut self, merge_factor: i32) -> Result<()> {
        if merge_factor < 2 {
            return Err(LuceneError::illegal_argument(
                "mergeFactor cannot be less than 2",
            ));
        }
        self.merge_factor = merge_factor;
        Ok(())
    }
    /// Sets whether the segment size should be calibrated by the number of deletes when choosing
    /// segments for merge.
    pub fn set_calibrate_size_by_deletes(&mut self, calibrate_size_by_deletes: bool) {
        self.calibrate_size_by_deletes = calibrate_size_by_deletes;
    }
    /// Returns true if the segment size should be calibrated by the number of deletes when choosing
    /// segments for merge.
    pub fn get_calibrate_size_by_deletes(&self) -> bool {
        self.calibrate_size_by_deletes
    }
    /// Sets the target search concurrency. This prevents creating segments that are bigger than
    /// maxDoc/targetSearchConcurrency, which in turn makes the work parallelizable into
    /// targetSearchConcurrency slices of similar doc counts.
    ///
    /// <p><b>NOTE:</b> Configuring a value greater than 1 will increase the number of segments in the
    /// index linearly with the value of `targetSearchConcurrency` and also increase write
    /// amplification.
    pub fn set_target_search_concurrency(&mut self, target_search_concurrency: i32) -> Result<()> {
        if target_search_concurrency < 1 {
            return Err(LuceneError::illegal_argument(format!(
                "targetSearchConcurrency must be >= 1 (got {})",
                target_search_concurrency
            )));
        }
        self.target_search_concurrency = target_search_concurrency;
        Ok(())
    }

    /// Returns the target search concurrency.
    pub fn get_target_search_concurrency(&self) -> i32 {
        self.target_search_concurrency
    }
    /// Return the number of documents in the provided [`SegmentCommitInfo`], pro-rated by
    /// percentage of non-deleted documents if [`LogMergePolicy::set_calibrate_size_by_deletes`]
    /// is set.
    pub(crate) fn size_docs<D, MC>(
        &self,
        info: &SegmentCommitInfo<D>,
        merge_context: &MC,
    ) -> Result<i64>
    where
        D: Directory,
        MC: MergeContext<D>,
    {
        if self.calibrate_size_by_deletes {
            let del_count = merge_context.num_deletes_to_merge(info)?;
            debug_assert!(self.assert_del_count(del_count, info)?);
            Ok((info.info.max_doc()? - del_count) as i64)
        } else {
            Ok(info.info.max_doc()? as i64)
        }
    }
    /// Return the byte size of the provided [`SegmentCommitInfo`], pro-rated by percentage of
    /// non-deleted documents if [`LogMergePolicy::set_calibrate_size_by_deletes`]
    /// is set.
    pub(crate) fn size_bytes<D, MC>(
        &self,
        info: &SegmentCommitInfo<D>,
        merge_context: &MC,
    ) -> Result<i64>
    where
        D: Directory,
        MC: MergeContext<D>,
    {
        if self.calibrate_size_by_deletes {
            self.size(info, merge_context)
        } else {
            Ok(info.size_in_bytes()?)
        }
    }
    /// Returns true if the number of segments eligible for merging is less than or equal to the
    /// specified `maxNumSegments`.
    pub(crate) fn is_merged<D, MC>(
        &self,
        infos: &SegmentInfos<D>,
        max_num_segments: i32,
        segments_to_merge: &HashMap<String, Option<bool>>,
        merge_context: &MC,
    ) -> Result<bool>
    where
        D: Directory,
        MC: MergeContext<D>,
    {
        let num_segments = infos.size();
        let mut num_to_merge = 0;
        let mut merge_info = None;
        let mut segment_is_original = false;

        let mut i = 0;
        while i < num_segments && num_to_merge <= max_num_segments {
            let info = infos
                .info_idx(i)
                .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
            let seg_id = info.info.get_id_str();
            let is_original = segments_to_merge.get(&seg_id).copied();
            if let Some(Some(v)) = is_original {
                segment_is_original = v;
                num_to_merge += 1;
                merge_info = Some(seg_id);
            }
            i += 1;
        }

        Ok(num_to_merge <= max_num_segments
            && (num_to_merge != 1 || !segment_is_original || {
                let merge_info = merge_info.ok_or_else(|| LuceneError::illegal_state(""))?;
                let info = infos
                    .info(&merge_info)
                    .ok_or_else(|| LuceneError::illegal_state(""))?;
                self.has_merged(infos, info, merge_context)?
            }))
    }

    /// Returns the merges necessary to merge the index, taking the max merge size or max merge docs
    /// into consideration. This method attempts to respect the `maxNumSegments` parameter,
    /// however it might be, due to size constraints, that more than that number of segments will
    /// remain in the index. Also, this method does not guarantee that exactly `maxNumSegments`
    /// will remain, but <= that number.
    pub(crate) fn find_forced_merges_size_limit<D, MC>(
        &self,
        infos: &SegmentInfos<D>,
        mut last: i32,
        merge_context: &MC,
    ) -> Result<Option<MergeSpecificationNoReader<D>>>
    where
        D: Directory,
        MC: MergeContext<D>,
    {
        let mut spec = MergeSpecification::new();
        let segments = infos.iter();

        let mut start = last - 1;
        while start >= 0 {
            let start_idx = start as usize;
            let info = infos
                .info_idx(start_idx)
                .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
            if self.size(info, merge_context)? > self.max_merge_size_for_forced_merge
                || self.size_docs(info, merge_context)? > self.max_merge_docs as i64
            {
                // need to skip that segment + add a merge for the 'right' segments,
                // unless there is only 1 which is merged.
                if last - start - 1 > 1
                    || (start != last - 1 && {
                        let info = infos
                            .info_idx(start_idx + 1)
                            .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
                        !self.has_merged(infos, info, merge_context)?
                    })
                {
                    // there is more than 1 segment to the right of
                    // this one, or a mergeable single segment.
                    let mut meta = Vec::new();
                    for seg in segments.iter().take(last as usize).skip(start_idx + 1) {
                        meta.push(SegmentDocAndID::new(
                            seg.info.get_id_str(),
                            seg.info.max_doc()?,
                        ));
                    }
                    spec.add(OneMerge::new(meta)?);
                }
                last = start;
            } else if last - start == self.merge_factor {
                // mergeFactor eligible segments were found, add them as a merge.
                let mut meta = Vec::new();
                for seg in segments.iter().take(last as usize).skip(start_idx) {
                    meta.push(SegmentDocAndID::new(
                        seg.info.get_id_str(),
                        seg.info.max_doc()?,
                    ));
                }
                spec.add(OneMerge::new(meta)?);
                last = start;
            }
            start -= 1;
        }

        // Add any left-over segments, unless there is just 1
        // already fully merged
        start += 1;
        if last > 0
            && (start + 1 < last || {
                let info = infos
                    .info_idx(start as usize)
                    .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
                !self.has_merged(infos, info, merge_context)?
            })
        {
            let mut meta = Vec::new();
            for seg in segments.iter().take(last as usize).skip(start as usize) {
                meta.push(SegmentDocAndID::new(
                    seg.info.get_id_str(),
                    seg.info.max_doc()?,
                ));
            }
            spec.add(OneMerge::new(meta)?);
        }

        if spec.merges.is_empty() {
            Ok(None)
        } else {
            Ok(Some(spec))
        }
    }
    /// Returns the merges necessary to forceMerge the index. This method constraints the returned
    /// merges only by the `maxNumSegments` parameter, and guaranteed that exactly that number of
    /// segments will remain in the index.
    pub(crate) fn find_forced_merges_max_num_segments<D, MC>(
        &self,
        infos: &SegmentInfos<D>,
        max_num_segments: i32,
        mut last: i32,
        merge_context: &MC,
    ) -> Result<Option<MergeSpecificationNoReader<D>>>
    where
        D: Directory,
        MC: MergeContext<D>,
    {
        let mut spec = MergeSpecification::new();
        let segments = infos.iter();

        // First, enroll all "full" merges (size
        // mergeFactor) to potentially be run concurrently:
        while last - max_num_segments + 1 >= self.merge_factor {
            let start = (last - self.merge_factor) as usize;
            let end = last as usize;
            let mut meta = Vec::new();
            for seg in segments.iter().take(end).skip(start) {
                meta.push(SegmentDocAndID::new(
                    seg.info.get_id_str(),
                    seg.info.max_doc()?,
                ));
            }
            spec.add(OneMerge::new(meta)?);
            last -= self.merge_factor;
        }

        // Only if there are no full merges pending do we
        // add a final partial (< mergeFactor segments) merge:
        if spec.merges.is_empty() {
            if max_num_segments == 1 {
                // Since we must merge down to 1 segment, the
                // choice is simple:
                if last > 1 || {
                    let info = infos
                        .info_idx(0)
                        .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
                    !self.has_merged(infos, info, merge_context)?
                } {
                    let mut meta = Vec::new();
                    for seg in segments.iter().take(last as usize) {
                        meta.push(SegmentDocAndID::new(
                            seg.info.get_id_str(),
                            seg.info.max_doc()?,
                        ));
                    }
                    spec.add(OneMerge::new(meta)?);
                }
            } else if last > max_num_segments {
                // Take care to pick a partial merge that is
                // least cost, but does not make the index too
                // lopsided.  If we always just picked the
                // partial tail then we could produce a highly
                // lopsided index over time:

                // We must merge this many segments to leave
                // maxNumSegments in the index (from when
                // forceMerge was first kicked off):
                let final_merge_size = last - max_num_segments + 1;

                // Consider all possible starting points:
                let mut best_size = 0;
                let mut best_start = 0;

                let limit = last - final_merge_size + 1;
                let mut i = 0;
                while i < limit {
                    let mut sum_size = 0;
                    let mut j = 0;
                    while j < final_merge_size {
                        let idx = (j + i) as usize;
                        let info = infos
                            .info_idx(idx)
                            .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
                        sum_size += self.size(info, merge_context)?;
                        j += 1;
                    }

                    if i == 0
                        || (sum_size
                            < 2 * {
                                let prev = infos.info_idx((i - 1) as usize).ok_or_else(|| {
                                    LuceneError::illegal_state("segment missing?")
                                })?;
                                self.size(prev, merge_context)?
                            }
                            && sum_size < best_size)
                    {
                        best_start = i;
                        best_size = sum_size;
                    }

                    i += 1;
                }

                let start = best_start as usize;
                let end = (best_start + final_merge_size) as usize;
                let mut meta = Vec::new();
                for seg in segments.iter().take(end).skip(start) {
                    meta.push(SegmentDocAndID::new(
                        seg.info.get_id_str(),
                        seg.info.max_doc()?,
                    ));
                }
                spec.add(OneMerge::new(meta)?);
            }
        }

        if spec.merges.is_empty() {
            Ok(None)
        } else {
            Ok(Some(spec))
        }
    }
    /// Determines the largest segment (measured by document count) that may be merged with other
    /// segments. Small values (e.g., less than 10,000) are best for interactive indexing, as this
    /// limits the length of pauses while indexing to a few seconds. Larger values are best for batched
    /// indexing and speedier searches.
    ///
    /// The default value is [`i32::MAX`].
    ///
    /// The default merge policy ([`LogByteSizeMergePolicy`]) also allows you to set this limit
    /// by net size (in MB) of the segment, using [`LogByteSizeMergePolicy::set_max_merge_mb`].
    pub fn set_max_merge_docs(&mut self, max_merge_docs: i32) {
        self.max_merge_docs = max_merge_docs;
    }
    /// Returns the largest segment (measured by document count) that may be merged with other
    /// segments.
    pub fn get_max_merge_docs(&self) -> i32 {
        self.max_merge_docs
    }
}
impl Default for LogMergePolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for LogMergePolicy {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl MergePolicy for LogMergePolicy {
    fn get_base(&self) -> &MergePolicyBase {
        &self.base
    }

    fn get_base_mut(&mut self) -> &mut MergePolicyBase {
        &mut self.base
    }

    fn find_merges<D, MC>(
        &self,
        _merge_trigger: MergeTrigger,
        _segment_infos: &SegmentInfos<D>,
        _inner: Option<&Inner<D>>,
        _merge_context: &MC,
    ) -> Result<Option<MergeSpecificationNoReader<D>>>
    where
        D: Directory,
        MC: MergeContext<D>,
    {
        todo!()
    }

    fn find_forced_merges<D, MC>(
        &self,
        _segment_infos: &SegmentInfos<D>,
        _max_segment_count: i32,
        _segments_to_merge: &HashMap<String, Option<bool>>,
        _inner: Option<&Inner<D>>,
        _merge_context: &MC,
    ) -> Result<Option<MergeSpecificationNoReader<D>>>
    where
        D: Directory,
        MC: MergeContext<D>,
    {
        todo!()
    }

    fn find_forced_deletes_merges<D, MC>(
        &self,
        _segment_infos: &SegmentInfos<D>,
        _inner: Option<&Inner<D>>,
        _merge_context: &MC,
    ) -> Result<Option<MergeSpecificationNoReader<D>>>
    where
        MC: MergeContext<D>,
        D: Directory,
    {
        todo!()
    }

    fn max_full_flush_merge_size(&self) -> i64 {
        self.max_merge_size
    }
}
