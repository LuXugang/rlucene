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
use crate::test::core::util::lucene_test_case::{at_least, random};
use rand::RngExt;

use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::float_heap::FloatHeap;

#[allow(dead_code)] // for quick search
struct TestFloatHeap;
#[test]
fn test_basic_operations() -> Result<()> {
  let mut heap = FloatHeap::new(3)?;

  heap.offer(2.0);
  heap.offer(4.0);
  heap.offer(1.0);
  heap.offer(3.0);
  assert_eq!(heap.size(), 3);
  assert!((heap.peek() - 2.0).abs() < f32::EPSILON);
  assert!((heap.poll()? - 2.0).abs() < f32::EPSILON);
  assert!((heap.poll()? - 3.0).abs() < f32::EPSILON);
  assert!((heap.poll()? - 4.0).abs() < f32::EPSILON);
  assert_eq!(heap.size(), 0);
  Ok(())
}

#[test]
fn test_basic_operations2() -> Result<()> {
  let mut random = random();
  let size = at_least(&mut random, 10);
  let mut heap = FloatHeap::new(size as usize)?;

  let mut sum = 0.0;
  let mut sum2 = 0.0;

  for _ in 0..size {
    let next: f32 = random.random_range(0.0..100.0);
    sum += next as f64;
    heap.offer(next);
  }

  let mut last = f32::NEG_INFINITY;
  for _ in 0..size {
    let next = heap.poll()?;
    assert!(next >= last);
    last = next;
    sum2 += last as f64;
  }

  assert!((sum - sum2).abs() < 0.01);
  Ok(())
}
#[test]
fn test_clear() -> Result<()> {
  let mut heap = FloatHeap::new(3)?;

  heap.offer(20.0);
  heap.offer(40.0);
  heap.offer(30.0);

  assert_eq!(heap.size(), 3);
  assert!((heap.peek() - 20.0).abs() < f32::EPSILON);

  heap.clear();
  assert_eq!(heap.size(), 0);
  assert!((heap.peek() - 20.0).abs() < f32::EPSILON);

  heap.offer(15.0);
  heap.offer(35.0);

  assert_eq!(heap.size(), 2);
  assert!((heap.peek() - 15.0).abs() < f32::EPSILON);

  assert!((heap.poll()? - 15.0).abs() < f32::EPSILON);
  assert!((heap.poll()? - 35.0).abs() < f32::EPSILON);
  assert_eq!(heap.size(), 0);

  Ok(())
}
