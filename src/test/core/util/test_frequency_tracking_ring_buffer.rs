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
// Migrated from src/core/util/frequency_tracking_ring_buffer.rs

use crate::core::util::error::lucene_error::Result;
use crate::core::util::frequency_tracking_ring_buffer::*;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::RngExt;
use std::collections::HashMap;
#[allow(dead_code)] // for quick search
struct TestFrequencyTrackingRingBuffer;
fn assert_buffer(
  buffer: &FrequencyTrackingRingBuffer,
  max_size: usize,
  sentinel: i32,
  items: &[i32],
) {
  let recent_items = if items.len() <= max_size {
    let mut v = vec![sentinel; max_size - items.len()];
    v.extend_from_slice(items);
    v
  } else {
    items[items.len() - max_size..].to_vec()
  };

  let mut expected_frequencies: HashMap<i32, i32> = HashMap::new();
  for &item in &recent_items {
    *expected_frequencies.entry(item).or_insert(0) += 1;
  }

  assert_eq!(expected_frequencies, buffer.as_frequency_map());
}
#[test]
fn test_frequency_tracking_ring_buffer_randomized() -> Result<()> {
  let mut random = random();
  let iterations = 100 + random.random_range(0..50);

  for _ in 0..iterations {
    let max_size = 2 + random.random_range(0..100);
    let num_items = random.random_range(0..5000);
    let max_item = 1 + random.random_range(0..100);
    let sentinel = random.random_range(0..200);

    let mut items = Vec::with_capacity(num_items);
    let mut buffer = FrequencyTrackingRingBuffer::new(max_size, sentinel)?;

    for _ in 0..num_items {
      let item = random.random_range(0..max_item);
      items.push(item);
      buffer.add(item);
    }

    assert_buffer(&buffer, max_size, sentinel, &items);
  }
  Ok(())
}

#[test]
fn test_ram_bytes_used() {
  // TODO
}
