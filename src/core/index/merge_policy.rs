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
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::dummy::dummy_doc_map_sorter::DummyDocMap;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_writer::Inner;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::log_byte_size_merge_policy::LogByteSizeMergePolicy;
use crate::core::index::log_doc_merge_policy::LogDocMergePolicy;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::segment_reader::SegmentReader;
use crate::core::index::sorter::DocMap;
use crate::core::index::tiered_merge_policy::{
  SegmentCommitInfoMeta, SegmentDocAndID, TieredMergePolicy,
};
use crate::core::store::directory::Directory;
use crate::core::store::merge_info::MergeInfo;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::info_stream::{InfoStream, InfoStreamMT};
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test::core::index::force_merge_policy::ForceMergePolicy;
use parking_lot::{Condvar, Mutex};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

/// Default ratio for compound file system usage.
/// Set to `1.0`, always use compound file system.
pub(crate) const DEFAULT_NO_CFS_RATIO: f64 = 1.0;
/// Default max segment size in order to use compound file system.
/// Set to `i64::MAX`.
pub(crate) const DEFAULT_MAX_CFS_SEGMENT_SIZE: i64 = i64::MAX;
/// Expert: a `MergePolicy` determines the sequence of primitive merge operations.
///
/// Whenever the segments in an index have been altered by [`IndexWriter`](crate::core::index::index_writer::IndexWriter), either by:
/// - the addition of a newly flushed segment,
/// - the addition of many segments from `addIndexes*` calls, or
/// - a previous merge that may now need to cascade,
///
/// [`IndexWriter`](crate::core::index::index_writer::IndexWriter) invokes [`Self::find_merges`] to give the `MergePolicy` a chance to
/// select merges that are now required.
///
/// This method returns a [`MergeSpecification`] describing the set of merges
/// that should be executed, or `None` if no merges are necessary.
///
/// When `IndexWriter::force_merge`(crate::core::index::index_writer::IndexWriter::force_merge) is called, it invokes
/// [`Self::find_forced_merges`] and the `MergePolicy` should then return the merges
/// required to satisfy that request.
///
/// Note that a policy may return more than one merge at a time.
/// - When using [`SerialMergeScheduler`](crate::core::index::serial_merge_scheduler::SerialMergeScheduler), these merges are run sequentially.
/// - When using `ConcurrentMergeScheduler`, they may run concurrently.
///
/// The default merge policy is [`TieredMergePolicy`].
pub trait MergePolicy: Display {
  fn get_base(&self) -> &MergePolicyBase;
  fn get_base_mut(&mut self) -> &mut MergePolicyBase;
  /// Determine what set of merge operations are now necessary on the index.
  /// [`IndexWriter`](crate::core::index::index_writer::IndexWriter) calls this whenever there is a change to the segments.
  /// This call is always synchronized on the [`IndexWriter`](crate::core::index::index_writer::IndexWriter) instance so only
  /// one thread at a time will call this method.
  ///
  /// * `merge_trigger` — the event that triggered the merge  
  /// * `segment_infos` — the total set of segments in the index  
  /// * `merge_context` — the `MergeContext` to find merges on
  fn find_merges<D, MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>;

  /// Define the set of merge operations to perform on provided codec readers in
  /// [`IndexWriter::add_indexes`].
  ///
  /// The merge operation is required to convert provided readers into segments
  /// that can be added to the writer. This API can be overridden in custom merge
  /// policies to control concurrency for `addIndexes`.
  ///
  /// Default implementation creates a single merge operation for all provided
  /// readers (lowest concurrency). Creating a merge for each reader would provide
  /// the highest level of concurrency possible with the configured merge scheduler.
  ///
  /// * `readers` — codec readers to merge into the main index
  fn find_merges_readers<CR, D>(
    &self,
    readers: Vec<CR>,
  ) -> Result<Option<MergeSpecification<D, CR>>>
  where
    CR: CodecReader,
    D: Directory,
  {
    let mut merge_spec = MergeSpecification::new();
    merge_spec.add(OneMerge::from_codec_readers(readers)?);
    Ok(Some(merge_spec))
  }

  ///   Determine what set of merge operations is necessary in order to merge to
  ///   `<=` the specified segment count. [`IndexWriter`](crate::core::index::index_writer::IndexWriter) calls this when its
  ///   `forceMerge` method is invoked. This call is always synchronized on the
  ///   [`IndexWriter`](crate::core::index::index_writer::IndexWriter) instance so only one thread at a time will call it.
  ///
  /// * `segment_infos` — the total set of segments in the index  
  /// * `max_segment_count` — requested maximum number of segments  
  /// * `segments_to_merge` — map of `SegmentCommitInfo` → boolean indicating
  ///   which segments must be merged away  
  /// * `merge_context` — the `MergeContext` to find merges on
  fn find_forced_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    max_segment_count: usize,
    segments_to_merge: &HashMap<String, Option<bool>>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>;

