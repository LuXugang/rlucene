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
  MergeContext, MergePolicy, MergePolicyBase, MergeSpecification, MergeSpecificationNoReader,
  OneMerge, assert_del_count, size,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::tiered_merge_policy::SegmentDocAndID;
use crate::core::store::directory::Directory;
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

/// This struct implements a [`MergePolicy`] that tries to merge segments into levels of
/// exponentially increasing size, where each level has fewer segments than the value of the merge
/// factor. Whenever extra segments (beyond the merge factor upper bound) are encountered, all
/// segments within the level are merged. You can get or set the merge factor using
/// [`LogMergePolicy::get_merge_factor()`] and [`LogMergePolicy::set_merge_factor()`] respectively.
///
/// A subclass to define the [`LogMergePolicy::size`] method
/// which specifies how a segment's size is determined. [`LogDocMergePolicy`] is one subclass that
/// measures size by document count in the segment. [`LogByteSizeMergePolicy`] is another
/// subclass that measures size as the total byte size of the file(s) for the segment.
///
/// **NOTE**: This policy returns natural merges whose size is below the [`LogMergePolicy::min_merge_size`]
/// minimum merge size for [`LogMergePolicy::find_full_flush_merges`] full-flush merges.
#[derive(Clone)]
pub struct LogMergePolicy<T>
where
  T: LogMergePolicyBase,
{
  /// How many segments to merge at a time.
  pub(crate) merge_factor: usize,
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
  pub(crate) sub: T,
}

