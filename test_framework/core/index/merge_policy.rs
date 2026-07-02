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
  MergeSpecificationNoReader, OneMerge, size,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::tiered_merge_policy::{SegmentDocAndID, TieredMergePolicy};
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MockMergePolicy {
  base: MergePolicyBase,
  merge_factor: i32,
}

impl Default for MockMergePolicy {
  fn default() -> Self {
    Self {
      base: MergePolicyBase::default(),
      merge_factor: 10,
    }
  }
}

impl MockMergePolicy {
  pub(crate) fn get_merge_factor(&self) -> i32 {
    self.merge_factor
  }

  pub fn set_merge_factor(&mut self, merge_factor: i32) {
    self.merge_factor = merge_factor;
  }
}

impl From<MockMergePolicy> for MergePolicyEnum {
  fn from(value: MockMergePolicy) -> Self {
    Self::Mock(value)
  }
}

impl Display for MockMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "MockMergePolicy")
  }
}

impl MergePolicy for MockMergePolicy {
  fn get_base(&self) -> &MergePolicyBase {
    &self.base
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    &mut self.base
  }

  fn find_merges<D, MC>(
    &self,
    _merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    _inner: Option<&Inner<D>>,
    _merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    let segments = segment_infos.iter();
    let merge_factor = self.merge_factor as usize;
    let mut spec = None;
    let mut start = 0;
    while start + merge_factor <= segments.len() {
      let start_doc_count = segments[start].info.max_doc()?;
      let mut end = start + 1;
      for i in (start + 1..segments.len()).rev() {
        let doc_count = segments[i].info.max_doc()?;
        if i64::from(doc_count) * i64::from(self.merge_factor) > i64::from(start_doc_count)
          && i64::from(doc_count) < i64::from(self.merge_factor) * i64::from(start_doc_count)
        {
          end = i + 1;
          break;
        }
      }

      if start + merge_factor <= end {
        let merge_spec = spec.get_or_insert_with(MergeSpecificationNoReader::new);
        let mut merge_segments = Vec::with_capacity(merge_factor);
        for info in &segments[start..start + merge_factor] {
          merge_segments.push(SegmentDocAndID::new(
            info.info.get_id_key().to_string(),
            info.info.max_doc()?,
          ));
        }
        merge_spec.add(OneMerge::new(merge_segments)?);
        start += merge_factor;
      } else {
        start += 1;
      }
    }
    Ok(spec)
  }

  fn find_forced_merges<D, MC>(
    &self,
    _segment_infos: &SegmentInfos<D>,
    _max_segment_count: usize,
    _segments_to_merge: &HashMap<String, Option<bool>>,
    _inner: Option<&Inner<D>>,
    _merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    Ok(None)
  }

  fn find_forced_deletes_merges<D, MC>(
    &self,
    _segment_infos: &SegmentInfos<D>,
    _inner: Option<&Inner<D>>,
    _merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    Ok(None)
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    size(info, merge_context)
  }
}

#[derive(Clone)]
pub struct MergeOnXMergePolicy {
  pub(crate) in_: Box<MergePolicyEnum>,
  pub(crate) trigger: MergeTrigger,
}

impl MergeOnXMergePolicy {
  pub(crate) fn new(in_: MergePolicyEnum, trigger: MergeTrigger) -> Self {
    Self {
      in_: Box::new(in_),
      trigger,
    }
  }
}

impl Display for MergeOnXMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "MergeOnCommit({})", self.in_)
  }
}

impl MergePolicy for MergeOnXMergePolicy {
  fn get_base(&self) -> &MergePolicyBase {
    self.in_.get_base()
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    self.in_.get_base_mut()
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
      .in_
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
    self.in_.find_merges_readers(readers)
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
    self.in_.find_forced_merges(
      segment_infos,
      max_segment_count,
      segments_to_merge,
      inner,
      merge_context,
    )
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
    self
      .in_
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
    if merge_trigger == self.trigger && segment_infos.iter().len() > 1 {
      let merging = merge_context.get_merging_segments(inner);
      let mut non_merging_segments = Vec::new();
      for sci in segment_infos.iter() {
        if !merging.contains(sci.info.get_id_key()) {
          non_merging_segments.push(SegmentDocAndID::new(
            sci.info.get_id_key().to_string(),
            sci.info.max_doc()?,
          ));
        }
      }
      if non_merging_segments.len() > 1 {
        let mut spec = MergeSpecificationNoReader::new();
        spec.add(OneMerge::new(non_merging_segments)?);
        return Ok(Some(spec));
      }
    }
    Ok(None)
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
      .in_
      .use_compound_file(infos, merged_info, merge_context)
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.in_.size(info, merge_context)
  }
}