  /// Determine what set of merge operations is necessary in order to expunge all deletes
  /// from the index.
  ///
  /// * `segment_infos` — the total set of segments in the index  
  /// * `merge_context` — the `MergeContext` to find merges on
  fn find_forced_deletes_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    MC: MergeContext<D>,
    D: Directory;
  /// Identifies merges that we want to execute **synchronously** on commit.
  /// By default, this will return the same merges as `find_merges`
  /// (“natural merges”) whose segments are all less than the
  /// `max_full_flush_merge_size` (the max segment size for full flushes).
  ///
  /// Any merges returned here will make:
  /// - [`IndexWriter::commit`](crate::core::index::index_writer::IndexWriter::commit),
  /// - `IndexWriter::prepare_commit` or
  /// - `IndexWriter::get_reader`
  ///
  /// block until the merges complete, or until
  /// `IndexWriterConfig::get_max_full_flush_merge_wait_millis` has elapsed.
  ///
  /// This may be used to merge small segments that have just been flushed,
  /// reducing the number of segments in the point-in-time snapshot. If a merge
  /// does not complete in the allotted time, it will continue to execute and
  /// eventually finish and apply to future point-in-time snapshots, but it will
  /// **not** be reflected in the current one.
  ///
  /// If a [`OneMerge`] in the returned [`MergeSpecification`] includes a segment
  /// that is already included in a registered merge, then
  /// [`IndexWriter::commit`](crate::core::index::index_writer::IndexWriter::commit) or `IndexWriter::prepare_commit` will throw an
  /// error. Use [`MergeContext::get_merging_segments`] to determine which
  /// segments are currently registered to merge.
  ///
  /// # Parameters
  ///
  /// * `merge_trigger` — the event that triggered the merge (COMMIT or GET_READER)
  /// * `segment_infos` — the total set of segments in the index (while preparing the commit)
  /// * `merge_context` — the [`MergeContext`] to find merges on, which should be
  ///   used to determine which segments are already in a registered merge
  ///   (see [`MergeContext::get_merging_segments`])
  fn find_full_flush_merges<D, MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    // This returns natural merges that contain segments below the minimum size
    let merge_spec = self.find_merges(merge_trigger, segment_infos, inner, merge_context)?;

    match merge_spec {
      None => Ok(None),
      Some(merge_spec) => {
        let mut new_merge_spec = None;

        for one_merge in merge_spec.merges.into_iter() {
          let mut below_max_full_flush_size = true;

          for seg_id in &one_merge.stat.segments {
            match segment_infos.info(seg_id) {
              Some(sci) => {
                if self.size(sci, merge_context)? >= self.max_full_flush_merge_size() {
                  below_max_full_flush_size = false;
                  break;
                }
              },
              None => {
                return Err(LuceneError::illegal_state(
                  "could not find SegmentCommitInfo from segment_infos",
                ));
              },
            }
          }

          if below_max_full_flush_size {
            if new_merge_spec.is_none() {
              new_merge_spec = Some(MergeSpecificationNoReader::new());
            }
            if let Some(ref mut spec) = new_merge_spec {
              spec.add(one_merge);
            }
          }
        }

        Ok(new_merge_spec)
      },
    }
  }

  /// Returns `true` if a new segment (regardless of its origin) should use the
  /// compound file format.
  ///
  /// The default implementation returns `true` iff:
  ///
  /// - the size of the given `merged_info` is less than or equal to
  ///   [`MergePolicyBase::get_max_cfs_segment_size_mb`], **and**
  /// - the size is less than or equal to `total_index_size * get_no_cfs_ratio()`
  ///
  /// otherwise returns `false`.
  fn use_compound_file<D, MC>(
    &self,
    infos: &SegmentInfos<D>,
    merged_info: &SegmentCommitInfo<D>,
    merge_context: &MC,
  ) -> Result<bool>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    let (no_cfs_ratio, max_cfs_segment_size) = {
      let base = self.get_base();
      if base.get_no_cfs_ratio() == 0.0 {
        return Ok(false);
      }
      (base.get_no_cfs_ratio(), base.max_cfs_segment_size)
    };

    let merged_info_size = self.size(merged_info, merge_context)?;
    if merged_info_size > max_cfs_segment_size {
      return Ok(false);
    }

    if no_cfs_ratio >= 1.0 {
      return Ok(true);
    }
    let mut total_size = 0_i64;

    for sci in infos.iter() {
      total_size += self.size(sci, merge_context)?;
    }
    Ok((merged_info_size as f64) <= no_cfs_ratio * (total_size as f64))
  }

  /// Return the byte size of the provided [`SegmentCommitInfo`], prorated by the
  /// percentage of non-deleted documents that remain.
  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>;
  /// Return the maximum size of segments to be included in full-flush merges
  /// by the default implementation of `find_full_flush_merges`.
  fn max_full_flush_merge_size(&self) -> i64 {
    0
  }

  /// Returns `true` if this single info is already fully merged (has no pending
  /// deletes, is in the same directory as the writer, and matches the current
  /// compound file setting).
  fn has_merged<D, MC>(
    &self,
    infos: &SegmentInfos<D>,
    info: &SegmentCommitInfo<D>,
    merge_context: &MC,
  ) -> Result<bool>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    let del_count = merge_context.num_deletes_to_merge(info)?;
    debug_assert!(assert_del_count(del_count, info)?);

    Ok(
      del_count == 0
        && self.use_compound_file(infos, info, merge_context)? == info.info.get_use_compound_file(),
    )
  }
  /// Returns `true` if the segment represented by the given `CodecReader`
  /// should be kept even if it is fully deleted.
  ///
  /// This is useful for testing, or for merge policies that implement
  /// retention rules for soft deletes.
  fn keep_fully_deleted_segment<D, F>(&self, _reader_supplier: F) -> Result<bool>
  where
    D: Directory,
    F: Fn() -> Result<Arc<SegmentReader<D>>>,
  {
    Ok(false)
  }

  /// Returns the number of deletes that a merge would claim on the given segment.
  ///
  /// By default, this returns the sum of:
  /// - the number of deletes on disk, and
  /// - the number of pending deletes.
  ///
  /// Subclasses that wrap merge readers may override this in order to reflect
  /// deletes that are carried over into the target segment in the case of soft deletes.
  ///
  /// Soft deletes allow deleted documents to survive across merges so that the
  /// application controls when soft-deleted data is truly removed.
  ///
  /// * `info` — the segment being merged
  /// * `del_count` — the current delete count for this segment
  /// * `reader_supplier` — a supplier for obtaining a [`CodecReader`] of this segment
  fn num_deletes_to_merge<D, F>(
    &self,
    _info: &SegmentCommitInfo<D>,
    del_count: i32,
    _reader_supplier: F,
  ) -> Result<i32>
  where
    D: Directory,
    F: Fn() -> Result<Arc<SegmentReader<D>>>,
  {
    Ok(del_count)
  }

  /// Builds a string representation of the given [`SegmentCommitInfo`] instances.
  fn seg_string<MC, D>(&self, merge_context: &MC, infos: &[SegmentCommitInfo<D>]) -> String
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    infos
      .iter()
      .map(|info| {
        let del = merge_context.num_deleted_docs(info) - info.get_del_count();
        info.to_string_with_pending_del_count(del)
      })
      .collect::<Vec<_>>()
      .join(" ")
  }

  /// Print a debug message to the [`MergeContext`]’s `infoStream`.
  fn message<MC, D>(&self, message: &str, merge_context: &MC)
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    if self.verbose(merge_context) {
      merge_context.get_info_stream().message("MP", message)
    }
  }

  /// Returns `true` if the info-stream is in verbose mode.
  ///
  /// See `message`.
  fn verbose<MC, D>(&self, merge_context: &MC) -> bool
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    merge_context.get_info_stream().enabled("MP")
  }
}
/// Asserts that the `delCount` for this [`SegmentCommitInfo`] is valid.
pub(crate) fn assert_del_count<D>(del_count: i32, info: &SegmentCommitInfo<D>) -> Result<bool>
where
  D: Directory,
{
  debug_assert!(del_count >= 0, "delCount must be positive: {}", del_count);
  debug_assert!(
    del_count <= info.info.max_doc()?,
    "delCount: {} must be ≤ maxDoc: {}",
    del_count,
    info.info.max_doc()?
  );
  Ok(true)
}
pub(crate) fn size<D, MC>(info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
where
  D: Directory,
  MC: MergeContext<D>,
{
  let byte_size = info.size_in_bytes()?;
  let del_count = merge_context.num_deletes_to_merge(info)?;
  debug_assert!(assert_del_count(del_count, info)?);
  let max_doc = info.info.max_doc()?;
  let del_ratio = if max_doc <= 0 {
    0.0
  } else {
    del_count as f64 / max_doc as f64
  };

  debug_assert!(del_ratio <= 1.0);

  if max_doc <= 0 {
    Ok(byte_size)
  } else {
    Ok((byte_size as f64 * (1.0 - del_ratio)) as i64)
  }
}
#[derive(Clone)]
pub enum MergePolicyEnum {
  No(NoMergePolicy),
  Tiered(TieredMergePolicy),
  LogDoc(LogMergePolicy<LogDocMergePolicy>),
  LogBytesSize(LogMergePolicy<LogByteSizeMergePolicy>),
  #[cfg(test)]
  Force(ForceMergePolicy<MergePolicyEnum>),
}
impl_from_for_enum!(
    MergePolicyEnum,
    NoMergePolicy => No,
    TieredMergePolicy => Tiered,
    LogMergePolicy<LogDocMergePolicy> => LogDoc,
    LogMergePolicy<LogByteSizeMergePolicy> => LogBytesSize
);
impl Display for MergePolicyEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      MergePolicyEnum::No(mp) => write!(f, "{}", mp),
      MergePolicyEnum::Tiered(mp) => write!(f, "{}", mp),
      MergePolicyEnum::LogDoc(mp) => write!(f, "{}", mp),
      MergePolicyEnum::LogBytesSize(mp) => write!(f, "{}", mp),
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => write!(f, "{}", mp),
    }
  }
}