impl<T> LogMergePolicy<T>
where
  T: LogMergePolicyBase,
{
  /// Defines the allowed range of log(size) for each level. A level is computed by taking the max
  /// segment log size, minus LEVEL_LOG_SPAN, and finding all segments falling within that range.
  pub const LEVEL_LOG_SPAN: f64 = 0.75;
  /// Default merge factor, which is how many segments are merged at a time
  pub const DEFAULT_MERGE_FACTOR: usize = 10;
  /// Default maximum segment size. A segment of this size or larger will never be merged. @see
  /// setMaxMergeDocs
  pub const DEFAULT_MAX_MERGE_DOCS: i32 = i32::MAX;
  /// Default noCFSRatio. If a merge's size is `>= 10%` of the index, then we disable compound
  /// file for it.
  ///
  /// @see MergePolicy#setNoCFSRatio
  pub const DEFAULT_NO_CFS_RATIO: f64 = 0.1;
  /// Returns the number of segments that are merged at once and also controls the total number of
  /// segments allowed to accumulate in the index.
  pub fn get_merge_factor(&self) -> usize {
    self.merge_factor
  }
  /// Determines how often segment indices are merged by addDocument(). With smaller values, less RAM
  /// is used while indexing, and searches are faster, but indexing speed is slower. With larger
  /// values, more RAM is used during indexing, and while searches is slower, indexing is faster.
  /// Thus larger values (`> 10`) are best for batch index creation, and smaller values (`< 10`)
  /// for indices that are interactively maintained.
  pub fn set_merge_factor(&mut self, merge_factor: usize) -> Result<()> {
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

  /// Returns true if the number of segments eligible for merging is less than or equal to the
  /// specified `maxNumSegments`.
  pub(crate) fn is_merged<D, MC>(
    &self,
    infos: &SegmentInfos<D>,
    max_num_segments: usize,
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
        .info(i)
        .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
      let seg_id = info.info.get_id_key();
      let is_original = segments_to_merge.get(seg_id).copied();
      if let Some(Some(v)) = is_original {
        segment_is_original = v;
        num_to_merge += 1;
        merge_info = Some(seg_id.to_string());
      }
      i += 1;
    }

    Ok(
      num_to_merge <= max_num_segments
        && (num_to_merge != 1 || !segment_is_original || {
          let merge_info = merge_info.ok_or_else(|| LuceneError::illegal_state(""))?;
          let info = infos
            .index_of(&merge_info)
            .ok_or_else(|| LuceneError::illegal_state(""))?;
          self.has_merged(infos, info, merge_context)?
        }),
    )
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
    debug_assert!(last > 0);
    let mut spec = MergeSpecification::new();
    let segments = infos.iter();

    let mut start = last - 1;
    while start >= 0 {
      let start_idx = start as usize;
      let info = infos
        .info(start_idx)
        .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
      if self.size(info, merge_context)? > self.max_merge_size_for_forced_merge
        || size_docs(info, merge_context, self.calibrate_size_by_deletes)?
          > self.max_merge_docs as i64
      {
        // need to skip that segment + add a merge for the 'right' segments,
        // unless there is only 1 which is merged.
        if last - start - 1 > 1
          || (start != last - 1 && {
            let info = infos
              .info(start_idx + 1)
              .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
            !self.has_merged(infos, info, merge_context)?
          })
        {
          // there is more than 1 segment to the right of
          // this one, or a mergeable single segment.
          let meta = Self::get_meta(start_idx + 1, last as usize, segments)?;
          spec.add(OneMerge::new(meta)?);
        }
        last = start;
      } else if last - start == self.merge_factor as i32 {
        // mergeFactor eligible segments were found, add them as a merge.
        let meta = Self::get_meta(start_idx, last as usize, segments)?;
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
          .info(start as usize)
          .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
        !self.has_merged(infos, info, merge_context)?
      })
    {
      let meta = Self::get_meta(start as usize, last as usize, segments)?;
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
    max_num_segments: usize,
    mut last: usize,
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
    while last + 1 >= self.merge_factor + max_num_segments {
      let start = last - self.merge_factor;
      let meta = Self::get_meta(start, last, segments)?;
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
            .info(0)
            .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
          !self.has_merged(infos, info, merge_context)?
        } {
          let meta = Self::get_meta(0, last, segments)?;
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
            let idx = j + i;
            let info = infos
              .info(idx)
              .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
            sum_size += self.size(info, merge_context)?;
            j += 1;
          }

          if i == 0
            || (sum_size
              < 2 * {
                let prev = infos
                  .info(i - 1)
                  .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
                self.size(prev, merge_context)?
              }
              && sum_size < best_size)
          {
            best_start = i;
            best_size = sum_size;
          }

          i += 1;
        }

        let start = best_start;
        let end = best_start + final_merge_size;
        let meta = Self::get_meta(start, end, segments)?;
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
  pub fn get_meta<D>(
    start: usize,
    end: usize,
    sci: &[SegmentCommitInfo<D>],
  ) -> Result<Vec<SegmentDocAndID>>
  where
    D: Directory,
  {
    let mut meta = Vec::new();
    for seg in sci.iter().take(end).skip(start) {
      meta.push(SegmentDocAndID::new(
        seg.info.get_id_key().to_string(),
        seg.info.max_doc()?,
      ));
    }
    Ok(meta)
  }
}
/// Return the number of documents in the provided [`SegmentCommitInfo`], pro-rated by
/// percentage of non-deleted documents if [`LogMergePolicy::set_calibrate_size_by_deletes`]
/// is set.
pub(crate) fn size_docs<D, MC>(
  info: &SegmentCommitInfo<D>,
  merge_context: &MC,
  calibrate_size_by_deletes: bool,
) -> Result<i64>
where
  D: Directory,
  MC: MergeContext<D>,
{
  if calibrate_size_by_deletes {
    let del_count = merge_context.num_deletes_to_merge(info)?;
    debug_assert!(assert_del_count(del_count, info)?);
    Ok((info.info.max_doc()? - del_count) as i64)
  } else {
    Ok(info.info.max_doc()? as i64)
  }
}
/// Return the byte size of the provided [`SegmentCommitInfo`], pro-rated by percentage of
/// non-deleted documents if [`LogMergePolicy::set_calibrate_size_by_deletes`]
/// is set.
pub(crate) fn size_bytes<D, MC>(
  info: &SegmentCommitInfo<D>,
  merge_context: &MC,
  calibrate_size_by_deletes: bool,
) -> Result<i64>
where
  D: Directory,
  MC: MergeContext<D>,
{
  if calibrate_size_by_deletes {
    size(info, merge_context)
  } else {
    Ok(info.size_in_bytes()?)
  }
}

impl<T> Display for LogMergePolicy<T>
where
  T: LogMergePolicyBase,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "[{}: minMergeSize={}, mergeFactor={}, maxMergeSize={}, maxMergeSizeForForcedMerge={}, calibrateSizeByDeletes={}, maxMergeDocs={}, maxCFSSegmentSizeMB={}, noCFSRatio={}]",
      std::any::type_name::<Self>()
        .rsplit("::")
        .next()
        .unwrap_or("LogMergePolicy"),
      self.min_merge_size,
      self.merge_factor,
      self.max_merge_size,
      self.max_merge_size_for_forced_merge,
      self.calibrate_size_by_deletes,
      self.max_merge_docs,
      self.base.get_max_cfs_segment_size_mb(),
      self.base.no_cfs_ratio,
    )
  }
}

pub trait LogMergePolicyBase {
  fn size<D, MC>(
    &self,
    info: &SegmentCommitInfo<D>,
    merge_context: &MC,
    calibrate_size_by_deletes: bool,
  ) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>;
}

