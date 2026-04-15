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
  DEFAULT_MAX_CFS_SEGMENT_SIZE, MergeContext, MergePolicy, MergePolicyBase, MergeSpecification,
  MergeSpecificationNoReader, OneMerge, assert_del_count, size,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::store::directory::Directory;
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

/// Default noCFSRatio. If a merge's size is >= 10% of the index, then we disable compound file for it.
pub const DEFAULT_NO_CFS_RATIO: f64 = 0.1;
/// Merges segments of approximately equal size, subject to an allowed number of segments per tier.
/// This is similar to `LogByteSizeMergePolicy`, except this merge policy is able to merge
/// non-adjacent segment, and separates how many segments are merged at once
/// ([`TieredMergePolicy::set_max_merge_at_once`]) from how many segments are allowed per tier
/// ([`TieredMergePolicy::set_segments_per_tier`]). This merge policy also does not over-merge
/// (i.e. cascade merges).
///
/// For normal merging, this policy first computes a "budget" of how many segments are allowed to
/// be in the index. If the index is over-budget, then the policy sorts segments by decreasing size
/// (pro-rating by percent deletes), and then finds the least-cost merge. Merge cost is measured by a
/// combination of the "skew" of the merge (size of largest segment divided by smallest segment),
/// total merge size and percent deletes reclaimed, so that merges with lower skew, smaller size and
/// those reclaiming more deletes, are favored.
///
/// If a merge will produce a segment that's larger than
/// [`TieredMergePolicy::set_max_merged_segment_mb`], then the policy will merge fewer segments
/// (down to 1 at once, if that one has deletions) to keep the segment size under budget.
///
/// **NOTE**: this policy freely merges non-adjacent segments; if this is a problem, use
/// `LogMergePolicy`.
///
/// **NOTE**: This policy always merges by byte size of the segments, always pro-rates by
/// percent deletes.
///
/// **NOTE** Starting with Lucene 7.5, if you call `IndexWriter::force_merge` with
/// this (default) merge policy, if [`TieredMergePolicy::set_max_merged_segment_mb`] is in conflict
/// with `maxNumSegments` passed to `IndexWriter::force_merge` then `maxNumSegments` wins. For
/// example, if your index has 50 1 GB segments, and you have
/// [`TieredMergePolicy::set_max_merged_segment_mb`] at 1024 (1 GB), and you call `force_merge(10)`,
/// the two settings are clearly in conflict. `TieredMergePolicy` will choose to break the
/// [`TieredMergePolicy::set_max_merged_segment_mb`] constraint and try to merge down to at most ten
/// segments, each up to 5 * 1.25 GB in size (since an extra 25% buffer increase in the expected
/// segment size is targetted).
///
/// findForcedDeletesMerges should never produce segments greater than maxSegmentSize.
///
/// **NOTE**: This policy returns natural merges whose size is below the
/// [`TieredMergePolicy::set_floor_segment_mb`] floor segment size for
/// [`TieredMergePolicy::find_full_flush_merges`] full-flush merges.
#[derive(Clone)]
pub struct TieredMergePolicy {
  // User-specified maxMergeAtOnce. In practice we always take the min of its
  // value and segsPerTier for segments above the floor size to avoid suboptimal merging.
  max_merge_at_once: i32,
  max_merged_segment_bytes: i64,
  floor_segment_bytes: i64,
  segs_per_tier: f64,
  force_merge_deletes_pct_allowed: f64,
  deletes_pct_allowed: f64,
  target_search_concurrency: i32,
  base: MergePolicyBase,
}
impl Default for TieredMergePolicy {
  fn default() -> Self {
    Self::new()
  }
}

impl TieredMergePolicy {
  pub fn new() -> Self {
    let base = MergePolicyBase::new(DEFAULT_NO_CFS_RATIO, DEFAULT_MAX_CFS_SEGMENT_SIZE);
    Self {
      max_merge_at_once: 10,
      max_merged_segment_bytes: 5 * 1024 * 1024 * 1024,
      floor_segment_bytes: 2 * 1024 * 1024,
      segs_per_tier: 10.0,
      force_merge_deletes_pct_allowed: 10.0,
      deletes_pct_allowed: 20.0,
      target_search_concurrency: 1,
      base,
    }
  }
  /// Maximum number of segments to be merged at a time during "normal" merging. Default is 10.
  ///
  /// **NOTE**: Merges above the [`TieredMergePolicy::set_floor_segment_mb`] floor segment size also
  /// bound the number of merged segments by [`TieredMergePolicy::set_segments_per_tier`] the number
  /// of segments per tier.
  pub fn set_max_merge_at_once(&mut self, v: i32) -> Result<&mut Self> {
    if v < 2 {
      return Err(LuceneError::illegal_argument(format!(
        "maxMergeAtOnce must be > 1 (got {})",
        v
      )));
    }
    self.max_merge_at_once = v;
    Ok(self)
  }

  /// Returns the current maxMergeAtOnce setting.
  pub fn get_max_merge_at_once(&self) -> i32 {
    self.max_merge_at_once
  }

  /// Maximum sized segment to produce during normal merging. This setting is approximate: the
  /// estimate of the merged segment size is made by summing sizes of to-be-merged segments
  /// (compensating for percent deleted docs). Default is 5 GB.
  pub fn set_max_merged_segment_mb(&mut self, mut v: f64) -> Result<&mut Self> {
    if v < 0.0 {
      return Err(LuceneError::illegal_argument(format!(
        "maxMergedSegmentMB must be >=0 (got {})",
        v
      )));
    }
    v *= 1024.0 * 1024.0;
    self.max_merged_segment_bytes = if v > i64::MAX as f64 {
      i64::MAX
    } else {
      v as i64
    };
    Ok(self)
  }
  /// Returns the current maxMergedSegmentMB setting.
  pub fn get_max_merged_segment_mb(&self) -> f64 {
    self.max_merged_segment_bytes as f64 / 1024.0 / 1024.0
  }

  /// Controls the maximum percentage of deleted documents that is tolerated in the index. Lower
  /// values make the index more space efficient at the expense of increased CPU and I/O activity.
  /// Values must be between 5 and 50. Default value is 20.
  ///
  /// When the maximum delete percentage is lowered, the indexing thread will call for merges more
  /// often, meaning that write amplification factor will be increased. Write amplification factor
  /// measures the number of times each document in the index is written. A higher write
  /// amplification factor will lead to higher CPU and I/O activity as indicated above.
  pub fn set_deletes_pct_allowed(&mut self, v: f64) -> Result<&mut Self> {
    if !(5.0..=50.0).contains(&v) {
      return Err(LuceneError::illegal_argument(format!(
        "indexPctDeletedTarget must be >= 5.0 and <= 50 (got {})",
        v
      )));
    }
    self.deletes_pct_allowed = v;
    Ok(self)
  }

  /// Returns the current deletesPctAllowed setting.
  pub fn get_deletes_pct_allowed(&self) -> f64 {
    self.deletes_pct_allowed
  }

  /// Segments smaller than this size are merged more aggressively:
  ///
  /// - They are candidates for full-flush merges, in order to reduce the number of segments in
  ///   the index prior to opening a new point-in-time view of the index.
  /// - For background merges, smaller segments are "rounded up" to this size.
  ///
  /// In both cases, this helps prevent frequent flushing of tiny segments to create a long tail of
  /// small segments in the index. Default is 2MB.
  pub fn set_floor_segment_mb(&mut self, mut v: f64) -> Result<&mut Self> {
    if v <= 0.0 {
      return Err(LuceneError::illegal_argument(format!(
        "floorSegmentMB must be > 0.0 (got {})",
        v
      )));
    }
    v *= 1024.0 * 1024.0;
    self.floor_segment_bytes = if v > i64::MAX as f64 {
      i64::MAX
    } else {
      v as i64
    };
    Ok(self)
  }

  /// Returns the current floorSegmentMB.
  ///
  /// @see [`TieredMergePolicy::set_floor_segment_mb`]
  pub fn get_floor_segment_mb(&self) -> f64 {
    self.floor_segment_bytes as f64 / (1024.0 * 1024.0)
  }
  /// When forceMergeDeletes is called, we only merge away a segment if its delete percentage is over
  /// this threshold. Default is 10%.
  pub fn set_force_merge_deletes_pct_allowed(&mut self, v: f64) -> Result<&mut Self> {
    if !(0.0..=100.0).contains(&v) {
      return Err(LuceneError::illegal_argument(format!(
        "forceMergeDeletesPctAllowed must be between 0.0 and 100.0 inclusive (got {})",
        v
      )));
    }
    self.force_merge_deletes_pct_allowed = v;
    Ok(self)
  }

  /// Returns the current forceMergeDeletesPctAllowed setting.
  ///
  /// @see [`TieredMergePolicy::set_force_merge_deletes_pct_allowed`]
  pub fn force_merge_deletes_pct_allowed(&self) -> f64 {
    self.force_merge_deletes_pct_allowed
  }

  /// Sets the allowed number of segments per tier. Smaller values mean more merging but fewer
  /// segments.
  ///
  /// Default is 10.0.
  pub fn set_segments_per_tier(&mut self, v: f64) -> Result<&mut Self> {
    if v < 2.0 {
      return Err(LuceneError::illegal_argument(format!(
        "segmentsPerTier must be >= 2.0 (got {})",
        v
      )));
    }
    self.segs_per_tier = v;
    Ok(self)
  }

  /// Returns the current segmentsPerTier setting.
  ///
  /// @see [`TieredMergePolicy::set_segments_per_tier`]
  pub fn get_segments_per_tier(&self) -> f64 {
    self.segs_per_tier
  }

  /// Sets the target search concurrency. This prevents creating segments that are bigger than
  /// maxDoc/targetSearchConcurrency, which in turn makes the work parallelizable into
  /// targetSearchConcurrency slices of similar doc counts. It also makes merging less aggressive,
  /// as higher values result in indices that do less merging and have more segments
  pub fn set_target_search_concurrency(
    &mut self,
    target_search_concurrency: i32,
  ) -> Result<&mut Self> {
    if target_search_concurrency < 1 {
      return Err(LuceneError::illegal_argument(format!(
        "targetSearchConcurrency must be >= 1 (got {})",
        target_search_concurrency
      )));
    }
    self.target_search_concurrency = target_search_concurrency;
    Ok(self)
  }