#[derive(Clone)]
pub struct OnlyForceMergeMergePolicy {
  base: TieredMergePolicy,
}

impl OnlyForceMergeMergePolicy {
  pub(crate) fn new(base: TieredMergePolicy) -> Self {
    Self { base }
  }
}

impl From<OnlyForceMergeMergePolicy> for MergePolicyEnum {
  fn from(value: OnlyForceMergeMergePolicy) -> Self {
    Self::OnlyForceMerge(value)
  }
}

impl Display for OnlyForceMergeMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.base)
  }
}

impl MergePolicy for OnlyForceMergeMergePolicy {
  fn get_base(&self) -> &MergePolicyBase {
    self.base.get_base()
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    self.base.get_base_mut()
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
    Ok(None)
  }

  fn find_merges_readers<CR, D>(
    &self,
    readers: Vec<CR>,
  ) -> Result<Option<MergeSpecification<D, CR>>>
  where
    CR: CodecReader,
    D: Directory,
  {
    self.base.find_merges_readers(readers)
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
    self.base.find_forced_merges(
      segment_infos,
      max_segment_count,
      segments_to_merge,
      inner,
      merge_context,
    )
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
    self
      .base
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
      .base
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
      .base
      .use_compound_file(infos, merged_info, merge_context)
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.base.size(info, merge_context)
  }

  fn max_full_flush_merge_size(&self) -> i64 {
    self.base.max_full_flush_merge_size()
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
    self.base.has_merged(infos, info, merge_context)
  }

  fn keep_fully_deleted_segment<D, F>(&self, reader_supplier: F) -> Result<bool>
  where
    D: Directory,
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    self.base.keep_fully_deleted_segment(reader_supplier)
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
      .base
      .num_deletes_to_merge(info, del_count, reader_supplier)
  }

  fn seg_string<MC, D>(&self, merge_context: &MC, infos: &[SegmentCommitInfo<D>]) -> String
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.base.seg_string(merge_context, infos)
  }

  fn message<MC, D>(&self, message: &str, merge_context: &MC) -> Result<()>
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.base.message(message, merge_context)
  }

  fn verbose<MC, D>(&self, merge_context: &MC) -> bool
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.base.verbose(merge_context)
  }
}

#[derive(Clone, Default)]
pub struct KeepFullyDeletedSegmentsMergePolicy {
  in_: NoMergePolicy,
  merge_fully_deleted_on_full_flush: bool,
  keep_fully_deleted_segments: Option<Arc<AtomicBool>>,
}

impl KeepFullyDeletedSegmentsMergePolicy {
  pub(crate) fn with_full_flush_merges() -> Self {
    Self {
      in_: NoMergePolicy::default(),
      merge_fully_deleted_on_full_flush: true,
      keep_fully_deleted_segments: None,
    }
  }

  pub(crate) fn with_keep_fully_deleted_segments(
    keep_fully_deleted_segments: Arc<AtomicBool>,
  ) -> Self {
    Self {
      in_: NoMergePolicy::default(),
      merge_fully_deleted_on_full_flush: false,
      keep_fully_deleted_segments: Some(keep_fully_deleted_segments),
    }
  }
}

impl From<KeepFullyDeletedSegmentsMergePolicy> for MergePolicyEnum {
  fn from(value: KeepFullyDeletedSegmentsMergePolicy) -> Self {
    MergePolicyEnum::KeepFullyDeletedSegments(value)
  }
}

impl Display for KeepFullyDeletedSegmentsMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "KeepFullyDeletedSegmentsMergePolicy")
  }
}

