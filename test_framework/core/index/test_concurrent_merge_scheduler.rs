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
use crate::core::index::concurrent_merge_scheduler::{
  ConcurrentMergeScheduler, ConcurrentMergeSchedulerBase, ConcurrentMergeSchedulerDefaults, Inner,
  MergeThread,
};
use crate::core::index::merge_policy::OneMerge;
use crate::core::index::merge_policy::{
  DefaultMergeSpecification, MergeContext, MergePolicy, MergePolicyBase, size,
};
use crate::core::index::merge_scheduler::MergeSource;
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::tiered_merge_policy::SegmentDocAndID;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::MutexGuard;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

#[allow(dead_code)] // for quick search
struct TestConcurrentMergeScheduler;

#[derive(Clone)]
pub struct CountDownLatch {
  inner: Arc<(Mutex<usize>, Condvar)>,
}

impl CountDownLatch {
  pub fn new(count: usize) -> Self {
    Self {
      inner: Arc::new((Mutex::new(count), Condvar::new())),
    }
  }

  pub fn get_count(&self) -> usize {
    *self.inner.0.lock().expect("latch mutex poisoned")
  }

  pub fn count_down(&self) {
    let (lock, condvar) = &*self.inner;
    let mut count = lock.lock().expect("latch mutex poisoned");
    if *count > 0 {
      *count -= 1;
      if *count == 0 {
        condvar.notify_all();
      }
    }
  }

  pub fn wait(&self) {
    let (lock, condvar) = &*self.inner;
    let mut count = lock.lock().expect("latch mutex poisoned");
    while *count > 0 {
      count = condvar.wait(count).expect("latch mutex poisoned");
    }
  }

  pub fn wait_timeout(&self, timeout: Duration) -> bool {
    let (lock, condvar) = &*self.inner;
    let count = lock.lock().expect("latch mutex poisoned");
    let (count, _) = condvar
      .wait_timeout_while(count, timeout, |count| *count > 0)
      .expect("latch mutex poisoned");
    *count == 0
  }
}

impl Default for CountDownLatch {
  fn default() -> Self {
    Self::new(0)
  }
}

#[derive(Clone, Default)]
pub struct MergeThreadMessagesConcurrentMergeScheduler {
  merge_thread_names: Arc<Mutex<HashSet<String>>>,
}

impl MergeThreadMessagesConcurrentMergeScheduler {
  pub fn merge_thread_names(&self) -> HashSet<String> {
    self
      .merge_thread_names
      .lock()
      .expect("merge thread names mutex poisoned")
      .clone()
  }
}

impl ConcurrentMergeSchedulerBase for MergeThreadMessagesConcurrentMergeScheduler {
  fn get_merge_thread<MS, D>(
    &self,
    scheduler: &ConcurrentMergeScheduler,
    inner: &mut Inner,
    merge_source: MS,
    merge: OneMerge<D, MS::Reader>,
  ) -> Result<MergeThread<MS, D>>
  where
    MS: MergeSource<D> + Clone + 'static,
    D: Directory + 'static,
    OneMerge<D, MS::Reader>: Send + 'static,
  {
    let merge_thread =
      ConcurrentMergeSchedulerDefaults::get_merge_thread(scheduler, inner, merge_source, merge)?;
    self
      .merge_thread_names
      .lock()
      .expect("merge thread names mutex poisoned")
      .insert(merge_thread.name().to_string());
    Ok(merge_thread)
  }
}

#[derive(Clone)]
pub struct MaxMergeCountConcurrentMergeScheduler {
  max_merge_count: i32,
  enough_merges_waiting: CountDownLatch,
  running_merge_count: Arc<AtomicI32>,
  failed: Arc<AtomicBool>,
}

impl MaxMergeCountConcurrentMergeScheduler {
  pub fn new(max_merge_count: i32) -> Self {
    Self {
      max_merge_count,
      enough_merges_waiting: CountDownLatch::new(max_merge_count as usize),
      running_merge_count: Arc::new(AtomicI32::new(0)),
      failed: Arc::new(AtomicBool::new(false)),
    }
  }

  pub fn enough_merges_waiting(&self) -> &CountDownLatch {
    &self.enough_merges_waiting
  }