impl MergePolicy for MergePolicyEnum {
  fn get_base(&self) -> &MergePolicyBase {
    match self {
      MergePolicyEnum::No(mp) => mp.get_base(),
      MergePolicyEnum::Tiered(mp) => mp.get_base(),
      MergePolicyEnum::LogDoc(mp) => mp.get_base(),
      MergePolicyEnum::LogBytesSize(mp) => mp.get_base(),
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => mp.get_base(),
    }
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    match self {
      MergePolicyEnum::No(mp) => mp.get_base_mut(),
      MergePolicyEnum::Tiered(mp) => mp.get_base_mut(),
      MergePolicyEnum::LogDoc(mp) => mp.get_base_mut(),
      MergePolicyEnum::LogBytesSize(mp) => mp.get_base_mut(),
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => mp.get_base_mut(),
    }
  }

  fn find_merges<D, MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    match self {
      MergePolicyEnum::No(mp) => mp.find_merges(merge_trigger, segment_infos, inner, merge_context),
      MergePolicyEnum::Tiered(mp) => {
        mp.find_merges(merge_trigger, segment_infos, inner, merge_context)
      },
      MergePolicyEnum::LogDoc(mp) => {
        mp.find_merges(merge_trigger, segment_infos, inner, merge_context)
      },
      MergePolicyEnum::LogBytesSize(mp) => {
        mp.find_merges(merge_trigger, segment_infos, inner, merge_context)
      },
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => {
        mp.find_merges(merge_trigger, segment_infos, inner, merge_context)
      },
    }
  }

  fn find_forced_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    max_segment_count: usize,
    segments_to_merge: &HashMap<String, Option<bool>>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    match self {
      MergePolicyEnum::No(mp) => mp.find_forced_merges(
        segment_infos,
        max_segment_count,
        segments_to_merge,
        inner,
        merge_context,
      ),
      MergePolicyEnum::Tiered(mp) => mp.find_forced_merges(
        segment_infos,
        max_segment_count,
        segments_to_merge,
        inner,
        merge_context,
      ),
      MergePolicyEnum::LogDoc(mp) => mp.find_forced_merges(
        segment_infos,
        max_segment_count,
        segments_to_merge,
        inner,
        merge_context,
      ),
      MergePolicyEnum::LogBytesSize(mp) => mp.find_forced_merges(
        segment_infos,
        max_segment_count,
        segments_to_merge,
        inner,
        merge_context,
      ),
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => mp.find_forced_merges(
        segment_infos,
        max_segment_count,
        segments_to_merge,
        inner,
        merge_context,
      ),
    }
  }

  fn find_forced_deletes_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    match self {
      MergePolicyEnum::No(mp) => mp.find_forced_deletes_merges(segment_infos, inner, merge_context),
      MergePolicyEnum::Tiered(mp) => {
        mp.find_forced_deletes_merges(segment_infos, inner, merge_context)
      },
      MergePolicyEnum::LogDoc(mp) => {
        mp.find_forced_deletes_merges(segment_infos, inner, merge_context)
      },
      MergePolicyEnum::LogBytesSize(mp) => {
        mp.find_forced_deletes_merges(segment_infos, inner, merge_context)
      },
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => {
        mp.find_forced_deletes_merges(segment_infos, inner, merge_context)
      },
    }
  }

  fn find_full_flush_merges<D, MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    match self {
      MergePolicyEnum::No(mp) => {
        mp.find_full_flush_merges(merge_trigger, segment_infos, inner, merge_context)
      },
      MergePolicyEnum::Tiered(mp) => {
        mp.find_full_flush_merges(merge_trigger, segment_infos, inner, merge_context)
      },
      MergePolicyEnum::LogDoc(mp) => {
        mp.find_full_flush_merges(merge_trigger, segment_infos, inner, merge_context)
      },
      MergePolicyEnum::LogBytesSize(mp) => {
        mp.find_full_flush_merges(merge_trigger, segment_infos, inner, merge_context)
      },
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => {
        mp.find_full_flush_merges(merge_trigger, segment_infos, inner, merge_context)
      },
    }
  }

  fn use_compound_file<D, MC>(
    &self,
    infos: &SegmentInfos<D>,
    merged_info: &SegmentCommitInfo<D>,
    merge_context: &MC,
  ) -> Result<bool>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    match self {
      MergePolicyEnum::No(mp) => mp.use_compound_file(infos, merged_info, merge_context),
      MergePolicyEnum::Tiered(mp) => mp.use_compound_file(infos, merged_info, merge_context),
      MergePolicyEnum::LogDoc(mp) => mp.use_compound_file(infos, merged_info, merge_context),
      MergePolicyEnum::LogBytesSize(mp) => mp.use_compound_file(infos, merged_info, merge_context),
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => mp.use_compound_file(infos, merged_info, merge_context),
    }
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    match self {
      MergePolicyEnum::No(mp) => mp.size(info, merge_context),
      MergePolicyEnum::Tiered(mp) => mp.size(info, merge_context),
      MergePolicyEnum::LogDoc(mp) => mp.size(info, merge_context),
      MergePolicyEnum::LogBytesSize(mp) => mp.size(info, merge_context),
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => mp.size(info, merge_context),
    }
  }

  fn max_full_flush_merge_size(&self) -> i64 {
    match self {
      MergePolicyEnum::No(mp) => mp.max_full_flush_merge_size(),
      MergePolicyEnum::Tiered(mp) => mp.max_full_flush_merge_size(),
      MergePolicyEnum::LogDoc(mp) => mp.max_full_flush_merge_size(),
      MergePolicyEnum::LogBytesSize(mp) => mp.max_full_flush_merge_size(),
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => mp.max_full_flush_merge_size(),
    }
  }

  fn has_merged<D, MC>(
    &self,
    infos: &SegmentInfos<D>,
    info: &SegmentCommitInfo<D>,
    merge_context: &MC,
  ) -> Result<bool>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    match self {
      MergePolicyEnum::No(mp) => mp.has_merged(infos, info, merge_context),
      MergePolicyEnum::Tiered(mp) => mp.has_merged(infos, info, merge_context),
      MergePolicyEnum::LogDoc(mp) => mp.has_merged(infos, info, merge_context),
      MergePolicyEnum::LogBytesSize(mp) => mp.has_merged(infos, info, merge_context),
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => mp.has_merged(infos, info, merge_context),
    }
  }

  fn keep_fully_deleted_segment<D, F>(&self, reader_supplier: F) -> Result<bool>
  where
    D: Directory,
    F: Fn() -> Result<Arc<SegmentReader<D>>>,
  {
    match self {
      MergePolicyEnum::No(mp) => mp.keep_fully_deleted_segment(reader_supplier),
      MergePolicyEnum::Tiered(mp) => mp.keep_fully_deleted_segment(reader_supplier),
      MergePolicyEnum::LogDoc(mp) => mp.keep_fully_deleted_segment(reader_supplier),
      MergePolicyEnum::LogBytesSize(mp) => mp.keep_fully_deleted_segment(reader_supplier),
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => mp.keep_fully_deleted_segment(reader_supplier),
    }
  }

  fn num_deletes_to_merge<D, F>(
    &self,
    info: &SegmentCommitInfo<D>,
    del_count: i32,
    reader_supplier: F,
  ) -> Result<i32>
  where
    D: Directory,
    F: Fn() -> Result<Arc<SegmentReader<D>>>,
  {
    match self {
      MergePolicyEnum::No(mp) => mp.num_deletes_to_merge(info, del_count, reader_supplier),
      MergePolicyEnum::Tiered(mp) => mp.num_deletes_to_merge(info, del_count, reader_supplier),
      MergePolicyEnum::LogDoc(mp) => mp.num_deletes_to_merge(info, del_count, reader_supplier),
      MergePolicyEnum::LogBytesSize(mp) => {
        mp.num_deletes_to_merge(info, del_count, reader_supplier)
      },
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => mp.num_deletes_to_merge(info, del_count, reader_supplier),
    }
  }

  fn seg_string<MC, D>(&self, merge_context: &MC, infos: &[SegmentCommitInfo<D>]) -> String
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    match self {
      MergePolicyEnum::No(mp) => mp.seg_string(merge_context, infos),
      MergePolicyEnum::Tiered(mp) => mp.seg_string(merge_context, infos),
      MergePolicyEnum::LogDoc(mp) => mp.seg_string(merge_context, infos),
      MergePolicyEnum::LogBytesSize(mp) => mp.seg_string(merge_context, infos),
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => mp.seg_string(merge_context, infos),
    }
  }

  fn message<MC, D>(&self, message: &str, merge_context: &MC)
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    match self {
      MergePolicyEnum::No(mp) => mp.message(message, merge_context),
      MergePolicyEnum::Tiered(mp) => mp.message(message, merge_context),
      MergePolicyEnum::LogDoc(mp) => mp.message(message, merge_context),
      MergePolicyEnum::LogBytesSize(mp) => mp.message(message, merge_context),
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => mp.message(message, merge_context),
    }
  }

  fn verbose<MC, D>(&self, merge_context: &MC) -> bool
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    match self {
      MergePolicyEnum::No(mp) => mp.verbose(merge_context),
      MergePolicyEnum::Tiered(mp) => mp.verbose(merge_context),
      MergePolicyEnum::LogDoc(mp) => mp.verbose(merge_context),
      MergePolicyEnum::LogBytesSize(mp) => mp.verbose(merge_context),
      #[cfg(test)]
      MergePolicyEnum::Force(mp) => mp.verbose(merge_context),
    }
  }
}
#[derive(Clone)]
pub struct MergePolicyBase {
  /// If the size of the merge segment exceeds this ratio of the total index size
  /// then it will remain in non-compound format.
  pub(crate) no_cfs_ratio: f64,
  /// If the size of the merged segment exceeds this value
  /// then it will not use compound file format.
  pub(crate) max_cfs_segment_size: i64,
}
impl Default for MergePolicyBase {
  fn default() -> Self {
    Self {
      no_cfs_ratio: DEFAULT_NO_CFS_RATIO,
      max_cfs_segment_size: DEFAULT_MAX_CFS_SEGMENT_SIZE,
    }
  }
}
impl MergePolicyBase {
  pub fn new(no_cfs_ratio: f64, max_cfs_segment_size: i64) -> Self {
    Self {
      no_cfs_ratio,
      max_cfs_segment_size,
    }
  }
  /// Returns current `noCFSRatio`.
  ///
  /// See `set_no_cfs_ratio`.
  pub fn get_no_cfs_ratio(&self) -> f64 {
    self.no_cfs_ratio
  }