  /// Returns the target search concurrency.
  pub fn get_target_search_concurrency(&self) -> i32 {
    self.target_search_concurrency
  }
  // The size can change concurrently while we are running here, because deletes
  // are now applied concurrently, and this can piss off TimSort! So we
  // call size() once per segment and sort by that:
  fn get_sorted_by_segment_size<D, MC>(
    &self,
    infos: &SegmentInfos<D>,
    merge_context: &MC,
  ) -> Result<Vec<SegmentSizeAndDocs>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    let mut sorted_by_size = Vec::new();

    for info in infos.iter() {
      sorted_by_size.push(SegmentSizeAndDocs::new(
        info,
        self.size(info, merge_context)?,
        merge_context.num_deletes_to_merge(info)?,
      )?);
    }

    sorted_by_size.sort_by(|o1, o2| {
      // Sort by largest size:
      let mut cmp = o2.size_in_bytes.cmp(&o1.size_in_bytes);
      if cmp == std::cmp::Ordering::Equal {
        cmp = o1.name.cmp(&o2.name);
      }
      cmp
    });

    Ok(sorted_by_size)
  }
  #[allow(clippy::too_many_arguments)]
  fn do_find_merges<MC, D>(
    &self,
    sorted_eligible_infos: &[SegmentSizeAndDocs],
    max_merged_segment_bytes: i64,
    merge_factor: i32,
    allowed_seg_count: usize,
    allowed_del_count: i32,
    allowed_doc_count: i32,
    merge_type: MergeType,
    merge_context: &MC,
    max_merge_is_running: bool,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    let mut sorted_eligible: Vec<SegmentSizeAndDocs> = sorted_eligible_infos.to_vec();

    let mut seg_infos_sizes = HashMap::new();
    for seg in &sorted_eligible {
      seg_infos_sizes.insert(seg.seg_info.clone(), seg.clone());
    }

    let original_sorted_size = sorted_eligible.len();
    if self.verbose(merge_context) {
      self.message(
        &format!("findMerges: {} segments", original_sorted_size),
        merge_context,
      );
    }
    if original_sorted_size == 0 {
      return Ok(None);
    }

    let mut to_be_merged = HashSet::new();

    let mut spec = None;
    // Cycle to possibly select more than one merge:
    // The trigger point for total deleted documents in the index leads to a bunch of large segment
    // merges at the same time. So only put one large merge in the list of merges per cycle. We'll
    // pick up another
    // merge next time around.
    let mut have_one_large_merge = false;

    loop {
      // Gather eligible segments for merging, ie segments
      // not already being merged and not already picked (by
      // prior iteration of this loop) for merging:

      // Remove ineligible segments. These are either already being merged or already picked by
      // prior iterations
      sorted_eligible.retain(|s| !to_be_merged.contains(&s.seg_info));

      if self.verbose(merge_context) {
        self.message(
          &format!(
            "  allowedSegmentCount={} vs count={} (eligible count={})",
            allowed_seg_count,
            original_sorted_size,
            sorted_eligible.len()
          ),
          merge_context,
        );
      }

      if sorted_eligible.is_empty() {
        return Ok(spec);
      }

      let remaining_del_count: i32 = sorted_eligible.iter().map(|c| c.del_count).sum();
      if merge_type == MergeType::Natural
        && sorted_eligible.len() <= allowed_seg_count
        && remaining_del_count <= allowed_del_count
      {
        return Ok(spec);
      }
      // OK we are over budget -- find best merge!
      let mut best_score: Option<MergeScoreImpl> = None;
      let mut best = None;
      let mut best_too_large = false;

      for start_idx in 0..sorted_eligible.len() {
        let mut candidate = Vec::new();
        let mut hit_too_large = false;
        let mut bytes_this_merge = 0;
        let mut doc_count_this_merge: i64 = 0;

        let mut idx = start_idx;
        while idx < sorted_eligible.len()
                    && candidate.len() < self.max_merge_at_once as usize
                    // We allow merging more than mergeFactor segments together if the merged segment
                    // would be less than the floor segment size. This is important because segments
                    // below the floor segment size are more aggressively merged by this policy, so we
                    // need to grow them as quickly as possible.
                    && (candidate.len() < merge_factor as usize
                    || bytes_this_merge < self.floor_segment_bytes)
                    && bytes_this_merge < max_merged_segment_bytes
                    && (bytes_this_merge < self.floor_segment_bytes
                    || doc_count_this_merge <= allowed_doc_count as i64)
        {
          let seg_size_docs = &sorted_eligible[idx];
          let seg_bytes = seg_size_docs.size_in_bytes;
          let seg_doc_count = seg_size_docs.max_doc - seg_size_docs.del_count;

          if bytes_this_merge + seg_bytes > max_merged_segment_bytes
            || (bytes_this_merge > self.floor_segment_bytes
              && doc_count_this_merge + seg_doc_count as i64 > allowed_doc_count as i64)
          {
            // Only set hitTooLarge when reaching the maximum byte size, as this will create
            // segments of the maximum size which will no longer be eligible for merging for a long
            // time (until they accumulate enough deletes).
            hit_too_large |= bytes_this_merge + seg_bytes > max_merged_segment_bytes;
            // We should never have something coming in that _cannot_ be merged, so handle
            // singleton merges
            if !candidate.is_empty() {
              // NOTE: we continue, so that we can try
              // "packing" smaller segments into this merge
              // to see if we can get closer to the max
              // size; this in general is not perfect since
              // this is really "bin packing" and we'd have
              // to try different permutations.
              idx += 1;
              continue;
            }
          }

          candidate.push(SegmentCommitInfoMeta::new(
            seg_size_docs.seg_info.clone(),
            seg_size_docs.size_in_seg,
            seg_size_docs.max_doc,
            seg_size_docs.name.clone(),
          ));
          bytes_this_merge += seg_bytes;
          doc_count_this_merge += seg_doc_count as i64;
          idx += 1;
        }
        // We should never see an empty candidate: we iterated over maxMergeAtOnce
        // segments, and already pre-excluded the too-large segments:
        debug_assert!(!candidate.is_empty());

        let max_candidate_segment_size = match seg_infos_sizes.get(&candidate[0].seg_id) {
          Some(c) => c,
          None => return Err(LuceneError::illegal_state("could not  find candidate")),
        };

        if !hit_too_large
          && merge_type == MergeType::Natural
          && bytes_this_merge < (max_candidate_segment_size.size_in_bytes as f64 * 1.5) as i64
          && max_candidate_segment_size.del_count
            < (max_candidate_segment_size.max_doc as f64 * self.deletes_pct_allowed / 100.0) as i32
        {
          // Ignore any merge where the resulting segment is not at least 50% larger than the
          // biggest input segment.
          // Otherwise we could run into pathological O(N^2) merging where merges keep rewriting
          // again and again the biggest input segment into a segment that is barely bigger.
          // The only exception we make is when the merge would reclaim lots of deletes in the
          // biggest segment. This is important for cases when lots of documents get deleted at once
          // without introducing new segments of a similar size for instance.
          continue;
        }
        // A singleton merge with no deletes makes no sense. We can get here when forceMerge is
        // looping around...
        if candidate.len() == 1 && max_candidate_segment_size.del_count == 0 {
          continue;
        }
        // If we didn't find a too-large merge and have a list of candidates
        // whose length is less than the merge factor, it means we are reaching
        // the tail of the list of segments and will only find smaller merges.
        // Stop here.
        if best_score.is_some() && !hit_too_large && candidate.len() < merge_factor as usize {
          break;
        }

        let score = self.score(&candidate, hit_too_large, &seg_infos_sizes)?;

        if (best_score.is_none() || score.score() < best_score.as_ref().unwrap().score())
          && (!hit_too_large || !max_merge_is_running)
        {
          best = Some(candidate);
          best_score = Some(score);
          best_too_large = hit_too_large;
        }
      }

      let best = match best {
        Some(b) => b,
        None => return Ok(spec),
      };
      // The mergeType == FORCE_MERGE_DELETES behaves as the code does currently and can create a
      // large number of
      // concurrent big merges. If we make findForcedDeletesMerges behave as findForcedMerges and
      // cycle through
      // we should remove this.
      if !have_one_large_merge || !best_too_large || merge_type == MergeType::ForceMergeDeletes {
        have_one_large_merge |= best_too_large;

        let spec_ref = spec.get_or_insert_with(MergeSpecification::new);
        let merge = OneMerge::from_meta(best.as_ref())?;
        spec_ref.add(merge);
      }
      // whether we're going to return this list in the spec of not, we need to remove it from
      // consideration on the next loop.
      for s in best {
        to_be_merged.insert(s.seg_id);
      }
    }
  }

  /// Expert: scores one merge; subclasses can override.
  fn score(
    &self,
    candidate: &[SegmentCommitInfoMeta],
    hit_too_large: bool,
    segments_sizes: &HashMap<String, SegmentSizeAndDocs>,
  ) -> Result<MergeScoreImpl> {
    let mut tot_before_merge_bytes: i64 = 0;
    let mut tot_after_merge_bytes: i64 = 0;
    let mut tot_after_merge_bytes_floored: i64 = 0;

    for info in candidate {
      let seg_bytes = segments_sizes.get(&info.seg_id).unwrap().size_in_bytes;
      tot_after_merge_bytes += seg_bytes;
      tot_after_merge_bytes_floored += self.floor_size(seg_bytes);
      tot_before_merge_bytes += info.size_in_seg;
    }

    // Roughly measure "skew" of the merge, i.e. how
    // "balanced" the merge is (whether the segments are
    // about the same size), which can range from
    // 1.0/numSegsBeingMerged (good) to 1.0 (poor). Heavily
    // lopsided merges (skew near 1.0) is no good; it means
    // O(N^2) merge cost over time:
    let skew: f64 = if hit_too_large {
      // Pretend the merge has perfect skew; skew doesn't
      // matter in this case because this merge will not
      // "cascade" and so it cannot lead to N^2 merge cost
      // over time:
      let merge_factor = std::cmp::min(self.max_merge_at_once, self.segs_per_tier as i32);
      1.0 / (merge_factor as f64)
    } else {
      (self.floor_size(
        segments_sizes
          .get(&candidate[0].seg_id)
          .unwrap()
          .size_in_bytes,
      ) as f64)
        / (tot_after_merge_bytes_floored as f64)
    };

    // Strongly favor merges with less skew (smaller
    // mergeScore is better):
    let mut merge_score = skew;

    // Gently favor smaller merges over bigger ones. We
    // don't want to make this exponent too large else we
    // can end up doing poor merges of small segments in
    // order to avoid the large merges:
    merge_score *= (tot_after_merge_bytes as f64).powf(0.05);

    // Strongly favor merges that reclaim deletes:
    let non_del_ratio = (tot_after_merge_bytes as f64) / (tot_before_merge_bytes as f64);
    merge_score *= non_del_ratio.powf(2.0);

    let final_merge_score = merge_score;

    Ok(MergeScoreImpl {
      final_merge_score,
      skew,
      non_del_ratio,
    })
  }
  pub(crate) fn get_max_allowed_docs(&self, total_max_doc: i32, total_del_docs: i32) -> i32 {
    let v = total_max_doc - total_del_docs;
    (v + self.target_search_concurrency - 1) / self.target_search_concurrency
  }

  fn floor_size(&self, bytes: i64) -> i64 {
    std::cmp::max(self.floor_segment_bytes, bytes)
  }
}
pub struct SegmentCommitInfoMeta {
  pub(crate) seg_id: String,
  pub(crate) size_in_seg: i64,
  pub(crate) max_doc: i32,
  pub(crate) name: String,
}
impl SegmentCommitInfoMeta {
  fn new(seg_id: String, size_in_seg: i64, max_doc: i32, name: String) -> Self {
    Self {
      seg_id,
      size_in_seg,
      max_doc,
      name,
    }
  }
}
pub struct SegmentDocAndID {
  pub(crate) seg_id: String,
  pub(crate) max_doc: i32,
}
impl SegmentDocAndID {
  pub(crate) fn new(seg_id: String, max_doc: i32) -> Self {
    Self { seg_id, max_doc }
  }
}

