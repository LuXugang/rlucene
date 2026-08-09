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
use crate::core::index::dummy::dummy_codec_reader::DummyCodecReader;
use crate::core::index::index_writer::Inner;
use crate::core::index::merge_policy::{MergeStat, OneMerge};
use crate::core::index::merge_scheduler::{MergeScheduler, MergeSource};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_scheduler::NoMergeScheduler as NoMergeSchedulerImpl;
use crate::core::store::dummy::dummy_directory::DummyDirectory;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::lucene_test_case::random;

#[allow(dead_code)] // for quick search
struct TestNoMergeScheduler;

#[derive(Clone)]
struct UnreachableMergeSource;

impl MergeSource<DummyDirectory> for UnreachableMergeSource {
  type Reader = DummyCodecReader;

  fn get_next_merge(&self) -> Result<Option<OneMerge<DummyDirectory, Self::Reader>>> {
    panic!("NoMergeScheduler must not request a merge")
  }

  fn on_merge_finished(&self, _merge: &MergeStat, _inner: Option<&mut Inner<DummyDirectory>>) {
    panic!("NoMergeScheduler must not finish a merge")
  }

  fn has_pending_merges(&self, _inner: Option<&mut Inner<DummyDirectory>>) -> Result<bool> {
    panic!("NoMergeScheduler must not inspect pending merges")
  }

  fn merge(&self, _merge: OneMerge<DummyDirectory, Self::Reader>) -> Result<()> {
    panic!("NoMergeScheduler must not execute a merge")
  }
}

#[test]
fn test_no_merge_scheduler() -> Result<()> {
  let mut random = random();
  let merge_scheduler = NoMergeSchedulerImpl::new();
  merge_scheduler.close()?;
  merge_scheduler.merge::<UnreachableMergeSource, DummyDirectory>(
    UnreachableMergeSource,
    MergeTrigger::random_trigger(&mut random),
  )?;
  Ok(())
}

#[test]
#[ignore = "Java-only: final-class, private-constructor, and singleton modifiers are checked by Java reflection"]
fn test_final_singleton() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust trait implementation completeness is checked statically"]
fn test_methods_overridden() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