  /// If a merged segment will be more than this percentage of the total size of the index,
  /// leave the segment as non-compound file even if compound file is enabled.
  ///
  /// Set to `1.0` to always use CFS regardless of merge size.
  pub fn set_no_cfs_ratio(&mut self, ratio: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&ratio) {
      return Err(LuceneError::illegal_argument(format!(
        "noCFSRatio must be 0.0 to 1.0 inclusive; got {}",
        ratio
      )));
    }
    self.no_cfs_ratio = ratio;
    Ok(())
  }

  /// Returns the largest size allowed for a compound file segment (in MB).
  pub fn get_max_cfs_segment_size_mb(&self) -> f64 {
    self.max_cfs_segment_size as f64 / 1024.0 / 1024.0
  }

  /// If a merged segment will be more than this value (MB), leave the segment as non-compound
  /// even if compound file is enabled.
  ///
  /// Set this to `f64::INFINITY` and `noCFSRatio` to `1.0` to always use CFS regardless of size.
  pub fn set_max_cfs_segment_size_mb(&mut self, mut v: f64) -> Result<()> {
    if v < 0.0 {
      return Err(LuceneError::illegal_argument(format!(
        "maxCFSSegmentSizeMB must be >=0 (got {})",
        v
      )));
    }
    v *= 1024.0 * 1024.0;

    self.max_cfs_segment_size = if v > i64::MAX as f64 {
      i64::MAX
    } else {
      v as i64
    };

    Ok(())
  }
}
pub type OneMergeSR<D> = OneMerge<D, Arc<SegmentReader<D>>>;
/// OneMerge provides the information necessary to perform an individual
/// primitive merge operation, resulting in a single new segment.
///
/// The merge spec includes:
/// - the subset of segments to be merged
/// - whether the new segment should use the compound file format
pub struct OneMerge<D, CR>
where
  D: Directory,
  CR: CodecReader,
{
  pub(crate) register_done: bool,
  pub(crate) is_external: bool,
  pub(crate) uses_pooled_readers: bool,
  /// Estimated size in bytes of the merged segment.
  pub estimated_merge_bytes: AtomicI64,
  /// Sum of sizeInBytes of all SegmentInfos; set by IW.mergeInit
  pub(crate) total_merge_bytes: AtomicI64,
  merge_readers: Vec<MergeReader<CR, CR::Bits>>,
  /// Control used to pause/stop/resume the merge thread.
  merge_progress: OneMergeProgress,
  pub(crate) merge_start_ns: Instant,
  /// Total number of documents in segments to be merged, not accounting for deletions.
  pub(crate) total_max_doc: i32,
  error: Mutex<Option<LuceneError>>,
  pub(crate) stat: MergeStat,
  pub(crate) info: Option<SegmentCommitInfo<D>>,
  pub(crate) merge_completed: OnceLock<bool>,
}
#[derive(Clone)]
pub struct MergeStat {
  pub(crate) id: Identity,
  pub(crate) max_num_segments: i32,
  pub(crate) info_id: Option<String>,
  /// Segments to be merged.
  /// SegmentInfo#name and SegmentInfo#Id
  pub(crate) segments: Vec<String>,
  /// SegmentInfo#name
  pub(crate) name: Option<String>,
  pub(crate) merge_gen: i64,
}
impl PartialEq for MergeStat {
  fn eq(&self, other: &Self) -> bool {
    self.id.eq(&other.id)
  }
}
impl Eq for MergeStat {}

