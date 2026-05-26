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
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

#[derive(Clone)]
pub struct MergeOnFlushMergePolicy {
  base: MergePolicyBase,
  inner: Box<MergePolicyEnum>,
  small_segment_threshold_bytes: i64,
}

impl MergeOnFlushMergePolicy {
  pub fn new(inner: MergePolicyEnum) -> Self {
    Self {
      base: MergePolicyBase::default(),
      inner: Box::new(inner),
      small_segment_threshold_bytes: Units::mb_to_bytes(100.0),
    }
  }

  pub fn get_small_segment_threshold_mb(&self) -> f64 {
    Units::bytes_to_mb(self.small_segment_threshold_bytes)
  }

  pub fn set_small_segment_threshold_mb(&mut self, small_segment_threshold_mb: f64) {
    self.small_segment_threshold_bytes = Units::mb_to_bytes(small_segment_threshold_mb);
  }
}

impl Display for MergeOnFlushMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl MergePolicy for MergeOnFlushMergePolicy {
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
    self.inner.find_forced_merges(
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
    D: Directory,
    MC: MergeContext<D>,
  {
    self
      .inner
      .find_forced_deletes_merges(segment_infos, inner, merge_context)
  }

  fn find_full_flush_merges<D, MC>(
    &self,
    _merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    let merging_segments = merge_context.get_merging_segments(inner);
    let mut small_segments = Vec::new();
    for sci in segment_infos.iter() {
      if sci.size_in_bytes()? < self.small_segment_threshold_bytes
        && !merging_segments.contains(sci.info.get_id_key())
      {
        small_segments.push(SegmentDocAndID::new(
          sci.info.get_id_key().to_string(),
          sci.info.max_doc()?,
        ));
      }
    }

    if small_segments.len() > 1 {
      let mut merge_spec = MergeSpecificationNoReader::new();
      merge_spec.add(OneMerge::new(small_segments)?);
      Ok(Some(merge_spec))
    } else {
      Ok(None)
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

pub struct Units;

impl Units {
  pub fn bytes_to_mb(bytes: i64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
  }

  pub fn mb_to_bytes(megabytes: f64) -> i64 {
    (megabytes * 1024.0 * 1024.0) as i64
  }
}
