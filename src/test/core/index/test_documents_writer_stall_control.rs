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
use crate::core::index::documents_writer_stall_control::DocumentsWriterStallControl;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, at_least_usize, is_night_mode, random,
};
use parking_lot::{Condvar, Mutex};
use rand::RngExt;
use rand::rng;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::{JoinHandle, ThreadId};
use std::time::{Duration, Instant};

#[allow(dead_code)] // for quick search
struct TestDocumentsWriterStallControl;

#[test]
fn test_simple_stall() -> Result<()> {
  let mut random = random();
  let ctrl = Arc::new(DocumentsWriterStallControl::new());

  ctrl.update_stalled(false);
  let mut wait_thread_handles = wait_threads(at_least(&mut random, 1), ctrl.clone());
  start(&wait_thread_handles);
  assert!(!ctrl.has_blocked());
  assert!(!ctrl.any_stalled_threads());
  join(wait_thread_handles);

  // now stall threads and wake them up again
  ctrl.update_stalled(true);
  wait_thread_handles = wait_threads(at_least(&mut random, 1), ctrl.clone());
  start(&wait_thread_handles);
  await_state(wait_thread_handles.len() as i32, &ctrl);
  assert!(ctrl.has_blocked());
  assert!(ctrl.any_stalled_threads());
  ctrl.update_stalled(false);
  assert!(!ctrl.any_stalled_threads());
  join(wait_thread_handles);
  Ok(())
}

#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let ctrl = Arc::new(DocumentsWriterStallControl::new());
  ctrl.update_stalled(false);

  let mut stall_threads = Vec::new();
  for _ in 0..at_least_usize(&mut random, 3) {
    let ctrl = ctrl.clone();
    let stall_probability = 1 + random.random_range(0..10);
    stall_threads.push(thread::spawn(move || {
      let mut random = rng();
      let iters = at_least(&mut random, 100);
      for _ in 0..iters {
        ctrl.update_stalled(random.random_range(0..stall_probability) == 0);
        if random.random_range(0..5) == 0 {
          ctrl.wait_if_stalled();
        }
      }
    }));
  }
  start(&stall_threads);
  /*
   * use a 100 maximum iterations check to make sure we not hang forever. join will fail in
   * that case
   */
  let mut iterations = 0;
  while {
    iterations += 1;
    iterations < 100 && !terminated(&stall_threads)
  } {
    ctrl.update_stalled(false);
    if random.random_bool(0.5) {
      thread::yield_now();
    } else {
      thread::sleep(Duration::from_millis(1));
    }
  }
  join(stall_threads);
  Ok(())
}

#[test]
fn test_acquire_release_race() -> Result<()> {
  let mut random = random();
  let ctrl = Arc::new(DocumentsWriterStallControl::new());
  ctrl.update_stalled(false);
  let stop = Arc::new(AtomicBool::new(false));
  let check_point = Arc::new(AtomicBool::new(true));

  let num_stallers = at_least_usize(&mut random, 1);
  let num_releasers = at_least_usize(&mut random, 1);
  let num_waiters = at_least_usize(&mut random, 1);
  let sync = Arc::new(Synchronizer::new(
    num_stallers + num_releasers,
    num_stallers + num_releasers + num_waiters,
  ));
  let exceptions = Arc::new(Mutex::new(Vec::new()));
  let mut threads = Vec::with_capacity(num_releasers + num_stallers + num_waiters);
  for _ in 0..num_releasers {
    threads.push(Updater::new(
      stop.clone(),
      check_point.clone(),
      ctrl.clone(),
      sync.clone(),
      true,
      exceptions.clone(),
    ));
  }
  for _ in num_releasers..num_releasers + num_stallers {
    threads.push(Updater::new(
      stop.clone(),
      check_point.clone(),
      ctrl.clone(),
      sync.clone(),
      false,
      exceptions.clone(),
    ));
  }
  for _ in num_releasers + num_stallers..num_releasers + num_stallers + num_waiters {
    threads.push(Waiter::new(
      stop.clone(),
      check_point.clone(),
      ctrl.clone(),
      sync.clone(),
      exceptions.clone(),
    ));
  }

  start(&threads);
  let iters = if is_night_mode() {
    at_least(&mut random, 10000)
  } else {
    at_least(&mut random, 1000)
  };
  let check_point_probability = if is_night_mode() { 0.5 } else { 0.1 };
  for _ in 0..iters {
    if check_point.load(Ordering::SeqCst) {
      assert!(
        sync.await_update_join(Duration::from_secs(10)),
        "timed out waiting for update threads - deadlock?"
      );
      if !exceptions.lock().is_empty() {
        unreachable!("got exceptions in threads: {:?}", exceptions.lock());
      }

      if ctrl.has_blocked() && ctrl.is_healthy() {
        assert_state(num_releasers, num_stallers, num_waiters, &threads, &ctrl);
      }

      check_point.store(false, Ordering::SeqCst);
      sync.waiter_count_down();
      sync.await_left_checkpoint();
    }
    assert!(!check_point.load(Ordering::SeqCst));
    assert_eq!(0, sync.waiter_get_count());
    if check_point_probability >= random.random::<f32>() {
      sync.reset(
        num_stallers + num_releasers,
        num_stallers + num_releasers + num_waiters,
      );
      check_point.store(true, Ordering::SeqCst);
    }
  }
  if !check_point.load(Ordering::SeqCst) {
    sync.reset(
      num_stallers + num_releasers,
      num_stallers + num_releasers + num_waiters,
    );
    check_point.store(true, Ordering::SeqCst);
  }

  assert!(sync.await_update_join(Duration::from_secs(10)));
  assert_state(num_releasers, num_stallers, num_waiters, &threads, &ctrl);
  check_point.store(false, Ordering::SeqCst);
  stop.store(true, Ordering::SeqCst);
  sync.waiter_count_down();
  sync.await_left_checkpoint();

  for thread in threads {
    ctrl.update_stalled(false);
    thread.join(Duration::from_secs(2));
    if thread.is_alive() && thread.is_waiter() && thread.is_queued(&ctrl) {
      unreachable!(
        "waiter is not released - anyThreadsStalled: {}",
        ctrl.any_stalled_threads()
      );
    }
  }
  Ok(())
}

