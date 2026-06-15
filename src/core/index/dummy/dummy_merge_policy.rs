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
  MergeContext, MergePolicy, MergePolicyBase, MergeSpecificationNoReader,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

pub struct DummyMergePolicy;

impl Display for DummyMergePolicy {
  fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
    dummy_unreachable!()
  }
}

impl MergePolicy for DummyMergePolicy {
  fn get_base(&self) -> &MergePolicyBase {
    dummy_unreachable!()
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    dummy_unreachable!()
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
    dummy_unreachable!()
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
    dummy_unreachable!()
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
    dummy_unreachable!()
  }

  fn find_full_flush_merges<D, MC>(
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
    dummy_unreachable!()
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
    dummy_unreachable!()
  }

  fn size<D, MC>(&self, _info: &SegmentCommitInfo<D>, _merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    dummy_unreachable!()
  }

  fn has_merged<D, MC>(
    &self,
    _infos: &SegmentInfos<D>,
    _info: &SegmentCommitInfo<D>,
    _merge_context: &MC,
  ) -> Result<bool>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    dummy_unreachable!()
  }

  fn keep_fully_deleted_segment<D, F>(&self, _reader_supplier: F) -> Result<bool>
  where
    D: Directory,
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    dummy_unreachable!()
  }

  fn num_deletes_to_merge<D, F>(
    &self,
    _info: &SegmentCommitInfo<D>,
    _del_count: i32,
    _reader_supplier: F,
  ) -> Result<i32>
  where
    D: Directory,
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    dummy_unreachable!()
  }

  fn seg_string<MC, D>(&self, _merge_context: &MC, _infos: &[SegmentCommitInfo<D>]) -> String
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    dummy_unreachable!()
  }

  fn message<MC, D>(&self, _message: &str, _merge_context: &MC) -> Result<()>
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    dummy_unreachable!()
  }

  fn verbose<MC, D>(&self, _merge_context: &MC) -> bool
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    dummy_unreachable!()
  }
}
