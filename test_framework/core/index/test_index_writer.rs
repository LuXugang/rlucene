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
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::field_type::FieldType;
use crate::core::document::string_field::StringField;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::concurrent_merge_scheduler::{
  ConcurrentMergeScheduler, ConcurrentMergeSchedulerBase, ConcurrentMergeSchedulerDefaults,
};
use crate::core::index::dummy::dummy_doc_map_sorter::DummyDocMap;
use crate::core::index::index_writer::{IndexWriter, Inner};
use crate::core::index::merge_policy::{
  MergeReader, MergeStat, OneMerge, OneMergeBase, OneMergeDefaults, OneMergeHook, OneMergeSR,
};
use crate::core::index::merge_scheduler::MergeSource;
use crate::core::index::one_merge_wrapping_merge_policy::OneMergeUnaryOperatorBase;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::test_concurrent_merge_scheduler::CountDownLatch;
use crate::test_framework::core::util::lucene_test_case::{
  new_field, new_index_writer_config_with_analyzer, new_text_field, random,
};
use rand::Rng;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
pub static STORED_TEXT_TYPE: LazyLock<FieldType> = LazyLock::new(|| {
  FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)
    .expect("should not fail")
});
#[allow(dead_code)]
struct TestIndexWriter;

#[derive(Clone)]
pub struct CloseWhileMergeIsRunningConcurrentMergeScheduler {
  merge_started: CountDownLatch,
  close_started: CountDownLatch,
}

impl CloseWhileMergeIsRunningConcurrentMergeScheduler {
  pub fn new(merge_started: CountDownLatch, close_started: CountDownLatch) -> Self {
    Self {
      merge_started,
      close_started,
    }
  }
}

impl ConcurrentMergeSchedulerBase for CloseWhileMergeIsRunningConcurrentMergeScheduler {
  fn close(&self, _scheduler: &ConcurrentMergeScheduler) -> Result<()> {
    Ok(())
  }

  fn do_merge<MS, D>(
    &self,
    scheduler: &ConcurrentMergeScheduler,
    merge_source: &MS,
    merge: OneMerge<D, MS::Reader>,
  ) -> Result<()>
  where
    MS: MergeSource<D>,
    D: Directory + 'static,
  {
    self.merge_started.count_down();
    self.close_started.wait();
    ConcurrentMergeSchedulerDefaults::do_merge(scheduler, merge_source, merge)
  }
}

pub(crate) fn add_doc<D, R>(
  random: &mut R,
  writer: &IndexWriter<D>,
  field_types: &mut HashMap<String, FieldType>,
) -> crate::core::util::error::lucene_error::Result<()>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "content",
    "aaa",
    Store::No,
    field_types,
  )?);
  let _ = writer.add_document(doc)?;
  Ok(())
}
pub(crate) fn add_doc_with_index<D, R>(
  random: &mut R,
  writer: &IndexWriter<D>,
  index: i32,
  field_types: &mut HashMap<String, FieldType>,
) -> crate::core::util::error::lucene_error::Result<()>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(new_field(
    random,
    "content",
    format!("aaa {}", index),
    &STORED_TEXT_TYPE,
    field_types,
  )?);
  doc.add(StringField::from_string(
    "id",
    index.to_string(),
    Store::No,
  )?);

  match writer.add_document(doc) {
    Ok(_) => Ok(()),
    Err(e) => Err(e),
  }
}

pub(crate) fn assert_no_unreferenced_files<D>(
  dir: Arc<D>,
  message: &str,
) -> crate::core::util::error::lucene_error::Result<()>
where
  D: Directory + 'static,
{
  let mut start_files = dir.list_all()?;
  let mut random = random();
  let mock = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, mock)?,
  )?;
  writer.close()?;
  let mut end_files = dir.list_all()?;

  start_files.sort();
  end_files.sort();

  assert_eq!(
    start_files,
    end_files,
    "{}: before delete:\n    {}\n  after delete:\n    {}",
    message,
    start_files.join("\n    "),
    end_files.join("\n    ")
  );

  Ok(())
}

#[derive(Clone)]
pub struct MergeFinishedOnceOneMergeUnaryOperator;

impl<D> OneMergeUnaryOperatorBase<D> for MergeFinishedOnceOneMergeUnaryOperator
where
  D: Directory,
{
  fn apply(&self, merge: OneMergeSR<D>) -> Result<OneMergeSR<D>> {
    Ok(
      OneMerge::new(merge.segments)?.with_hook(OneMergeHook::MergeFinishedOnce(
        MergeFinishedOnceOneMerge::new(),
      )),
    )
  }
}

pub(crate) struct MergeFinishedOnceOneMerge<D, CR> {
  only_finish_once: AtomicBool,
  _marker: PhantomData<fn(D, CR)>,
}

impl<D, CR> MergeFinishedOnceOneMerge<D, CR> {
  fn new() -> Self {
    Self {
      only_finish_once: AtomicBool::new(false),
      _marker: PhantomData,
    }
  }
}