impl Hash for MergeStat {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.id.hash(state);
  }
}

impl<D, CR> OneMerge<D, CR>
where
  D: Directory,
  CR: CodecReader,
{
  pub fn new(segments: Vec<SegmentDocAndID>) -> Result<Self> {
    if segments.is_empty() {
      return Err(LuceneError::illegal_state(
        "segments must include at least one segment",
      ));
    }
    let mut v = Vec::with_capacity(segments.len());
    let mut total_max_doc = 0;
    for s in segments.into_iter() {
      v.push(s.seg_id);
      total_max_doc += s.max_doc
    }

    Ok(Self {
      register_done: false,
      is_external: false,
      uses_pooled_readers: true,
      estimated_merge_bytes: AtomicI64::new(0),
      total_merge_bytes: AtomicI64::new(0),
      merge_readers: Vec::new(),
      merge_progress: OneMergeProgress::new(),
      merge_start_ns: Instant::now(),
      total_max_doc,
      error: Mutex::new(None),
      stat: MergeStat {
        id: Identity::new(),
        max_num_segments: -1,
        info_id: None,
        segments: v,
        name: None,
        merge_gen: 0,
      },
      info: None,
      merge_completed: OnceLock::new(),
    })
  }
  pub fn from_meta(segments: &[SegmentCommitInfoMeta]) -> Result<Self> {
    let mut segments_meta = Vec::with_capacity(segments.len());
    for v in segments {
      segments_meta.push(SegmentDocAndID::new(v.seg_id.clone(), v.max_doc))
    }
    Self::new(segments_meta)
  }
  /// Constructor for wrapping.
  pub(crate) fn from_other(one_merge: OneMerge<D, CR>) -> Self {
    let mut one_merge = Self {
      merge_readers: one_merge.merge_readers,
      total_max_doc: one_merge.total_max_doc,
      merge_progress: OneMergeProgress::new(),
      uses_pooled_readers: one_merge.uses_pooled_readers,
      register_done: false,
      is_external: false,
      estimated_merge_bytes: AtomicI64::new(0),
      total_merge_bytes: AtomicI64::new(0),
      merge_start_ns: Instant::now(),
      error: Mutex::new(None),
      stat: one_merge.stat,
      info: one_merge.info,
      merge_completed: OnceLock::new(),
    };
    one_merge.stat.max_num_segments = -1;
    one_merge.stat.info_id = None;
    one_merge
  }
  /// Create a OneMerge directly from CodecReaders. Used to merge incoming readers in
  /// IndexWriter::add_indexes(reader...). This OneMerge works directly on readers and has an
  /// empty segments list.
  pub fn from_codec_readers(readers: Vec<CR>) -> Result<Self> {
    let mut merge_readers = Vec::with_capacity(readers.len());
    let mut total_docs = 0;

    for r in readers.into_iter() {
      let live_docs = r.get_live_docs()?;
      total_docs += r.num_docs()?;
      merge_readers.push(MergeReader::new(r, live_docs));
    }

    Ok(Self {
      register_done: false,
      is_external: false,
      uses_pooled_readers: false,
      estimated_merge_bytes: AtomicI64::new(0),
      total_merge_bytes: AtomicI64::new(0),
      merge_readers,
      merge_progress: OneMergeProgress::new(),
      merge_start_ns: Instant::now(),
      total_max_doc: total_docs,
      error: Mutex::new(None),
      stat: MergeStat {
        id: Identity::new(),
        max_num_segments: -1,
        info_id: None,
        segments: Vec::new(),
        name: None,
        merge_gen: 0,
      },
      info: None,
      merge_completed: OnceLock::new(),
    })
  }
  /// Called by IndexWriter after the merge started and from the thread that will be executing the merge.
  pub fn merge_init(&self) {
    self.merge_progress.set_merge_thread()
  }
  /// Record that an exception occurred while executing this merge.
  pub fn set_exception(&self, error: LuceneError) {
    let mut guard = self.error.lock();
    *guard = Some(error);
  }

  /// Retrieve previous exception set by `set_exception`.
  pub fn get_exception(&self) -> Option<LuceneError> {
    let mut guard = self.error.lock();
    guard.take()
  }
  /// Returns a readable description of the current merge state.
  pub fn seg_string(&self, segments: &SegmentInfos<D>) -> Result<String> {
    let mut s = String::new();

    for (i, seg) in self.stat.segments.iter().enumerate() {
      if i > 0 {
        s.push(' ');
      }
      let v = segments.info(seg).ok_or_else(|| {
        LuceneError::illegal_state("merge's segment could find from IndexWriter's SegmentInfos")
      })?;
      s.push_str(&v.to_string_with_pending_del_count(0));
    }

    if let Some(info_id) = &self.stat.info_id {
      s.push_str(" into ");
      let v = segments.info(info_id).ok_or_else(|| {
        LuceneError::illegal_state("merge's segment could find from IndexWriter's SegmentInfos")
      })?;
      s.push_str(&v.info.name);
    }

    if self.stat.max_num_segments != -1 {
      s.push_str(" [maxNumSegments=");
      s.push_str(&self.stat.max_num_segments.to_string());
      s.push(']');
    }

    if self.is_aborted() {
      s.push_str(" [ABORTED]");
    }

    Ok(s)
  }
  pub fn get_store_merge_info(&self) -> MergeInfo {
    MergeInfo::new(
      self.total_max_doc,
      self.estimated_merge_bytes.load(Relaxed),
      self.is_external,
      self.stat.max_num_segments,
    )
  }
  pub fn set_aborted(&mut self) -> Result<()> {
    Ok(())
  }
  pub fn is_aborted(&self) -> bool {
    // TODO
    false
  }
  pub fn check_aborted(&self, segments: &SegmentInfos<D>) -> Result<()> {
    if self.is_aborted() {
      return Err(LuceneError::merge_abort(format!(
        "merge is aborted: {}",
        self.seg_string(segments)?
      )));
    }
    Ok(())
  }
  pub fn get_merge_reader(&self) -> &[MergeReader<CR, CR::Bits>] {
    &self.merge_readers
  }

  pub(crate) fn has_finished(&self) -> bool {
    self.merge_completed.get().copied().unwrap_or(false)
  }
}
impl<D> OneMerge<D, Arc<SegmentReader<D>>>
where
  D: Directory,
{
  pub(crate) fn close<F>(
    &mut self,
    success: bool,
    segment_dropped: bool,
    reader_consumer: F,
  ) -> Result<()>
  where
    F: FnOnce(&[MergeReaderSR<D>]) -> Result<()>,
  {
    if self.merge_completed.set(true).is_err() {
      return Err(LuceneError::illegal_state("merge has already finished"));
    }
    let result = (|| -> Result<()> {
      self.merge_finished(success, segment_dropped)?;
      Ok(())
    })();
    let merge_readers = std::mem::take(&mut self.merge_readers);
    reader_consumer(merge_readers.as_ref())?;
    result
  }
}
impl<D> OneMergeBase<D, Arc<SegmentReader<D>>> for OneMerge<D, Arc<SegmentReader<D>>>
where
  D: Directory,
{
  type CodecReader = Arc<SegmentReader<D>>;

  fn wrap_for_merge(&self, reader: Arc<SegmentReader<D>>) -> Result<Self::CodecReader> {
    Ok(reader.clone())
  }

  type DocMap = DummyDocMap;

  fn set_merge_info(&mut self, info: SegmentCommitInfo<D>) {
    self.stat.info_id = Some(info.info.get_id_str());
    self.stat.name = Some(info.info.name.clone());
    self.info = Some(info);
  }

  type MergeCodecReader = Arc<SegmentReader<D>>;
  type Bits = <Arc<SegmentReader<D>> as LeafReader>::Bits;

  fn init_merge_readers<F>(&mut self, reader_factory: F) -> Result<()>
  where
    F: Fn(&String) -> Result<MergeReader<Self::MergeCodecReader, Self::Bits>>,
  {
    debug_assert!(self.merge_readers.is_empty());
    // TODO merge_completed未实现
    let mut readers = Vec::with_capacity(self.stat.segments.len());
    let result: Result<_> = (|| {
      for seg_id in self.stat.segments.iter() {
        readers.push(reader_factory(seg_id)?);
      }
      Ok(())
    })();
    self.merge_readers = readers;
    result
  }
}

