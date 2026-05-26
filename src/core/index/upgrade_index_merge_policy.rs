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
use crate::core::index::index_writer::Inner;
use crate::core::index::merge_policy::{
  MergeContext, MergePolicy, MergePolicyBase, MergePolicyEnum, MergeSpecification,
  MergeSpecificationNoReader, OneMerge,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::tiered_merge_policy::SegmentDocAndID;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::version::LATEST;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

/// This [`MergePolicy`] is used for upgrading all existing segments of an index when calling
/// [`IndexWriter::force_merge`].
///
/// All other methods delegate to the base [`MergePolicy`] given to the constructor. This allows
/// for an as-cheap-as possible upgrade of an older index by only upgrading segments that are
/// created by previous Lucene versions. `force_merge` does no longer really merge; it is just
/// used to "force_merge" older segment versions away.
///
/// In general one would use `IndexUpgrader`, but for a fully customizable upgrade, you can use
/// this like any other [`MergePolicy`] and call [`IndexWriter::force_merge`]:
///
/// ```ignore
/// let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
/// iwc.set_merge_policy(UpgradeIndexMergePolicy::new(iwc.get_merge_policy().clone()));
/// let w = IndexWriter::new(dir, iwc)?;
/// w.force_merge(1)?;
/// w.close()?;
/// ```
///
/// **Warning:** This merge policy may reorder documents if the index was partially upgraded
/// before calling `force_merge` (e.g., documents were added). If your application relies on
/// "monotonicity" of doc IDs (which means that the order in which the documents were added to
/// the index is preserved), do a `force_merge(1)` instead. Please note, the delegate
/// [`MergePolicy`] may also reorder documents.
///
/// @see IndexUpgrader
#[derive(Clone)]
pub struct UpgradeIndexMergePolicy {
  base: MergePolicyBase,
  inner: Box<MergePolicyEnum>,
}

impl UpgradeIndexMergePolicy {
  /// Wrap the given [`MergePolicy`] and intercept `force_merge` requests to only upgrade
  /// segments written with previous Lucene versions.
  pub fn new(inner: MergePolicyEnum) -> Self {
    Self {
      base: MergePolicyBase::default(),
      inner: Box::new(inner),
    }
  }

  /// Returns `true` if the given segment should be upgraded.
  ///
  /// The default implementation returns `sci.info.get_version_ref() != Some(&*LATEST)`,
  /// so all segments created with a different version number than this Lucene version will
  /// get upgraded.
  pub fn should_upgrade_segment<D: Directory>(sci: &SegmentCommitInfo<D>) -> bool {
    sci.info.get_version_ref() != Some(&*LATEST)
  }
}

impl Display for UpgradeIndexMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl MergePolicy for UpgradeIndexMergePolicy {
  fn get_base(&self) -> &MergePolicyBase {
    &self.base
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    &mut self.base
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
    self
      .inner
      .find_merges(merge_trigger, segment_infos, inner, merge_context)
  }

  fn find_merges_readers<CR, D>(
    &self,
    readers: Vec<CR>,
  ) -> Result<Option<MergeSpecification<D, CR>>>
  where
    CR: CodecReader,
    D: Directory,
  {
    self.inner.find_merges_readers(readers)
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
    // first find all old segments
    let mut old_segments: HashMap<String, Option<bool>> = HashMap::new();
    for i in 0..segment_infos.size() {
      if let Some(sci) = segment_infos.info(i) {
        let seg_key = sci.info.get_id_key().to_string();
        if let Some(v) = segments_to_merge.get(&seg_key)
          && Self::should_upgrade_segment(sci)
        {
          old_segments.insert(seg_key, *v);
        }
      }
    }

    if old_segments.is_empty() {
      return Ok(None);
    }

    let mut spec = self.inner.find_forced_merges(
      segment_infos,
      max_segment_count,
      &old_segments,
      inner,
      merge_context,
    )?;

    // remove segments that the inner policy decided to merge
    if let Some(ref spec_inner) = spec {
      for om in &spec_inner.merges {
        for seg_key in &om.stat.segments {
          old_segments.remove(seg_key);
        }
      }
    }

    // merge any remaining old segments that the inner policy didn't handle
    if !old_segments.is_empty() {
      let mut new_infos: Vec<SegmentDocAndID> = Vec::new();
      for i in 0..segment_infos.size() {
        if let Some(sci) = segment_infos.info(i) {
          let seg_key = sci.info.get_id_key().to_string();
          if old_segments.contains_key(&seg_key) {
            new_infos.push(SegmentDocAndID::new(seg_key, sci.info.max_doc()?));
          }
        }
      }
      if !new_infos.is_empty() {
        let merge = OneMerge::new(new_infos)?;
        if spec.is_none() {
          spec = Some(MergeSpecificationNoReader::new());
        }
        spec.as_mut().unwrap().add(merge);
      }
    }

    Ok(spec)
  }

  fn find_forced_deletes_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self
      .inner
      .find_forced_deletes_merges(segment_infos, inner, merge_context)
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
    self
      .inner
      .find_full_flush_merges(merge_trigger, segment_infos, inner, merge_context)
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
    self
      .inner
      .use_compound_file(infos, merged_info, merge_context)
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.inner.size(info, merge_context)
  }

  fn max_full_flush_merge_size(&self) -> i64 {
    self.inner.max_full_flush_merge_size()
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
    self.inner.has_merged(infos, info, merge_context)
  }

  fn keep_fully_deleted_segment<D, F>(&self, reader_supplier: F) -> Result<bool>
  where
    D: Directory,
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    self.inner.keep_fully_deleted_segment(reader_supplier)
  }

  fn num_deletes_to_merge<D, F>(
    &self,
    info: &SegmentCommitInfo<D>,
    del_count: i32,
    reader_supplier: F,
  ) -> Result<i32>
  where
    D: Directory,
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    self
      .inner
      .num_deletes_to_merge(info, del_count, reader_supplier)
  }

  fn seg_string<MC, D>(&self, merge_context: &MC, infos: &[SegmentCommitInfo<D>]) -> String
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.inner.seg_string(merge_context, infos)
  }

  fn message<MC, D>(&self, message: &str, merge_context: &MC)
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.inner.message(message, merge_context)
  }

  fn verbose<MC, D>(&self, merge_context: &MC) -> bool
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.inner.verbose(merge_context)
  }
}
