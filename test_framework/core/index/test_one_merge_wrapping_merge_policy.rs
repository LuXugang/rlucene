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
use crate::core::index::index_reader::Identity;
use crate::core::index::index_writer::Inner;
use crate::core::index::merge_policy::{
  DefaultMergeSpecification, MergeContext, MergePolicy, MergePolicyBase, MergePolicyEnum, OneMerge,
  OneMergeSR, size,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::one_merge_wrapping_merge_policy::OneMergeUnaryOperatorBase;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestOneMergeWrappingMergePolicy;

pub struct PredeterminedMergePolicy<D>
where
  D: Directory,
{
  base: MergePolicyBase,
  state: Arc<Mutex<PredeterminedMergePolicyState<D>>>,
}

struct PredeterminedMergePolicyState<D>
where
  D: Directory,
{
  merges: Option<DefaultMergeSpecification<D>>,
  forced_merges: Option<DefaultMergeSpecification<D>>,
  forced_deletes_merges: Option<DefaultMergeSpecification<D>>,
}

impl<D> PredeterminedMergePolicy<D>
where
  D: Directory,
{
  pub(crate) fn new(
    merges: Option<DefaultMergeSpecification<D>>,
    forced_merges: Option<DefaultMergeSpecification<D>>,
    forced_deletes_merges: Option<DefaultMergeSpecification<D>>,
  ) -> Self {
    Self {
      base: MergePolicyBase::default(),
      state: Arc::new(Mutex::new(PredeterminedMergePolicyState {
        merges,
        forced_merges,
        forced_deletes_merges,
      })),
    }
  }
}

impl<D> Clone for PredeterminedMergePolicy<D>
where
  D: Directory,
{
  fn clone(&self) -> Self {
    Self {
      base: self.base.clone(),
      state: self.state.clone(),
    }
  }
}

impl<D> From<PredeterminedMergePolicy<D>> for MergePolicyEnum<D>
where
  D: Directory,
{
  fn from(value: PredeterminedMergePolicy<D>) -> Self {
    Self::Predetermined(value)
  }
}

impl<D> Display for PredeterminedMergePolicy<D>
where
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "PredeterminedMergePolicy")
  }
}

impl<D> MergePolicy<D> for PredeterminedMergePolicy<D>
where
  D: Directory,
{
  fn get_base(&self) -> &MergePolicyBase {
    &self.base
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    &mut self.base
  }

  fn find_merges<MC>(
    &self,
    _merge_trigger: MergeTrigger,
    _segment_infos: &SegmentInfos<D>,
    _inner: Option<&Inner<D>>,
    _merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    Ok(self.state.lock().merges.take())
  }

  fn find_forced_merges<MC>(
    &self,
    _segment_infos: &SegmentInfos<D>,
    _max_segment_count: usize,
    _segments_to_merge: &HashMap<String, Option<bool>>,
    _inner: Option<&Inner<D>>,
    _merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    Ok(self.state.lock().forced_merges.take())
  }

  fn find_forced_deletes_merges<MC>(
    &self,
    _segment_infos: &SegmentInfos<D>,
    _inner: Option<&Inner<D>>,
    _merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    Ok(self.state.lock().forced_deletes_merges.take())
  }

  fn size<MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    MC: MergeContext<D>,
  {
    size(info, merge_context)
  }
}

#[derive(Clone)]
pub(crate) struct WrappedOneMerge {
  pub(crate) original: Identity,
  pub(crate) wrapped: Identity,
}

#[derive(Clone, Default)]
pub struct WrappedOneMergeUnaryOperator {
  wrapped_merges: Arc<Mutex<Vec<WrappedOneMerge>>>,
}

impl WrappedOneMergeUnaryOperator {
  pub(crate) fn new() -> Self {
    Self::default()
  }

  pub(crate) fn take_wrapped_merges(&self) -> Vec<WrappedOneMerge> {
    std::mem::take(&mut *self.wrapped_merges.lock())
  }
}

impl<D> OneMergeUnaryOperatorBase<D> for WrappedOneMergeUnaryOperator
where
  D: Directory,
{
  fn apply(&self, merge: OneMergeSR<D>) -> Result<OneMergeSR<D>> {
    let original = merge.stat.id.clone();
    let wrapped = OneMerge::new(merge.segments)?;
    self.wrapped_merges.lock().push(WrappedOneMerge {
      original,
      wrapped: wrapped.stat.id.clone(),
    });
    Ok(wrapped)
  }
}
