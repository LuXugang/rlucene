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
use crate::util::error::runtime_error::RuntimeError;
use std::iter::repeat_with;
use std::mem;

/// A priority queue maintains a partial ordering of its elements such that the least element can
/// always be found in constant time. `put()` and `pop()` operations require `O(log(size))` time, but the `remove()`
/// operation is implemented with a linear cost.
///
/// # Note
/// This class pre-allocates an array of length `max_size + 1` and pre-fills it with elements if instantiated via the
/// [`PriorityQueue::with_sentinel_object`](#method.with_sentinel_object) constructor.
///
/// # Note
/// Iteration order is not specified.
///
/// # Note
/// This is an internal API.
pub struct PriorityQueue<T, C>
where
    C: Compare<T>,
{
    size: usize,
    max_size: usize,
    heap: Vec<T>,
    compare: C,
}

impl<T, C> PriorityQueue<T, C>
where
    C: Compare<T>,
    T: Default + PartialEq,
{
    pub fn heap(&self) -> &Vec<T> {
        &self.heap
    }
    pub fn get_compare(&self) -> &C {
        &self.compare
    }
    /// Creates a priority queue that is pre-filled with sentinel objects, so that the code which uses
    /// that queue can always assume it's full and only change the top without attempting to insert any
    /// new object.
    ///
    /// # Description
    /// Those sentinel values should always compare worse than any non-sentinel value (i.e.,
    /// [`lessThan`](Compare::less_than) should always favor the non-sentinel values).
    ///
    /// By default, the supplier returns `None`, which means the queue will not be filled with
    /// sentinel values. Otherwise, the value returned will be used to pre-populate the queue.
    ///
    /// # Usage
    /// If this method is extended to return a non-None value, the following usage pattern is
    /// recommended:
    ///
    /// ```text
    /// let mut pq: MyQueue<MyObject> = MyQueue::new(num_hits);
    /// // Save the 'top' element, which is guaranteed to not be null.
    /// let mut pq_top = pq.top();
    /// // Now, in order to add a new element that is 'better' than the top (after
    /// // you've verified it is better), it is as simple as:
    /// pq_top.change();
    /// pq_top = pq.update_top();
    /// ```
    ///
    /// # Note
    /// The given supplier will be called `max_size` times, relying on a new object to be returned
    /// and will not check if it's `None` again. Therefore, you should ensure any call to this method
    /// creates a new instance and behaves consistently, e.g., it cannot return `None` if it previously
    /// returned a non-null value, and all returned instances must be comparable using [`lessThan`](Compare::less_than).
    pub fn with_sentinel_object<F>(
        max_size: i32,
        sentinel_object_supplier: F,
        compare: C,
    ) -> Result<PriorityQueue<T, C>, RuntimeError>
    where
        F: Fn() -> Option<T>,
        C: Compare<T>,
    {
        let heap_size = if 0 == max_size {
            // We allocate 1 extra to avoid if statement in top()
            2
        } else {
            if !(0..i32::MAX).contains(&max_size) {
                return Err(RuntimeError::illegal_argument(format!(
                    "maxSize must be >= 0 and < {}; got: {}",
                    i32::MAX,
                    max_size
                )));
            }
            // NOTE: we add +1 because all access to heap is
            // 1-based not 0-based.  heap[0] is unused.
            (max_size + 1) as usize
        };
        let mut heap: Vec<T> = Vec::with_capacity(heap_size);
        heap.resize_with(heap_size, Default::default);
        if let Some(sentinel) = sentinel_object_supplier() {
            heap[1] = sentinel;
            for (i, value) in repeat_with(|| sentinel_object_supplier().unwrap())
                .take(heap_size)
                .enumerate()
                .skip(2)
            {
                heap[i] = value;
            }
            return Ok(PriorityQueue {
                max_size: max_size as usize,
                size: heap_size,
                heap,
                compare,
            });
        }
        Ok(PriorityQueue {
            max_size: max_size as usize,
            size: 0,
            heap,
            compare,
        })
    }

    // construct
    pub fn new(max_size: i32, compare: C) -> Result<PriorityQueue<T, C>, RuntimeError> {
        Self::with_sentinel_object(max_size, || None, compare)
    }

    /// Adds all elements of the collection into the queue. This method should be preferred over
    /// calling [`add`](#method.add) in a loop if all elements are known in advance, as it builds the queue
    /// faster.
    ///
    /// # Errors
    /// If one tries to add more objects than the `max_size` passed in the constructor, an
    /// [`ArrayIndexOutOfBoundsError`](crate::util::error::array_index_out_of_bounds) is thrown.
    pub fn add_all(&mut self, elements: Vec<T>) -> Result<(), RuntimeError> {
        if (self.size + elements.len()) > self.max_size {
            return Err(RuntimeError::array_index_out_of_bounds(format!(
                "Cannot add {} elements to a queue with remaining capacity: {}",
                elements.len(),
                self.max_size - self.size
            )));
        }
        // Heap with size S always takes first S elements of the array,
        // and thus it's safe to fill array further - no actual non-sentinel value will be overwritten.
        for element in elements.into_iter() {
            self.heap[self.size + 1] = element;
            self.size += 1;
        }

        // The loop goes down to 1 as heap is 1-based not 0-based.
        for i in (1..=(self.size >> 1)).rev() {
            self.down_heap(i);
        }
        Ok(())
    }

    /// Adds an object to a priority queue in `O(log(size))` time. If more objects are added than the
    /// `max_size` initialized, an [`ArrayIndexOutOfBoundsError`](crate::util::error::array_index_out_of_bounds) is thrown.
    ///
    /// # Returns
    /// The new 'top' element in the queue.
    pub fn add(&mut self, element: T) -> &T {
        let index = self.size + 1;
        self.heap[index] = element;
        self.size = index;
        self.up_heap(index);
        &self.heap[1]
    }

    /// Adds an object to a priority queue in `O(log(size))` time. It returns the object (if any) that was
    /// dropped off the heap because it was full. This can be the given parameter (if it is smaller than the
    /// full heap's minimum and couldn't be added), or another object that was previously the smallest value
    /// in the heap and now has been replaced by a larger one, or `None` if the queue wasn't yet full with `max_size` elements.
    pub fn insert_with_overflow(&mut self, element: T) -> Option<T> {
        if self.size < self.max_size {
            self.add(element);
            None
        } else if self.size > 0 && self.compare.less_than(&self.heap[1], &element) {
            let ret = mem::replace(&mut self.heap[1], element);
            self.update_top();
            Some(ret)
        } else {
            Some(element)
        }
    }

    /// Returns the least element of the PriorityQueue in constant time.
    pub fn top(&self) -> &T {
        // We don't need to check size here: if maxSize is 0,
        // then heap is length 2 array with both entries null.
        // If size is 0 then heap[1] is already null.
        &self.heap[1]
    }

    /// Removes and returns the least element of the PriorityQueue in log(size) time.
    pub fn pop(&mut self) -> Option<T> {
        if self.size > 0 {
            self.heap.swap(1, self.size);
            let result = self.heap.remove(self.size);
            // With size as a sentinel value, we add an invalid value to prevent the length of the Vec from changing
            self.heap.push(T::default());
            self.size -= 1;
            self.down_heap(1);
            Some(result)
        } else {
            None
        }
    }

    /// Should be called when the object at the top changes values. It's still `O(log(n))` in the worst case,
    /// but it's at least twice as fast to:
    ///
    /// ```text
    /// pq.top().change();
    /// pq.update_top();
    /// ```
    ///
    /// instead of:
    ///
    /// ```text
    /// let mut o = pq.pop();
    /// o.change();
    /// pq.push(o);
    /// ```
    ///
    /// # Returns
    /// The new 'top' element.

    pub fn update_top(&mut self) -> &T {
        self.down_heap(1);
        &self.heap[1]
    }

    /// Replace the top of the pq with `newTop` and run `updateTop()`.
    pub fn update_top_with_new_top(&mut self, new_top: T) -> &T {
        self.heap[1] = new_top;
        self.update_top()
    }

    /// Returns the number of elements currently stored in the PriorityQueue.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Removes all entries from the PriorityQueue.
    pub fn clear(&mut self) {
        self.heap.clear();
        self.size = 0;
    }

    /// Removes an existing element currently stored in the priority queue. The cost is linear with the size
    /// of the queue. (A specialization of the priority queue that tracks element positions would provide a
    /// constant remove time, but the trade-off would be extra cost to all additions/insertions.)
    pub fn remove(&mut self, element: &T) -> bool {
        if let Some(i) = (1..=self.size).next() {
            if self.heap[i] == *element {
                self.heap.swap(i, self.size);
            }
            self.size -= 1;
            if i <= self.size && !self.up_heap(i) {
                self.down_heap(i);
            }
            return true;
        }
        false
    }

    pub fn up_heap(&mut self, orig_pos: usize) -> bool {
        let mut i = orig_pos;
        let mut j = i >> 1;
        while j > 0 && self.compare.less_than(&self.heap[i], &self.heap[j]) {
            self.heap.swap(i, j);
            i = j;
            j = i >> 1;
        }
        i != orig_pos
    }

    pub fn down_heap(&mut self, mut i: usize) {
        let size = self.size;
        while i * 2 <= size {
            let mut j = i * 2;
            let k = j + 1;

            if k <= size && self.compare.less_than(&self.heap[k], &self.heap[j]) {
                j = k;
            }

            if !self.compare.less_than(&self.heap[j], &self.heap[i]) {
                break;
            }

            self.heap.swap(i, j);
            i = j;
        }
    }

    /// This method returns the internal heap array as `Vec<Object>`.
    ///
    /// # Note
    /// This is an internal API.
    #[allow(dead_code)]
    fn get_heap_array(&self) -> &Vec<T> {
        &self.heap
    }

    pub fn iterator(&self) -> PriorityQueueIterator<T, C> {
        PriorityQueueIterator::new(self)
    }
}

