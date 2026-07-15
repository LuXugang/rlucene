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
use crate::core::index::concurrent_merge_scheduler::ConcurrentMergeScheduler;
use crate::core::index::index_writer::Inner;
use crate::core::index::merge_policy::{MergeStat, OneMerge};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_scheduler::NoMergeScheduler;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::store::directory::{Directory, DirectoryEnum3};
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::info_stream::InfoStreamMT;
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test_framework::core::index::base_knn_vectors_format_test_case::TestMergeScheduler;
#[cfg(test)]
use crate::test_framework::core::index::base_merge_policy_test_case::SerialMergeSchedulerImpl;
#[cfg(test)]
use crate::test_framework::core::index::test_add_indexes::{
  CountingSerialMergeScheduler, PartialMergeScheduler,
};
#[cfg(test)]
use crate::test_framework::core::index::test_index_writer_merge_policy::LatchedSerialMergeScheduler;
#[cfg(test)]
use crate::test_framework::core::index::test_index_writer_merging::MyMergeScheduler;

/// Expert: [IndexWriter] uses an instance implementing this
/// trait to execute the merges selected by a [MergePolicy].
/// The default MergeScheduler is [ConcurrentMergeScheduler].
///
/// @lucene.experimental
pub trait MergeScheduler: CloseableRef {
  /// Run the merges provided by [MergeSource::get_next_merge()].
  ///
  /// * `merge_source` - the [IndexWriter] to obtain the merges from.
  /// * `trigger` - the [MergeTrigger] that caused this merge to happen
  fn merge<MS, D>(&self, merge_source: MS, trigger: MergeTrigger) -> Result<()>
  where
    MS: MergeSource<D> + Clone + 'static,
    D: Directory + 'static,
    OneMerge<D, MS::Reader>: Send + 'static;
  type Directory<D>: Directory
  where
    D: Directory;
  /// Wraps the incoming [Directory] so that we can
  /// merge-throttle it using [RateLimitedIndexOutput].
  fn wrap_for_merge<D>(&self, _in_: D) -> Result<Self::Directory<D>>
  where
    D: Directory;

  /// [IndexWriter] calls this on init.
  fn initialize<D>(&mut self, _info_stream: InfoStreamMT, _directory: &D) -> Result<()>
  where
    D: Directory,
  {
    Ok(())
  }
}

/// Provides access to new merges and executes the actual merge
pub trait MergeSource<D>: Send
where
  D: Directory,
{
  type Reader: CodecReader;

  /// The `MergeScheduler` calls this method to retrieve the next merge
  /// requested by the `MergePolicy`.
  fn get_next_merge(&self) -> Result<Option<OneMerge<D, Self::Reader>>>;

  /// Does finishing for a merge.
  fn on_merge_finished(&self, merge: &MergeStat, inner: Option<&mut Inner<D>>);

  /// Expert: returns true if there are merges waiting to be scheduled.
  fn has_pending_merges(&self, inner: Option<&mut Inner<D>>) -> Result<bool>;

  /// Merges the indicated segments, replacing them in the stack
  /// with a single segment.
  fn merge(&self, merge: OneMerge<D, Self::Reader>) -> Result<()>
  where
    D: 'static;
}

pub enum MergeSchedulerEnum {
  Serial(SerialMergeScheduler),
  No(NoMergeScheduler),
  Concurrent(ConcurrentMergeScheduler),
  #[cfg(test)]
  SerialTest(SerialMergeSchedulerImpl),
  #[cfg(test)]
  LatchedSerial(LatchedSerialMergeScheduler),
  #[cfg(test)]
  KnnMergeScheduler(TestMergeScheduler),
  #[cfg(test)]
  IndexWriterMerging(MyMergeScheduler),
  #[cfg(test)]
  PartialAddIndexes(PartialMergeScheduler),
  #[cfg(test)]
  CountingAddIndexes(CountingSerialMergeScheduler),
}
impl_from_for_enum!(
    MergeSchedulerEnum,
    SerialMergeScheduler => Serial,
    NoMergeScheduler => No,
    ConcurrentMergeScheduler => Concurrent,
);
impl Default for MergeSchedulerEnum {
  fn default() -> Self {
    Self::Concurrent(ConcurrentMergeScheduler::new())
  }
}

impl CloseableRef for MergeSchedulerEnum {
  fn close(&self) -> Result<()> {
    match self {
      MergeSchedulerEnum::Serial(s) => s.close(),
      MergeSchedulerEnum::No(n) => n.close(),
      MergeSchedulerEnum::Concurrent(c) => c.close(),
      #[cfg(test)]
      MergeSchedulerEnum::SerialTest(s) => s.close(),
      #[cfg(test)]
      MergeSchedulerEnum::LatchedSerial(s) => s.close(),
      #[cfg(test)]
      MergeSchedulerEnum::KnnMergeScheduler(s) => s.close(),
      #[cfg(test)]
      MergeSchedulerEnum::IndexWriterMerging(s) => s.close(),
      #[cfg(test)]
      MergeSchedulerEnum::PartialAddIndexes(s) => s.close(),
      #[cfg(test)]
      MergeSchedulerEnum::CountingAddIndexes(s) => s.close(),
    }
  }
}