impl<T> MergePolicy for LogMergePolicy<T>
where
  T: LogMergePolicyBase,
{
  fn get_base(&self) -> &MergePolicyBase {
    &self.base
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    &mut self.base
  }
  /// Checks if any merges are now necessary and returns a [`MergeSpecification`] if
  /// so. A merge is necessary when there are more than [`LogMergePolicy::set_merge_factor`] segments
  /// at a given level. When multiple levels have too many segments, this method will return multiple
  /// merges, allowing the [`MergeScheduler`] to use concurrency.
  fn find_merges<D, MC>(
    &self,
    _merge_trigger: MergeTrigger,
    infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    let num_segments = infos.size();

    // Compute levels, which is just log (base mergeFactor)
    // of the size of each segment

    let norm: f32 = (self.merge_factor as f64).ln() as f32;

    let merging_segments = merge_context.get_merging_segments(inner);

    let mut total_doc_count: i64 = 0;
    let mut levels = Vec::with_capacity(num_segments);

    for i in 0..num_segments {
      let info = infos
        .info(i)
        .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;

      total_doc_count += size_docs(info, merge_context, self.calibrate_size_by_deletes)?;
      let mut size = self.size(info, merge_context)?;

      // Floor tiny segments
      if size < 1 {
        size = 1;
      }

      let level = ((size as f64).ln() as f32) / norm;
      levels.push(SegmentInfoAndLevel::new(
        info.info.get_id_key().to_string(),
        level,
      ));
    }

    let level_floor: f32 = if self.min_merge_size <= 0 {
      0.0
    } else {
      ((self.min_merge_size as f64).ln() as f32) / norm
    };

    // Now, we quantize the log values into levels.  The
    // first level is any segment whose log size is within
    // LEVEL_LOG_SPAN of the max size, or, who has such as
    // segment "to the right".  Then, we find the max of all
    // other segments and use that to define the next level
    // segment, etc.

    let mut spec = None;

    let num_mergeable_segments = levels.len();

    // precompute the max level on the right side.
    // arr size is numMergeableSegments + 1 to handle the case
    // when numMergeableSegments is 0.
    let mut max_levels = vec![0.0; num_mergeable_segments + 1];
    // -1 is definitely the minimum value, because ln(1) is 0.
    max_levels[num_mergeable_segments] = -1.0;
    for i in (0..num_mergeable_segments).rev() {
      max_levels[i] = levels[i].level.max(max_levels[i + 1]);
    }

    let mut start = 0;
    while start < num_mergeable_segments {
      // Find max level of all segments not already
      // quantized.
      let max_level = max_levels[start];

      // Now search backwards for the rightmost segment that
      // falls into this level:
      let level_bottom: f32 = if max_level > level_floor {
        // With a merge factor of 10, this means that the biggest segment and the smallest segment
        // that take part of a merge have a size difference of at most 5.6x.
        (max_level as f64 - Self::LEVEL_LOG_SPAN) as f32
      } else {
        // For segments below the floor size, we allow more unbalanced merges, but still somewhat
        // balanced to avoid running into O(n^2) merging.
        // With a merge factor of 10, this means that the biggest segment and the smallest segment
        // that take part of a merge have a size difference of at most 31.6x.
        (max_level as f64 - 2.0 * Self::LEVEL_LOG_SPAN) as f32
      };

      let mut upto = num_mergeable_segments - 1;
      while upto >= start {
        if levels[upto].level >= level_bottom {
          break;
        }
        upto -= 1;
      }

      let max_merge_docs: i64 = {
        let tsc = self.target_search_concurrency as i64;
        let ceil_div = (total_doc_count + tsc - 1) / tsc;
        (self.max_merge_docs as i64).min(ceil_div)
      };

      // Finally, record all merges that are viable at this level:
      let mut end = start + self.merge_factor;
      while end <= 1 + upto {
        let mut any_merging = false;
        let mut merge_size = 0;
        let mut merge_docs = 0;

        let mut i = start;
        while i < end {
          let seg_level = &levels[i];
          let info = infos
            .index_of(&seg_level.info_id)
            .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;

          if merging_segments.contains(info.info.get_id_key()) {
            any_merging = true;
            break;
          }

          let segment_size = self.size(info, merge_context)?;
          let segment_docs = size_docs(info, merge_context, self.calibrate_size_by_deletes)?;

          if merge_size + segment_size > self.max_merge_size
            || merge_docs + segment_docs > max_merge_docs
          {
            // This merge is full, stop adding more segments to it
            if i == start {
              // This segment alone is too large, return a singleton merge
              end = i + 1;
            } else {
              // Previous segments are under the max merge size, return them
              end = i;
            }
            break;
          }

          merge_size += segment_size;
          merge_docs += segment_docs;

          i += 1;
        }

        if any_merging || end - start <= 1 {
          // skip: there is an ongoing merge at the current level or the computed merge has a single
          // segment and this merge policy doesn't do singleton merges
        } else {
          let v = spec.get_or_insert_with(MergeSpecification::new);

          let mut meta = Vec::new();
          for level in levels.iter().take(end).skip(start) {
            let idx = &level.info_id;
            let info = infos
              .index_of(idx)
              .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
            debug_assert!(infos.contains(idx));
            meta.push(SegmentDocAndID::new(idx.clone(), info.info.max_doc()?));
          }

          v.add(OneMerge::new(meta)?);
        }

        start = end;
        end = start + self.merge_factor;
      }

      start = 1 + upto;
    }

    Ok(spec)
  }
  /// Returns the merges necessary to merge the index down to a specified number of segments. This
  /// respects the [`LogMergePolicy::max_merge_size_for_forced_merge`] setting. By default, and assuming
  /// `maxNumSegments=1`, only one segment will be left in the index, where that segment has no
  /// deletions pending nor separate norms, and it is in compound file format if the current
  /// useCompoundFile setting is true. This method returns multiple merges (mergeFactor at a time) so
  /// the [`MergeScheduler`] in use may make use of concurrency.
  fn find_forced_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    max_segment_count: usize,
    segments_to_merge: &HashMap<String, Option<bool>>,
    _inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    debug_assert!(max_segment_count > 0);

    // If the segments are already merged (e.g. there's only 1 segment), or
    // there are <maxNumSegments:.
    if self.is_merged(
      segment_infos,
      max_segment_count,
      segments_to_merge,
      merge_context,
    )? {
      return Ok(None);
    }

    // Find the newest (rightmost) segment that needs to
    // be merged (other segments may have been flushed
    // since merging started):
    let mut last = segment_infos.size();
    while last > 0 {
      last -= 1;
      let info = segment_infos
        .info(last)
        .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
      if segments_to_merge.get(info.info.get_id_key()).is_some() {
        last += 1;
        break;
      }
    }

    if last == 0 {
      return Ok(None);
    }

    // There is only one segment already, and it is merged
    if max_segment_count == 1 && last == 1 && {
      let info0 = segment_infos
        .info(0)
        .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
      self.has_merged(segment_infos, info0, merge_context)?
    } {
      return Ok(None);
    }

    // Check if there are any segments above the threshold
    let mut any_too_large = false;
    let mut i = 0;
    while i < last {
      let info = segment_infos
        .info(i)
        .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
      if self.size(info, merge_context)? > self.max_merge_size_for_forced_merge
        || size_docs(info, merge_context, self.calibrate_size_by_deletes)?
          > self.max_merge_docs as i64
      {
        any_too_large = true;
        break;
      }
      i += 1;
    }

    if any_too_large {
      self.find_forced_merges_size_limit(segment_infos, last.try_convert()?, merge_context)
    } else {
      self.find_forced_merges_max_num_segments(
        segment_infos,
        max_segment_count,
        last,
        merge_context,
      )
    }
  }
  /// Finds merges necessary to force-merge all deletes from the index.
  /// We simply merge adjacent segments that have deletes, up to mergeFactor at a time.
  fn find_forced_deletes_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    _inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    let segments = segment_infos.iter();
    let num_segments = segments.len();

    let mut spec = MergeSpecification::new();
    let mut first_segment_with_deletions: Option<usize> = None;

    let merge_factor = self.merge_factor;

    let mut i: usize = 0;
    while i < num_segments {
      let info = segment_infos
        .info(i)
        .ok_or_else(|| LuceneError::illegal_state("segment missing?"))?;
      let del_count = merge_context.num_deletes_to_merge(info)?;
      debug_assert!(assert_del_count(del_count, info)?);

      if del_count > 0 {
        match first_segment_with_deletions {
          None => {
            first_segment_with_deletions = Some(i);
          },
          Some(first) => {
            if i == first + merge_factor {
              // We've seen mergeFactor segments in a row with deletions, so force a merge now:
              let meta = Self::get_meta(first, i, segments)?;
              spec.add(OneMerge::new(meta)?);
              first_segment_with_deletions = Some(i);
            }
          },
        }
      } else if let Some(first) = first_segment_with_deletions {
        // End of a sequence of segments with deletions, so,
        // merge those past segments even if it's fewer than
        // mergeFactor segments
        let meta = Self::get_meta(first, i, segments)?;
        spec.add(OneMerge::new(meta)?);
        first_segment_with_deletions = None;
      }

      i += 1;
    }

    if let Some(first) = first_segment_with_deletions {
      let meta = Self::get_meta(first, num_segments, segments)?;
      spec.add(OneMerge::new(meta)?);
    }

    Ok(Some(spec))
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self
      .sub
      .size(info, merge_context, self.calibrate_size_by_deletes)
  }

  fn max_full_flush_merge_size(&self) -> i64 {
    self.max_merge_size
  }
}
#[derive(Clone)]
pub(crate) struct SegmentInfoAndLevel {
  pub(crate) info_id: String,
  pub(crate) level: f32,
}
impl SegmentInfoAndLevel {
  fn new(info_id: String, level: f32) -> Self {
    Self { info_id, level }
  }
}