fn assert_state(
  num_releasers: usize,
  num_stallers: usize,
  num_waiters: usize,
  threads: &[TestThread],
  ctrl: &DocumentsWriterStallControl,
) {
  let mut millis_to_sleep = 100;
  while ctrl.has_blocked() && ctrl.is_healthy() {
    for thread in threads
      .iter()
      .skip(num_releasers + num_stallers)
      .take(num_waiters)
    {
      if thread.is_queued(ctrl) {
        if millis_to_sleep < 60000 {
          thread::sleep(Duration::from_millis(millis_to_sleep));
          millis_to_sleep *= 2;
          break;
        } else {
          unreachable!("control claims no stalled threads but waiter seems to be blocked ");
        }
      }
    }
  }
}

struct Waiter;

impl Waiter {
  #[allow(clippy::new_ret_no_self)]
  fn new(
    stop: Arc<AtomicBool>,
    check_point: Arc<AtomicBool>,
    ctrl: Arc<DocumentsWriterStallControl>,
    sync: Arc<Synchronizer>,
    exceptions: Arc<Mutex<Vec<String>>>,
  ) -> TestThread {
    TestThread::spawn(true, move || {
      while !stop.load(Ordering::SeqCst) {
        ctrl.wait_if_stalled();
        if check_point.load(Ordering::SeqCst) && !sync.await_waiter(Duration::from_secs(10)) {
          exceptions.lock().push(format!(
            "[Waiter] timed out - wait count: {}",
            sync.waiter_get_count()
          ));
        }
      }
    })
  }
}

struct Updater;

impl Updater {
  #[allow(clippy::new_ret_no_self)]
  fn new(
    stop: Arc<AtomicBool>,
    check_point: Arc<AtomicBool>,
    ctrl: Arc<DocumentsWriterStallControl>,
    sync: Arc<Synchronizer>,
    release: bool,
    exceptions: Arc<Mutex<Vec<String>>>,
  ) -> TestThread {
    TestThread::spawn(false, move || {
      let mut random = rng();
      while !stop.load(Ordering::SeqCst) {
        let internal_iters = if release && random.random_bool(0.5) {
          at_least(&mut random, 5)
        } else {
          1
        };
        for _ in 0..internal_iters {
          ctrl.update_stalled(random.random_bool(0.5));
        }
        if check_point.load(Ordering::SeqCst) {
          sync.update_join_count_down();
          if !sync.await_waiter(Duration::from_secs(10)) {
            exceptions.lock().push(format!(
              "[Updater] timed out - wait count: {}",
              sync.waiter_get_count()
            ));
          }
          sync.left_checkpoint_count_down();
        }
        if random.random_bool(0.5) {
          thread::yield_now();
        }
      }
      sync.update_join_count_down();
    })
  }
}

struct TestThread {
  handle: Mutex<Option<JoinHandle<()>>>,
  thread_id: ThreadId,
  alive: Arc<AtomicBool>,
  waiter: bool,
}

