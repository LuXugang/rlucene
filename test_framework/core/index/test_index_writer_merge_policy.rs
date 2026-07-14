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
use crate::core::index::merge_policy::OneMerge;
use crate::core::index::merge_scheduler::{MergeScheduler, MergeSource};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::store::directory::Directory;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[allow(dead_code)] // for quick search
struct TestIndexWriterMergePolicy;

#[derive(Clone)]
pub struct TestLatch {
  inner: Arc<(Mutex<bool>, Condvar)>,
}

impl TestLatch {
  pub fn new() -> Self {
    Self {
      inner: Arc::new((Mutex::new(false), Condvar::new())),
    }
  }

  pub fn count_down(&self) {
    let (lock, cvar) = &*self.inner;
    *lock.lock().expect("test latch mutex poisoned") = true;
    cvar.notify_all();
  }

  pub fn wait(&self) {
    let (lock, cvar) = &*self.inner;
    let mut signaled = lock.lock().expect("test latch mutex poisoned");
    while !*signaled {
      signaled = cvar.wait(signaled).expect("test latch mutex poisoned");
    }
  }

  pub fn wait_timeout(&self, timeout: Duration) -> bool {
    let (lock, cvar) = &*self.inner;
    let signaled = lock.lock().expect("test latch mutex poisoned");
    let (signaled, _) = cvar
      .wait_timeout_while(signaled, timeout, |signaled| !*signaled)
      .expect("test latch mutex poisoned");
    *signaled
  }
}

impl Default for TestLatch {
  fn default() -> Self {
    Self::new()
  }
}

pub struct LatchedSerialMergeScheduler {
  merge_started: TestLatch,
  merge_released: TestLatch,
  base: SerialMergeScheduler,
}

impl LatchedSerialMergeScheduler {
  pub fn new(merge_started: TestLatch, merge_released: TestLatch) -> Self {
    Self {
      merge_started,
      merge_released,
      base: SerialMergeScheduler::new(),
    }
  }
}

impl CloseableRef for LatchedSerialMergeScheduler {
  fn close(&self) -> Result<()> {
    self.base.close()
  }
}

impl MergeScheduler for LatchedSerialMergeScheduler {
  fn merge<MS, D>(&self, merge_source: MS, trigger: MergeTrigger) -> Result<()>
  where
    MS: MergeSource<D> + Clone + 'static,
    D: Directory + 'static,
    OneMerge<D, MS::Reader>: Send + 'static,
  {
    self.merge_started.count_down();
    self.merge_released.wait();
    self.base.merge(merge_source, trigger)
  }

  type Directory<D>
    = <SerialMergeScheduler as MergeScheduler>::Directory<D>
  where
    D: Directory;

  fn wrap_for_merge<D>(&self, in_: D) -> Result<Self::Directory<D>>
  where
    D: Directory,
  {
    self.base.wrap_for_merge(in_)
  }

  fn initialize<D>(&mut self, directory: &D) -> Result<()>
  where
    D: Directory,
  {
    self.base.initialize(directory)
  }
}