impl Ord for SegmentInfoAndLevel {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    other
      .level
      .partial_cmp(&self.level)
      .unwrap_or(std::cmp::Ordering::Equal)
  }
}

impl PartialOrd for SegmentInfoAndLevel {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl PartialEq for SegmentInfoAndLevel {
  fn eq(&self, other: &Self) -> bool {
    self.level.to_bits() == other.level.to_bits()
  }
}

impl Eq for SegmentInfoAndLevel {}

#[cfg(test)]
mod tests {
  use crate::core::index::codec_reader::CodecReader;
  use crate::core::index::index_writer::{SOURCE_FLUSH, SOURCE_MERGE};
  use crate::core::index::log_byte_size_merge_policy::LogByteSizeMergePolicy;
  use crate::core::index::log_doc_merge_policy::LogDocMergePolicy;
  use crate::core::index::log_merge_policy::LogMergePolicy;
  use crate::core::index::merge_policy::{
    MergePolicy, MergePolicyEnum, MergeSpecification, MergeSpecificationNoReader,
  };
  use crate::core::index::merge_trigger::MergeTrigger;
  use crate::core::index::segment_commit_info::SegmentCommitInfo;
  use crate::core::index::segment_infos::SegmentInfos;
  use crate::core::store::directory::Directory;
  use crate::core::util::LATEST;
  use crate::core::util::error::lucene_error::{LuceneError, Result};
  use crate::test::core::index::base_merge_policy_test_case::{
    BaseMergePolicyTestCase, FakeDirectory, IOStats, MockMergeContext, apply_merge,
    make_segment_commit_info,
  };
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_log_merge_policy, random,
  };
  use rand::Rng;
  use std::sync::Arc;
  use std::sync::atomic::{AtomicU64, Ordering};