impl TestThread {
  fn spawn<F>(waiter: bool, f: F) -> Self
  where
    F: FnOnce() + Send + 'static,
  {
    let alive = Arc::new(AtomicBool::new(true));
    let thread_alive = alive.clone();
    let handle = thread::spawn(move || {
      f();
      thread_alive.store(false, Ordering::SeqCst);
    });
    let thread_id = handle.thread().id();
    Self {
      handle: Mutex::new(Some(handle)),
      thread_id,
      alive,
      waiter,
    }
  }

  fn join(&self, timeout: Duration) {
    let start = Instant::now();
    while self.alive.load(Ordering::SeqCst) && start.elapsed() < timeout {
      thread::sleep(Duration::from_millis(1));
    }
    if !self.alive.load(Ordering::SeqCst)
      && let Some(handle) = self.handle.lock().take()
    {
      handle.join().expect("thread panicked");
    }
  }

  fn is_alive(&self) -> bool {
    self.alive.load(Ordering::SeqCst)
  }

  fn is_waiter(&self) -> bool {
    self.waiter
  }

  fn is_queued(&self, ctrl: &DocumentsWriterStallControl) -> bool {
    ctrl.is_thread_queued(&self.thread_id)
  }
}

pub fn terminated(threads: &[JoinHandle<()>]) -> bool {
  threads.iter().all(JoinHandle::is_finished)
}

pub fn start<T>(_to_start: &[T]) {
  thread::sleep(Duration::from_millis(1)); // let them start
}

pub fn join(to_join: Vec<JoinHandle<()>>) {
  for thread in to_join {
    thread.join().expect("thread panicked");
  }
}

pub fn wait_threads(num: i32, ctrl: Arc<DocumentsWriterStallControl>) -> Vec<JoinHandle<()>> {
  let mut array = Vec::new();
  for _ in 0..num {
    let ctrl = ctrl.clone();
    array.push(thread::spawn(move || {
      ctrl.wait_if_stalled();
    }));
  }
  array
}

/// Waits for all incoming threads to be in wait() methods.
pub fn await_state(num_waiting: i32, ctrl: &DocumentsWriterStallControl) {
  while ctrl.get_num_waiting() != num_waiting {
    let mut random = rng();
    if random.random_bool(0.5) {
      thread::yield_now();
    } else {
      thread::sleep(Duration::from_millis(1));
    }
  }
}

struct Synchronizer {
  state: Mutex<SynchronizerState>,
  cond: Condvar,
}

struct SynchronizerState {
  waiter: usize,
  update_join: usize,
  left_checkpoint: usize,
}

impl Synchronizer {
  pub fn new(num_updater: usize, num_threads: usize) -> Self {
    let sync = Self {
      state: Mutex::new(SynchronizerState {
        waiter: 0,
        update_join: 0,
        left_checkpoint: 0,
      }),
      cond: Condvar::new(),
    };
    sync.reset(num_updater, num_threads);
    sync
  }

  pub fn reset(&self, num_updaters: usize, _num_threads: usize) {
    let mut state = self.state.lock();
    state.waiter = 1;
    state.update_join = num_updaters;
    state.left_checkpoint = num_updaters;
    self.cond.notify_all();
  }

  pub fn await_waiter(&self, timeout: Duration) -> bool {
    let start = Instant::now();
    let mut state = self.state.lock();
    while state.waiter != 0 {
      let elapsed = start.elapsed();
      if elapsed >= timeout {
        return false;
      }
      let _ = self.cond.wait_for(&mut state, timeout - elapsed);
    }
    true
  }

  pub fn await_update_join(&self, timeout: Duration) -> bool {
    let start = Instant::now();
    let mut state = self.state.lock();
    while state.update_join != 0 {
      let elapsed = start.elapsed();
      if elapsed >= timeout {
        return false;
      }
      let _ = self.cond.wait_for(&mut state, timeout - elapsed);
    }
    true
  }

  pub fn await_left_checkpoint(&self) {
    let mut state = self.state.lock();
    while state.left_checkpoint != 0 {
      self.cond.wait(&mut state);
    }
  }

  pub fn waiter_count_down(&self) {
    let mut state = self.state.lock();
    if state.waiter > 0 {
      state.waiter -= 1;
    }
    self.cond.notify_all();
  }

  pub fn update_join_count_down(&self) {
    let mut state = self.state.lock();
    if state.update_join > 0 {
      state.update_join -= 1;
    }
    self.cond.notify_all();
  }

  pub fn left_checkpoint_count_down(&self) {
    let mut state = self.state.lock();
    if state.left_checkpoint > 0 {
      state.left_checkpoint -= 1;
    }
    self.cond.notify_all();
  }

  pub fn waiter_get_count(&self) -> usize {
    self.state.lock().waiter
  }
}
