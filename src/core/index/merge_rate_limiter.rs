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
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use crate::core::index::merge_policy::OneMergeProgress;
use crate::core::index::merge_policy::PauseReason;
use crate::core::store::rate_limiter::RateLimiter;
use crate::core::util::error::lucene_error::{LuceneError, Result};

const MIN_PAUSE_CHECK_MSEC: i64 = 25;

const MIN_PAUSE_NS: i64 = 2_000_000; // 2 milliseconds in nanoseconds.
const MAX_PAUSE_NS: i64 = 250_000_000; // 250 milliseconds in nanoseconds.

/// This is the [`RateLimiter`] that
/// [`IndexWriter`](crate::core::index::index_writer::IndexWriter) assigns to each running merge,
/// giving merge schedulers I/O-priority-like control.
///
/// @lucene.internal
pub struct MergeRateLimiter {
  mb_per_sec: AtomicU64,            // f64 bits (volatile double)
  min_pause_check_bytes: AtomicU64, // i64 bits (volatile long)
  last_ns: AtomicI64,
  total_bytes_written: AtomicI64,
  merge_progress: Arc<OneMergeProgress>,
}

impl MergeRateLimiter {
  /// Creates a new instance.
  pub fn new(merge_progress: Arc<OneMergeProgress>) -> Self {
    // Initially no IO limit; use setter here so minPauseCheckBytes is set:
    let limiter = Self {
      mb_per_sec: AtomicU64::new(0),
      min_pause_check_bytes: AtomicU64::new(0),
      last_ns: AtomicI64::new(0),
      total_bytes_written: AtomicI64::new(0),
      merge_progress,
    };
    // Safe: initialized with a valid MB-per-second rate; ignore the error.
    let _ = limiter.set_mb_per_sec(f64::INFINITY);
    limiter
  }

  /// Returns total bytes written by this merge.
  pub fn get_total_bytes_written(&self) -> i64 {
    self.total_bytes_written.load(Ordering::SeqCst)
  }

  /// Total NS merge was stopped.
  pub fn get_total_stopped_ns(&self) -> Result<u64> {
    self
      .merge_progress
      .get_pause_times()
      .get(&PauseReason::Stopped)
      .copied()
      .ok_or_else(|| LuceneError::illegal_state("Stopped pause time not found"))
  }

  /// Total NS merge was paused to rate limit IO.
  pub fn get_total_paused_ns(&self) -> Result<u64> {
    self
      .merge_progress
      .get_pause_times()
      .get(&PauseReason::Paused)
      .copied()
      .ok_or_else(|| LuceneError::illegal_state("Paused pause time not found"))
  }

  /// Returns true if the merge associated with this limiter has been aborted.
  pub fn is_aborted(&self) -> bool {
    self.merge_progress.is_aborted()
  }

  /**
   * Returns the number of nanoseconds spent in a paused state or `-1` if no pause was
   * applied. If the thread needs pausing, this method delegates to the linked [`OneMergeProgress`].
   */
  fn maybe_pause(&self, bytes: i64) -> Result<i64> {
    // Now is a good time to abort the merge:
    if self.merge_progress.is_aborted() {
      return Err(LuceneError::merge_abort("Merge aborted."));
    }

    let rate = self.get_mb_per_sec(); // read from volatile rate once.
    let seconds_to_pause = (bytes as f64 / 1024.0 / 1024.0) / rate;

    let cur_pause_ns = AtomicI64::new(0);

    // While we use update to avoid a race condition between multiple threads, this doesn't
    // mean that multiple threads will end up getting paused at the same time.
    // We only pause the calling thread. This means if the upstream caller (e.g.
    // ConcurrentMergeScheduler) is using multiple intra-threads, they will all be paused independently.
    self
      .last_ns
      .update(Ordering::SeqCst, Ordering::SeqCst, |last| {
        let cur_ns = nano_time();

        // Time we should sleep until; this is purely instantaneous
        // rate (just adds seconds onto the last time we had paused to);
        // maybe we should also offer decayed recent history one?
        let target_ns = last + (1_000_000_000_f64 * seconds_to_pause) as i64;
        let cur_pause = target_ns - cur_ns;

        // We don't bother with thread pausing if the pause is smaller than 2 msec.
        if cur_pause <= MIN_PAUSE_NS {
          // Set to curNS, not targetNS, to enforce the instant rate, not
          // the "averaged over all history" rate:
          cur_pause_ns.store(0, Ordering::SeqCst);
          cur_ns
        } else {
          cur_pause_ns.store(cur_pause, Ordering::SeqCst);
          last
        }
      });

    let cur_pause_ns_val = cur_pause_ns.load(Ordering::SeqCst);
    if cur_pause_ns_val == 0 {
      return Ok(-1);
    }

    let mut cur_pause_ns_val = cur_pause_ns_val;
    // Defensive: don't sleep for too long; the loop above will call us again if
    // we should keep sleeping and the rate may be adjusted in between.
    if cur_pause_ns_val > MAX_PAUSE_NS {
      cur_pause_ns_val = MAX_PAUSE_NS;
    }

    let start = nano_time();
    self.merge_progress.pause_nanos(
      cur_pause_ns_val as u64,
      if rate == 0.0 {
        PauseReason::Stopped
      } else {
        PauseReason::Paused
      },
      || rate == self.get_mb_per_sec(),
    );
    Ok(nano_time() - start)
  }
}

impl RateLimiter for MergeRateLimiter {
  fn set_mb_per_sec(&self, mb_per_sec: f64) -> Result<()> {
    // 0.0 is allowed: it means the merge is paused
    if mb_per_sec < 0.0 {
      return Err(LuceneError::illegal_argument(format!(
        "mbPerSec must be positive; got: {}",
        mb_per_sec
      )));
    }

    self
      .mb_per_sec
      .store(mb_per_sec.to_bits(), Ordering::SeqCst);

    // `f64::INFINITY` saturates to `i64::MAX` when cast to `i64`.
    let min_check = ((MIN_PAUSE_CHECK_MSEC as f64 / 1000.0) * mb_per_sec * 1024.0 * 1024.0) as i64;
    self
      .min_pause_check_bytes
      .store(min_check.min(1024 * 1024) as u64, Ordering::SeqCst);

    self.merge_progress.wakeup();
    Ok(())
  }

  fn get_mb_per_sec(&self) -> f64 {
    f64::from_bits(self.mb_per_sec.load(Ordering::SeqCst))
  }

  fn pause(&self, bytes: i64) -> Result<i64> {
    self.total_bytes_written.fetch_add(bytes, Ordering::SeqCst);

    // While loop because we may wake up and check again when our rate limit
    // is changed while we were pausing:
    let mut paused: i64 = 0;
    loop {
      match self.maybe_pause(bytes) {
        Ok(delta) if delta >= 0 => {
          // Keep waiting.
          paused += delta;
        },
        Ok(_) => {
          // delta == -1, done
          return Ok(paused);
        },
        Err(e) => {
          return Err(e);
        },
      }
    }
  }

  fn get_min_pause_check_bytes(&self) -> i64 {
    self.min_pause_check_bytes.load(Ordering::SeqCst) as i64
  }
}

/// Returns the current value of the high-resolution time source, in nanoseconds.
/// Returns elapsed monotonic time in nanoseconds.
fn nano_time() -> i64 {
  static BASE: OnceLock<Instant> = OnceLock::new();
  let base = BASE.get_or_init(Instant::now);
  base.elapsed().as_nanos() as i64
}
