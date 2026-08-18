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
use crate::core::index::concurrent_approximate_priority_queue::ConcurrentApproximatePriorityQueue;
use crate::core::util::error::lucene_error::Result;
use std::sync::atomic::{AtomicI32, Ordering};

/// A `ConcurrentApproximatePriorityQueue` of [`Lock`] objects.
pub(crate) struct LockableConcurrentApproximatePriorityQueue<T> {
  queue: ConcurrentApproximatePriorityQueue<T>,
  add_and_unlock_counter: AtomicI32,
}
impl<T> LockableConcurrentApproximatePriorityQueue<T> {
  #[cfg(test)]
  pub(crate) fn with_concurrency(concurrency: usize) -> Result<Self> {
    Ok(Self {
      queue: ConcurrentApproximatePriorityQueue::with_concurrency(concurrency)?,
      add_and_unlock_counter: AtomicI32::new(0),
    })
  }

  pub(crate) fn new() -> Result<Self> {
    Ok(Self {
      queue: ConcurrentApproximatePriorityQueue::new()?,
      add_and_unlock_counter: AtomicI32::new(0),
    })
  }
}

impl<T> LockableConcurrentApproximatePriorityQueue<T>
where
  T: Clone + Lock + IdentityId,
{
  /// Lock an entry, and poll it from the queue, in that order. If no entry can be found and locked, None is returned.
  pub(crate) fn lock_and_poll(&self) -> Option<T> {
    loop {
      let prev = self.add_and_unlock_counter.load(Ordering::SeqCst);
      if let Some(entry) = self.queue.poll(|e| e.try_lock()) {
        return Some(entry);
      }
      if prev == self.add_and_unlock_counter.load(Ordering::SeqCst) {
        break;
      }
    }
    None
  }
  /// Remove an entry from the queue.
  pub(crate) fn remove(&self, o: &str) -> Option<T> {
    self.queue.remove(o)
  }
  ///  Only used for assertions
  pub(crate) fn contains(&self, o: &str) -> bool {
    self.queue.contains(o)
  }

  ///  Add an entry to the queue and unlock it, in that order.
  pub(crate) fn add_and_unlock(&self, entry: T, weight: i64) {
    let entry_to_unlock = entry.clone();
    self.queue.add(entry, weight);
    entry_to_unlock.unlock();
    self.add_and_unlock_counter.fetch_add(1, Ordering::SeqCst);
  }
}

pub(crate) trait Lock {
  fn lock(&self);
  fn try_lock(&self) -> bool;
  fn unlock(&self);
  fn is_locked(&self) -> bool;
}
