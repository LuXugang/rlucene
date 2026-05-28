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
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// A min heap that stores `i64` values.
/// This is a primitive priority queue that maintains a partial ordering of
/// elements such that the smallest element can always be found in constant
/// time.
///
/// `push()` and `pop()` require O(log(n)) time complexity.
/// This heap supports both unbounded growth (via `push()`) and bounded-size
/// insertion (via `insert_with_overflow()`).
///
/// The heap is 1-based internally: index 0 is unused.
pub struct LongHeap {
  max_size: usize,
  heap: Vec<i64>,
  size: usize,
}

impl LongHeap {
  /// Create an empty priority queue of the configured initial size.
  ///
  /// # Arguments
  ///
  /// * `max_size` - The maximum size of the heap. Must be > 0 and <
  ///   ArrayUtil::MAX_ARRAY_LENGTH.
  ///
  /// # Errors
  ///
  /// Returns `Err` if `max_size` is invalid to prevent confusing
  /// out-of-memory errors.
  pub fn new(max_size: usize) -> Result<Self> {
    // TODO
    // if max_size < 1 || max_size >= ArrayUtil::MAX_ARRAY_LENGTH {
    if max_size < 1 {
      return Err(LuceneError::illegal_argument(format!(
        "max_size must be > 0 and < {}; got: {}",
        ArrayUtil::MAX_ARRAY_LENGTH - 1,
        max_size
      )));
    }
    // We add +1 because index 0 is unused.
    let heap_size = max_size + 1;
    let heap = vec![0i64; heap_size];

    Ok(Self {
      max_size,
      heap,
      size: 0,
    })
  }
  /// Adds a value in O(log(n)) time. Grows unbounded as needed to accommodate
  /// new values. Returns the new top element.
  pub fn push(&mut self, element: i64) -> i64 {
    self.size += 1;
    if self.size == self.heap.len() {
      let new_capacity = (self.size * 3).div_ceil(2);
      debug_assert!(new_capacity <= i32::MAX as usize);
      ArrayUtil::grow_with_len(&mut self.heap, new_capacity);
    }
    self.heap[self.size] = element;
    self.up_heap(self.size);
    self.heap[1]
  }
  /// Adds a value in O(log(n)) time. If the number of values would exceed
  /// `max_size`, the least value is discarded.
  ///
  /// Returns whether the value was added.
  pub fn insert_with_overflow(&mut self, value: i64) -> bool {
    if self.size >= self.max_size {
      if value < self.heap[1] {
        return false;
      }
      self.update_top(value);
      return true;
    }
    self.push(value);
    true
  }
  /// Returns the least element of the heap in constant time.
  /// The caller must ensure the heap is not empty.
  pub fn top(&self) -> i64 {
    self.heap[1]
  }

  /// Removes and returns the least element of the heap in O(log(n)) time.
  ///
  /// # Errors
  ///
  /// Returns error if the heap is empty.
  pub fn pop(&mut self) -> Result<i64> {
    if self.size > 0 {
      let result = self.heap[1];
      self.heap[1] = self.heap[self.size];
      self.size -= 1;
      self.down_heap(1);
      Ok(result)
    } else {
      Err(LuceneError::illegal_state("The heap is empty"))
    }
  }
  /// Replaces the top of the heap with `new_top`.
  /// This is faster than calling `pop()` followed by `push()`.
  /// No-op if the heap is empty.
  pub fn update_top(&mut self, value: i64) -> i64 {
    if self.size > 0 {
      self.heap[1] = value;
      self.down_heap(1);
    }
    self.heap[1]
  }

  /// Returns the number of elements currently stored in the heap.
  pub fn size(&self) -> usize {
    self.size
  }

  /// Removes all entries from the heap.
  pub fn clear(&mut self) {
    self.size = 0;
  }
  fn up_heap(&mut self, mut i: usize) {
    let value = self.heap[i]; // save bottom value
    let mut j = i >> 1;
    while j > 0 && value < self.heap[j] {
      self.heap[i] = self.heap[j]; // shift parents down
      i = j;
      j >>= 1;
    }
    self.heap[i] = value;
  }

  fn down_heap(&mut self, mut i: usize) {
    let value = self.heap[i];
    let mut j = i << 1;
    let mut k = j + 1;

    if k <= self.size && self.heap[k] < self.heap[j] {
      j = k;
    }

    while j <= self.size && self.heap[j] < value {
      self.heap[i] = self.heap[j];
      i = j;
      j = i << 1;
      k = j + 1;
      if k <= self.size && self.heap[k] < self.heap[j] {
        j = k;
      }
    }

    self.heap[i] = value;
  }
  /// Pushes all elements from another heap into this heap.
  pub fn push_all(&mut self, other: &LongHeap) {
    for i in 1..=other.size {
      self.push(other.heap[i]);
    }
  }

  /// Returns the element at the ith location in the heap array.
  /// Valid indices are in [1, size].
  pub fn get(&self, i: usize) -> i64 {
    self.heap[i]
  }

  /// Returns the internal heap array.
  #[cfg(test)]
  pub fn get_heap_array(&self) -> &[i64] {
    &self.heap
  }
}