pub trait OneMergeBase<D, CR>
where
  D: Directory,
  CR: CodecReader,
{
  fn merge_finished(&self, _success: bool, _segment_dropped: bool) -> Result<()> {
    Ok(())
  }
  type CodecReader: CodecReader;
  fn wrap_for_merge(&self, _reader: CR) -> Result<Self::CodecReader>;
  // TODO IMPORTANT 多线程参数未定义
  type DocMap: DocMap + Clone;
  fn reorder<CR1, D1>(&self, _reader: &CR1, _dir: D1) -> Result<Option<Self::DocMap>>
  where
    CR1: CodecReader,
    D1: Directory,
  {
    Ok(None)
  }
  fn set_merge_info(&mut self, info: SegmentCommitInfo<D>);
  fn on_merge_complete(&self) -> Result<()> {
    Ok(())
  }
  type MergeCodecReader: CodecReader;
  type Bits: Bits;
  fn init_merge_readers<F>(&mut self, reader_factory: F) -> Result<()>
  where
    F: Fn(&String) -> Result<MergeReader<Self::MergeCodecReader, Self::Bits>>;
  fn close(&mut self) -> Result<()> {
    todo!()
  }
}
pub type MergeSpecificationNoReader<D> = MergeSpecification<D, Arc<SegmentReader<D>>>;
pub struct MergeSpecification<D, CR>
where
  D: Directory,
  CR: CodecReader,
{
  /// The subset of segments to be included in the primitive merge.
  pub(crate) merges: Vec<OneMerge<D, CR>>,
}
impl<D, CR> Default for MergeSpecification<D, CR>
where
  D: Directory,
  CR: CodecReader,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<D, CR> MergeSpecification<D, CR>