impl MergeScheduler for MergeSchedulerEnum {
  fn merge<MS, D>(&self, merge_source: MS, trigger: MergeTrigger) -> Result<()>
  where
    MS: MergeSource<D> + Clone + 'static,
    D: Directory + 'static,
    OneMerge<D, MS::Reader>: Send + 'static,
  {
    match self {
      MergeSchedulerEnum::Serial(s) => s.merge(merge_source, trigger),
      MergeSchedulerEnum::No(n) => n.merge(merge_source, trigger),
      MergeSchedulerEnum::Concurrent(c) => c.merge(merge_source, trigger),
      #[cfg(test)]
      MergeSchedulerEnum::SerialTest(s) => s.merge(merge_source, trigger),
      #[cfg(test)]
      MergeSchedulerEnum::LatchedSerial(s) => s.merge(merge_source, trigger),
      #[cfg(test)]
      MergeSchedulerEnum::KnnMergeScheduler(s) => s.merge(merge_source, trigger),
      #[cfg(test)]
      MergeSchedulerEnum::IndexWriterMerging(s) => s.merge(merge_source, trigger),
      #[cfg(test)]
      MergeSchedulerEnum::PartialAddIndexes(s) => s.merge(merge_source, trigger),
      #[cfg(test)]
      MergeSchedulerEnum::CountingAddIndexes(s) => s.merge(merge_source, trigger),
    }
  }

  type Directory<D>
    = DirectoryEnum3<
    <SerialMergeScheduler as MergeScheduler>::Directory<D>,
    <NoMergeScheduler as MergeScheduler>::Directory<D>,
    <ConcurrentMergeScheduler as MergeScheduler>::Directory<D>,
  >
  where
    D: Directory;

  fn wrap_for_merge<D>(&self, in_: D) -> Result<Self::Directory<D>>
  where
    D: Directory,
  {
    match self {
      MergeSchedulerEnum::Serial(s) => Ok(DirectoryEnum3::A(s.wrap_for_merge(in_)?)),
      MergeSchedulerEnum::No(n) => Ok(DirectoryEnum3::B(n.wrap_for_merge(in_)?)),
      MergeSchedulerEnum::Concurrent(c) => Ok(DirectoryEnum3::C(c.wrap_for_merge(in_)?)),
      #[cfg(test)]
      MergeSchedulerEnum::SerialTest(s) => Ok(DirectoryEnum3::A(s.wrap_for_merge(in_)?)),
      #[cfg(test)]
      MergeSchedulerEnum::LatchedSerial(s) => Ok(DirectoryEnum3::A(s.wrap_for_merge(in_)?)),
      #[cfg(test)]
      MergeSchedulerEnum::KnnMergeScheduler(s) => Ok(DirectoryEnum3::A(s.wrap_for_merge(in_)?)),
      #[cfg(test)]
      MergeSchedulerEnum::IndexWriterMerging(s) => Ok(DirectoryEnum3::A(s.wrap_for_merge(in_)?)),
      #[cfg(test)]
      MergeSchedulerEnum::PartialAddIndexes(s) => Ok(DirectoryEnum3::A(s.wrap_for_merge(in_)?)),
      #[cfg(test)]
      MergeSchedulerEnum::CountingAddIndexes(s) => Ok(DirectoryEnum3::A(s.wrap_for_merge(in_)?)),
    }
  }

  fn initialize<D>(&mut self, info_stream: InfoStreamMT, directory: &D) -> Result<()>
  where
    D: Directory,
  {
    match self {
      MergeSchedulerEnum::Serial(s) => s.initialize(info_stream, directory),
      MergeSchedulerEnum::No(n) => n.initialize(info_stream, directory),
      MergeSchedulerEnum::Concurrent(c) => c.initialize(info_stream, directory),
      #[cfg(test)]
      MergeSchedulerEnum::SerialTest(s) => s.initialize(info_stream, directory),
      #[cfg(test)]
      MergeSchedulerEnum::LatchedSerial(s) => s.initialize(info_stream, directory),
      #[cfg(test)]
      MergeSchedulerEnum::KnnMergeScheduler(s) => s.initialize(info_stream, directory),
      #[cfg(test)]
      MergeSchedulerEnum::IndexWriterMerging(s) => s.initialize(info_stream, directory),
      #[cfg(test)]
      MergeSchedulerEnum::PartialAddIndexes(s) => s.initialize(info_stream, directory),
      #[cfg(test)]
      MergeSchedulerEnum::CountingAddIndexes(s) => s.initialize(info_stream, directory),
    }
  }
}