impl MergePolicy for KeepFullyDeletedSegmentsMergePolicy {
  fn get_base(&self) -> &MergePolicyBase {
    self.in_.get_base()
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    self.in_.get_base_mut()
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
      .in_
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
    self.in_.find_merges_readers(readers)
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
    self.in_.find_forced_merges(
      segment_infos,
      max_segment_count,
      segments_to_merge,
      inner,
      merge_context,
    )
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
    self
      .in_
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
    if !self.merge_fully_deleted_on_full_flush {
      return self
        .in_
        .find_full_flush_merges(merge_trigger, segment_infos, inner, merge_context);
    }

    let mut fully_deleted_segments = Vec::new();
    for sci in segment_infos.iter() {
      let max_doc = sci.info.max_doc()?;
      if max_doc - sci.get_del_count() == 0 {
        fully_deleted_segments.push(SegmentDocAndID::new(
          sci.info.get_id_key().to_string(),
          max_doc,
        ));
      }
    }

    if fully_deleted_segments.is_empty() {
      return Ok(None);
    }

    let mut spec = MergeSpecificationNoReader::new();
    spec.add(OneMerge::new(fully_deleted_segments)?);
    Ok(Some(spec))
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
      .in_
      .use_compound_file(infos, merged_info, merge_context)
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.in_.size(info, merge_context)
  }

  fn max_full_flush_merge_size(&self) -> i64 {
    self.in_.max_full_flush_merge_size()
  }

  fn keep_fully_deleted_segment<D, F>(&self, _reader_supplier: F) -> Result<bool>
  where
    D: Directory,
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    Ok(
      self
        .keep_fully_deleted_segments
        .as_ref()
        .map(|keep_fully_deleted_segments| keep_fully_deleted_segments.load(SeqCst))
        .unwrap_or(true),
    )
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
      .in_
      .num_deletes_to_merge(info, del_count, reader_supplier)
  }
}

pub struct RangeMergePolicy {
  base: MergePolicyBase,
  state: Mutex<RangeMergePolicyState>,
  use_compound_file: bool,
}

#[derive(Clone, Copy)]
struct RangeMergePolicyState {
  do_merge: bool,
  start: usize,
  length: usize,
}

impl RangeMergePolicy {
  pub(crate) fn new(use_compound_file: bool) -> Self {
    Self {
      base: MergePolicyBase::default(),
      state: Mutex::new(RangeMergePolicyState {
        do_merge: false,
        start: 0,
        length: 0,
      }),
      use_compound_file,
    }
  }

  pub(crate) fn set_merge(&self, start: usize, length: usize) {
    let mut state = self.state.lock().unwrap();
    state.start = start;
    state.length = length;
    state.do_merge = true;
  }
}

impl Clone for RangeMergePolicy {
  fn clone(&self) -> Self {
    Self {
      base: self.base.clone(),
      state: Mutex::new(*self.state.lock().unwrap()),
      use_compound_file: self.use_compound_file,
    }
  }
}

impl Display for RangeMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "RangeMergePolicy")
  }
}

impl MergePolicy for RangeMergePolicy {
  fn get_base(&self) -> &MergePolicyBase {
    &self.base
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    &mut self.base
  }

  fn find_merges<D, MC>(
    &self,
    _merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    _inner: Option<&Inner<D>>,
    _merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    let mut state = self.state.lock().unwrap();
    if state.do_merge {
      state.do_merge = false;
      let start = state.start;
      let length = state.length;
      drop(state);

      let mut merge_segments = Vec::with_capacity(length);
      for info in &segment_infos.iter()[start..start + length] {
        merge_segments.push(SegmentDocAndID::new(
          info.info.get_id_key().to_string(),
          info.info.max_doc()?,
        ));
      }
      let mut ms = MergeSpecificationNoReader::new();
      ms.add(OneMerge::new(merge_segments)?);
      return Ok(Some(ms));
    }
    Ok(None)
  }

  fn find_forced_merges<D, MC>(
    &self,
    _segment_infos: &SegmentInfos<D>,
    _max_segment_count: usize,
    _segments_to_merge: &HashMap<String, Option<bool>>,
    _inner: Option<&Inner<D>>,
    _merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    Ok(None)
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
    Ok(None)
  }

  fn use_compound_file<D, MC>(
    &self,
    _infos: &SegmentInfos<D>,
    _merged_info: &SegmentCommitInfo<D>,
    _merge_context: &MC,
  ) -> Result<bool>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    Ok(self.use_compound_file)
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    size(info, merge_context)
  }
}
