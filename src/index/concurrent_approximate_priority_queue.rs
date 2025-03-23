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
use crate::index::approximate_priority_queue::ApproximatePriorityQueue;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::cmp::{max, min};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

const MIN_CONCURRENCY: i32 = 1;
const MAX_CONCURRENCY: i32 = 256;
/// Concurrent version of [`ApproximatePriorityQueue`], which trades a bit more of ordering for
/// better concurrency by maintaining multiple sub [`ApproximatePriorityQueue`]s
/// that are locked independently. The number of subs is computed dynamically based on hardware concurrency.
pub struct ConcurrentApproximatePriorityQueue<T>
where
    T: PartialEq,
{
    concurrency: i32,
    queues: Vec<Mutex<ApproximatePriorityQueue<T>>>,
}
#[allow(unused)]
impl<T: PartialEq> ConcurrentApproximatePriorityQueue<T> {
    fn get_concurrency() -> i32 {
        let core_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        // Aim for ~4 entries per slot when indexing with one thread per CPU core. The trade-off is
        // that if we set the concurrency too high then we'll completely lose the bias towards larger
        // DWPTs. And if we set it too low then we risk seeing contention.
        debug_assert!(core_count <= i32::MAX as usize);
        let mut concurrency = (core_count as i32) / 4;
        concurrency = max(MIN_CONCURRENCY, concurrency);
        concurrency = min(MAX_CONCURRENCY, concurrency);
        concurrency
    }

    pub fn new() -> Result<Self> {
        Self::with_concurrency(Self::get_concurrency())
    }

    pub fn with_concurrency(concurrency: i32) -> Result<Self> {
        if !(MIN_CONCURRENCY..=MAX_CONCURRENCY).contains(&concurrency) {
            return Err(LuceneError::illegal_argument(format!(
                "concurrency must be in [{}, {}], got {}",
                MIN_CONCURRENCY, MAX_CONCURRENCY, concurrency
            )));
        }
        let mut queues = Vec::with_capacity(concurrency as usize);
        for _ in 0..concurrency {
            queues.push(Mutex::new(ApproximatePriorityQueue::new()));
        }
        Ok(Self {
            concurrency,
            queues,
        })
    }

    fn thread_hash() -> i32 {
        let thread_id = std::thread::current().id();
        let mut hasher = DefaultHasher::new();
        thread_id.hash(&mut hasher);
        ((hasher.finish() as usize) & 0xFFFF) as i32
    }

    pub fn add(&self, entry: T, weight: i64) -> Result<()> {
        let thread_hash = Self::thread_hash();
        for i in 0..self.concurrency {
            let index = ((thread_hash + i) % self.concurrency) as usize;
            if let Ok(mut queue) = self.queues[index].try_lock() {
                queue.add(entry, weight);
                return Ok(());
            }
        }
        let index = (thread_hash % self.concurrency) as usize;
        let mut queue = self.queues[index]
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        queue.add(entry, weight);
        Ok(())
    }

    pub fn poll<F>(&self, predicate: F) -> Option<T>
    where
        F: Fn(&T) -> bool,
    {
        let thread_hash = Self::thread_hash();
        for i in 0..self.concurrency {
            let index = ((thread_hash + i) % self.concurrency) as usize;
            if let Ok(mut queue) = self.queues[index].try_lock() {
                if let Some(entry) = queue.poll(&predicate) {
                    return Some(entry);
                }
            }
        }
        for i in 0..self.concurrency {
            let index = ((thread_hash + i) % self.concurrency) as usize;
            let mut queue = self.queues[index].lock().unwrap();
            if let Some(entry) = queue.poll(&predicate) {
                return Some(entry);
            }
        }
        None
    }

    pub fn contains(&self, o: &T) -> bool
    where
        T: PartialEq,
    {
        for mutex in &self.queues {
            let queue = mutex.lock().unwrap();
            if queue.contains(o) {
                return true;
            }
        }
        false
    }

    pub fn remove(&self, o: &T) -> Result<bool>
    where
        T: PartialEq,
    {
        for mutex in &self.queues {
            let mut queue = mutex.lock().unwrap();
            if queue.remove(o) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use crate::index::concurrent_approximate_priority_queue::{
        ConcurrentApproximatePriorityQueue, MAX_CONCURRENCY, MIN_CONCURRENCY,
    };
    use crate::test::util::lucene_test_case::random;
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;
    use std::sync::{mpsc, Arc};
    use std::thread;

    #[test]
    fn test_poll_from_same_thread() -> Result<()> {
        let mut random = random();
        let concurrency = TestUtil::next_int(&mut random, MIN_CONCURRENCY, MAX_CONCURRENCY);
        let pq = ConcurrentApproximatePriorityQueue::<i32>::with_concurrency(concurrency)?;

        pq.add(3, 3)?;
        pq.add(10, 10)?;
        pq.add(7, 7)?;

        assert_eq!(Some(10), pq.poll(|_| true));
        assert_eq!(Some(7), pq.poll(|_| true));
        assert_eq!(Some(3), pq.poll(|_| true));
        assert_eq!(None, pq.poll(|_| true));
        Ok(())
    }
    #[test]
    fn test_poll_from_different_thread() -> Result<()> {
        let mut random = random();
        let concurrency = TestUtil::next_int(&mut random, MIN_CONCURRENCY, MAX_CONCURRENCY);
        let pq = Arc::new(ConcurrentApproximatePriorityQueue::<i32>::with_concurrency(
            concurrency,
        )?);

        pq.add(3, 3)?;
        pq.add(10, 10)?;
        pq.add(7, 7)?;

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
        let concurrency = TestUtil::next_int(&mut random, 2, MAX_CONCURRENCY);
        let pq = Arc::new(ConcurrentApproximatePriorityQueue::<i32>::with_concurrency(
            concurrency,
        )?);

        pq.add(3, 3)?;

        let (take_lock_tx, take_lock_rx) = mpsc::channel::<()>();
        let (release_lock_tx, release_lock_rx) = mpsc::channel::<()>();

        let pq_clone = Arc::clone(&pq);
        let handle = thread::spawn(move || {
            let mut chosen_index: Option<usize> = None;
            #[allow(unused_variables)]
            let mut chosen_lock = None;
            for (i, mutex) in pq_clone.queues.iter().enumerate() {
                if let Ok(guard) = mutex.try_lock() {
                    if !guard.is_empty() {
                        chosen_index = Some(i);
                        chosen_lock = Some(guard);
                        break;
                    }
                }
            }
            assert!(chosen_index.is_some());
            take_lock_tx.send(()).unwrap();
            release_lock_rx.recv().unwrap();
        });

        take_lock_rx.recv().unwrap();

        pq.add(1, 1)?;
        assert_eq!(Some(1), pq.poll(|_| true));

        release_lock_tx.send(()).unwrap();
        assert_eq!(Some(3), pq.poll(|_| true));
        assert_eq!(None, pq.poll(|_| true));

        handle.join().unwrap();
        Ok(())
    }
}