  pub fn failed(&self) -> bool {
    self.failed.load(Ordering::SeqCst)
  }
}

impl ConcurrentMergeSchedulerBase for MaxMergeCountConcurrentMergeScheduler {
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
    let merge_stat = merge.stat.clone();
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
      // Stall all incoming merges until we see
      // max_merge_count:
      let count = self.running_merge_count.fetch_add(1, Ordering::SeqCst) + 1;
      let merge_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
        assert!(
          count <= self.max_merge_count,
          "count={count} vs maxMergeCount={}",
          self.max_merge_count
        );
        self.enough_merges_waiting.count_down();

        // Stall this merge until we see exactly
        // max_merge_count merges waiting
        while !self
          .enough_merges_waiting
          .wait_timeout(Duration::from_millis(10))
          && !self.failed.load(Ordering::SeqCst)
        {}
        // Then sleep a bit to give a chance for the bug
        // (too many pending merges) to appear:
        thread::sleep(Duration::from_millis(20));
        ConcurrentMergeSchedulerDefaults::do_merge(scheduler, merge_source, merge)
      }));
      self.running_merge_count.fetch_sub(1, Ordering::SeqCst);
      match merge_result {
        Ok(result) => result,
        Err(payload) => resume_unwind(payload),
      }
    }));

    match result {
      Ok(Ok(())) => Ok(()),
      Ok(Err(error)) => {
        self.failed.store(true, Ordering::SeqCst);
        merge_source.on_merge_finished(&merge_stat, None);
        let mut runtime_error = LuceneError::illegal_state(error.to_string());
        runtime_error.add_suppressed(error);
        Err(runtime_error)
      },
      Err(payload) => {
        self.failed.store(true, Ordering::SeqCst);
        merge_source.on_merge_finished(&merge_stat, None);
        Err(LuceneError::illegal_state(
          LuceneError::panic_payload_message(payload.as_ref()),
        ))
      },
    }
  }
}

#[derive(Clone)]
pub struct TrackingConcurrentMergeScheduler {
  total_merged_bytes: Arc<AtomicI64>,
  at_least_one_merge: CountDownLatch,
}

impl TrackingConcurrentMergeScheduler {
  pub fn new(at_least_one_merge: CountDownLatch) -> Self {
    Self {
      total_merged_bytes: Arc::new(AtomicI64::new(0)),
      at_least_one_merge,
    }
  }

  pub fn total_merged_bytes(&self) -> i64 {
    self.total_merged_bytes.load(Ordering::Relaxed)
  }
}

impl ConcurrentMergeSchedulerBase for TrackingConcurrentMergeScheduler {
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
    self
      .total_merged_bytes
      .fetch_add(merge.total_bytes_size(), Ordering::Relaxed);
    self.at_least_one_merge.count_down();
    ConcurrentMergeSchedulerDefaults::do_merge(scheduler, merge_source, merge)
  }
}

#[derive(Clone, Default)]
pub struct LiveMaxMergeCountMergePolicy {
  base: MergePolicyBase,
}

impl Display for LiveMaxMergeCountMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "LiveMaxMergeCountMergePolicy")
  }
}

impl<D> MergePolicy<D> for LiveMaxMergeCountMergePolicy
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
    _inner: Option<&crate::core::index::index_writer::Inner<D>>,
    _merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    // no natural merges
    Ok(None)
  }

  fn find_forced_merges<MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    _max_segment_count: usize,
    segments_to_merge: &HashMap<String, Option<bool>>,
    _inner: Option<&crate::core::index::index_writer::Inner<D>>,
    _merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    // The test is about testing that CMS bounds the number of merging threads, so we just return
    // many merges.
    let mut spec = DefaultMergeSpecification::new();
    let mut one_merge = Vec::new();
    for info in segment_infos.iter() {
      if !segments_to_merge.contains_key(info.info.get_id_key()) {
        continue;
      }
      one_merge.push(SegmentDocAndID::new(
        info.info.get_id_key().to_string(),
        info.info.max_doc()?,
      ));
      if one_merge.len() >= 10 {
        spec.add(OneMerge::new(std::mem::take(&mut one_merge))?);
      }
    }
    Ok(Some(spec))
  }

  fn find_forced_deletes_merges<MC>(
    &self,
    _segment_infos: &SegmentInfos<D>,
    _inner: Option<&crate::core::index::index_writer::Inner<D>>,
    _merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    // not needed
    Ok(None)
  }

  fn size<MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    MC: MergeContext<D>,
  {
    size(info, merge_context)
  }
}