  struct TestLogMergePolicy;

  impl BaseMergePolicyTestCase for TestLogMergePolicy {
    type MergePolicy = MergePolicyEnum;

    fn merge_policy<R>(&self, random: &mut R) -> Self::MergePolicy
    where
      R: Rng + ?Sized,
    {
      new_log_merge_policy(random).expect("")
    }

    fn assert_segment_infos<D>(policy: &Self::MergePolicy, infos: &SegmentInfos<D>) -> Result<()>
    where
      D: Directory,
    {
      let merge_context = MockMergeContext::new(|s: &SegmentCommitInfo<D>| Ok(s.get_del_count()));
      match policy {
        MergePolicyEnum::LogDoc(mp) => {
          for info in infos.iter() {
            assert!(
              mp.size(info, &merge_context)? / (mp.get_merge_factor() as i64) < mp.max_merge_size
            );
          }
          Ok(())
        },
        MergePolicyEnum::LogBytesSize(mp) => {
          for info in infos.iter() {
            assert!(
              mp.size(info, &merge_context)? / (mp.get_merge_factor() as i64) < mp.max_merge_size
            );
          }
          Ok(())
        },
        _ => Err(LuceneError::illegal_state(
          "expected LogMergePolicy variant",
        )),
      }
    }

    fn assert_merge<D, CR>(
      policy: &Self::MergePolicy,
      merge: &MergeSpecification<D, CR>,
    ) -> Result<()>
    where
      D: Directory,
      CR: CodecReader,
    {
      match policy {
        MergePolicyEnum::LogDoc(mp) => {
          for one_merge in &merge.merges {
            assert!(one_merge.stat.segments.len() <= mp.get_merge_factor());
          }
          Ok(())
        },
        MergePolicyEnum::LogBytesSize(mp) => {
          for one_merge in &merge.merges {
            assert!(one_merge.stat.segments.len() <= mp.get_merge_factor());
          }
          Ok(())
        },
        _ => Err(LuceneError::illegal_state(
          "expected LogMergePolicy variant",
        )),
      }
    }
  }

