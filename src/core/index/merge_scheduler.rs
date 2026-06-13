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
use crate::core::index::index_writer::{IndexWriter, Inner};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_scheduler::NoMergeScheduler;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::store::directory::{Directory, DirectoryEnum2};
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test::core::index::base_knn_vectors_format_test_case::TestMergeScheduler;
#[cfg(test)]
use crate::test::core::index::base_merge_policy_test_case::SerialMergeSchedulerImpl;
#[cfg(test)]
use crate::test::core::index::test_index_writer_merging::MyMergeScheduler;
use parking_lot::MutexGuard;

pub trait MergeScheduler: Closeable {
  fn merge<MS, D>(
    &self,
    merge_source: &MS,
    trigger: MergeTrigger,
    writer: &IndexWriter<D>,
  ) -> Result<()>
  where
    MS: MergeSource,
    D: Directory;
  type Directory<D>: Directory
  where
    D: Directory;
  fn wrap_for_merge<D>(&self, _in_: D) -> Result<Self::Directory<D>>
  where
    D: Directory;

  fn initialize<D>(&mut self, _directory: &D) -> Result<()>
  where
    D: Directory,
  {
    Ok(())
  }
}

/// Provides access to new merges and executes the actual merge
pub trait MergeSource {
  /// The merge type produced by this source.
  type OneMerge<D>
  where
    D: Directory;

  /// The `MergeScheduler` calls this method to retrieve the next merge
  /// requested by the `MergePolicy`.
  fn get_next_merge<D>(&self, writer: &IndexWriter<D>) -> Result<Option<Self::OneMerge<D>>>
  where
    D: Directory;

  /// Does finishing for a merge.
  fn on_merge_finished<D>(
    &self,
    merge: &Self::OneMerge<D>,
    writer: &IndexWriter<D>,
    inner: Option<&mut Inner<D>>,
  ) where
    D: Directory;

  /// Expert: returns true if there are merges waiting to be scheduled.
  fn has_pending_merges<D>(
    &self,
    inner: Option<&MutexGuard<'_, Inner<D>>>,
    writer: Option<&IndexWriter<D>>,
  ) -> Result<bool>
  where
    D: Directory;

  /// Merges the indicated segments, replacing them in the stack
  /// with a single segment.
  fn merge<D>(&self, merge: &mut Self::OneMerge<D>, writer: &IndexWriter<D>) -> Result<()>
  where
    D: Directory;

  fn merge_segment_ids<'a, D>(&self, _merge: &'a Self::OneMerge<D>) -> Option<&'a [String]>
  where
    D: Directory,
  {
    None
  }

  fn merge_info_max_doc<D>(&self, _merge: &Self::OneMerge<D>) -> Result<Option<i32>>
  where
    D: Directory,
  {
    Ok(None)
  }
}
pub enum MergeSchedulerEnum {
  Serial(SerialMergeScheduler),
  No(NoMergeScheduler),
  #[cfg(test)]
  SerialTest(SerialMergeSchedulerImpl),
  #[cfg(test)]
  KnnMergeScheduler(TestMergeScheduler),
  #[cfg(test)]
  IndexWriterMerging(MyMergeScheduler),
}
impl_from_for_enum!(
    MergeSchedulerEnum,
    SerialMergeScheduler => Serial,
    NoMergeScheduler => No,
);
impl Default for MergeSchedulerEnum {
  fn default() -> Self {
    Self::Serial(SerialMergeScheduler)
  }
}

impl Closeable for MergeSchedulerEnum {
  fn close(&mut self) -> Result<()> {
    match self {
      MergeSchedulerEnum::Serial(s) => s.close(),
      MergeSchedulerEnum::No(n) => n.close(),
      #[cfg(test)]
      MergeSchedulerEnum::SerialTest(s) => s.close(),
      #[cfg(test)]
      MergeSchedulerEnum::KnnMergeScheduler(s) => s.close(),
      #[cfg(test)]
      MergeSchedulerEnum::IndexWriterMerging(s) => s.close(),
    }
  }
}

impl MergeScheduler for MergeSchedulerEnum {
  fn merge<MS, D>(
    &self,
    merge_source: &MS,
    trigger: MergeTrigger,
    index_writer: &IndexWriter<D>,
  ) -> Result<()>
  where
    MS: MergeSource,
    D: Directory,
  {
    match self {
      MergeSchedulerEnum::Serial(s) => s.merge(merge_source, trigger, index_writer),
      MergeSchedulerEnum::No(n) => n.merge(merge_source, trigger, index_writer),
      #[cfg(test)]
      MergeSchedulerEnum::SerialTest(s) => s.merge(merge_source, trigger, index_writer),
      #[cfg(test)]
      MergeSchedulerEnum::KnnMergeScheduler(s) => s.merge(merge_source, trigger, index_writer),
      #[cfg(test)]
      MergeSchedulerEnum::IndexWriterMerging(s) => s.merge(merge_source, trigger, index_writer),
    }
  }

  type Directory<D>
    = DirectoryEnum2<
    <SerialMergeScheduler as MergeScheduler>::Directory<D>,
    <NoMergeScheduler as MergeScheduler>::Directory<D>,
  >
  where
    D: Directory;

  fn wrap_for_merge<D>(&self, in_: D) -> Result<Self::Directory<D>>
  where
    D: Directory,
  {
    match self {
      MergeSchedulerEnum::Serial(s) => Ok(DirectoryEnum2::A(s.wrap_for_merge(in_)?)),
      MergeSchedulerEnum::No(n) => Ok(DirectoryEnum2::B(n.wrap_for_merge(in_)?)),
      #[cfg(test)]
      MergeSchedulerEnum::SerialTest(s) => Ok(DirectoryEnum2::A(s.wrap_for_merge(in_)?)),
      #[cfg(test)]
      MergeSchedulerEnum::KnnMergeScheduler(s) => Ok(DirectoryEnum2::A(s.wrap_for_merge(in_)?)),
      #[cfg(test)]
      MergeSchedulerEnum::IndexWriterMerging(s) => Ok(DirectoryEnum2::A(s.wrap_for_merge(in_)?)),
    }
  }
}
