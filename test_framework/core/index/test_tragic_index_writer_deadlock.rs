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
use parking_lot::MutexGuard;

use crate::core::index::concurrent_merge_scheduler::{
  ConcurrentMergeScheduler, ConcurrentMergeSchedulerBase, ConcurrentMergeSchedulerDefaults, Inner,
};
use crate::core::index::index_writer::IndexWriterHooks;
use crate::core::index::merge_policy::{MergeStat, OneMerge};
use crate::core::index::merge_scheduler::MergeSource;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{CaughtResult, Result};
use crate::test_framework::core::index::test_concurrent_merge_scheduler::CountDownLatch;

#[allow(dead_code)] // for quick search
struct TestTragicIndexWriterDeadlock;

#[derive(Clone)]
pub struct StalledMergesConcurrentMergeScheduler {
  done: CountDownLatch,
}

impl StalledMergesConcurrentMergeScheduler {
  pub fn new(done: CountDownLatch) -> Self {
    Self { done }
  }
}

impl ConcurrentMergeSchedulerBase for StalledMergesConcurrentMergeScheduler {
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
    // Let the merge take forever, until the commit thread is stalled.
    self.done.wait();
    ConcurrentMergeSchedulerDefaults::do_merge(scheduler, merge_source, merge)
  }

  fn do_stall(&self, scheduler: &ConcurrentMergeScheduler, inner: &mut MutexGuard<'_, Inner>) {
    self.done.count_down();
    ConcurrentMergeSchedulerDefaults::do_stall(scheduler, inner);
  }

  fn handle_merge_exception(
    &self,
    _scheduler: &ConcurrentMergeScheduler,
    _result: CaughtResult,
  ) -> Result<()> {
    Ok(())
  }
}

pub struct TragicIndexWriter;

impl IndexWriterHooks for TragicIndexWriter {
  fn merge_success(&self, _merge: &MergeStat) -> Result<()> {
    // Tragedy strikes!
    panic!("OutOfMemoryError");
  }
}