impl Display for TieredMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "[{}: maxMergeAtOnce={}, maxMergedSegmentMB={}, floorSegmentMB={}, \
forceMergeDeletesPctAllowed={}, segmentsPerTier={}, maxCFSSegmentSizeMB={}, \
noCFSRatio={}, deletesPctAllowed={}, targetSearchConcurrency={}",
      std::any::type_name::<Self>()
        .rsplit("::")
        .next()
        .unwrap_or("TieredMergePolicy"),
      self.max_merge_at_once,
      self.max_merged_segment_bytes as f64 / 1024.0 / 1024.0,
      self.floor_segment_bytes as f64 / 1024.0 / 1024.0,
      self.force_merge_deletes_pct_allowed,
      self.segs_per_tier,
      self.base.get_max_cfs_segment_size_mb(),
      self.base.get_no_cfs_ratio(),
      self.deletes_pct_allowed,
      self.target_search_concurrency,
    )
  }
}

impl MergePolicy for TieredMergePolicy {
  fn get_base(&self) -> &MergePolicyBase {
    &self.base
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    &mut self.base
  }

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
    // Compute total index bytes & print details about the index
    let mut tot_index_bytes: i64 = 0;
    let mut min_segment_bytes: i64 = i64::MAX;

    let mut total_del_docs: i32 = 0;
    let mut total_max_doc: i32 = 0;

    let mut merging_bytes: i64 = 0;

    let merging = merge_context.get_merging_segments(inner);
    let mut sorted_infos = self.get_sorted_by_segment_size(infos, merge_context)?;
    sorted_infos.retain(|seg| {
      let seg_bytes = seg.size_in_bytes;

      min_segment_bytes = std::cmp::min(seg_bytes, min_segment_bytes);
      tot_index_bytes += seg_bytes;

      if merging.contains(&seg.seg_info) {
        merging_bytes += seg_bytes;
        // if this segment is merging, then its deletes are being reclaimed already.
        // only count live docs in the total max doc
        total_max_doc += seg.max_doc - seg.del_count;
        false
      } else {
        total_del_docs += seg.del_count;
        total_max_doc += seg.max_doc;
        true
      }
    });

    debug_assert!(total_max_doc >= 0);
    debug_assert!(total_del_docs >= 0);

    let total_del_pct = 100.0 * total_del_docs as f64 / total_max_doc as f64;
    let mut allowed_del_count = (self.deletes_pct_allowed * total_max_doc as f64 / 100.0) as i32;
    // If we have too-large segments, grace them out of the maximum segment count
    // If we're above certain thresholds of deleted docs, we can merge very large segments.
    let mut too_big_count = 0;
    // We relax merging for the bigger segments for concurrency reasons, as we want to have several
    // segments on the highest tier without over-merging on the lower tiers.
    let mut concurrency_count = 0;
    let mut allowed_seg_count: f64 = 0.0;
    // remove large segments from consideration under two conditions.
    // 1> Overall percent deleted docs relatively small and this segment is larger than 50%
    // maxSegSize
    // 2> overall percent deleted docs large and this segment is large and has few deleted docs
    sorted_infos.retain(|seg| {
      let seg_del_pct = 100.0 * seg.del_count as f64 / seg.max_doc as f64;

      if seg.size_in_bytes > self.max_merged_segment_bytes / 2
        && (total_del_pct <= self.deletes_pct_allowed || seg_del_pct <= self.deletes_pct_allowed)
      {
        too_big_count += 1;
        tot_index_bytes -= seg.size_in_bytes;
        allowed_del_count -= seg.del_count;
        false
      } else if concurrency_count + too_big_count < self.target_search_concurrency - 1 {
        // Make sure we count a whole segment for the first targetSearchConcurrency-1 segments to
        // avoid over merging on the lower levels.
        concurrency_count += 1;
        allowed_seg_count += 1.0;
        tot_index_bytes -= seg.size_in_bytes;
        true
      } else {
        true
      }
    });

    allowed_del_count = std::cmp::max(0, allowed_del_count);

    let merge_factor = std::cmp::min(self.max_merge_at_once, self.segs_per_tier as i32);
    // Compute max allowed segments for the remainder of the index
    let mut level_size = std::cmp::max(min_segment_bytes, self.floor_segment_bytes);
    let mut bytes_left = tot_index_bytes;

    loop {
      let seg_count_level = bytes_left as f64 / level_size as f64;
      if seg_count_level < self.segs_per_tier || level_size == self.max_merged_segment_bytes {
        allowed_seg_count += seg_count_level.ceil();
        break;
      }
      allowed_seg_count += self.segs_per_tier;
      bytes_left -= (self.segs_per_tier * level_size as f64) as i64;
      level_size = std::cmp::min(
        self.max_merged_segment_bytes,
        level_size * merge_factor as i64,
      );
    }
    // allowedSegCount may occasionally be less than segsPerTier
    // if segment sizes are below the floor size

    allowed_seg_count = allowed_seg_count.max(self.segs_per_tier);
    // No need to merge if the total number of segments (including too big segments) is less than or
    // equal to the target search concurrency.
    allowed_seg_count =
      allowed_seg_count.max((self.target_search_concurrency - too_big_count) as f64);

    let allowed_doc_count = self.get_max_allowed_docs(total_max_doc, total_del_docs);