/// Each call can start iterating over the elements in the priority queue from the beginning.
/// The access order is not sorted; if a sorted order is required, you can directly use [`pop`](PriorityQueue::pop).
pub struct PriorityQueueIterator<'a, T, C>
where
    C: Compare<T>,
    T: PartialEq,
{
    pq: &'a PriorityQueue<T, C>,
    index: usize,
}
impl<'a, T, C> PriorityQueueIterator<'a, T, C>
where
    C: Compare<T>,
    T: PartialEq,
{
    fn new(pq: &'a PriorityQueue<T, C>) -> Self {
        Self { pq, index: 0 }
    }
}
impl<'a, T, C> Iterator for PriorityQueueIterator<'a, T, C>
where
    C: Compare<T>,
    T: PartialEq,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.pq.size {
            let result = &self.pq.heap[self.index + 1];
            self.index += 1;
            return Some(result);
        }
        None
    }
}

pub trait Compare<T> {
    /// Determines the ordering of objects in this priority queue. Subclasses must define this method.
    ///
    /// # Arguments
    /// * `a` - The first object to compare.
    /// * `b` - The second object to compare.
    ///
    /// # Returns
    /// `true` if parameter `a` is less than parameter `b`.
    fn less_than(&self, a: &T, b: &T) -> bool;
}