  #[test]
  fn test_default_forced_merge_mb() {
    let mp = LogMergePolicy::<LogByteSizeMergePolicy>::log_bytes_size();
    assert!(mp.get_max_merge_mb_for_forced_merge() > 0.0);
  }

  #[test]
  fn test_increasing_segment_sizes() -> Result<()> {
    let mut r = random();
    let merge_policy = LogMergePolicy::<LogDocMergePolicy>::log_doc();
    let mut stats = IOStats::default();
    let seg_name_generator = AtomicU64::new(0);
    let merge_context = MockMergeContext::new(|s| Ok(s.get_del_count()));
    let fake_directory = Arc::new(FakeDirectory::new());
    let mut segment_infos = SegmentInfos::new(LATEST.major)?;

    for i in 0..11 {
      segment_infos.add(make_segment_commit_info(
        &mut r,
        fake_directory.clone(),
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        (i + 1) * 1000,
        0,
        0.0,
        SOURCE_MERGE,
      )?)?;
    }

    let spec_opt: Option<MergeSpecificationNoReader<FakeDirectory>> =
      merge_policy.find_merges(MergeTrigger::Explicit, &segment_infos, None, &merge_context)?;
    assert!(spec_opt.is_some());
    let spec = spec_opt.unwrap();

    for one_merge in &spec.merges {
      segment_infos = apply_merge(
        &mut r,
        &segment_infos,
        one_merge,
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        &mut stats,
        fake_directory.clone(),
      )?;
    }

    assert_eq!(2, segment_infos.size());
    assert_eq!(55_000, segment_infos.info(0).unwrap().info.max_doc()?);
    assert_eq!(11_000, segment_infos.info(1).unwrap().info.max_doc()?);
    Ok(())
  }

  #[test]
  fn test_one_small_middle_segment() -> Result<()> {
    let mut r = random();
    let merge_policy = LogMergePolicy::<LogDocMergePolicy>::log_doc();
    let mut stats = IOStats::default();
    let seg_name_generator = AtomicU64::new(0);
    let merge_context = MockMergeContext::new(|s| Ok(s.get_del_count()));
    let fake_directory = Arc::new(FakeDirectory::new());
    let mut segment_infos = SegmentInfos::new(LATEST.major)?;

    for _ in 0..5 {
      segment_infos.add(make_segment_commit_info(
        &mut r,
        fake_directory.clone(),
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        10_000,
        0,
        0.0,
        SOURCE_MERGE,
      )?)?;
    }

    segment_infos.add(make_segment_commit_info(
      &mut r,
      fake_directory.clone(),
      &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
      100,
      0,
      0.0,
      SOURCE_MERGE,
    )?)?;

    for _ in 0..5 {
      segment_infos.add(make_segment_commit_info(
        &mut r,
        fake_directory.clone(),
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        10_000,
        0,
        0.0,
        SOURCE_MERGE,
      )?)?;
    }

    let spec_opt: Option<MergeSpecificationNoReader<FakeDirectory>> =
      merge_policy.find_merges(MergeTrigger::Explicit, &segment_infos, None, &merge_context)?;
    assert!(spec_opt.is_some());
    let spec = spec_opt.unwrap();

    for one_merge in &spec.merges {
      segment_infos = apply_merge(
        &mut r,
        &segment_infos,
        one_merge,
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        &mut stats,
        fake_directory.clone(),
      )?;
    }

    assert_eq!(2, segment_infos.size());
    assert_eq!(90_100, segment_infos.info(0).unwrap().info.max_doc()?);
    assert_eq!(10_000, segment_infos.info(1).unwrap().info.max_doc()?);
    Ok(())
  }

  #[test]
  fn test_many_small_middle_segment() -> Result<()> {
    let mut r = random();
    let merge_policy = LogMergePolicy::<LogDocMergePolicy>::log_doc();
    let mut stats = IOStats::default();
    let seg_name_generator = AtomicU64::new(0);
    let merge_context = MockMergeContext::new(|s| Ok(s.get_del_count()));
    let fake_directory = Arc::new(FakeDirectory::new());
    let mut segment_infos = SegmentInfos::new(LATEST.major)?;

    segment_infos.add(make_segment_commit_info(
      &mut r,
      fake_directory.clone(),
      &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
      10_000,
      0,
      0.0,
      SOURCE_MERGE,
    )?)?;

    for _ in 0..9 {
      segment_infos.add(make_segment_commit_info(
        &mut r,
        fake_directory.clone(),
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        100,
        0,
        0.0,
        SOURCE_MERGE,
      )?)?;
    }

    segment_infos.add(make_segment_commit_info(
      &mut r,
      fake_directory.clone(),
      &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
      10_000,
      0,
      0.0,
      SOURCE_MERGE,
    )?)?;

    let spec_opt: Option<MergeSpecificationNoReader<FakeDirectory>> =
      merge_policy.find_merges(MergeTrigger::Explicit, &segment_infos, None, &merge_context)?;
    assert!(spec_opt.is_some());
    let spec = spec_opt.unwrap();

    for one_merge in &spec.merges {
      segment_infos = apply_merge(
        &mut r,
        &segment_infos,
        one_merge,
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        &mut stats,
        fake_directory.clone(),
      )?;
    }

    assert_eq!(2, segment_infos.size());
    assert_eq!(10_900, segment_infos.info(0).unwrap().info.max_doc()?);
    assert_eq!(10_000, segment_infos.info(1).unwrap().info.max_doc()?);
    Ok(())
  }

