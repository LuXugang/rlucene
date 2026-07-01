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
// Migrated from src/core/util/priority_queue.rs

use crate::test::support::core::util::lucene_test_case::{at_least, at_least_usize, random};
use std::fmt::Debug;

use rand::RngExt;

use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::priority_queue::{Compare, PriorityQueue};
use crate::test::support::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestPriorityQueue;

struct I32Compare;

impl Compare<i32> for I32Compare {
  fn less_than(&self, a: &i32, b: &i32) -> Result<bool> {
    Ok(a < b)
  }
}
#[test]
fn test_zero_sized_queue() -> Result<()> {
  let mut random = random();
  let mut pq = PriorityQueue::new(0, I32Compare)?;
  assert_eq!(1, pq.insert_with_overflow(1)?.expect("not fail"));
  assert_eq!(0, pq.size());

  pq.add(1)?;
  match random.random_bool(0.5) {
    true => assert_eq!(
      1,
      *pq
        .top_mut()
        .expect("priority queue top element should exist")
    ),
    false => assert_eq!(
      1,
      *pq.top().expect("priority queue top element should exist")
    ),
  }
  Ok(())
}

#[derive(Default)]
struct ObjectCompare {
  index: i32,
  value: i32,
}

impl PartialEq for ObjectCompare {
  fn eq(&self, other: &Self) -> bool {
    if self.index == other.index && self.value == other.value {
      return true;
    }
    false
  }
}

impl ObjectCompare {
  fn new(index: i32, value: i32) -> Self {
    ObjectCompare { index, value }
  }
}

impl Compare<ObjectCompare> for ObjectCompare {
  fn less_than(&self, a: &ObjectCompare, b: &ObjectCompare) -> Result<bool> {
    Ok(a.value < b.value)
  }
}

#[test]
fn test_no_extra_work_on_equal_elements() -> Result<()> {
  let mut pq = PriorityQueue::new(5, ObjectCompare::default())?;
  for i in 0..100 {
    pq.insert_with_overflow(ObjectCompare::new(i, 0))?;
  }
  // Ref
  {
    let mut indexes: Vec<i32> = Vec::new();
    let iter = pq.iter_ref();
    for e in iter {
      indexes.push(e.index)
    }
    assert_eq!(indexes, vec![0, 1, 2, 3, 4]);
  }
  // ownership
  {
    let mut indexes: Vec<i32> = Vec::new();
    let into_iter = pq.iter();
    for e in into_iter {
      indexes.push(e.index)
    }
    assert_eq!(indexes, vec![0, 1, 2, 3, 4]);
  }

  Ok(())
}

#[test]
fn test_pq() -> Result<()> {
  let mut random = random();
  let count = at_least(&mut random, 10000);
  let pq = PriorityQueue::new(count as usize, I32Compare);
  if let Ok(mut heap) = pq {
    let mut sum: i32 = 0;
    let mut sum2: i32 = 0;
    for _i in 0..count {
      let next: i32 = random.random();
      sum = sum.wrapping_add(next);
      heap.add(next)?;
    }

    let mut last = i32::MIN;
    for _i in 0..count {
      let next = heap.pop()?.expect("not fail");
      assert!(next >= last);
      last = next;
      sum2 = sum2.wrapping_add(last);
    }

    assert_eq!(sum, sum2);
  } else {
    assert!(count == 0 || count == i32::MAX);
  }
  Ok(())
}

#[test]
fn test_clear() -> Result<()> {
  let mut pq = PriorityQueue::new(3, I32Compare)?;
  pq.add(2)?;
  pq.add(3)?;
  pq.add(1)?;
  assert_eq!(3, pq.size());
  pq.clear();
  assert_eq!(0, pq.size());
  Ok(())
}

#[test]
fn test_fixed_size() -> Result<()> {
  let mut pq = PriorityQueue::new(3, I32Compare)?;
  pq.insert_with_overflow(2)?;
  pq.insert_with_overflow(3)?;
  pq.insert_with_overflow(1)?;
  pq.insert_with_overflow(5)?;
  pq.insert_with_overflow(7)?;
  pq.insert_with_overflow(1)?;
  assert_eq!(3, pq.size());
  assert_eq!(3, pq.pop_unchecked()?);
  Ok(())
}

