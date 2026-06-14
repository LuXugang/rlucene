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
use std::cmp::{max, min};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use parking_lot::Mutex;

use crate::core::index::approximate_priority_queue::{ApproximatePriorityQueue, IdentityId};
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub(crate) const MIN_CONCURRENCY: usize = 1;
pub(crate) const MAX_CONCURRENCY: usize = 256;
/// Concurrent version of [`ApproximatePriorityQueue`], which trades a bit more
/// of ordering for better concurrency by maintaining multiple sub
/// [`ApproximatePriorityQueue`]s that are locked independently. The number of
/// subs is computed dynamically based on hardware concurrency.
pub struct ConcurrentApproximatePriorityQueue<T>
where
  T: IdentityId,
{
  concurrency: usize,
  pub(crate) queues: Vec<Mutex<ApproximatePriorityQueue<T>>>,
}

impl<T> ConcurrentApproximatePriorityQueue<T>
where
  T: IdentityId,
{
  fn get_concurrency() -> usize {
    let core_count = std::thread::available_parallelism()
      .map(|n| n.get())
      .unwrap_or(1);
    // Aim for ~4 entries per slot when indexing with one thread per CPU
    // core. The trade-off is that if we set the concurrency too
    // high then we'll completely lose the bias towards larger
    // DWPTs. And if we set it too low then we risk seeing contention.
    let mut concurrency = core_count / 4;
    concurrency = max(MIN_CONCURRENCY, concurrency);
    concurrency = min(MAX_CONCURRENCY, concurrency);
    concurrency
  }

  pub(crate) fn new() -> Result<Self> {
    Self::with_concurrency(Self::get_concurrency())
  }

  pub(crate) fn with_concurrency(concurrency: usize) -> Result<Self> {
    if !(MIN_CONCURRENCY..=MAX_CONCURRENCY).contains(&concurrency) {
      return Err(LuceneError::illegal_argument(format!(
        "concurrency must be in [{MIN_CONCURRENCY}, {MAX_CONCURRENCY}], got {concurrency}"
      )));
    }
    let mut queues = Vec::with_capacity(concurrency);
    for _ in 0..concurrency {
      queues.push(Mutex::new(ApproximatePriorityQueue::new()));
    }
    Ok(Self {
      concurrency,
      queues,
    })
  }

  fn thread_hash() -> usize {
    let thread_id = std::thread::current().id();
    let mut hasher = DefaultHasher::new();
    thread_id.hash(&mut hasher);
    (hasher.finish() as usize) & 0xFFFF
  }

  pub(crate) fn add(&self, entry: T, weight: i64) {
    let thread_hash = Self::thread_hash();
    for i in 0..self.concurrency {
      let index = (thread_hash + i) % self.concurrency;
      if let Some(mut queue) = self.queues[index].try_lock() {
        queue.add(entry, weight);
        return;
      }
    }
    let index = thread_hash % self.concurrency;
    let mut queue = self.queues[index].lock();
    queue.add(entry, weight)
  }

  pub(crate) fn poll<F>(&self, predicate: F) -> Option<T>
  where
    F: Fn(&T) -> bool,
  {
    let thread_hash = Self::thread_hash();
    for i in 0..self.concurrency {
      let index = (thread_hash + i) % self.concurrency;
      if let Some(mut queue) = self.queues[index].try_lock()
        && let Some(entry) = queue.poll(&predicate)
      {
        return Some(entry);
      }
    }
    for i in 0..self.concurrency {
      let index = (thread_hash + i) % self.concurrency;
      let mut queue = self.queues[index].lock();
      if let Some(entry) = queue.poll(&predicate) {
        return Some(entry);
      }
    }
    None
  }

  pub(crate) fn contains(&self, o: &str) -> bool {
    for mutex in &self.queues {
      let queue = mutex.lock();
      if queue.contains(o) {
        return true;
      }
    }
    false
  }

  pub(crate) fn remove(&self, o: &str) -> Option<T> {
    for mutex in &self.queues {
      let mut queue = mutex.lock();
      match queue.remove(o) {
        Some(entry) => return Some(entry),
        None => continue,
      }
    }
    None
  }
}
