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
use crate::test_framework::core::util::lucene_test_case::{at_least, random};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use parking_lot::Mutex;
use rand::RngExt;
use rand::rng;

use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::blocking_float_heap::BlockingFloatHeap;

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