#[test]
fn test_insert_with_overflow() -> Result<()> {
  let size = 4;
  let mut pq = PriorityQueue::new(size, I32Compare)?;
  let i1 = 2;
  let i2 = 3;
  let i3 = 1;
  let i4 = 5;
  let i5 = 7;
  let i6 = 1;

  assert_eq!(pq.insert_with_overflow(i1)?, None);
  assert_eq!(pq.insert_with_overflow(i2)?, None);
  assert_eq!(pq.insert_with_overflow(i3)?, None);
  assert_eq!(pq.insert_with_overflow(i4)?, None);
  assert_eq!(pq.insert_with_overflow(i5)?.expect("not fail"), i3);
  assert_eq!(pq.insert_with_overflow(i6)?.expect("not fail"), i6);
  assert_eq!(size, pq.size());
  let mut random = random();
  match random.random_bool(0.5) {
    true => assert_eq!(
      2,
      *pq
        .top_mut()
        .expect("priority queue top element should exist")
    ),
    false => assert_eq!(
      2,
      *pq.top().expect("priority queue top element should exist")
    ),
  }
  Ok(())
}

#[test]
fn test_add_all_to_empty_queue() -> Result<()> {
  let mut random = random();
  let size = 10;
  let mut list: Vec<i32> = Vec::new();
  let mut list2: Vec<i32> = Vec::new();
  let mut value: i32;
  for _i in 0..size {
    value = random.random();
    list.push(value);
    list2.push(value);
  }
  let mut pq = PriorityQueue::new(size, I32Compare)?;
  pq.add_all(list)?;
  check_validity(&pq);
  assert_ordered_when_drained(&mut pq, list2);
  Ok(())
}

#[test]
fn test_add_all_to_partially_filled_queue() -> Result<()> {
  let mut pq = PriorityQueue::new(20, I32Compare)?;
  let mut one_by_one: Vec<i32> = Vec::new();
  let mut bulk_added: Vec<i32> = Vec::new();
  let mut bulk_added2: Vec<i32> = Vec::new();
  let mut random = random();

  for _i in 0..10 {
    let value: i32 = random.random();
    bulk_added.push(value);
    bulk_added2.push(value);
    let x: i32 = random.random();
    pq.add(x)?;
    one_by_one.push(x);
  }

  pq.add_all(bulk_added)?;
  check_validity(&pq);

  one_by_one.append(&mut bulk_added2);
  assert_ordered_when_drained(&mut pq, one_by_one);
  Ok(())
}

#[test]
fn test_add_all_does_not_fit_into_queue() -> Result<()> {
  let mut pq = PriorityQueue::new(20, I32Compare)?;
  let mut list: Vec<i32> = Vec::new();
  let mut random = random();
  for _i in 0..11 {
    list.push(random.random());
    pq.add(random.random())?;
  }
  let result = pq.add_all(list).unwrap_err().to_string();
  assert_eq!(
    result,
    "Cannot add 11 elements to a queue with remaining capacity: 9"
  );
  Ok(())
}

#[test]
fn test_removals_and_insertions() -> Result<()> {
  let mut random = random();
  let num_docs_in_pq = TestUtil::next_usize(&mut random, 1, 100);
  let mut pq = PriorityQueue::new(num_docs_in_pq, I32Compare)?;
  let mut last_least: Option<i32> = None;

  // Basic insertion of new content
  let mut sds: Vec<i32> = Vec::with_capacity(num_docs_in_pq);
  for _i in 0..num_docs_in_pq * 10 {
    let new_entry = random.random::<i32>().abs();
    sds.push(new_entry);
    let evicted = pq.insert_with_overflow(new_entry)?;
    check_validity(&pq);
    if let Some(evicted_value) = evicted {
      let pos = sds.iter().position(|&x| x == evicted_value);
      assert_ne!(pos, None);
      sds.remove(pos.expect("not fail"));
      if evicted_value != new_entry {
        assert_eq!(evicted_value, last_least.expect("not fail"));
      }
    }
    let new_least = match random.random_bool(0.5) {
      true => *pq
        .top_mut()
        .expect("priority queue top element should exist"),
      false => *pq.top().expect("priority queue top element should exist"),
    };
    if let Some(last) = last_least
      && new_least != new_entry
      && new_least != last
    {
      // If there has been a change of least entry and it wasn't our
      // new addition we expect the scores to increase
      assert!(new_least <= new_entry);
      assert!(new_least >= last);
    }
    last_least = Some(new_least);
  }
  // Try many random additions to existing entries - we should always see
  // increasing scores in the lowest entry in the PQ
  for _i in 0..500000 {
    let element = (random.random::<f32>() * ((sds.len() - 1) as f32)) as usize;
    let object_to_remove = sds[element];
    assert_eq!(sds.remove(element), object_to_remove);
    assert!(pq.remove(&object_to_remove)?);
    check_validity(&pq);
    let new_entry = random.random::<i32>().abs();
    sds.push(new_entry);
    assert_eq!(pq.insert_with_overflow(new_entry)?, None);
    check_validity(&pq);
    let new_least = match random.random_bool(0.5) {
      true => *pq
        .top_mut()
        .expect("priority queue top element should exist"),
      false => *pq.top().expect("priority queue top element should exist"),
    };
    if let Some(last) = last_least
      && object_to_remove != last
      && new_least != new_entry
    {
      // If there has been a change of least entry and it wasn't our
      // new addition or the loss of our randomly
      // removed entry we expect the
      // scores to increase
      assert!(new_least <= new_entry);
      assert!(new_least >= last);
    }
    last_least = Some(new_least);
  }
  Ok(())
}