  #[test]
  fn test_reject_unbalanced_merges() -> Result<()> {
    let mut r = random();
    let mut merge_policy = LogMergePolicy::<LogDocMergePolicy>::log_doc();
    merge_policy.set_min_merge_docs(10_000);
    let mut stats = IOStats::default();
    let seg_name_generator = AtomicU64::new(0);
    let merge_context = MockMergeContext::new(|s| Ok(s.get_del_count()));
    let fake_directory = Arc::new(FakeDirectory::new());
    let mut segment_infos = SegmentInfos::new(LATEST.major)?;

    segment_infos.add(make_segment_commit_info(
      &mut r,
      fake_directory.clone(),
      &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
      100,
      0,
      0.0,
      SOURCE_MERGE,
    )?)?;

    for _ in 0..9 {
      segment_infos.add(make_segment_commit_info(
        &mut r,
        fake_directory.clone(),
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        1,
        0,
        0.0,
        SOURCE_FLUSH,
      )?)?;
    }

    let spec_opt: Option<MergeSpecificationNoReader<FakeDirectory>> =
      merge_policy.find_merges(MergeTrigger::Explicit, &segment_infos, None, &merge_context)?;
    assert!(spec_opt.is_none());

    segment_infos.add(make_segment_commit_info(
      &mut r,
      fake_directory.clone(),
      &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
      1,
      0,
      0.0,
      SOURCE_FLUSH,
    )?)?;

    let spec_opt: Option<MergeSpecificationNoReader<FakeDirectory>> =
      merge_policy.find_merges(MergeTrigger::Explicit, &segment_infos, None, &merge_context)?;
    assert!(spec_opt.is_some());
    let spec = spec_opt.unwrap();

    for one_merge in &spec.merges {
      segment_infos = apply_merge(
        &mut r,
        &segment_infos,
        one_merge,
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        &mut stats,
        fake_directory.clone(),
      )?;
    }

    assert_eq!(2, segment_infos.size());
    assert_eq!(100, segment_infos.info(0).unwrap().info.max_doc()?);
    assert_eq!(10, segment_infos.info(1).unwrap().info.max_doc()?);
    Ok(())
  }

  #[test]
  fn test_pack_large_segments() -> Result<()> {
    let mut r = random();
    let mut merge_policy = LogMergePolicy::<LogDocMergePolicy>::log_doc();
    merge_policy.set_max_merge_docs(10_000);
    let mut stats = IOStats::default();
    let seg_name_generator = AtomicU64::new(0);
    let merge_context = MockMergeContext::new(|s| Ok(s.get_del_count()));
    let fake_directory = Arc::new(FakeDirectory::new());
    let mut segment_infos = SegmentInfos::new(LATEST.major)?;

    for _ in 0..10 {
      segment_infos.add(make_segment_commit_info(
        &mut r,
        fake_directory.clone(),
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        3_000,
        0,
        0.0,
        SOURCE_MERGE,
      )?)?;
    }

    let spec_opt: Option<MergeSpecificationNoReader<FakeDirectory>> =
      merge_policy.find_merges(MergeTrigger::Explicit, &segment_infos, None, &merge_context)?;
    assert!(spec_opt.is_some());
    let spec = spec_opt.unwrap();

    for one_merge in &spec.merges {
      segment_infos = apply_merge(
        &mut r,
        &segment_infos,
        one_merge,
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        &mut stats,
        fake_directory.clone(),
      )?;
    }

    assert_eq!(9_000, segment_infos.info(0).unwrap().info.max_doc()?);
    Ok(())
  }

