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
use crate::test::core::util::lucene_test_case::random;
use rand::RngExt;

use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::neighbor_queue::NeighborQueue;

#[allow(dead_code)] // for quick search
struct TestNeighborQueue;
#[test]
fn test_neighbors_product() -> Result<()> {
  let mut nn = NeighborQueue::new(2, false)?;

  assert!(nn.insert_with_overflow(2, 0.5)?);
  assert!(nn.insert_with_overflow(1, 0.2)?);
  assert!(nn.insert_with_overflow(3, 1.0)?);

  assert!((nn.top_score() - 0.5).abs() < f32::EPSILON);
  nn.pop()?;
  assert!((nn.top_score() - 1.0).abs() < f32::EPSILON);
  nn.pop()?;
  Ok(())
}
#[test]
fn test_neighbors_max_heap() -> Result<()> {
  let mut nn = NeighborQueue::new(2, true)?;

  assert!(nn.insert_with_overflow(2, 2.0)?);
  assert!(nn.insert_with_overflow(1, 1.0)?);
  assert!(!nn.insert_with_overflow(3, 3.0)?);

  assert!((nn.top_score() - 2.0).abs() < f32::EPSILON);
  nn.pop()?;
  assert!((nn.top_score() - 1.0).abs() < f32::EPSILON);

  Ok(())
}
#[test]
fn test_top_max_heap() -> Result<()> {
  let mut nn = NeighborQueue::new(2, true)?;

  nn.add(1, 2.0)?;
  nn.add(2, 1.0)?;

  assert!((nn.top_score() - 2.0).abs() < f32::EPSILON);
  assert_eq!(nn.top_node(), 1);

  Ok(())
}
#[test]
fn test_top_min_heap() -> Result<()> {
  let mut nn = NeighborQueue::new(2, false)?;

  nn.add(1, 0.5)?;
  nn.add(2, -0.5)?;

  assert!((nn.top_score() + 0.5).abs() < f32::EPSILON);
  assert_eq!(nn.top_node(), 2);

  Ok(())
}
#[test]
fn test_visited_count() -> Result<()> {
  let mut nn = NeighborQueue::new(2, false)?;

  nn.set_visited_count(100);
  assert_eq!(nn.visited_count(), 100);

  Ok(())
}
#[test]
fn test_clear() -> Result<()> {
  let mut nn = NeighborQueue::new(2, false)?;

  nn.add(1, 1.1)?;
  nn.add(2, -2.2)?;
  nn.set_visited_count(42);
  nn.mark_incomplete();
  nn.clear();

  assert_eq!(nn.size(), 0);
  assert_eq!(nn.visited_count(), 0);
  assert!(!nn.incomplete());

  Ok(())
}
#[test]
fn test_max_size_queue() -> Result<()> {
  let mut nn = NeighborQueue::new(2, false)?;

  nn.add(1, 1.0)?;
  nn.add(2, 2.0)?;
  assert_eq!(nn.size(), 2);
  assert_eq!(nn.top_node(), 1);
  // insertWithOverflow does not extend the queue
  assert!(nn.insert_with_overflow(3, 3.0)?);
  assert_eq!(nn.size(), 2);
  assert_eq!(nn.top_node(), 2);
  // add does extend the queue beyond maxSize
  nn.add(4, 1.0)?;
  assert_eq!(nn.size(), 3);

  Ok(())
}

#[test]
fn test_unbounded_queue() -> Result<()> {
  let mut random = random();
  let mut nn = NeighborQueue::new(1, true)?;
  let mut max_score = -2.0f32;
  let mut max_node: Option<usize> = None;

  for i in 0..256 {
    // initial size is 32
    let score: f32 = random.random();
    if score > max_score {
      max_score = score;
      max_node = Some(i);
    }
    nn.add(i, score)?;
  }

  assert!((nn.top_score() - max_score).abs() < f32::EPSILON);
  assert_eq!(Some(nn.top_node()), max_node);

  Ok(())
}
#[test]
fn test_invalid_arguments() {
  let result = NeighborQueue::new(0, false);
  assert!(result.is_err());
}
#[test]
fn test_to_string() -> Result<()> {
  let nn = NeighborQueue::new(2, false)?;
  assert_eq!(nn.to_string(), "Neighbors[0]");
  Ok(())
}