    self.do_find_merges(
      &sorted_infos,
      self.max_merged_segment_bytes,
      merge_factor,
      allowed_seg_count as usize,
      allowed_del_count,
      allowed_doc_count,
      MergeType::Natural,
      merge_context,
      merging_bytes >= self.max_merged_segment_bytes,
    )
  }

  fn find_forced_merges<D, MC>(
    &self,
    infos: &SegmentInfos<D>,
    max_segment_count: usize,
    segments_to_merge: &HashMap<String, Option<bool>>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    let mut sorted_size_and_docs = self.get_sorted_by_segment_size(infos, merge_context)?;

    let mut total_merge_bytes: i64 = 0;
    let merging = merge_context.get_merging_segments(inner);
    let mut force_merge_running = false;
    // Trim the list down, remove if we're respecting max segment size and it's not original.
    // Presumably it's been merged before and is close enough to the max segment size we
    // shouldn't add it in again.
    sorted_size_and_docs.retain(|seg| {
      let is_original = segments_to_merge.get(&seg.seg_info).copied();
      if let Some(Some(_)) = is_original {
        if merging.contains(&seg.seg_info) {
          force_merge_running = true;
          false
        } else {
          total_merge_bytes += seg.size_in_bytes;
          true
        }
      } else {
        false
      }
    });

    let mut max_merge_bytes = self.max_merged_segment_bytes;

    // Set the maximum segment size based on how many segments have been specified.
    if max_segment_count == 1 {
      max_merge_bytes = i64::MAX;
    } else if max_segment_count != i32::MAX as usize {
      max_merge_bytes = std::cmp::max(
        ((total_merge_bytes as f64) / (max_segment_count as f64)) as i64,
        self.max_merged_segment_bytes,
      );
      // Fudge this up a bit so we have a better chance of not having to do a second pass of merging
      // to get
      // down to the requested target segment count. If we use the exact size, it's almost
      // guaranteed
      // that the segments selected below won't fit perfectly and we'll be left with more segments
      // than
      // we want and have to re-merge in the code at the bottom of this method.
      max_merge_bytes = (max_merge_bytes as f64 * 1.25) as i64;
    }

    let mut found_deletes = false;

    sorted_size_and_docs.retain(|seg| {
      let is_original = segments_to_merge.get(&seg.seg_info).copied();

      if seg.del_count != 0 {
        // This is forceMerge; all segments with deleted docs should be merged.
        if matches!(is_original, Some(Some(true))) {
          found_deletes = true;
        }
        return true;
      }

      // Let the scoring handle whether to merge large segments.
      if max_segment_count == i32::MAX as usize && matches!(is_original, Some(Some(false))) {
        return false;
      }

      // Don't try to merge a segment with no deleted docs that's over the max size.
      if max_segment_count != i32::MAX as usize && seg.size_in_bytes >= max_merge_bytes {
        return false;
      }

      true
    });

    // Nothing to merge this round.
    if sorted_size_and_docs.is_empty() {
      return Ok(None);
    }
    let sorted_size_and_docs_len = sorted_size_and_docs.len();
    // We only bail if there are no deletions
    if !found_deletes {
      let info_zero = &sorted_size_and_docs[0].seg_info;

      let already = if max_segment_count != i32::MAX as usize
        && max_segment_count > 1
        && sorted_size_and_docs_len <= max_segment_count
      {
        true
      } else {
        max_segment_count == 1
          && sorted_size_and_docs_len == 1
          && (segments_to_merge.get(info_zero).is_some()
            || self.has_merged(
              infos,
              infos
                .index_of(info_zero)
                .ok_or_else(|| LuceneError::illegal_argument("Missing numeric value"))?,
              merge_context,
            )?)
      };

      if already {
        return Ok(None);
      }
    }

    let starting_segment_count = sorted_size_and_docs.len();
    if force_merge_running {
      // hmm this is a little dangerous -- if a user kicks off a forceMerge, it is taking forever,
      // lots of
      // new indexing/segments happened since, and they want to kick off another to ensure those
      // newly
      // indexed segments partake in the force merge, they (silently) won't due to this?
      return Ok(None);
    }

    // This is the special case of merging down to one segment
    if max_segment_count == 1 && total_merge_bytes < max_merge_bytes {
      let mut spec = MergeSpecificationNoReader::new();
      let all_of_them: Vec<SegmentCommitInfoMeta> = sorted_size_and_docs
        .iter()
        .map(|s| {
          SegmentCommitInfoMeta::new(s.seg_info.clone(), s.size_in_seg, s.max_doc, s.name.clone())
        })
        .collect();
      spec.add(OneMerge::from_meta(all_of_them.as_ref())?);
      return Ok(Some(spec));
    }

    let mut spec: Option<MergeSpecificationNoReader<D>> = None;

    let mut index: i32 = (starting_segment_count - 1).try_convert()?;
    let mut resulting_segments = starting_segment_count;

    loop {
      let mut candidate = Vec::new();
      let mut current_candidate_bytes: i64 = 0;

      while index >= 0 && resulting_segments > max_segment_count {
        let sorted_size_and_doc = &sorted_size_and_docs[index as usize];
        let initial_candidate_size = candidate.len();
        let current_segment_size = sorted_size_and_doc.size_in_seg;
        // We either add to the bin because there's space or because the it is the smallest possible
        // bin since
        // decrementing the index will move us to even larger segments.
        if current_candidate_bytes + current_segment_size <= max_merge_bytes
          || initial_candidate_size < 2
        {
          candidate.push(SegmentCommitInfoMeta::new(
            sorted_size_and_doc.seg_info.clone(),
            sorted_size_and_doc.size_in_seg,
            sorted_size_and_doc.max_doc,
            sorted_size_and_doc.name.clone(),
          ));
          index -= 1;
          current_candidate_bytes += current_segment_size;
          if initial_candidate_size > 0 {
            // Any merge that handles two or more segments reduces the resulting number of segments
            // by the number of segments handled - 1
            resulting_segments -= 1;
          }
        } else {
          break;
        }
      }

      let candidate_size = candidate.len();

      // While a force merge is running, only merges that cover the maximum allowed number of
      // segments or that create a segment close to the
      // maximum allowed segment sized are permitted
      if candidate_size > 1
        && (!force_merge_running
          || (current_candidate_bytes as f64) > 0.7 * (max_merge_bytes as f64))
      {
        let merge = OneMerge::from_meta(candidate.as_ref())?;

        let spec_ref = spec.get_or_insert_with(MergeSpecificationNoReader::new);
        spec_ref.add(merge);
      } else {
        return Ok(spec);
      }
    }
  }

  fn find_forced_deletes_merges<D, MC>(
    &self,
    infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    // First do a quick check that there's any work to do.
    // NOTE: this makes BaseMergePOlicyTestCase.testFindForcedDeletesMerges work
    let mut have_work = false;
    let mut total_del_count: i32 = 0;

    for info in infos.iter() {
      let del_count = merge_context.num_deletes_to_merge(info)?;
      debug_assert!(assert_del_count(del_count, info)?);
      total_del_count += del_count;

      let pct_deletes = 100.0 * (del_count as f64) / (info.info.max_doc()? as f64);
      have_work = have_work
        || (pct_deletes > self.force_merge_deletes_pct_allowed
          && !merge_context
            .get_merging_segments(inner)
            .contains(&info.info.name));
    }

    if !have_work {
      return Ok(None);
    }

    let mut sorted_infos = self.get_sorted_by_segment_size(infos, merge_context)?;

    sorted_infos.retain(|seg| {
      let pct_deletes = 100.0 * (seg.del_count as f64) / (seg.max_doc as f64);
      !(merge_context
        .get_merging_segments(inner)
        .contains(&seg.seg_info)
        || pct_deletes <= self.force_merge_deletes_pct_allowed)
    });

    self.do_find_merges(
      &sorted_infos,
      self.max_merged_segment_bytes,
      i32::MAX,
      usize::MAX,
      0,
      self.get_max_allowed_docs(infos.total_max_doc()?, total_del_count),
      MergeType::ForceMergeDeletes,
      merge_context,
      false,
    )
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    size(info, merge_context)
  }

  fn max_full_flush_merge_size(&self) -> i64 {
    self.floor_segment_bytes
  }
}
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum MergeType {
  Natural,
  ForceMerge,
  ForceMergeDeletes,
}
#[derive(Clone)]
struct SegmentSizeAndDocs {
  seg_info: String,
  /// Size of the segment in bytes, pro-rated by the number of live documents.
  size_in_bytes: i64,
  size_in_seg: i64,
  del_count: i32,
  max_doc: i32,
  name: String,
}

impl SegmentSizeAndDocs {
  fn new<D>(info: &SegmentCommitInfo<D>, size_in_bytes: i64, seg_del_count: i32) -> Result<Self>
  where
    D: Directory,
  {
    let max_doc = info.info.max_doc()?;
    Ok(Self {
      seg_info: info.info.get_id_key().to_string(),
      name: info.info.name.clone(),
      size_in_bytes,
      size_in_seg: info.size_in_bytes()?,
      del_count: seg_del_count,
      max_doc,
    })
  }
}
/// Holds score and explanation for a single candidate merge.
pub(crate) trait MergeScore {
  /// Returns the score for this merge candidate; lower scores are better.
  fn score(&self) -> f64;

  /// Human readable explanation of how the merge got this score.
  fn explanation(&self) -> String;
}
struct MergeScoreImpl {
  final_merge_score: f64,
  skew: f64,
  non_del_ratio: f64,
}

impl MergeScore for MergeScoreImpl {
  fn score(&self) -> f64 {
    self.final_merge_score
  }