#[derive(Clone, Default)]
pub struct LiveMaxMergeCountConcurrentMergeScheduler {
  running_merge_count: Arc<AtomicI32>,
  max_running_merge_count: Arc<AtomicI32>,
  monitor: Arc<Mutex<()>>,
}

impl LiveMaxMergeCountConcurrentMergeScheduler {
  pub fn max_running_merge_count(&self) -> i32 {
    self.max_running_merge_count.load(Ordering::SeqCst)
  }

  pub fn reset_max_running_merge_count(&self) {
    self.max_running_merge_count.store(0, Ordering::SeqCst);
  }
}

impl ConcurrentMergeSchedulerBase for LiveMaxMergeCountConcurrentMergeScheduler {
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
    let count = self.running_merge_count.fetch_add(1, Ordering::SeqCst) + 1;
    // evil?
    {
      let _monitor = self.monitor.lock().expect("scheduler monitor poisoned");
      if count > self.max_running_merge_count.load(Ordering::SeqCst) {
        self.max_running_merge_count.store(count, Ordering::SeqCst);
      }
    }
    let merge_result = catch_unwind(AssertUnwindSafe(|| {
      ConcurrentMergeSchedulerDefaults::do_merge(scheduler, merge_source, merge)
    }));
    self.running_merge_count.fetch_sub(1, Ordering::SeqCst);
    match merge_result {
      Ok(result) => result,
      Err(payload) => resume_unwind(payload),
    }
  }
}

#[derive(Clone, Default)]
pub struct MaybeStallCalledConcurrentMergeScheduler {
  was_called: Arc<AtomicBool>,
}

impl MaybeStallCalledConcurrentMergeScheduler {
  pub fn was_called(&self) -> bool {
    self.was_called.load(Ordering::SeqCst)
  }
}

impl ConcurrentMergeSchedulerBase for MaybeStallCalledConcurrentMergeScheduler {
  fn maybe_stall<MS, D>(
    &self,
    _scheduler: &ConcurrentMergeScheduler,
    _inner: &mut MutexGuard<'_, Inner>,
    _merge_source: &MS,
  ) -> Result<bool>
  where
    MS: MergeSource<D>,
    D: Directory,
  {
    self.was_called.store(true, Ordering::SeqCst);
    Ok(true)
  }
}

#[derive(Clone)]
pub struct HangDuringRollbackConcurrentMergeScheduler {
  merge_start: CountDownLatch,
  merge_finish: CountDownLatch,
}

impl HangDuringRollbackConcurrentMergeScheduler {
  pub fn new(merge_start: CountDownLatch, merge_finish: CountDownLatch) -> Self {
    Self {
      merge_start,
      merge_finish,
    }
  }
}

impl ConcurrentMergeSchedulerBase for HangDuringRollbackConcurrentMergeScheduler {
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
    self.merge_start.count_down();
    self.merge_finish.wait();
    ConcurrentMergeSchedulerDefaults::do_merge(scheduler, merge_source, merge)
  }
}

#[derive(Clone, Default)]
pub struct NoStallMergeThreadsConcurrentMergeScheduler {
  failed: Arc<AtomicBool>,
}

impl NoStallMergeThreadsConcurrentMergeScheduler {
  pub fn failed(&self) -> bool {
    self.failed.load(Ordering::SeqCst)
  }
}

impl ConcurrentMergeSchedulerBase for NoStallMergeThreadsConcurrentMergeScheduler {
  fn do_stall(&self, scheduler: &ConcurrentMergeScheduler, inner: &mut MutexGuard<'_, Inner>) {
    if thread::current()
      .name()
      .is_some_and(|name| name.starts_with("Lucene Merge Thread"))
    {
      self.failed.store(true, Ordering::SeqCst);
    }
    ConcurrentMergeSchedulerDefaults::do_stall(scheduler, inner);
  }
}
