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
// Migrated from src/core/util/long_heap.rs

use rand::Rng;
use rand::RngExt;

use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::long_heap::LongHeap;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
#[allow(dead_code)] // for quick search
struct TestLongHeap;
/// Checks that the heap property is maintained.
fn check_validity(heap: &LongHeap) {
  let heap_array = heap.get_heap_array();
  let size = heap.size();
  for i in 2..=size {
    let parent = i >> 1;
    assert!(
      heap_array[parent] <= heap_array[i],
      "Heap property violated at index {}: parent={} > child={}",
      i,
      heap_array[parent],
      heap_array[i]
    );
  }
}

#[test]
fn test_pq_basic() -> Result<()> {
  let mut random = random();
  test_pq_with_random(10_000, &mut random)
}

fn test_pq_with_random<R>(count: usize, random: &mut R) -> Result<()>
where
  R: Rng + ?Sized,
{
  let mut pq = LongHeap::new(count)?;
  let mut sum: i64 = 0;
  let mut sum2: i64 = 0;

  for _ in 0..count {
    let next = random.random();
    sum += next;
    pq.push(next);
    check_validity(&pq);
  }

  let mut last = i64::MIN;
  for _ in 0..count {
    let next = pq.pop()?;
    assert!(
      next >= last,
      "Heap out of order: current {} < last {}",
      next,
      last
    );
    last = next;
    sum2 += last;
  }

  assert_eq!(sum, sum2, "Sum mismatch after all pops");
  Ok(())
}
#[test]
fn test_clear() -> Result<()> {
  let mut pq = LongHeap::new(3)?;
  pq.push(2);
  pq.push(3);
  pq.push(1);
  assert_eq!(3, pq.size());
  pq.clear();
  assert_eq!(0, pq.size());
  Ok(())
}
#[test]
fn test_exceed_bounds() -> Result<()> {
  let mut pq = LongHeap::new(1)?;
  pq.push(2);
  pq.push(0);
  assert_eq!(2, pq.size());
  assert_eq!(0, pq.top());
  Ok(())
}
#[test]
fn test_fixed_size() -> Result<()> {
  let mut pq = LongHeap::new(3)?;
  pq.insert_with_overflow(2);
  pq.insert_with_overflow(3);
  pq.insert_with_overflow(1);
  pq.insert_with_overflow(5);
  pq.insert_with_overflow(7);
  pq.insert_with_overflow(1);
  assert_eq!(3, pq.size());
  assert_eq!(3, pq.top());
  Ok(())
}

#[test]
fn test_duplicate_values() -> Result<()> {
  let mut pq = LongHeap::new(3)?;
  pq.push(2);
  pq.push(3);
  pq.push(1);
  assert_eq!(1, pq.top());
  pq.update_top(3);
  assert_eq!(3, pq.size());
  assert_eq!(&[0, 2, 3, 3], pq.get_heap_array());
  Ok(())
}

#[test]
fn test_insertions() -> Result<()> {
  let mut random = random();
  let num_docs_in_pq = random.random_range(1..=100);
  let mut pq = LongHeap::new(num_docs_in_pq)?;
  let mut last_least: Option<i64> = None;

  for _ in 0..(num_docs_in_pq * 10) {
    let new_entry = random.random();
    pq.insert_with_overflow(new_entry);
    check_validity(&pq);
    let new_least = pq.top();
    if let Some(last) = last_least.filter(|&last| new_least != new_entry && new_least != last) {
      assert!(new_least <= new_entry);
      assert!(new_least >= last);
    }
    last_least = Some(new_least);
  }
  Ok(())
}

#[test]
fn test_invalid() -> Result<()> {
  assert!(matches!(
    LongHeap::new(0),
    Err(LuceneError::IllegalArgument(_))
  ));
  // TODO: see ArrayUtil::MAX_ARRAY_LENGTH
  // assert!(matches!(
  //     LongHeap::new(ArrayUtil::MAX_ARRAY_LENGTH as i32),
  //     Err(LuceneError::IllegalArgument(_))
  // ));
  Ok(())
}

#[test]
fn test_unbounded() -> Result<()> {
  let mut random = random();
  let initial_size = random.random_range(1..=10);
  let mut pq = LongHeap::new(initial_size)?;
  let num = random.random_range(1..=100);
  let mut max_value = i64::MIN;
  let mut count = 0;

  for _ in 0..num {
    let value: i64 = random.random();
    if random.random_bool(0.5) {
      pq.push(value);
      count += 1;
    } else {
      let full = pq.size() >= initial_size;
      if pq.insert_with_overflow(value) && !full {
        count += 1;
      }
    }
    max_value = std::cmp::max(max_value, value);
  }

  assert_eq!(count, pq.size());
  let mut last = i64::MIN;
  while pq.size() > 0 {
    let top = pq.top();
    let next = pq.pop()?;
    assert_eq!(top, next);
    count -= 1;
    assert!(next >= last);
    last = next;
  }
  assert_eq!(0, count);
  assert_eq!(max_value, last);
  Ok(())
}