  fn explanation(&self) -> String {
    format!(
      "skew={:.3} nonDelRatio={:.3}",
      self.skew, self.non_del_ratio
    )
  }
}
#[cfg(test)]
mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::document::field_type::FieldType;
  use crate::core::document::stored_field::StoredField;
  use crate::core::document::string_field::string_field_type;
  use crate::core::index::codec_reader::CodecReader;
  use crate::core::index::composite_reader::get_context;
  use crate::core::index::directory_reader::directory_reader_util;
  use crate::core::index::index_reader::IndexReader;
  use crate::core::index::index_reader_context::IndexReaderContext;
  use crate::core::index::index_writer::{
    IndexWriter, IndexWriterBase, SOURCE_FLUSH, SOURCE_MERGE,
  };
  use crate::core::index::leaf_reader::LeafReader;
  use crate::core::index::leaf_reader_context::LeafReaderContext;
  use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
  use crate::core::index::merge_policy::{MergePolicy, MergePolicyEnum, MergeSpecification};
  use crate::core::index::merge_trigger::MergeTrigger;
  use crate::core::index::segment_infos::SegmentInfos;
  use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
  use crate::core::index::term::Term;
  use crate::core::index::terms::Terms;
  use crate::core::index::tiered_merge_policy::TieredMergePolicy;
  use crate::core::store::directory::Directory;
  use crate::core::util::LATEST;
  use crate::core::util::bytes_ref_iterator::BytesRefIterator;
  use crate::core::util::error::lucene_error::{LuceneError, Result};
  use rand::{Rng, RngExt};
  use std::collections::{HashMap, HashSet};
  use std::sync::Arc;
  use std::sync::atomic::{AtomicU64, Ordering};

  use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
  use crate::test::core::index::base_merge_policy_test_case::{
    BaseMergePolicyTestCase, FakeDirectory, IOStats, MockMergeContext, apply_deletes, apply_merge,
    make_segment_commit_info,
  };
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    at_least, is_night_mode, new_directory_shared, new_field,
    new_index_writer_config_with_analyzer, new_text_field, new_tiered_merge_policy, random,
    random_multiplier,
  };
  use crate::test::core::util::test_util::TestUtil;

  struct TestTieredMergePolicy;
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct DocCountAndSizeInBytes {
    pub doc_count: i32,
    pub size_in_bytes: i64,
  }
  #[test]
  fn test_force_merge_deletes() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);

    let mut tmp = new_tiered_merge_policy(&mut random);

    tmp.set_max_merge_at_once(100)?;
    tmp.set_segments_per_tier(100.0)?;
    tmp.set_deletes_pct_allowed(50.0)?;
    tmp.set_force_merge_deletes_pct_allowed(30.0)?;
    conf.set_merge_policy(tmp);
    conf.set_max_buffered_docs(4);

    let mut w = IndexWriter::new(dir.clone(), conf)?;

    let mut field_to_type = HashMap::new();

    for i in 0..80 {
      let mut doc = Document::new();
      let value = format!("aaa {}", i % 4);
      doc.add(new_text_field(
        &mut random,
        "content",
        &value,
        Store::No,
        &mut field_to_type,
      )?);
      w.add_document(doc)?;
    }

    assert_eq!(80, w.get_doc_stats()?.max_doc);
    assert_eq!(80, w.get_doc_stats()?.num_docs);

    if cfg!(feature = "test_log_verbose") {
      println!("\nTEST: delete docs");
    }

    w.delete_documents_with_terms(vec![Term::from_text("content", "0")])?;
    w.force_merge_deletes()?;

    assert_eq!(80, w.get_doc_stats()?.max_doc);
    assert_eq!(60, w.get_doc_stats()?.num_docs);

    if cfg!(feature = "test_log_verbose") {
      println!("\nTEST: forceMergeDeletes2");
    }

    let mp = match w.get_config_mut().get_merge_policy_mut() {
      MergePolicyEnum::Tiered(t) => t,
      _ => unreachable!(""),
    };
    mp.set_force_merge_deletes_pct_allowed(10.0)?;

    w.force_merge_deletes()?;

    assert_eq!(60, w.get_doc_stats()?.max_doc);
    assert_eq!(60, w.get_doc_stats()?.num_docs);

    w.close()?;
    Ok(())
  }
  #[test]
  fn test_partial_merge() -> Result<()> {
    let mut random = random();
    let num = at_least(&mut random, 10);

    for iter in 0..num {
      if cfg!(feature = "test_log_verbose") {
        println!("TEST: iter={}", iter);
      }

      let dir = new_directory_shared(&mut random)?;

      let analyzer = MockAnalyzer::new(&mut random);
      let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);

      conf.set_merge_scheduler(SerialMergeScheduler::new());

      let mut tmp = new_tiered_merge_policy(&mut random);
      tmp.set_max_merge_at_once(3)?;
      tmp.set_segments_per_tier(6.0)?;

      let max_merged_segment_mb = tmp.get_max_merged_segment_mb();
      let floor_segment_mb = tmp.get_floor_segment_mb();
      conf.set_merge_policy(tmp);
      conf.set_max_buffered_docs(2);

      let w = IndexWriter::new(dir.clone(), conf)?;

      let mut field_to_type = HashMap::new();

      let mut max_count = 0;
      let num_docs = TestUtil::next_int(&mut random, 20, 100);

      for i in 0..num_docs {
        let mut doc = Document::new();
        let value = format!("aaa {}", i % 4);
        doc.add(new_text_field(
          &mut random,
          "content",
          &value,
          Store::No,
          &mut field_to_type,
        )?);

        w.add_document(doc)?;

        let count = w.get_segment_count();
        max_count = std::cmp::max(count, max_count);

        assert!(
          count + 3 >= max_count,
          "count={} maxCount={}",
          count,
          max_count
        );
      }

      w.flush_with_apply_merge_deletes(true, true)?;

      let segment_count = w.get_segment_count();
      let target_count = TestUtil::next_int(&mut random, 1, segment_count as i32);

      if cfg!(feature = "test_log_verbose") {
        println!(
          "TEST: merge to {} segs (current count={})",
          target_count, segment_count
        );
      }

      w.force_merge(target_count)?;

      let max_segment_size = f64::max(max_merged_segment_mb, floor_segment_mb);

      let max125_pct = (max_segment_size * 1024.0 * 1024.0 * 1.25) as i64;

      if target_count == 1 {
        assert_eq!(target_count as usize, w.get_segment_count(),);
      } else {
        let infos = w.clone_segment_infos()?;

        for i in 0..infos.size() {
          let info = infos.info(i).unwrap();
          assert!(
            max125_pct >= info.size_in_bytes()?,
            "No segment should be more than 125% of max segment size"
          );
        }
      }

      w.close()?;
    }

    Ok(())
  }
  #[test]
  fn test_force_merge_deletes_max_seg_size() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);

    let mut tmp = TieredMergePolicy::new();
    tmp.set_max_merged_segment_mb(0.01)?;
    tmp.set_force_merge_deletes_pct_allowed(0.0)?;
    conf.set_merge_policy(tmp);

    let w = IndexWriter::new(dir.clone(), conf)?;

    let mut field_to_type = HashMap::new();

    let num_docs = at_least(&mut random, 200);

    for i in 0..num_docs {
      let mut doc = Document::new();
      doc.add(new_field(
        &mut random,
        "id",
        i.to_string(),
        &FieldType::from_ref(&*string_field_type::TYPE_NOT_STORED)?,
        &mut field_to_type,
      )?);
      doc.add(new_text_field(
        &mut random,
        "content",
        format!("aaa {}", i),
        Store::No,
        &mut field_to_type,
      )?);
      w.add_document(doc)?;
    }

    w.force_merge(1)?;

    let reader = directory_reader_util::open_from_writer(&w)?;
    assert_eq!(num_docs, reader.max_doc()?);
    assert_eq!(num_docs, reader.num_docs()?);
    reader.close()?;

    if cfg!(feature = "test_log_verbose") {
      println!("\nTEST: delete doc");
    }

    let term_val = (42 + 17).to_string();
    w.delete_documents_with_terms(vec![Term::from_text("id", &term_val)])?;

    let reader = directory_reader_util::open_from_writer(&w)?;
    assert_eq!(num_docs, reader.max_doc()?);
    assert_eq!(num_docs - 1, reader.num_docs()?);
    reader.close()?;

    w.force_merge_deletes()?;

    let reader = directory_reader_util::open_from_writer(&w)?;
    assert_eq!(num_docs - 1, reader.max_doc()?);
    assert_eq!(num_docs - 1, reader.num_docs()?);
    reader.close()?;

    w.close()?;
    Ok(())
  }
  #[test]
  fn test_forced_merges_respect_seg_size() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let mut tmp = TieredMergePolicy::new();

    let mb_size = 0.004;
    let max_seg_bytes = (1024.0 * 1024.0) as i64;
    tmp.set_max_merged_segment_mb(mb_size)?;
    conf.set_max_buffered_docs(100);
    conf.set_merge_policy(tmp);
    conf.set_merge_scheduler(SerialMergeScheduler::new());

    let mut w = IndexWriter::new(dir.clone(), conf)?;

    let mut field_to_type = HashMap::new();

    let num_docs = at_least(&mut random, 2400);
    for i in 0..num_docs {
      let mut doc = Document::new();
      doc.add(new_field(
        &mut random,
        "id",
        i.to_string(),
        &FieldType::from_ref(&*string_field_type::TYPE_NOT_STORED)?,
        &mut field_to_type,
      )?);
      doc.add(new_text_field(
        &mut random,
        "content",
        format!("aaa {}", i),
        Store::No,
        &mut field_to_type,
      )?);
      w.add_document(doc)?;
    }

    w.commit()?;

    let mut seg_names_before = get_segment_names(&w)?;
    w.force_merge_deletes()?;
    check_segments_in_expectations(&w, &seg_names_before, false)?;
    w.force_merge(i32::MAX)?;
    check_segments_in_expectations(&w, &seg_names_before, true)?;
    check_segment_size_not_exceeded(&w.clone_segment_infos()?, max_seg_bytes)?;

    let pct = TestUtil::next_int(&mut random, 0, 4) + 12;
    let mut remaining_docs = num_docs - delete_pct_docs_from_each_seg(&mut w, pct, true)?;
    w.force_merge_deletes()?;
    w.commit()?;
    check_segment_size_not_exceeded(&w.clone_segment_infos()?, max_seg_bytes)?;
    assert!(!w.has_deletions()?);

    seg_names_before = get_segment_names(&w)?;
    let pct = TestUtil::next_int(&mut random, 0, 3) + 3;
    let deleted_this_pass = delete_pct_docs_from_each_seg(&mut w, pct, false)?;
    w.force_merge_deletes()?;
    remaining_docs -= deleted_this_pass;
    check_segments_in_expectations(&w, &seg_names_before, false)?;
    assert_eq!(remaining_docs, w.get_doc_stats()?.num_docs);
    assert!(w.get_doc_stats()?.num_docs < w.get_doc_stats()?.max_doc);

    w.force_merge(i32::MAX)?;
    check_segment_size_not_exceeded(&w.clone_segment_infos()?, max_seg_bytes)?;

    w.force_merge(1)?;
    assert_eq!(1, w.get_segment_count());
    assert_eq!(w.get_doc_stats()?.num_docs, w.get_doc_stats()?.max_doc);
    assert_eq!(remaining_docs, w.get_doc_stats()?.num_docs);

    seg_names_before = get_segment_names(&w)?;
    let pct = TestUtil::next_int(&mut random, 0, 4) + 1;
    remaining_docs -= delete_pct_docs_from_each_seg(&mut w, pct, false)?;
    w.force_merge_deletes()?;
    check_segments_in_expectations(&w, &seg_names_before, false)?;
    assert_eq!(1, w.get_segment_count());
    assert!(w.get_doc_stats()?.num_docs < w.get_doc_stats()?.max_doc);

    w.force_merge(1)?;

    let pct = TestUtil::next_int(&mut random, 0, 4) + 20;
    remaining_docs -= delete_pct_docs_from_each_seg(&mut w, pct, true)?;
    w.force_merge_deletes()?;

    assert_eq!(1, w.get_segment_count());
    assert_eq!(w.get_doc_stats()?.num_docs, w.get_doc_stats()?.max_doc);

    assert!(w.get_doc_stats()?.num_docs > 1_000);

    let pct = (w.get_doc_stats()?.num_docs * 60) / 100;
    let deleted_this_pass = delete_pct_docs_from_each_seg(&mut w, pct, true)?;
    remaining_docs -= deleted_this_pass;

    for i in 0..50 {
      let mut doc = Document::new();
      doc.add(new_field(
        &mut random,
        "id",
        (i + num_docs).to_string(),
        &FieldType::from_ref(&*string_field_type::TYPE_NOT_STORED)?,
        &mut field_to_type,
      )?);
      doc.add(new_text_field(
        &mut random,
        "content",
        format!("aaa {}", i),
        Store::No,
        &mut field_to_type,
      )?);
      w.add_document(doc)?;
    }

    w.commit()?;

    let infos = w.clone_segment_infos()?;
    assert_eq!(2, infos.size());

    let info0 = infos.info(0).unwrap();
    let info1 = infos.info(1).unwrap();
    let large_seg_doc_count = std::cmp::max(info0.info.max_doc()?, info1.info.max_doc()?);
    let small_seg_doc_count = std::cmp::min(info0.info.max_doc()?, info1.info.max_doc()?);

    assert_eq!(large_seg_doc_count, remaining_docs);
    assert_eq!(small_seg_doc_count, 50);

    w.close()?;
    Ok(())
  }

  fn post_merges_segment_count<D, CR>(
    starting_segment_count: i32,
    spec: &MergeSpecification<D, CR>,
  ) -> i32
  where
    D: Directory,
    CR: CodecReader,
  {
    let mut count = starting_segment_count;

    for merge in &spec.merges {
      count -= merge.stat.segments.len() as i32;
    }

    count += spec.merges.len() as i32;

    count
  }
  fn assert_max_merged_size<D, CR>(
    specification: &MergeSpecification<D, CR>,
    max_merged_segment_size_mb: f64,
    index_total_size_in_mb: f64,
    max_merged_segment_count: i32,
    infos: &SegmentInfos<D>,
  ) -> Result<()>
  where
    D: Directory,
    CR: CodecReader,
  {
    let max_mb_per_segment = index_total_size_in_mb / (max_merged_segment_count as f64);

    for merge in &specification.merges {
      let mut merge_total_size_in_bytes = 0i64;
      for segment_id in &merge.stat.segments {
        let segment = infos.index_of(segment_id).unwrap();
        merge_total_size_in_bytes += segment.size_in_bytes()?;
      }

      let limit_bytes =
        (1024.0 * 1024.0 * f64::max(max_mb_per_segment, max_merged_segment_size_mb) * 1.5) as i64;

      assert!(
        merge_total_size_in_bytes < limit_bytes,
        "mergeTotalSizeInBytes={} limitBytes={} maxMergedSegmentSizeMb={}",
        merge_total_size_in_bytes,
        limit_bytes,
        max_merged_segment_size_mb
      );
    }

    Ok(())
  }

  #[test]
  fn test_forced_merges_use_least_number_of_merges() -> Result<()> {
    let mut random = random();
    let fake_directory = Arc::new(FakeDirectory::new());

    let mut tmp = TieredMergePolicy::new();
    let mut one_segment_size_mb = 1.0_f64;
    let max_merged_segment_size_mb = 10.0 * one_segment_size_mb;
    tmp.set_max_merged_segment_mb(max_merged_segment_size_mb)?;

    if cfg!(feature = "test_log_verbose") {
      println!(
        "TEST: maxMergedSegmentSizeMB={:.2}",
        max_merged_segment_size_mb
      );
    }

    let mut infos = SegmentInfos::new(LATEST.major)?;
    let segment_count = 30;
    for j in 0..segment_count {
      infos.add(make_segment_commit_info(
        &mut random,
        fake_directory.clone(),
        &format!("_{}", j),
        1000,
        0,
        one_segment_size_mb,
        SOURCE_MERGE,
      )?)?;
    }

    let mut index_total_size_mb = (segment_count as f64) * one_segment_size_mb;

    let max_segment_count_after_force_merge = random.random_range(0..10) + 3;
    if cfg!(feature = "test_log_verbose") {
      println!(
        "TEST: maxSegmentCountAfterForceMerge={}",
        max_segment_count_after_force_merge
      );
    }

    let specification = match tmp.find_forced_merges(
      &infos,
      max_segment_count_after_force_merge as usize,
      &segments_to_merge(&infos),
      None,
      &MockMergeContext::new(|s| Ok(s.get_del_count())),
    )? {
      Some(spec) => spec,
      None => {
        return Err(LuceneError::illegal_state(
          "find_forced_merges returned None",
        ));
      },
    };

    assert_max_merged_size(
      &specification,
      max_merged_segment_size_mb,
      index_total_size_mb,
      max_segment_count_after_force_merge,
      &infos,
    )?;

    assert_eq!(
      max_segment_count_after_force_merge,
      post_merges_segment_count(infos.size() as i32, &specification)
    );

    infos = SegmentInfos::new(LATEST.major)?;
    let many_segments_count = at_least(&mut random, 100);
    if cfg!(feature = "test_log_verbose") {
      println!("TEST: manySegmentsCount={}", many_segments_count);
    }

    one_segment_size_mb = 0.1_f64;
    for j in 0..many_segments_count {
      infos.add(make_segment_commit_info(
        &mut random,
        fake_directory.clone(),
        &format!("_{}", j),
        1000,
        0,
        one_segment_size_mb,
        SOURCE_MERGE,
      )?)?;
    }

    index_total_size_mb = (many_segments_count as f64) * one_segment_size_mb;

    let specification = match tmp.find_forced_merges(
      &infos,
      max_segment_count_after_force_merge as usize,
      &segments_to_merge(&infos),
      None,
      &MockMergeContext::new(|s| Ok(s.get_del_count())),
    )? {
      Some(spec) => spec,
      None => {
        return Err(LuceneError::illegal_state(
          "find_forced_merges returned None",
        ));
      },
    };

    assert_max_merged_size(
      &specification,
      max_merged_segment_size_mb,
      index_total_size_mb,
      max_segment_count_after_force_merge,
      &infos,
    )?;

    assert!(
      post_merges_segment_count(infos.size() as i32, &specification)
        >= max_segment_count_after_force_merge
    );

    Ok(())
  }
  #[test]
  fn test_forced_merge_with_pending() -> Result<()> {
    let mut random = random();
    let fake_directory = Arc::new(FakeDirectory::new());

    let mut tmp = TieredMergePolicy::new();
    let max_segment_size = 10.0_f64;
    tmp.set_max_merged_segment_mb(max_segment_size)?;

    let mut infos = SegmentInfos::new(LATEST.major)?;
    for j in 0..30 {
      infos.add(make_segment_commit_info(
        &mut random,
        fake_directory.clone(),
        &format!("_{}", j),
        1000,
        0,
        1.0_f64,
        SOURCE_MERGE,
      )?)?;
    }

    let mut merge_context = MockMergeContext::new(|s| Ok(s.get_del_count()));
    let merging = infos.info(0).unwrap();
    merge_context.set_merging_segments(HashSet::from([merging.info.get_id_key().to_string()]));

    let expected_count = random.random_range(0..10) + 3;

    let specification = tmp.find_forced_merges(
      &infos,
      expected_count as usize,
      &segments_to_merge(&infos),
      None,
      &merge_context,
    )?;

    assert!(specification.is_none());

    Ok(())
  }
  fn segments_to_merge<D>(infos: &SegmentInfos<D>) -> HashMap<String, Option<bool>>
  where
    D: Directory,
  {
    let mut segments_to_merge = HashMap::new();
    for i in 0..infos.size() {
      let info = infos.info(i).unwrap();
      segments_to_merge.insert(info.info.get_id_key().to_string(), Some(true));
    }
    segments_to_merge
  }
  // Having a segment with very few documents in it can happen because of the random nature of the
  // docs added to the index. For instance, let's say it just happens that the last segment has 3
  // docs in it.
  // It can easily be merged with a close-to-max sized segment during a forceMerge and still respect
  // the max segment
  // size.
  //
  // If the above is possible, the "twoMayHaveBeenMerged" will be true and we allow for a little
  // slop, checking that
  // exactly two segments are gone from the old list and exactly one is in the new list. Otherwise,
  // the lists must match
  // exactly.
  //
  // So forceMerge may not be a no-op, allow for that. There are two possibilities in forceMerge
  // only:
  // > there were no small segments, in which case the two lists will be identical
  // > two segments in the original list are replaced by one segment in the final list.
  //
  // finally, there are some cases of forceMerge where the expectation is that there be exactly no
  // differences.
  // this should be called after forceDeletesMerges with the boolean always false,
  // Depending on the state, forceMerge may call with the boolean true or false.
  fn check_segments_in_expectations<D, L, B>(
    w: &IndexWriter<D, L, B>,
    seg_names_before: &[String],
    two_may_have_been_merged: bool,
  ) -> Result<()>
  where
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
  {
    let seg_names_after = get_segment_names(w)?;

    if !two_may_have_been_merged || seg_names_after.len() == seg_names_before.len() {
      if seg_names_after.len() != seg_names_before.len() {
        panic!(
          "Segment lists different sizes!: {:?} After list: {:?}",
          seg_names_before, seg_names_after
        );
      }

      let before_set: HashSet<_> = seg_names_before.iter().collect();
      let after_set: HashSet<_> = seg_names_after.iter().collect();
      if !after_set.is_superset(&before_set) {
        panic!(
          "Segment lists should be identical: {:?} After list: {:?}",
          seg_names_before, seg_names_after
        );
      }
      return Ok(());
    }

    if seg_names_after.len() != seg_names_before.len() - 1 {
      panic!(
        "forceMerge didn't merge a small and large segment into one segment as expected: {:?} After list: {:?}",
        seg_names_before, seg_names_after
      );
    }

    let before_set: HashSet<_> = seg_names_before.iter().cloned().collect();
    let after_set: HashSet<_> = seg_names_after.iter().cloned().collect();

    let test_before: Vec<_> = before_set.difference(&after_set).cloned().collect();
    let test_after: Vec<_> = after_set.difference(&before_set).cloned().collect();

    if test_before.len() != 2 || test_after.len() != 1 {
      panic!(
        "Expected two unique 'before' segments and one unique 'after' segment: {:?} After list: {:?}",
        seg_names_before, seg_names_after
      );
    }

    Ok(())
  }
  fn get_segment_names<D, L, B>(w: &IndexWriter<D, L, B>) -> Result<Vec<String>>
  where
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
  {
    let infos = w.clone_segment_infos()?;
    let mut names = Vec::with_capacity(infos.size());
    for i in 0..infos.size() {
      let info = infos.info(i).unwrap();
      names.push(info.info.name.clone());
    }
    Ok(names)
  }

  fn delete_pct_docs_from_each_seg<D, L, B>(
    w: &mut IndexWriter<D, L, B>,
    pct: i32,
    round_up: bool,
  ) -> Result<i32>
  where
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
  {
    let reader = directory_reader_util::open_from_writer(w)?;
    let reader = get_context(reader)?;
    let mut to_delete = Vec::new();
    for ctx in reader.leaves()? {
      to_delete.extend(get_rand_terms(ctx, pct, round_up)?);
    }

    w.delete_documents_with_terms(to_delete.clone())?;
    w.commit()?;
    Ok(to_delete.len() as i32)
  }

  fn get_rand_terms<LR>(ctx: &LeafReaderContext<LR>, pct: i32, round_up: bool) -> Result<Vec<Term>>
  where
    LR: LeafReader,
  {
    assert!(
      !ctx.reader().has_deletions()?,
      "This method assumes no deleted documents"
    );

    let mut ret = Vec::with_capacity(100);

    let num_docs = ctx.reader().num_docs()? as f64;
    let tmp = (num_docs * (pct as f64)) / 100.0;

    if tmp <= 1.0 {
      return Ok(ret);
    }

    let mod_ = (num_docs / tmp) as i32;
    if mod_ == 0 {
      return Ok(ret);
    }

    let terms = match ctx.reader().terms("id")? {
      Some(v) => v,
      None => return Ok(ret),
    };
    let mut iter = terms.iterator()?;
    let mut counter = 0i32;

    let mut lim = (num_docs * (pct as f64) / 100.0) as i32;
    if round_up {
      lim += 1;
    }

    while ret.len() < lim as usize {
      let br = iter.next()?;
      match br {
        Some(br) => {
          if (counter % mod_) == 0 {
            ret.push(Term::new("id", br.into_owned()));
          }
          counter += 1;
        },
        None => break,
      }
    }

    Ok(ret)
  }

  fn check_segment_size_not_exceeded<D>(infos: &SegmentInfos<D>, max_seg_bytes: i64) -> Result<()>
  where
    D: Directory,
  {
    for i in 0..infos.size() {
      let info = infos.info(i).unwrap();
      assert!(
        info.size_in_bytes()? <= max_seg_bytes,
        "Found an unexpectedly large segment: {}",
        info
      );
    }
    Ok(())
  }
  const EPSILON: f64 = 1e-14;
  #[test]
  fn test_setters() -> Result<()> {
    let mut tmp = TieredMergePolicy::new();

    tmp.set_max_merged_segment_mb(0.5)?;
    assert!((tmp.get_max_merged_segment_mb() - 0.5).abs() < EPSILON);

    tmp.set_max_merged_segment_mb(f64::INFINITY)?;
    assert!(
      (tmp.get_max_merged_segment_mb() - (i64::MAX as f64 / 1024.0 / 1024.0)).abs()
        < EPSILON * i64::MAX as f64
    );

    tmp.set_max_merged_segment_mb(i64::MAX as f64 / 1024.0 / 1024.0)?;
    assert!(
      (tmp.get_max_merged_segment_mb() - (i64::MAX as f64 / 1024.0 / 1024.0)).abs()
        < EPSILON * i64::MAX as f64
    );

    let err = tmp.set_max_merged_segment_mb(-2.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));

    tmp.set_floor_segment_mb(2.0)?;
    assert!((tmp.get_floor_segment_mb() - 2.0).abs() < EPSILON);

    tmp.set_floor_segment_mb(f64::INFINITY)?;
    assert!(
      (tmp.get_floor_segment_mb() - (i64::MAX as f64 / 1024.0 / 1024.0)).abs()
        < EPSILON * i64::MAX as f64
    );

    tmp.set_floor_segment_mb(i64::MAX as f64 / 1024.0 / 1024.0)?;
    assert!(
      (tmp.get_floor_segment_mb() - (i64::MAX as f64 / 1024.0 / 1024.0)).abs()
        < EPSILON * i64::MAX as f64
    );

    let err = tmp.set_floor_segment_mb(-2.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));

    tmp.get_base_mut().set_max_cfs_segment_size_mb(2.0)?;
    assert!((tmp.get_base().get_max_cfs_segment_size_mb() - 2.0).abs() < EPSILON);

    tmp
      .get_base_mut()
      .set_max_cfs_segment_size_mb(f64::INFINITY)?;
    assert!(
      (tmp.get_base().get_max_cfs_segment_size_mb() - (i64::MAX as f64 / 1024.0 / 1024.0)).abs()
        < EPSILON * i64::MAX as f64
    );

    tmp
      .get_base_mut()
      .set_max_cfs_segment_size_mb(i64::MAX as f64 / 1024.0 / 1024.0)?;
    assert!(
      (tmp.get_base().get_max_cfs_segment_size_mb() - (i64::MAX as f64 / 1024.0 / 1024.0)).abs()
        < EPSILON * i64::MAX as f64
    );

    let err = tmp.get_base_mut().set_max_cfs_segment_size_mb(-2.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));

    Ok(())
  }
  #[test]
  fn test_unbalanced_merge_selection() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

    let tmp = match iwc.get_merge_policy_mut() {
      MergePolicyEnum::Tiered(t) => t,
      _ => unreachable!(),
    };
    tmp.set_floor_segment_mb(0.00001)?;

    iwc.set_merge_scheduler(SerialMergeScheduler::new());
    iwc.set_max_buffered_docs(100);
    iwc.set_ram_buffer_size_mb(-1.0);

    let w = IndexWriter::new(dir.clone(), iwc)?;

    for _ in 0..15000 * random_multiplier() {
      let mut doc = Document::new();
      let mut id_bytes = vec![0u8; 128];
      random.fill(&mut id_bytes[..]);
      doc.add(StoredField::from_binary("id", id_bytes)?);
      w.add_document(doc)?;
    }

    let r = get_context(directory_reader_util::open_from_writer(&w)?)?;

    for ctx in r.leaves()? {
      let num_docs = ctx.reader().num_docs()?;
      assert!(
        num_docs == 100 || num_docs == 1000 || num_docs == 10000,
        "got numDocs={}",
        num_docs
      );
    }
    w.close()?;
    Ok(())
  }
  #[test]
  fn test_many_max_size_segments() -> Result<()> {
    let mut random = random();
    let fake_directory = Arc::new(FakeDirectory::new());

    let mut policy = TieredMergePolicy::new();
    policy.set_max_merged_segment_mb(1024.0)?;

    let mut infos = SegmentInfos::new(LATEST.major)?;
    let mut i = 0;

    for _ in 0..30 {
      infos.add(make_segment_commit_info(
        &mut random,
        fake_directory.clone(),
        &format!("_{}", i),
        1000,
        0,
        1024.0,
        SOURCE_MERGE,
      )?)?;
      i += 1;
    }

    for _ in 0..8 {
      infos.add(make_segment_commit_info(
        &mut random,
        fake_directory.clone(),
        &format!("_{}", i),
        1000,
        0,
        102.0,
        SOURCE_FLUSH,
      )?)?;
      i += 1;
    }

    let merge_spec = policy.find_merges(
      MergeTrigger::SegmentFlush,
      &infos,
      None,
      &MockMergeContext::new(|s| Ok(s.get_del_count())),
    )?;
    assert!(merge_spec.is_none());

    for _ in 0..5 {
      infos.add(make_segment_commit_info(
        &mut random,
        fake_directory.clone(),
        &format!("_{}", i),
        1000,
        0,
        102.0,
        SOURCE_FLUSH,
      )?)?;
      i += 1;
    }

    let merge_spec = policy.find_merges(
      MergeTrigger::SegmentFlush,
      &infos,
      None,
      &MockMergeContext::new(|s| Ok(s.get_del_count())),
    )?;
    assert!(merge_spec.is_some());

    let merge_spec = merge_spec.unwrap();
    assert_eq!(1, merge_spec.merges.len());

    let merge = &merge_spec.merges[0];
    assert_eq!(10, merge.stat.segments.len());

    Ok(())
  }
  #[test]
  fn test_merge_purely_to_reclaim_deletes() -> Result<()> {
    let mut random = random();
    let fake_directory = Arc::new(FakeDirectory::new());
    let case = TestTieredMergePolicy;

    let merge_policy = case.merge_policy(&mut random);
    let mut infos = SegmentInfos::new(LATEST.major)?;

    infos.add(make_segment_commit_info(
      &mut random,
      fake_directory.clone(),
      "_0",
      1_000_000,
      0,
      1024.0,
      SOURCE_MERGE,
    )?)?;

    let merge_spec = merge_policy.find_merges(
      MergeTrigger::Explicit,
      &infos,
      None,
      &MockMergeContext::new(|s| Ok(s.get_del_count())),
    )?;
    assert!(merge_spec.is_none());

    infos = apply_deletes(infos, (0.15_f64 * 1_000_000_f64) as i32)?;
    let merge_spec = merge_policy.find_merges(
      MergeTrigger::Explicit,
      &infos,
      None,
      &MockMergeContext::new(|s| Ok(s.get_del_count())),
    )?;
    assert!(merge_spec.is_none());

    infos = apply_deletes(
      infos,
      (((merge_policy.get_deletes_pct_allowed() - 15.0 + 1.0) / 100.0) * 1_000_000.0) as i32,
    )?;
    let merge_spec = merge_policy.find_merges(
      MergeTrigger::Explicit,
      &infos,
      None,
      &MockMergeContext::new(|s| Ok(s.get_del_count())),
    )?;
    assert!(merge_spec.is_some());

    Ok(())
  }
  #[test]
  fn test_merge_size_is_less_than_floor_size() -> Result<()> {
    let mut random = random();
    let fake_directory = Arc::new(FakeDirectory::new());

    let merge_context = MockMergeContext::new(|s| Ok(s.get_del_count()));

    let mut infos = SegmentInfos::new(LATEST.major)?;
    for i in 0..50 {
      infos.add(make_segment_commit_info(
        &mut random,
        fake_directory.clone(),
        &format!("_{}", i),
        1_000_000,
        0,
        1.0,
        SOURCE_FLUSH,
      )?)?;
    }

    let mut merge_policy = TieredMergePolicy::new();
    merge_policy.set_max_merge_at_once(30)?;
    merge_policy.set_floor_segment_mb(0.1)?;

    let mut merge_spec =
      merge_policy.find_merges(MergeTrigger::FullFlush, &infos, None, &merge_context)?;
    assert!(merge_spec.is_some());

    let merge_spec = merge_spec.take().unwrap();
    assert_eq!(4, merge_spec.merges.len());
    for one_merge in &merge_spec.merges {
      assert_eq!(
        merge_policy.get_segments_per_tier() as usize,
        one_merge.stat.segments.len()
      );
    }

    merge_policy.set_floor_segment_mb(15.0)?;
    let mut merge_spec =
      merge_policy.find_merges(MergeTrigger::FullFlush, &infos, None, &merge_context)?;
    assert!(merge_spec.is_some());

    let merge_spec = merge_spec.take().unwrap();
    assert_eq!(3, merge_spec.merges.len());
    for one_merge in &merge_spec.merges {
      assert_eq!(15, one_merge.stat.segments.len());
    }

    merge_policy.set_floor_segment_mb(60.0)?;
    let mut merge_spec =
      merge_policy.find_merges(MergeTrigger::FullFlush, &infos, None, &merge_context)?;
    assert!(merge_spec.is_some());

    let merge_spec = merge_spec.take().unwrap();
    assert_eq!(2, merge_spec.merges.len());
    assert_eq!(30, merge_spec.merges[0].stat.segments.len());
    assert_eq!(20, merge_spec.merges[1].stat.segments.len());

    Ok(())
  }

  #[test]
  fn test_full_flush_merges() -> Result<()> {
    let mut random = random();
    let fake_directory = Arc::new(FakeDirectory::new());

    let seg_name_generator = AtomicU64::new(0);
    let mut stats = IOStats::default();
    let merge_context = MockMergeContext::new(|s| Ok(s.get_del_count()));
    let mut segment_infos = SegmentInfos::new(LATEST.major)?;

    let mp = TieredMergePolicy::new();

    for _ in 0..11 {
      segment_infos.add(make_segment_commit_info(
        &mut random,
        fake_directory.clone(),
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        1,
        0,
        f64::MIN_POSITIVE,
        SOURCE_FLUSH,
      )?)?;
    }

    let spec = mp.find_full_flush_merges(
      MergeTrigger::FullFlush,
      &segment_infos,
      None,
      &merge_context,
    )?;
    assert!(spec.is_some());

    let spec = spec.unwrap();
    for merge in &spec.merges {
      segment_infos = apply_merge(
        &mut random,
        &segment_infos,
        merge,
        &format!("_{}", seg_name_generator.fetch_add(1, Ordering::SeqCst)),
        &mut stats,
        fake_directory.clone(),
      )?;
    }

    assert_eq!(2, segment_infos.size());

    Ok(())
  }

  // Super test
  #[test]
  fn test_force_merge_not_needed() -> Result<()> {
    let mut random = random();
    let case = TestTieredMergePolicy;
    case.test_force_merge_not_needed(&mut random)?;
    Ok(())
  }
  #[test]
  fn test_find_forced_deletes_merges() -> Result<()> {
    let mut random = random();
    let case = TestTieredMergePolicy;
    case.test_find_forced_deletes_merges(&mut random)?;
    Ok(())
  }
  #[test]
  fn test_simulate_append_only() -> Result<()> {
    let mut random = random();
    let case = TestTieredMergePolicy;
    let mut mp = case.merge_policy(&mut random);
    let fake_dir = Arc::new(FakeDirectory::new());
    let v = TestUtil::next_int(&mut random, 1024, 10 * 1024) as f64;
    mp.set_max_merged_segment_mb(v)?;
    case.do_test_simulate_append_only(&mut random, &mp, fake_dir, 100_000_000, 10_000)?;
    Ok(())
  }
  #[test]
  fn test_simulate_updates() -> Result<()> {
    let mut random = random();
    let case = TestTieredMergePolicy;
    let v = TestUtil::next_int(&mut random, 1024, 10 * 1024) as f64;
    let mut mp = case.merge_policy(&mut random);
    mp.set_max_merged_segment_mb(v)?;
    let fake_dir = Arc::new(FakeDirectory::new());
    let num_docs = if is_night_mode() {
      10_000_000
    } else {
      1_000_000
    };
    case.do_test_simulate_updates(&mut random, &mp, fake_dir, num_docs, 2500)?;
    Ok(())
  }
  #[test]
  fn test_no_pathological_merges() -> Result<()> {
    let mut random = random();
    let case = TestTieredMergePolicy;
    let mp = case.merge_policy(&mut random);
    let fake_dir = Arc::new(FakeDirectory::new());
    case.test_no_pathological_merges(&mut random, &mp, fake_dir)?;
    Ok(())
  }

  impl BaseMergePolicyTestCase for TestTieredMergePolicy {
    type MergePolicy = TieredMergePolicy;

    fn merge_policy<R>(&self, random: &mut R) -> Self::MergePolicy
    where
      R: Rng + ?Sized,
    {
      new_tiered_merge_policy(random)
    }

    fn assert_segment_infos<D>(
      tmp: &Self::MergePolicy,
      infos: &SegmentInfos<D>,
    ) -> crate::core::util::error::lucene_error::Result<()>
    where
      D: Directory,
    {
      let max_merged_segment_bytes = (tmp.get_max_merged_segment_mb() * 1024.0 * 1024.0) as i64;

      let mut min_segment_bytes = i64::MAX;
      let mut total_del_count = 0i32;
      let mut total_max_doc = 0i32;
      let mut total_bytes = 0i64;
      let mut segment_sizes = Vec::new();

      for i in 0..infos.size() {
        let sci = infos.info(i).unwrap();
        total_del_count += sci.get_del_count();
        total_max_doc += sci.info.max_doc()?;
        let byte_size = sci.size_in_bytes()?;
        let live_ratio = 1.0 - (sci.get_del_count() as f64) / (sci.info.max_doc()? as f64);
        let weighted_byte_size = (live_ratio * byte_size as f64) as i64;
        total_bytes += weighted_byte_size;
        segment_sizes.push(DocCountAndSizeInBytes {
          doc_count: sci.info.max_doc()? - sci.get_del_count(),
          size_in_bytes: weighted_byte_size,
        });
        min_segment_bytes = std::cmp::min(min_segment_bytes, weighted_byte_size);
      }

      segment_sizes.sort_by_key(|v| v.size_in_bytes);

      let del_percentage = 100.0 * (total_del_count as f64) / (total_max_doc as f64);
      assert!(
        del_percentage <= tmp.get_deletes_pct_allowed(),
        "Percentage of deleted docs {} is larger than the target: {}",
        del_percentage,
        tmp.get_deletes_pct_allowed()
      );

      let mut level_size_bytes = std::cmp::max(
        min_segment_bytes,
        (tmp.get_floor_segment_mb() * 1024.0 * 1024.0) as i64,
      );
      let mut bytes_left = total_bytes;
      let mut allowed_seg_count = 0.0_f64;

      let mut biggest_segments = &segment_sizes[..];
      if biggest_segments.len() as i32 > tmp.get_target_search_concurrency() - 1 {
        biggest_segments = &biggest_segments
          [(biggest_segments.len() as i32 - tmp.get_target_search_concurrency() + 1) as usize..];
      }

      for size in biggest_segments {
        bytes_left -= size.size_in_bytes;
        allowed_seg_count += 1.0;
      }

      let mut too_big_count = 0i32;
      for size in &segment_sizes {
        if size.size_in_bytes >= max_merged_segment_bytes / 2 {
          too_big_count += 1;
        }
      }

      let merge_factor = std::cmp::min(
        tmp.get_segments_per_tier() as i32,
        tmp.get_max_merge_at_once(),
      );
      loop {
        let seg_count_level = bytes_left as f64 / level_size_bytes as f64;
        if seg_count_level <= tmp.get_segments_per_tier()
          || level_size_bytes >= max_merged_segment_bytes / 2
        {
          allowed_seg_count += seg_count_level.ceil();
          break;
        }
        allowed_seg_count += tmp.get_segments_per_tier();
        bytes_left -= (tmp.get_segments_per_tier() as i64) * level_size_bytes;
        level_size_bytes = std::cmp::min(
          level_size_bytes * merge_factor as i64,
          max_merged_segment_bytes / 2,
        );
      }

      allowed_seg_count = allowed_seg_count.max(too_big_count as f64 + tmp.get_segments_per_tier());
      allowed_seg_count = allowed_seg_count.max(tmp.get_target_search_concurrency() as f64);

      let max_docs_per_segment = tmp.get_max_allowed_docs(infos.total_max_doc()?, total_del_count);
      let mut has_legal_merges = false;

      for i in 0..segment_sizes.len().saturating_sub(1) {
        let size1 = &segment_sizes[i];
        let size2 = &segment_sizes[i + 1];
        let merged_segment_size_in_bytes = size1.size_in_bytes + size2.size_in_bytes;
        let merged_segment_doc_count = size1.doc_count + size2.doc_count;

        if merged_segment_size_in_bytes <= max_merged_segment_bytes
          && (size2.size_in_bytes as f64) * 1.5 <= merged_segment_size_in_bytes as f64
          && merged_segment_doc_count <= max_docs_per_segment
        {
          has_legal_merges = true;
          break;
        }
      }

      let num_segments = infos.size();

      assert!(
        num_segments as f64 <= allowed_seg_count || !has_legal_merges,
        "mergeFactor={} minSegmentBytes={:?} maxMergedSegmentBytes={} segmentsPerTier={} maxMergeAtOnce={} numSegments={} allowed={} totalBytes={} delPercentage={} deletesPctAllowed={} targetNumSegments={}",
        merge_factor,
        min_segment_bytes,
        max_merged_segment_bytes,
        tmp.get_segments_per_tier(),
        tmp.get_max_merge_at_once(),
        num_segments,
        allowed_seg_count,
        total_bytes,
        del_percentage,
        tmp.get_deletes_pct_allowed(),
        tmp.get_target_search_concurrency(),
      );

      Ok(())
    }

    fn assert_merge<D, CR>(
      tmp: &Self::MergePolicy,
      merges: &MergeSpecification<D, CR>,
    ) -> crate::core::util::error::lucene_error::Result<()>
    where
      D: Directory,
      CR: CodecReader,
    {
      for merge in &merges.merges {
        assert!(merge.stat.segments.len() <= tmp.get_max_merge_at_once() as usize);
      }
      Ok(())
    }
  }
}
