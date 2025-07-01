/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::cmp::{max, min};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use parking_lot::Mutex;

use crate::index::approximate_priority_queue::ApproximatePriorityQueue;
use crate::util::error::lucene_error::{LuceneError, Result};

const MIN_CONCURRENCY: i32 = 1;
const MAX_CONCURRENCY: i32 = 256;
/// Concurrent version of [`ApproximatePriorityQueue`], which trades a bit more
/// of ordering for better concurrency by maintaining multiple sub
/// [`ApproximatePriorityQueue`]s that are locked independently. The number of
/// subs is computed dynamically based on hardware concurrency.
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
        // Aim for ~4 entries per slot when indexing with one thread per CPU
        // core. The trade-off is that if we set the concurrency too
        // high then we'll completely lose the bias towards larger
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
                "concurrency must be in [{MIN_CONCURRENCY}, {MAX_CONCURRENCY}], got {concurrency}"
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

    pub fn add(&self, entry: T, weight: i64) {
        let thread_hash = Self::thread_hash();
        for i in 0..self.concurrency {
            let index = ((thread_hash + i) % self.concurrency) as usize;
            if let Some(mut queue) = self.queues[index].try_lock() {
                queue.add(entry, weight);
                return;
            }
        }
        let index = (thread_hash % self.concurrency) as usize;
        let mut queue = self.queues[index].lock();
        queue.add(entry, weight);
    }

    pub fn poll<F>(&self, predicate: F) -> Option<T>
    where
        F: Fn(&T) -> bool,
    {
        let thread_hash = Self::thread_hash();
        for i in 0..self.concurrency {
            let index = ((thread_hash + i) % self.concurrency) as usize;
            if let Some(mut queue) = self.queues[index].try_lock() {
                if let Some(entry) = queue.poll(&predicate) {
                    return Some(entry);
                }
            }
        }
        for i in 0..self.concurrency {
            let index = ((thread_hash + i) % self.concurrency) as usize;
            let mut queue = self.queues[index].lock();
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
            let queue = mutex.lock();
            if queue.contains(o) {
                return true;
            }
        }
        false
    }

    pub fn remove(&self, o: &T) -> bool
    where
        T: PartialEq,
    {
        for mutex in &self.queues {
            let mut queue = mutex.lock();
            if queue.remove(o) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{mpsc, Arc};
    use std::thread;

    use crate::index::concurrent_approximate_priority_queue::{
        ConcurrentApproximatePriorityQueue, MAX_CONCURRENCY, MIN_CONCURRENCY,
    };
    use crate::test::util::lucene_test_case::random;
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;

    #[test]
    fn test_poll_from_same_thread() -> Result<()> {
        let mut random = random();
        let concurrency = TestUtil::next_int(&mut random, MIN_CONCURRENCY, MAX_CONCURRENCY);
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
        let concurrency = TestUtil::next_int(&mut random, MIN_CONCURRENCY, MAX_CONCURRENCY);
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
        let concurrency = TestUtil::next_int(&mut random, 2, MAX_CONCURRENCY);
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
            let mut chosen_lock = None;
            for (i, mutex) in pq_clone.queues.iter().enumerate() {
                match mutex.try_lock() {
                    Some(guard) => {
                        if !guard.is_empty() {
                            chosen_index = Some(i);
                            chosen_lock = Some(guard);
                            break;
                        }
                    },
                    None => (),
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
}