where
  D: Directory,
  CR: CodecReader,
{
  pub fn new() -> Self {
    Self { merges: Vec::new() }
  }
}
impl<D, CR> MergeSpecification<D, CR>
where
  D: Directory,
  CR: CodecReader,
{
  pub fn add(&mut self, merge: OneMerge<D, CR>) {
    self.merges.push(merge);
  }
}

/// Reason for pausing the merge thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PauseReason {
  /// Stopped (because of throughput rate set to 0, typically).
  Stopped,
  /// Temporarily paused because of exceeded throughput rate.
  Paused,
  /// Other reason.
  Other,
}
/// Progress and state for an executing merge. This struct encapsulates the
/// logic to pause and resume the merge thread or to abort the merge entirely.
pub struct OneMergeProgress {
  pause_lock: Mutex<()>,
  pausing: Condvar,
  /// Pause times (in nanoseconds) for each [`PauseReason`](PauseReason).
  pause_times: PauseTimes,
  aborted: AtomicBool,
  /// This field is for sanity-check purpos only. Only the same thread that
  //     /// invoked `OneMerge#mergeInit()` is permiestted to be calling `pauseNanos`.
  /// This is always verified at runtime.
  owner: Mutex<Option<ThreadId>>,
}

#[derive(Default)]

struct PauseTimes {
  stopped: AtomicU64,
  paused: AtomicU64,
  other: AtomicU64,
}