#[test]
fn test_iterator_empty() -> Result<()> {
  let pq = PriorityQueue::new(3, I32Compare)?;
  // ref
  {
    let mut it = pq.iter_ref();
    assert_eq!(it.next(), None);
  }
  // ownership
  {
    let mut it = pq.iter();
    assert_eq!(it.next(), None);
  }
  Ok(())
}

#[test]
fn test_iterator_one() -> Result<()> {
  let mut pq = PriorityQueue::new(3, I32Compare)?;
  pq.add(1)?;
  // ref
  {
    let mut it = pq.iter_ref();
    assert_eq!(it.next(), Some(&1));
  }
  // ownership
  {
    let mut it = pq.iter();
    assert_eq!(it.next(), Some(1));
  }
  Ok(())
}

#[test]
fn test_iterator_two() -> Result<()> {
  let mut pq = PriorityQueue::new(3, I32Compare)?;
  pq.add(1)?;
  pq.add(2)?;
  // ref
  {
    let mut it = pq.iter_ref();
    assert_eq!(it.next(), Some(&1));
    assert_eq!(it.next(), Some(&2));
  }
  // ownership
  {
    let mut it = pq.iter();
    assert_eq!(it.next(), Some(1));
    assert_eq!(it.next(), Some(2));
  }
  Ok(())
}

#[test]
fn test_iterator_random() -> Result<()> {
  let mut random = random();
  let max_size = TestUtil::next_usize(&mut random, 1, 20);
  let mut queue = PriorityQueue::new(max_size, I32Compare)?;
  let iters: usize = at_least_usize(&mut random, 100);
  let mut expected: Vec<i32> = Vec::new();
  for _i in 0..iters {
    if queue.size() == 0 || (queue.size() < max_size) {
      // if queue.size() == 0 || (queue.size() < max_size &&
      // random.random::<bool>()) {
      let value: i32 = random.random_range(0..=10);
      queue.add(value)?;
      expected.push(value);
    } else {
      let pos = expected
        .iter()
        .position(|&x| x == queue.pop().expect("not fail").expect("not fail"));
      assert_ne!(pos, None);
      expected.remove(pos.expect("not fail"));
    }
    let mut actual: Vec<i32> = Vec::new();
    expected.sort();
    for value in queue.iter_ref() {
      actual.push(*value);
    }
    actual.sort();
    assert_eq!(actual, expected);
  }
  Ok(())
}

#[test]
fn test_max_int_size() -> Result<()> {
  let pq = PriorityQueue::new(i32::MAX.try_convert()?, I32Compare);
  assert!(pq.is_err());
  Ok(())
}

fn assert_ordered_when_drained<T, C>(
  pq: &mut PriorityQueue<T, C>,
  mut reference_data_list: Vec<i32>,
) where
  C: Compare<T>,
  T: Into<i32> + Debug + PartialEq,
{
  reference_data_list.sort();
  let mut i = 0;
  let mut value: i32;
  while pq.size() > 0 {
    value = pq.pop_unchecked().expect("not fail").into();
    assert_eq!(reference_data_list[i], value);
    i += 1;
  }
}

fn check_validity<T, C>(pq: &PriorityQueue<T, C>)
where
  C: Compare<T>,
  T: PartialEq + Debug,
{
  let size = pq.size();
  let heap = pq.heap();
  for i in 1..=size {
    let parent = i >> 1;
    if parent > 1 {
      let parent_value = heap[parent]
        .as_ref()
        .expect("priority queue parent should exist");
      let child_value = heap[i].as_ref().expect("priority queue child should exist");
      if !pq
        .get_compare()
        .less_than(parent_value, child_value)
        .expect("not fail")
      {
        assert_eq!(parent_value, child_value);
      }
    }
  }
}
