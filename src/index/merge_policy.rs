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
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};
pub struct MergePolicy;
/// Reason for pausing the merge thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(unused)]
pub enum PauseReason {
    /// Stopped (because of throughput rate set to 0, typically).
    Stopped,
    /// Temporarily paused because of exceeded throughput rate.
    Paused,
    /// Other reason.
    Other,
}
/// Progress and state for an executing merge. This struct encapsulates the
/// logic to pause and resume the merge thread or to abort the merge entirely.
#[allow(unused)]
pub struct OneMergeProgress {
    pause_lock: Mutex<()>,
    pausing: Condvar,
    /// Pause times (in nanoseconds) for each [`PauseReason`](PauseReason).
    pause_times: PauseTimes,
    aborted: AtomicBool,
    /// This field is for sanity-check purposes only. Only the same thread that
    /// invoked `OneMerge#mergeInit()` is permitted to be calling `pauseNanos`.
    /// This is always verified at runtime.
    owner: Mutex<Option<ThreadId>>,
}

#[derive(Default)]
#[allow(unused)]
struct PauseTimes {
    stopped: AtomicU64,
    paused: AtomicU64,
    other: AtomicU64,
}

#[allow(unused)]
impl OneMergeProgress {
    /// Creates a new merge progress info.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            pause_lock: Mutex::new(()),
            pausing: Condvar::new(),
            // Place all the pause reasons in there immediately so that we can
            // simply update values.
            pause_times: PauseTimes::default(),
            aborted: AtomicBool::new(false),
            owner: Mutex::new(None),
        })
    }
    /// Abort the merge this progress tracks at the next possible moment.
    pub fn abort(&self) {
        self.aborted.store(true, Ordering::Relaxed);
        self.wakeup(); // wakeup any paused merge thread.
    }
    /// Return the aborted state of this merge.
    pub fn is_aborted(&self) -> bool {
        self.aborted.load(Ordering::Relaxed)
    }

    /// Pauses the calling thread for at least `pause_nanos` nanoseconds unless
    /// the merge is aborted or the external condition returns `false`, in
    /// which case control returns immediately.
    ///
    /// The external condition is required so that other threads can terminate
    /// the pausing immediately before `pause_nanos` expires. We can't rely
    /// on just `Condvar::wait_timeout_while()` alone because it can return
    /// due to spurious wakeups too.
    ///
    /// # Arguments
    /// - `condition`: The pause condition that should return `false` if
    ///   immediate return from this method is needed. Other threads can wake up
    ///   any sleeping thread by calling [`wakeup()`](OneMergeProgress::wakeup),
    ///   but the thread may sleep for the remainder of the requested time if
    ///   this condition remains `true`.
    pub fn pause_nanos<F>(&self, pause_nanos: u64, reason: PauseReason, condition: F)
    where
        F: Fn() -> bool,
    {
        let owner = self.owner.lock();
        let current_id = thread::current().id();
        debug_assert_eq!(
            *owner,
            Some(current_id),
            "Only owner thread can pause merge"
        );
        drop(owner);

        let start = Instant::now();
        let deadline = start + Duration::from_nanos(pause_nanos);

        let mut lock = self.pause_lock.lock();
        while !self.aborted.load(Ordering::Relaxed) && condition() {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let timeout = deadline - now;
            self.pausing.wait_for(&mut lock, timeout);
        }

        let elapsed = start.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        self.add_pause_time(reason, elapsed);
    }

    fn add_pause_time(&self, reason: PauseReason, nanos: u64) {
        match reason {
            PauseReason::Stopped => self.pause_times.stopped.fetch_add(nanos, Ordering::Relaxed),
            PauseReason::Paused => self.pause_times.paused.fetch_add(nanos, Ordering::Relaxed),
            PauseReason::Other => self.pause_times.other.fetch_add(nanos, Ordering::Relaxed),
        };
    }
    /// Request a wakeup for any threads stalled in
    /// [`pauseNanos`](OneMergeProgress::pause_nanos).
    pub fn wakeup(&self) {
        let _lock = self.pause_lock.lock();
        self.pausing.notify_all();
    }
    /// Returns pause reasons and associated times in nanoseconds.
    pub fn get_pause_times(&self) -> HashMap<PauseReason, u64> {
        let mut map = HashMap::new();
        map.insert(
            PauseReason::Stopped,
            self.pause_times.stopped.load(Ordering::Relaxed),
        );
        map.insert(
            PauseReason::Paused,
            self.pause_times.paused.load(Ordering::Relaxed),
        );
        map.insert(
            PauseReason::Other,
            self.pause_times.other.load(Ordering::Relaxed),
        );
        map
    }
    pub fn set_merge_thread(&self) {
        let mut owner = self.owner.lock();
        debug_assert!(owner.is_none());
        *owner = Some(thread::current().id());
    }
}
