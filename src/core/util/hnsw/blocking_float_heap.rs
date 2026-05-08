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
use parking_lot::{Mutex, MutexGuard};

use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
/// A blocking bounded min heap that stores `f32` values. The top element is the
/// smallest value in the heap.
///
/// This is a primitive priority queue that maintains a partial ordering of its
/// elements, ensuring the minimum element can always be accessed in constant
/// time.
///
/// The implementation is based on [LongHeap`](crate::core::util::long_heap::LongHeap)
/// from Lucene.
pub struct BlockingFloatHeap {
  max_size: usize,
  lock: Mutex<Inner>,
}
pub struct Inner {
  heap: Vec<f32>,
  size: usize,
}
impl BlockingFloatHeap {
  pub fn new(max_size: usize) -> Self {
    let heap = vec![0f32; max_size + 1];
    let inner = Inner { heap, size: 0 };
    Self {
      max_size,
      lock: Mutex::new(inner),
    }
  }
  /// Inserts a value into this heap.
  ///
  /// If the number of values would exceed the heap's `max_size`, the least
  /// value is discarded.
  ///
  /// # Arguments
  ///
  /// * `value` - The value to add.
  ///
  /// # Returns
  ///
  /// The new 'top' element in the queue.
  pub fn offer(&self, value: f32) -> f32 {
    let mut inner = self.lock.lock();

    if inner.size < self.max_size {
      Self::push(&mut inner, value);
    } else if value >= inner.heap[1] {
      Self::update_top(&mut inner, value);
    }

    inner.heap[1]
  }
  /// Inserts an array of values into this heap.
  ///
  /// Values must be sorted in ascending order.
  ///
  /// # Arguments
  ///
  /// * `values` - A slice of values to insert, must be sorted in ascending
  ///   order.
  /// * `len` - Number of values from the `values` slice to insert.
  ///
  /// # Returns
  ///
  /// The new 'top' element in the queue.
  pub fn offer_array(&self, values: &[f32], len: usize) -> f32 {
    let mut inner = self.lock.lock();

    for i in (0..len).rev() {
      if inner.size < self.max_size {
        Self::push(&mut inner, values[i]);
      } else if values[i] >= inner.heap[1] {
        Self::update_top(&mut inner, values[i]);
      } else {
        break;
      }
    }

    inner.heap[1]
  }
  /// Removes and returns the head of the heap.
  ///
  /// # Returns
  ///
  /// The head of the heap, the smallest value.
  ///
  /// # Error
  ///
  /// Error if the heap is empty.
  pub fn poll(&self) -> Result<f32> {
    let mut inner = self.lock.lock();
    if inner.size == 0 {
      return Err(LuceneError::illegal_state("The heap is empty"));
    }

    let result = inner.heap[1];
    inner.heap[1] = inner.heap[inner.size];
    inner.size -= 1;
    Self::down_heap(&mut inner, 1);
    Ok(result)
  }

  /// Retrieves, but does not remove, the head of this heap.
  ///
  /// # Returns
  ///
  /// The head of the heap, the smallest value.
  pub fn peek(&self) -> f32 {
    let inner = self.lock.lock();
    inner.heap[1]
  }
  /// Returns the number of elements in this heap.
  ///
  /// # Returns
  ///
  /// The number of elements in this heap.
  pub fn size(&self) -> usize {
    let inner = self.lock.lock();
    inner.size
  }
  fn push(inner: &mut MutexGuard<'_, Inner>, element: f32) {
    inner.size += 1;
    let size = inner.size;
    inner.heap[size] = element;
    Self::up_heap(inner, size)
  }

  fn update_top(inner: &mut MutexGuard<'_, Inner>, value: f32) -> f32 {
    inner.heap[1] = value;
    Self::down_heap(inner, 1);
    inner.heap[1]
  }
  fn down_heap(inner: &mut MutexGuard<'_, Inner>, mut i: usize) {
    let value = inner.heap[i]; // save top value
    let mut j = i << 1; // find smaller child
    let mut k = j + 1;

    if k <= inner.size && inner.heap[k] < inner.heap[j] {
      j = k;
    }

    while j <= inner.size && inner.heap[j] < value {
      inner.heap[i] = inner.heap[j]; // shift up child
      i = j;
      j = i << 1;
      k = j + 1;
      if k <= inner.size && inner.heap[k] < inner.heap[j] {
        j = k;
      }
    }

    inner.heap[i] = value; // install saved value
  }
  fn up_heap(inner: &mut MutexGuard<'_, Inner>, orig_pos: usize) {
    let mut i = orig_pos;
    let value = inner.heap[i];
    let mut j = i >> 1;
    while j > 0 && value < inner.heap[j] {
      inner.heap[i] = inner.heap[j];
      i = j;
      j >>= 1;
    }
    inner.heap[i] = value;
  }
}
#[cfg(test)]
mod tests {
  use std::sync::{Arc, Barrier};
  use std::thread;
  use std::time::Duration;

  use parking_lot::Mutex;
  use rand::RngExt;
  use rand::rng;

  use crate::core::util::error::lucene_error::Result;
  use crate::core::util::hnsw::blocking_float_heap::BlockingFloatHeap;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, random};

  #[allow(dead_code)] // for quick search
  struct TestBlockingFloatHeap;

  #[test]
  fn test_basic_operations() -> Result<()> {
    let heap = BlockingFloatHeap::new(3);

    heap.offer(2.0);
    heap.offer(4.0);
    heap.offer(1.0);
    heap.offer(3.0);

    assert_eq!(heap.size(), 3);
    assert_eq!(heap.peek(), 2.0);

    assert_eq!(heap.poll()?, 2.0);
    assert_eq!(heap.poll()?, 3.0);
    assert_eq!(heap.poll()?, 4.0);
    assert_eq!(heap.size(), 0);

    Ok(())
  }
  #[test]
  fn test_basic_operations2() -> Result<()> {
    let mut random = random();
    let size = at_least(&mut random, 10);
    let heap = BlockingFloatHeap::new(size as usize);

    let mut sum = 0.0;
    for _ in 0..size {
      let next = random.random_range(0.0..100.0);
      sum += next;
      heap.offer(next);
    }

    let mut last = f32::NEG_INFINITY;
    let mut sum2 = 0.0;

    for _ in 0..size {
      let next = heap.poll()?;
      assert!(next >= last);
      last = next;
      sum2 += last;
    }

    assert!((sum - sum2).abs() <= 0.01);
    Ok(())
  }
  #[test]
  fn test_multiple_threads() -> Result<()> {
    let mut random = random();
    let thread_count = random.random_range(3..=5);
    let heap = Arc::new(Mutex::new(BlockingFloatHeap::new(1)));
    let barrier = Arc::new(Barrier::new(thread_count + 1));
    let mut handles = vec![];

    for _ in 0..thread_count {
      let heap = heap.clone();
      let barrier = barrier.clone();
      handles.push(thread::spawn(move || {
        barrier.wait();

        let mut rng = rng();
        let mut bottom_value = 0.0;

        for _ in 0..rng.random_range(10..100) {
          bottom_value += rng.random_range(0..=5) as f32;
          {
            let heap = heap.lock();
            let _ = heap.offer(bottom_value);
          }
          thread::sleep(Duration::from_millis(rng.random_range(0..50)));

          let global_bottom = {
            let heap = heap.lock();
            heap.peek()
          };

          assert!(global_bottom >= bottom_value);
          bottom_value = global_bottom;
        }
      }));
    }

    barrier.wait();

    for h in handles {
      h.join().expect("Thread panicked");
    }

    Ok(())
  }
}
