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
use crate::test::core::util::lucene_test_case::random as new_random;
use rand::RngExt;
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, Barrier};
use std::time::Instant;

use crate::core::store::rate_limiter::{RateLimiter, SimpleRateLimiter};
use crate::core::util::error::lucene_error::Result;

/// Simple testcase for RateLimiter.SimpleRateLimiter
#[allow(dead_code)] // for quick search
struct TestRateLimiter;

// LUCENE-6075
#[test]
fn test_overflow_int() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_threads() -> Result<()> {
  let mut random = new_random();

  let target_mb_per_sec = 10.0 + 20.0 * random.random::<f64>();
  let limiter = Arc::new(SimpleRateLimiter::new(target_mb_per_sec));

  let num_threads = random.random_range(3..=6);
  let barrier = Arc::new(Barrier::new(num_threads));
  let tot_bytes = Arc::new(AtomicI64::new(0));

  let mut handles = Vec::new();
  for _ in 0..num_threads {
    let limiter = Arc::clone(&limiter);
    let barrier = Arc::clone(&barrier);
    let tot_bytes = Arc::clone(&tot_bytes);
    let mut thread_random = new_random();

    handles.push(std::thread::spawn(move || {
      barrier.wait();
      let mut bytes_since_last_pause: i64 = 0;
      for _ in 0..500 {
        let num_bytes = thread_random.random_range(1000..=10000) as i64;
        tot_bytes.fetch_add(num_bytes, std::sync::atomic::Ordering::Relaxed);
        bytes_since_last_pause += num_bytes;
        if bytes_since_last_pause > limiter.get_min_pause_check_bytes() {
          limiter.pause(bytes_since_last_pause).unwrap();
          bytes_since_last_pause = 0;
        }
      }
    }));
  }

  let start = Instant::now();
  for handle in handles {
    handle.join().unwrap();
  }
  let elapsed_secs = start.elapsed().as_secs_f64();
  let actual_mb_per_sec =
    (tot_bytes.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1024.0 / 1024.0) / elapsed_secs;

  // TODO: this may false trip .... could be we can only assert that it never exceeds the max, so
  // slow jenkins doesn't trip:
  let ratio = actual_mb_per_sec / target_mb_per_sec;

  // Only enforce that it wasn't too fast; if machine is bogged down (can't schedule threads /
  // sleep properly) then it may falsely be too slow:
  assert!(
    ratio >= 0.9,
    "actualMBPerSec={actual_mb_per_sec} targetMBPerSec={target_mb_per_sec}"
  );
  assert!(
    ratio <= 1.1,
    "targetMBPerSec={target_mb_per_sec} actualMBPerSec={actual_mb_per_sec}"
  );
  Ok(())
}