  #[test]
  fn test_ignore_large_segments() -> Result<()> {
    let mut r = random();
    let mut merge_policy = LogMergePolicy::<LogDocMergePolicy>::log_doc();
    merge_policy.set_max_merge_docs(10_000);
    let mut stats = IOStats::default();
    let seg_name_generator = AtomicU64::new(0);
    let merge_context = MockMergeContext::new(|s| Ok(s.get_del_count()));
    let fake_directory = Arc::new(FakeDirectory::new());
    let mut segment_infos = SegmentInfos::new(LATEST.major)?;

    segment_infos.add(make_segment_commit_info(
      &mut r,
      fake_directory.clone(),
      &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
      11_000,
      0,
      0.0,
      SOURCE_MERGE,
    )?)?;

    for _ in 0..10 {
      segment_infos.add(make_segment_commit_info(
        &mut r,
        fake_directory.clone(),
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        2_000,
        0,
        0.0,
        SOURCE_MERGE,
      )?)?;
    }

    let spec_opt: Option<MergeSpecificationNoReader<FakeDirectory>> =
      merge_policy.find_merges(MergeTrigger::Explicit, &segment_infos, None, &merge_context)?;
    assert!(spec_opt.is_some());
    let spec = spec_opt.unwrap();

    for one_merge in &spec.merges {
      segment_infos = apply_merge(
        &mut r,
        &segment_infos,
        one_merge,
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        &mut stats,
        fake_directory.clone(),
      )?;
    }

    assert_eq!(11_000, segment_infos.info(0).unwrap().info.max_doc()?);
    assert_eq!(10_000, segment_infos.info(1).unwrap().info.max_doc()?);
    Ok(())
  }

  #[test]
  fn test_full_flush_merges() -> Result<()> {
    let mut r = random();
    let case = TestLogMergePolicy;
    let mp = case.merge_policy(&mut r);

    let seg_name_generator = AtomicU64::new(0);
    let mut stats = IOStats::default();
    let merge_context = MockMergeContext::new(|s| Ok(s.get_del_count()));
    let fake_directory = Arc::new(FakeDirectory::new());
    let mut segment_infos = SegmentInfos::new(LATEST.major)?;

    let num_segments_for_merging = match &mp {
      MergePolicyEnum::LogDoc(p) => {
        p.get_merge_factor() + p.get_target_search_concurrency() as usize
      },
      MergePolicyEnum::LogBytesSize(p) => {
        p.get_merge_factor() + p.get_target_search_concurrency() as usize
      },
      _ => {
        return Err(LuceneError::illegal_state(
          "expected LogMergePolicy variant",
        ));
      },
    };

    for _ in 0..num_segments_for_merging {
      segment_infos.add(make_segment_commit_info(
        &mut r,
        fake_directory.clone(),
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        1,
        0,
        f64::MIN_POSITIVE,
        SOURCE_FLUSH,
      )?)?;
    }

    let spec_opt = mp.find_full_flush_merges(
      MergeTrigger::FullFlush,
      &segment_infos,
      None,
      &merge_context,
    )?;
    assert!(spec_opt.is_some());
    let spec = spec_opt.unwrap();
    for merge in &spec.merges {
      segment_infos = apply_merge(
        &mut r,
        &segment_infos,
        merge,
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        &mut stats,
        fake_directory.clone(),
      )?;
    }
    assert!(segment_infos.size() < num_segments_for_merging);
    Ok(())
  }

  #[test]
  fn test_force_merge_not_needed() -> Result<()> {
    let mut r = random();
    let case = TestLogMergePolicy;
    case.test_force_merge_not_needed(&mut r)
  }

  #[test]
  fn test_find_forced_deletes_merges() -> Result<()> {
    let mut r = random();
    let case = TestLogMergePolicy;
    case.test_find_forced_deletes_merges(&mut r)
  }

  #[test]
  fn test_simulate_append_only() -> Result<()> {
    let mut r = random();
    let case = TestLogMergePolicy;
    let mp = case.merge_policy(&mut r);
    let fake_dir = Arc::new(FakeDirectory::new());
    // TODO IMPORTANT 默认的一亿篇文档速度很慢
    // case.test_simulate_append_only(&mut r, &mp, fake_dir)
    case.do_test_simulate_append_only(&mut r, &mp, fake_dir, 50_000_000, 10_000)
  }

  #[test]
  fn test_simulate_updates() -> Result<()> {
    let mut r = random();
    let case = TestLogMergePolicy;
    let mp = case.merge_policy(&mut r);
    let fake_dir = Arc::new(FakeDirectory::new());
    case.test_simulate_updates(&mut r, &mp, fake_dir)
  }

  #[test]
  fn test_no_pathological_merges() -> Result<()> {
    let mut r = random();
    let case = TestLogMergePolicy;
    let mp = case.merge_policy(&mut r);
    let fake_dir = Arc::new(FakeDirectory::new());
    case.test_no_pathological_merges(&mut r, &mp, fake_dir)
  }
}
