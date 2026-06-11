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
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::core::util::error::lucene_error::Result;

/**
 * Abstract base class to rate limit IO. Typically implementations are shared across multiple
 * IndexInputs or IndexOutputs (for example those involved all merging). Those IndexInputs and
 * IndexOutputs would call [`RateLimiter::pause`] whenever the have read or written more than
 * [`RateLimiter::get_min_pause_check_bytes`] bytes.
 */
pub trait RateLimiter {
  /**
   * Sets an updated MB per second rate limit. A subclass is allowed to perform dynamic updates of
   * the rate limit during use.
   */
  fn set_mb_per_sec(&self, mb_per_sec: f64);

  /** The current MB per second rate limit. */
  fn get_mb_per_sec(&self) -> f64;

  /**
   * Pauses, if necessary, to keep the instantaneous IO rate at or below the target.
   *
   * <p>Note: the implementation is thread-safe
   *
   * @return the pause time in nano seconds
   */
  fn pause(&self, bytes: i64) -> Result<i64>;

  /**
   * How many bytes caller should add up itself before invoking [`RateLimiter::pause`]. NOTE: The value
   * returned by this method may change over time and is not guaranteed to be constant throughout
   * the lifetime of the RateLimiter. Users are advised to refresh their local values with calls to
   * this method to ensure consistency.
   */
  fn get_min_pause_check_bytes(&self) -> i64;
}

const MIN_PAUSE_CHECK_MSEC: i64 = 5;

/** Simple class to rate limit IO. */
pub struct SimpleRateLimiter {
  mb_per_sec: AtomicU64,
  min_pause_check_bytes: AtomicU64,
  last_instant: Mutex<Instant>,
}

impl SimpleRateLimiter {
  /** mbPerSec is the MB/sec max IO rate */
  pub fn new(mb_per_sec: f64) -> Self {
    let limiter = Self {
      mb_per_sec: AtomicU64::new(0),
      min_pause_check_bytes: AtomicU64::new(0),
      last_instant: Mutex::new(Instant::now()),
    };
    limiter.set_mb_per_sec(mb_per_sec);
    limiter
  }
}

impl RateLimiter for SimpleRateLimiter {
  /** Sets an updated mb per second rate limit. */
  fn set_mb_per_sec(&self, mb_per_sec: f64) {
    self
      .mb_per_sec
      .store(mb_per_sec.to_bits(), Ordering::SeqCst);
    let min_pause_check =
      ((MIN_PAUSE_CHECK_MSEC as f64 / 1000.0) * mb_per_sec * 1024.0 * 1024.0) as i64;
    self
      .min_pause_check_bytes
      .store(min_pause_check as u64, Ordering::SeqCst);
  }

  fn get_min_pause_check_bytes(&self) -> i64 {
    self.min_pause_check_bytes.load(Ordering::SeqCst) as i64
  }

  /** The current mb per second rate limit. */
  fn get_mb_per_sec(&self) -> f64 {
    f64::from_bits(self.mb_per_sec.load(Ordering::SeqCst))
  }

  /**
   * Pauses, if necessary, to keep the instantaneous IO rate at or below the target. Be sure to
   * only call this method when bytes &gt; [`RateLimiter::get_min_pause_check_bytes`], otherwise it will pause
   * way too long!
   *
   * @return the pause time in nano seconds
   */
  fn pause(&self, bytes: i64) -> Result<i64> {
    let start = Instant::now();

    let seconds_to_pause = (bytes as f64 / 1024.0 / 1024.0) / self.get_mb_per_sec();

    let target;

    // Sync'd to read + write last_instant:
    {
      let mut last = self.last_instant.lock().unwrap();

      // Time we should sleep until; this is purely instantaneous
      // rate (just adds seconds onto the last time we had paused to);
      // maybe we should also offer decayed recent history one?
      target = *last + Duration::from_secs_f64(seconds_to_pause);

      if start >= target {
        // OK, current time is already beyond the target sleep time,
        // no pausing to do.

        // Set to start, not target, to enforce the instant rate, not
        // the "averaaged over all history" rate:
        *last = start;
        return Ok(0);
      }

      *last = target;
    }

    let mut cur = start;

    // While loop because Thread.sleep doesn't always sleep
    // enough:
    // NOTE: using park_timeout instead of sleep so that the thread can be
    // interrupted by another thread calling unpark(), analogous to Java's
    // Thread.interrupt() which interrupts Thread.sleep(). In Java this throws
    // ThreadInterruptedException; in Rust park_timeout returns early without
    // error, and the loop re-checks the condition.
    loop {
      match target.checked_duration_since(cur) {
        Some(pause_dur) if pause_dur > Duration::ZERO => {
          // NOTE: except maybe on real-time JVMs, minimum realistic sleep time
          // is 1 msec; if you pass just 1 nsec the default impl rounds
          // this up to 1 msec:
          thread::park_timeout(pause_dur);
          cur = Instant::now();
          continue;
        },
        _ => break,
      }
    }

    Ok((cur - start).as_nanos() as i64)
  }
}