impl Default for OneMergeProgress {
  fn default() -> Self {
    Self::new()
  }
}

impl OneMergeProgress {
  /// Creates a new merge progress info.
  pub fn new() -> Self {
    Self {
      pause_lock: Mutex::new(()),
      pausing: Condvar::new(),
      // Place all the pause reasons in there immediately so that we can
      // simply update values.
      pause_times: PauseTimes::default(),
      aborted: AtomicBool::new(false),
      owner: Mutex::new(None),
    }
  }
  /// Abort the merge this progress tracks at the next possible moment.
  pub fn abort(&self) {
    self.aborted.store(true, Ordering::Relaxed);
    self.wakeup(); // wakeup any paused merge thread.
  }
  /// Return the aborted state of this merge.
  pub fn is_aborted(&self) -> bool {
    self.aborted.load(Ordering::Relaxed)
  }

  /// Pauses the calling thread for at least `pause_nanos` nanoseconds unless
  /// the merge is aborted or the external condition returns `false`, in
  /// which case control returns immediately.
  ///
  /// The external condition is required so that other threads can terminate
  /// the pausing immediately before `pause_nanos` expires. We can't rely
  /// on just `Condvar::wait_timeout_while()` alone because it can return
  /// due to spurious wakeups too.
  ///
  /// # Arguments
  /// - `condition`: The pause condition that should return `false` if
  ///   immediate return from this method is needed. Other threads can wake up
  ///   any sleeping thread by calling [`wakeup()`](OneMergeProgress::wakeup),
  ///   but the thread may sleep for the remainder of the requested time if
  ///   this condition remains `true`.
  pub fn pause_nanos<F>(&self, pause_nanos: u64, reason: PauseReason, condition: F)
  where
    F: Fn() -> bool,
  {
    {
      let owner = self.owner.lock();
      let current_id = thread::current().id();
      debug_assert_eq!(
        *owner,
        Some(current_id),
        "Only owner thread can pause merge"
      );
    }

    let start = Instant::now();
    let deadline = start + Duration::from_nanos(pause_nanos);

    let mut lock = self.pause_lock.lock();
    while !self.aborted.load(Ordering::Relaxed) && condition() {
      let now = Instant::now();
      if now >= deadline {
        break;
      }
      let timeout = deadline - now;
      self.pausing.wait_for(&mut lock, timeout);
    }

    let elapsed = start.elapsed().as_nanos().min(u64::MAX as u128) as u64;
    self.add_pause_time(reason, elapsed);
  }

  fn add_pause_time(&self, reason: PauseReason, nanos: u64) {
    match reason {
      PauseReason::Stopped => self.pause_times.stopped.fetch_add(nanos, Ordering::Relaxed),
      PauseReason::Paused => self.pause_times.paused.fetch_add(nanos, Ordering::Relaxed),
      PauseReason::Other => self.pause_times.other.fetch_add(nanos, Ordering::Relaxed),
    };
  }
  /// Request a wakeup for any threads stalled in
  /// [`pauseNanos`](OneMergeProgress::pause_nanos).
  pub fn wakeup(&self) {
    let _lock = self.pause_lock.lock();
    self.pausing.notify_all();
  }
  /// Returns pause reasons and associated times in nanoseconds.
  pub fn get_pause_times(&self) -> HashMap<PauseReason, u64> {
    let mut map = HashMap::new();
    map.insert(
      PauseReason::Stopped,
      self.pause_times.stopped.load(Ordering::Relaxed),
    );
    map.insert(
      PauseReason::Paused,
      self.pause_times.paused.load(Ordering::Relaxed),
    );
    map.insert(
      PauseReason::Other,
      self.pause_times.other.load(Ordering::Relaxed),
    );
    map
  }
  pub fn set_merge_thread(&self) {
    let mut owner = self.owner.lock();
    debug_assert!(owner.is_none());
    *owner = Some(thread::current().id());
  }
}
/// This trait represents the current context of the merge selection process.
/// It allows access to real-time information such as:
/// - the segments currently being merged
/// - how many deletes a segment would reclaim if merged
///
/// This context may be stateful and can change during the execution of a
/// merge policy's selection processes.
pub trait MergeContext<D>
where
  D: Directory,
{
  /// Returns the number of deletes a merge would claim back
  /// if the given segment is merged.
  ///
  /// See [`MergePolicy::num_deletes_to_merge`].
  ///
  /// * `info` — the segment to get the number of deletes for
  fn num_deletes_to_merge(&self, info: &SegmentCommitInfo<D>) -> Result<i32>;

  /// Returns the number of deleted documents in the given segment.
  fn num_deleted_docs(&self, info: &SegmentCommitInfo<D>) -> i32;

  /// Returns the info stream that can be used to log messages.
  fn get_info_stream(&self) -> InfoStreamMT;

  /// Returns an unmodifiable set of segments that are currently merging.
  fn get_merging_segments(&self, inner: Option<&Inner<D>>) -> HashSet<String>;
}

pub type MergeReaderSR<D> =
  MergeReader<Arc<SegmentReader<D>>, <Arc<SegmentReader<D>> as LeafReader>::Bits>;
pub struct MergeReader<CR, B>
where
  CR: CodecReader,
  B: Bits,
{
  pub(crate) reader: CR,
  pub(crate) hard_live_docs: Option<B>,
}
impl<CR, B> MergeReader<CR, B>
where
  CR: CodecReader,
  B: Bits,
{
  pub(crate) fn new(codec_reader: CR, hard_live_docs: Option<B>) -> Self {
    Self {
      reader: codec_reader,
      hard_live_docs,
    }
  }
}
