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
pub(crate) struct LockableConcurrentApproximatePriorityQueue<T>
where
    T: Lock + IdentityId + FlushState,
{
    queue: ConcurrentApproximatePriorityQueue<T>,
    add_and_unlock_counter: AtomicI32,
}
impl<T> LockableConcurrentApproximatePriorityQueue<T>
where
    T: Lock + IdentityId + FlushState,
{
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
    pub(crate) fn lock_and_poll_flush(&self) -> Option<T> {
        loop {
            let prev = self.add_and_unlock_counter.load(Ordering::SeqCst);
            if let Some(entry) = self.queue.poll(|e| e.is_flush_pending() && e.try_lock()) {
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
        debug_assert!(entry.is_locked());
        self.queue.add(entry, weight);
        self.add_and_unlock_counter.fetch_add(1, Ordering::SeqCst);
    }
}

pub(crate) trait Lock {
    fn lock(&self);
    fn try_lock(&self) -> bool;
    fn unlock(&self);
    fn is_locked(&self) -> bool;
}
pub(crate) trait FlushState {
    fn is_flush_pending(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::core::index::approximate_priority_queue::IdentityId;
    use crate::core::index::lockable_concurrent_approximate_priority_queue::{
        FlushState, Lock, LockableConcurrentApproximatePriorityQueue,
    };
    use crate::test::util::lucene_test_case::lucene_test_case_util::random;

    use rand::Rng;

    use parking_lot::{Condvar, Mutex};
    use std::sync::{Arc, Barrier};
    use std::thread;

    struct WeightedLock {
        weight: i64,
        cvar: Condvar,
        available: Mutex<bool>,
    }
    impl WeightedLock {
        fn new() -> Self {
            Self {
                weight: 0,
                cvar: Condvar::new(),
                available: Mutex::new(true),
            }
        }
    }
    impl IdentityId for WeightedLock {
        fn id(&self) -> &str {
            ""
        }
    }

    impl FlushState for WeightedLock {}

    impl Lock for WeightedLock {
        fn lock(&self) {
            let mut guard = self.available.lock();
            while !*guard {
                self.cvar.wait(&mut guard);
            }
            *guard = false;
        }

        fn try_lock(&self) -> bool {
            let mut flag = self.available.lock();
            if *flag {
                *flag = false;
                true
            } else {
                false
            }
        }

        fn unlock(&self) {
            let mut guard = self.available.lock();
            *guard = true;
            self.cvar.notify_one();
        }

        fn is_locked(&self) -> bool {
            let flag = self.available.lock();
            !*flag
        }
    }
    impl PartialEq for WeightedLock {
        fn eq(&self, other: &Self) -> bool {
            self.weight == other.weight
        }
    }
    #[test]
    fn test_never_return_none_on_non_empty_queue() {
        let mut rng = random();
        let iters = rng.random_range(10..=20);
        for _ in 0..iters {
            let concurrency = rng.random_range(1..=16);
            let queue = Arc::new(
                LockableConcurrentApproximatePriorityQueue::with_concurrency(concurrency).unwrap(),
            );
            let num_threads = rng.random_range(2..=16);
            let barrier = Arc::new(Barrier::new(num_threads + 1));
            let mut handles = Vec::with_capacity(num_threads);

            for _ in 0..num_threads {
                let q = Arc::clone(&queue);
                let b = Arc::clone(&barrier);
                handles.push(thread::spawn(move || {
                    b.wait();
                    let mut lock = WeightedLock::new();
                    lock.lock();
                    lock.weight += 1;
                    let weight = lock.weight;
                    q.add_and_unlock(lock, weight);
                    for _ in 0..10_000 {
                        let lock = q.lock_and_poll().expect("Queue was non-empty");
                        let weight = lock.weight;
                        q.add_and_unlock(lock, weight);
                    }
                }));
            }

            barrier.wait();
            for h in handles {
                h.join().unwrap();
            }
        }
    }
}
