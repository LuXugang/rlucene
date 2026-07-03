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
#[cfg(test)]
use crate::core::index::merge_policy::OneMerge;
use crate::core::index::merge_policy::{
  DefaultMergeSpecification, MergeContext, MergePolicy, MergePolicyBase, MergePolicyEnum,
  MergeSpecification, OneMergeSR,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
/// A wrapping merge policy that wraps the `OneMerge` objects returned by the
/// wrapped merge policy.
///
/// # Experimental
///
/// This API is experimental and may change in incompatible ways.
#[derive(Clone)]
pub struct OneMergeWrappingMergePolicy {
  in_: Box<MergePolicyEnum>,
  wrap_one_merge: OneMergeUnaryOperator,
}

impl OneMergeWrappingMergePolicy {
  pub fn new<T, W>(in_: T, wrap_one_merge: W) -> Self
  where
    T: Into<MergePolicyEnum>,
    W: Into<OneMergeUnaryOperator>,
  {
    Self {
      in_: Box::new(in_.into()),
      wrap_one_merge: wrap_one_merge.into(),
    }
  }

  fn wrap_spec<D>(
    &self,
    spec: Option<DefaultMergeSpecification<D>>,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    D: Directory,
  {
    spec
      .map(|spec| {
        let mut wrapped = DefaultMergeSpecification::new();
        for merge in spec.merges {
          wrapped.add(self.wrap_one_merge.apply(merge)?);
        }
        Ok(wrapped)
      })
      .transpose()
  }
}

#[derive(Clone)]
pub enum OneMergeUnaryOperator {
  Identity(IdentityOneMergeUnaryOperator),
  #[cfg(test)]
  NewOneMerge(NewOneMergeUnaryOperator),
}

pub trait OneMergeUnaryOperatorBase {
  fn apply<D>(&self, merge: OneMergeSR<D>) -> Result<OneMergeSR<D>>
  where
    D: Directory;
}

impl OneMergeUnaryOperatorBase for OneMergeUnaryOperator {
  fn apply<D>(&self, merge: OneMergeSR<D>) -> Result<OneMergeSR<D>>
  where
    D: Directory,
  {
    match self {
      Self::Identity(operator) => operator.apply(merge),
      #[cfg(test)]
      Self::NewOneMerge(operator) => operator.apply(merge),
    }
  }
}

#[derive(Clone)]
pub struct IdentityOneMergeUnaryOperator;

impl From<IdentityOneMergeUnaryOperator> for OneMergeUnaryOperator {
  fn from(value: IdentityOneMergeUnaryOperator) -> Self {
    Self::Identity(value)
  }
}

impl OneMergeUnaryOperatorBase for IdentityOneMergeUnaryOperator {
  fn apply<D>(&self, merge: OneMergeSR<D>) -> Result<OneMergeSR<D>>
  where
    D: Directory,
  {
    Ok(merge)
  }
}

#[cfg(test)]
#[derive(Clone)]
pub struct NewOneMergeUnaryOperator;

#[cfg(test)]
impl From<NewOneMergeUnaryOperator> for OneMergeUnaryOperator {
  fn from(value: NewOneMergeUnaryOperator) -> Self {
    Self::NewOneMerge(value)
  }
}

#[cfg(test)]
impl OneMergeUnaryOperatorBase for NewOneMergeUnaryOperator {
  fn apply<D>(&self, merge: OneMergeSR<D>) -> Result<OneMergeSR<D>>
  where
    D: Directory,
  {
    OneMerge::new(merge.segments)
  }
}

impl Display for OneMergeWrappingMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "OneMergeWrappingMergePolicy({})", self.in_)
  }
}

impl MergePolicy for OneMergeWrappingMergePolicy {
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
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.wrap_spec(
      self
        .in_
        .find_merges(merge_trigger, segment_infos, inner, merge_context)?,
    )
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
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.wrap_spec(self.in_.find_forced_merges(
      segment_infos,
      max_segment_count,
      segments_to_merge,
      inner,
      merge_context,
    )?)
  }

  fn find_forced_deletes_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.wrap_spec(
      self
        .in_
        .find_forced_deletes_merges(segment_infos, inner, merge_context)?,
    )
  }

  fn find_full_flush_merges<D, MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.wrap_spec(self.in_.find_full_flush_merges(
      merge_trigger,
      segment_infos,
      inner,
      merge_context,
    )?)
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
    self.in_.has_merged(infos, info, merge_context)
  }

  fn keep_fully_deleted_segment<D, F>(&self, reader_supplier: F) -> Result<bool>
  where
    D: Directory,
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    self.in_.keep_fully_deleted_segment(reader_supplier)
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

  fn seg_string<MC, D>(&self, merge_context: &MC, infos: &[SegmentCommitInfo<D>]) -> String
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.in_.seg_string(merge_context, infos)
  }

  fn message<MC, D>(&self, message: &str, merge_context: &MC) -> Result<()>
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.in_.message(message, merge_context)
  }

  fn verbose<MC, D>(&self, merge_context: &MC) -> bool
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.in_.verbose(merge_context)
  }
}