impl<D, CR> OneMergeBase<D, CR> for MergeFinishedOnceOneMerge<D, CR>
where
  D: Directory,
  CR: CodecReader,
{
  fn merge_finished(
    &self,
    inner: &mut Inner<D>,
    stat: &MergeStat,
    success: bool,
    segment_dropped: bool,
  ) -> Result<()> {
    OneMergeDefaults::merge_finished(inner, stat, success, segment_dropped)?;
    if self.only_finish_once.swap(true, Ordering::SeqCst) {
      return Err(LuceneError::illegal_state(
        "mergeFinished may only be called once",
      ));
    }
    Ok(())
  }

  fn wrap_for_merge(&self, reader: CR) -> Result<CR> {
    OneMergeDefaults::wrap_for_merge(reader)
  }

  fn reorder<CR1, D1>(&self, reader: &CR1, dir: D1) -> Result<Option<DummyDocMap>>
  where
    CR1: CodecReader,
    D1: Directory,
  {
    OneMergeDefaults::reorder(reader, dir)
  }

  fn set_merge_info(
    &self,
    stat: &MergeStat,
    merge_info: &mut Option<SegmentCommitInfo<D>>,
    info: SegmentCommitInfo<D>,
  ) {
    OneMergeDefaults::set_merge_info(stat, merge_info, info)
  }

  fn on_merge_complete(
    &self,
    inner: &mut Inner<D>,
    stat: &MergeStat,
    merge_info: &Option<SegmentCommitInfo<D>>,
    is_aborted: bool,
  ) -> Result<()> {
    OneMergeDefaults::on_merge_complete(inner, stat, merge_info, is_aborted)
  }

  fn init_merge_readers<F>(
    &self,
    merge_readers: &mut Vec<MergeReader<CR, CR::Bits>>,
    stat: &MergeStat,
    reader_factory: F,
  ) -> Result<()>
  where
    F: FnMut(&String) -> Result<MergeReader<CR, CR::Bits>>,
  {
    OneMergeDefaults::init_merge_readers(merge_readers, stat, reader_factory)
  }
}

#[derive(Clone)]
pub struct AbortOnMergeCompleteOneMergeUnaryOperator {
  abort_merge_before_commit: Arc<AtomicBool>,
}

impl AbortOnMergeCompleteOneMergeUnaryOperator {
  pub(crate) fn new(abort_merge_before_commit: Arc<AtomicBool>) -> Self {
    Self {
      abort_merge_before_commit,
    }
  }
}

impl<D> OneMergeUnaryOperatorBase<D> for AbortOnMergeCompleteOneMergeUnaryOperator
where
  D: Directory,
{
  fn apply(&self, merge: OneMergeSR<D>) -> Result<OneMergeSR<D>> {
    Ok(
      OneMerge::new(merge.segments)?.with_hook(OneMergeHook::AbortOnMergeComplete(
        AbortOnMergeCompleteOneMerge::new(self.abort_merge_before_commit.clone()),
      )),
    )
  }
}

pub(crate) struct AbortOnMergeCompleteOneMerge<D, CR> {
  abort_merge_before_commit: Arc<AtomicBool>,
  _marker: PhantomData<fn(D, CR)>,
}

impl<D, CR> AbortOnMergeCompleteOneMerge<D, CR> {
  fn new(abort_merge_before_commit: Arc<AtomicBool>) -> Self {
    Self {
      abort_merge_before_commit,
      _marker: PhantomData,
    }
  }
}

impl<D, CR> OneMergeBase<D, CR> for AbortOnMergeCompleteOneMerge<D, CR>
where
  D: Directory,
  CR: CodecReader,
{
  fn merge_finished(
    &self,
    inner: &mut Inner<D>,
    stat: &MergeStat,
    success: bool,
    segment_dropped: bool,
  ) -> Result<()> {
    OneMergeDefaults::merge_finished(inner, stat, success, segment_dropped)
  }

  fn wrap_for_merge(&self, reader: CR) -> Result<CR> {
    OneMergeDefaults::wrap_for_merge(reader)
  }

  fn reorder<CR1, D1>(&self, reader: &CR1, dir: D1) -> Result<Option<DummyDocMap>>
  where
    CR1: CodecReader,
    D1: Directory,
  {
    OneMergeDefaults::reorder(reader, dir)
  }

  fn set_merge_info(
    &self,
    stat: &MergeStat,
    merge_info: &mut Option<SegmentCommitInfo<D>>,
    info: SegmentCommitInfo<D>,
  ) {
    OneMergeDefaults::set_merge_info(stat, merge_info, info)
  }

  fn on_merge_complete(
    &self,
    inner: &mut Inner<D>,
    stat: &MergeStat,
    merge_info: &Option<SegmentCommitInfo<D>>,
    is_aborted: bool,
  ) -> Result<()> {
    OneMergeDefaults::on_merge_complete(inner, stat, merge_info, is_aborted)?;
    if self.abort_merge_before_commit.load(Ordering::SeqCst) {
      stat.set_aborted();
    }
    Ok(())
  }

  fn init_merge_readers<F>(
    &self,
    merge_readers: &mut Vec<MergeReader<CR, CR::Bits>>,
    stat: &MergeStat,
    reader_factory: F,
  ) -> Result<()>
  where
    F: FnMut(&String) -> Result<MergeReader<CR, CR::Bits>>,
  {
    OneMergeDefaults::init_merge_readers(merge_readers, stat, reader_factory)
  }
}
