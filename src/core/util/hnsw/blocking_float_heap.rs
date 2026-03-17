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
use parking_lot::Mutex;

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
  heap: Vec<f32>,
  size: usize,
  lock: Mutex<()>,
}
impl BlockingFloatHeap {
  pub fn new(max_size: usize) -> Self {
    let mut heap = vec![0f32; max_size + 1];
    heap.push(0.0);
    Self {
      max_size,
      heap,
      size: 0,
      lock: Mutex::new(()),
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
  pub fn offer(&mut self, value: f32) -> Result<f32> {
    let _guard = self.lock.lock();

    if self.size < self.max_size {
      Self::push(&mut self.heap, value, &mut self.size);
    } else if value >= self.heap[1] {
      Self::update_top(&mut self.heap, value, self.size);
    }

    Ok(self.heap[1])
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
  pub fn offer_array(&mut self, values: &[f32], len: usize) -> Result<f32> {
    let _guard = self.lock.lock();

    for i in (0..len).rev() {
      if self.size < self.max_size {
        Self::push(&mut self.heap, values[i], &mut self.size);
      } else if values[i] >= self.heap[1] {
        Self::update_top(&mut self.heap, values[i], self.size);
      } else {
        break;
      }
    }

    Ok(self.heap[1])
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
  pub fn poll(&mut self) -> Result<f32> {
    if self.size == 0 {
      return Err(LuceneError::illegal_state("The heap is empty"));
    }
    let _guard = self.lock.lock();

    let result = self.heap[1];
    self.heap[1] = self.heap[self.size];
    self.size -= 1;
    Self::down_heap(&mut self.heap, 1, self.size);
    Ok(result)
  }

  /// Retrieves, but does not remove, the head of this heap.
  ///
  /// # Returns
  ///
  /// The head of the heap, the smallest value.
  pub fn peek(&self) -> f32 {
    let _guard = self.lock.lock();
    self.heap[1]
  }
  /// Returns the number of elements in this heap.
  ///
  /// # Returns
  ///
  /// The number of elements in this heap.
  pub fn size(&self) -> usize {
    let _guard = self.lock.lock();
    self.size
  }
  fn push(heap: &mut [f32], element: f32, size: &mut usize) {
    *size += 1;
    heap[*size] = element;
    Self::up_heap(heap, *size)
  }

  fn update_top(heap: &mut [f32], value: f32, size: usize) -> f32 {
    heap[1] = value;
    Self::down_heap(heap, 1, size);
    heap[1]
  }
  fn down_heap(heap: &mut [f32], mut i: usize, size: usize) {
    let value = heap[i]; // save top value
    let mut j = i << 1; // find smaller child
    let mut k = j + 1;

    if k <= size && heap[k] < heap[j] {
      j = k;
    }

    while j <= size && heap[j] < value {
      heap[i] = heap[j]; // shift up child
      i = j;
      j = i << 1;
      k = j + 1;
      if k <= size && heap[k] < heap[j] {
        j = k;
      }
    }

    heap[i] = value; // install saved value
  }
  fn up_heap(heap: &mut [f32], orig_pos: usize) {
    let mut i = orig_pos;
    let value = heap[i];
    let mut j = i >> 1;
    while j > 0 && value < heap[j] {
      heap[i] = heap[j];
      i = j;
      j >>= 1;
    }
    heap[i] = value;
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
    let mut heap = BlockingFloatHeap::new(3);

    heap.offer(2.0)?;
    heap.offer(4.0)?;
    heap.offer(1.0)?;
    heap.offer(3.0)?;

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
    let mut heap = BlockingFloatHeap::new(size as usize);

    let mut sum = 0.0;
    for _ in 0..size {
      let next = random.random_range(0.0..100.0);
      sum += next;
      heap.offer(next)?;
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
            let mut heap = heap.lock();
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
