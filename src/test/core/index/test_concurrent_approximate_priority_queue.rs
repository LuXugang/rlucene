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
use crate::core::index::approximate_priority_queue::IdentityId;
use crate::core::index::concurrent_approximate_priority_queue::{
  ConcurrentApproximatePriorityQueue, MAX_CONCURRENCY, MIN_CONCURRENCY,
};
use crate::core::index::lockable_concurrent_approximate_priority_queue::Lock;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::util::lucene_test_case::random;
use crate::test::support::core::util::test_util::TestUtil;
use std::sync::{Arc, mpsc};
use std::thread;
#[allow(dead_code)] // for quick search
struct TestConcurrentApproximatePriorityQueue;
impl Lock for i32 {
  fn lock(&self) {
    unreachable!()
  }

  fn try_lock(&self) -> bool {
    unreachable!()
  }
  fn unlock(&self) {}

  fn is_locked(&self) -> bool {
    unreachable!()
  }
}
impl IdentityId for i32 {
  fn id(&self) -> &str {
    ""
  }
}

#[test]
fn test_poll_from_same_thread() -> Result<()> {
  let mut random = random();
  let concurrency = TestUtil::next_usize(&mut random, MIN_CONCURRENCY, MAX_CONCURRENCY);
  let pq = ConcurrentApproximatePriorityQueue::<i32>::with_concurrency(concurrency)?;

  pq.add(3, 3);
  pq.add(10, 10);
  pq.add(7, 7);

  assert_eq!(Some(10), pq.poll(|_| true));
  assert_eq!(Some(7), pq.poll(|_| true));
  assert_eq!(Some(3), pq.poll(|_| true));
  assert_eq!(None, pq.poll(|_| true));
  Ok(())
}
#[test]
fn test_poll_from_different_thread() -> Result<()> {
  let mut random = random();
  let concurrency = TestUtil::next_usize(&mut random, MIN_CONCURRENCY, MAX_CONCURRENCY);
  let pq = Arc::new(ConcurrentApproximatePriorityQueue::<i32>::with_concurrency(
    concurrency,
  )?);

  pq.add(3, 3);
  pq.add(10, 10);
  pq.add(7, 7);

  let pq_clone = Arc::clone(&pq);
  let handle = thread::spawn(move || {
    assert_eq!(Some(10), pq_clone.poll(|_| true));
    assert_eq!(Some(7), pq_clone.poll(|_| true));
    assert_eq!(Some(3), pq_clone.poll(|_| true));
    assert_eq!(None, pq_clone.poll(|_| true));
  });
  handle.join().unwrap();

  Ok(())
}
#[test]
fn test_current_lock_is_busy() -> Result<()> {
  let mut random = random();
  let concurrency = TestUtil::next_usize(&mut random, 2, MAX_CONCURRENCY);
  let pq = Arc::new(ConcurrentApproximatePriorityQueue::<i32>::with_concurrency(
    concurrency,
  )?);

  pq.add(3, 3);

  let (take_lock_tx, take_lock_rx) = mpsc::channel::<()>();
  let (release_lock_tx, release_lock_rx) = mpsc::channel::<()>();

  let pq_clone = Arc::clone(&pq);
  let handle = thread::spawn(move || {
    let mut chosen_index: Option<usize> = None;
    #[allow(unused_variables)]
    let mut _chosen_lock;
    for (i, mutex) in pq_clone.queues.iter().enumerate() {
      if let Some(guard) = mutex.try_lock()
        && !guard.is_empty()
      {
        chosen_index = Some(i);
        _chosen_lock = Some(guard);
        break;
      }
    }
    assert!(chosen_index.is_some());
    take_lock_tx.send(()).unwrap();
    release_lock_rx.recv().unwrap();
  });

  take_lock_rx.recv().unwrap();

  pq.add(1, 1);
  assert_eq!(Some(1), pq.poll(|_| true));

  release_lock_tx.send(()).unwrap();
  assert_eq!(Some(3), pq.poll(|_| true));
  assert_eq!(None, pq.poll(|_| true));

  handle.join().unwrap();
  Ok(())
}
