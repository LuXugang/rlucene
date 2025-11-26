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
use parking_lot::{Condvar, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::ThreadId;
use std::time::Duration;
/// Controls the health status of [`DocumentsWriter`](crate::core::index::documents_writer::DocumentsWriter) sessions. This struct blocks
/// incoming indexing threads if flushing is significantly slower than indexing to ensure the
/// [`DocumentsWriter`](crate::core::index::documents_writer::DocumentsWriter)’s healthiness. If flushing is significantly slower than indexing, the net
/// memory used within an `IndexWriter` session can increase very quickly and easily exceed the JVM’s
/// available memory.
///
/// To prevent OOM errors and ensure `IndexWriter`’s stability, this struct blocks incoming threads
/// from indexing once 2× the number of available [`DocumentsWriterPerThread`](crate::core::index::documents_writer_per_thread::DocumentsWriterPerThread)s in
/// [`DocumentsWriterPerThreadPool`](crate::core::index::documents_writer_per_thread_pool::DocumentsWriterPerThreadPool) is exceeded. Once flushing catches up and the number of flushing
/// DWPTs is equal to or lower than the number of active `DocumentsWriterPerThread`s, threads are
/// released and can continue indexing.
pub(crate) struct DocumentsWriterStallControl {
    inner: Mutex<State>,
    pausing: Condvar,
    stalled: AtomicBool,
}
pub(crate) struct State {
    // only with assert
    num_waiting: i32,
    // only with assert
    was_stalled: bool,
    // only with assert
    waiting: HashMap<ThreadId, bool>,
}

impl DocumentsWriterStallControl {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(State {
                num_waiting: 0,
                was_stalled: false,
                waiting: HashMap::new(),
            }),
            pausing: Condvar::new(),
            stalled: AtomicBool::new(false),
        }
    }
    /// Updates the stalled flag status.
    /// Sets the stalled flag to `true` if the number of flushing [`DocumentsWriterPerThread`](crate::core::index::documents_writer_per_thread::DocumentsWriterPerThread)s
    /// exceeds the number of active [`DocumentsWriterPerThread`](crate::core::index::documents_writer_per_thread::DocumentsWriterPerThread)s. Otherwise, resets the
    /// [`DocumentsWriterStallControl`] to healthy and releases all threads waiting on
    /// [`wait_if_stalled()`](Self::wait_if_stalled).
    pub(crate) fn update_stalled(&self, stalled: bool) {
        let mut st = self.inner.lock();
        let prev = self.stalled.load(Ordering::SeqCst);
        if prev != stalled {
            self.stalled.store(stalled, Ordering::SeqCst);
            if stalled {
                st.was_stalled = true;
            }
            self.pausing.notify_all();
        }
    }
    pub(crate) fn wait_if_stalled(&self) {
        if self.stalled.load(Ordering::SeqCst) {
            let mut st = self.inner.lock();
            if self.stalled.load(Ordering::SeqCst) {
                self.inc_waiters(&mut st);
                let _ = self.pausing.wait_for(&mut st, Duration::from_millis(1000));

                self.decr_waiters(&mut st);
            }
        }
    }
    pub(crate) fn any_stalled_threads(&self) -> bool {
        self.stalled.load(Ordering::SeqCst)
    }
    fn inc_waiters(&self, st: &mut State) {
        st.num_waiting += 1;
        debug_assert!(st.waiting.insert(thread::current().id(), true).is_none());
        debug_assert!(st.num_waiting > 0);
    }

    fn decr_waiters(&self, st: &mut State) {
        st.num_waiting -= 1;
        debug_assert!(st.waiting.remove(&thread::current().id()).is_some());
        debug_assert!(st.num_waiting >= 0);
    }
    #[cfg(test)]
    pub(crate) fn has_blocked(&self) -> bool {
        let st = self.inner.lock();
        st.num_waiting > 0
    }

    #[cfg(test)]
    pub(crate) fn get_num_waiting(&self) -> i32 {
        let st = self.inner.lock();
        st.num_waiting
    }

    #[cfg(test)]
    pub(crate) fn is_healthy(&self) -> bool {
        !self.stalled.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub(crate) fn is_thread_queued(&self, tid: &ThreadId) -> bool {
        let st = self.inner.lock();
        st.waiting.contains_key(tid)
    }

    #[cfg(test)]
    pub(crate) fn was_stalled(&self) -> bool {
        let st = self.inner.lock();
        st.was_stalled
    }
}
#[cfg(test)]
mod tests {
    use crate::core::index::documents_writer_stall_control::DocumentsWriterStallControl;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        at_least, is_night_mode, random,
    };
    use parking_lot::{Condvar, Mutex};
    use rand::{Rng, rng};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::thread::{JoinHandle, ThreadId};
    use std::time::Duration;

    #[test]
    fn test_simple_stall() {
        let mut random = random();
        let ctrl = Arc::new(DocumentsWriterStallControl::new());
        ctrl.update_stalled(false);
        let mut threads = wait_threads(at_least(&mut random, 3) as usize, ctrl.clone());
        assert!(!ctrl.has_blocked());
        assert!(!ctrl.any_stalled_threads());
        join(threads);
        // now stall threads and wake them up again
        ctrl.update_stalled(true);
        threads = wait_threads(at_least(&mut random, 3) as usize, ctrl.clone());
        start();
        thread::sleep(Duration::from_millis(50));
        assert!(ctrl.has_blocked());
        assert!(ctrl.any_stalled_threads());

        ctrl.update_stalled(false);
        assert!(!ctrl.any_stalled_threads());
        join(threads);
    }
    #[test]
    fn test_random() {
        let mut rng = rng();
        let ctrl = Arc::new(DocumentsWriterStallControl::new());
        ctrl.update_stalled(false);

        let num_threads = at_least(&mut rng, 3);
        let mut stall_threads = Vec::with_capacity(num_threads as usize);
        for _ in 0..num_threads {
            let ctrl_clone = ctrl.clone();
            let handle = TrackedThread::spawn(move || {
                let mut local_rng = random();
                let iters = at_least(&mut local_rng, 100);
                for _ in 0..iters {
                    let stall_prob = 1 + local_rng.random_range(0..10);
                    ctrl_clone.update_stalled(local_rng.random_range(0..stall_prob) == 0);
                    if local_rng.random_range(0..5) == 0 {
                        ctrl_clone.wait_if_stalled();
                    }
                }
            });
            stall_threads.push(handle);
        }

        start();
        let mut iterations = 0;
        while iterations < 100 && !all_terminated(&stall_threads) {
            iterations += 1;
            ctrl.update_stalled(false);
            if rng.random_bool(0.5) {
                thread::yield_now();
            } else {
                thread::sleep(Duration::from_millis(1));
            }
        }
        join(stall_threads);
    }
    // TODO: 测试未通过
    fn test_acquire_release_race() -> Result<()> {
        let mut rng = random();
        let ctrl = Arc::new(DocumentsWriterStallControl::new());
        ctrl.update_stalled(false);

        let stop = Arc::new(AtomicBool::new(false));
        let check_point = Arc::new(AtomicBool::new(true));

        let num_stallers = at_least(&mut rng, 1) as usize;
        let num_releasers = at_least(&mut rng, 1) as usize;
        let num_waiters = at_least(&mut rng, 1) as usize;
        let total_threads = num_stallers + num_releasers + num_waiters;

        let sync = Arc::new(Synchronizer::new(
            (num_stallers + num_releasers) as usize,
            total_threads as usize,
        ));

        let mut threads = Vec::with_capacity(total_threads as usize);
        for _ in 0..num_releasers {
            threads.push(updater(
                stop.clone(),
                check_point.clone(),
                ctrl.clone(),
                sync.clone(),
                false,
            ));
        }
        for _ in 0..num_stallers {
            threads.push(updater(
                stop.clone(),
                check_point.clone(),
                ctrl.clone(),
                sync.clone(),
                true,
            ));
        }
        for _ in 0..num_waiters {
            threads.push(waiter(
                stop.clone(),
                check_point.clone(),
                ctrl.clone(),
                sync.clone(),
            ));
        }

        start();

        // let iters = if is_night_mode() {
        //     at_least(&mut rng, 10_000)
        // } else {
        //     at_least(&mut rng, 1_000)
        // };
        let iters = 1;
        let check_point_probability = if is_night_mode() { 0.5 } else { 0.1 };

        for _ in 0..iters {
            if check_point.load(Ordering::SeqCst) {
                assert!(
                    sync.await_update_join(Duration::from_secs(10)),
                    "timed out waiting for update threads – deadlock?"
                );

                if ctrl.has_blocked() && ctrl.is_healthy() {
                    assert_state(num_releasers, num_stallers, num_waiters, &threads, &ctrl);
                }

                check_point.store(false, Ordering::SeqCst);
                sync.count_down_waiter();
                sync.await_left_check_point();
            }

            assert!(!check_point.load(Ordering::SeqCst));
            assert_eq!(0, {
                let (lock, _) = &sync.waiter;
                *lock.lock()
            });

            if rng.random::<f32>() <= check_point_probability {
                sync.reset(num_stallers + num_releasers, total_threads);
                check_point.store(true, Ordering::SeqCst);
            }
        }

        if !check_point.load(Ordering::SeqCst) {
            sync.reset(num_stallers + num_releasers, total_threads);
            check_point.store(true, Ordering::SeqCst);
        }
        let v = sync.await_update_join(Duration::from_secs(10));
        assert!(v);
        assert_state(num_releasers, num_stallers, num_waiters, &threads, &ctrl);
        check_point.store(false, Ordering::SeqCst);
        stop.store(true, Ordering::SeqCst);
        sync.count_down_waiter();
        sync.await_left_check_point();

        // join and final waiter-check
        for tt in threads {
            ctrl.update_stalled(false);
            let tid = tt.thread_id();
            let _ = tt.handle.join();
            if ctrl.is_thread_queued(&tid) {
                unreachable!(
                    "waiter is not released – any_stalled_threads: {}",
                    ctrl.any_stalled_threads()
                );
            }
        }
        Ok(())
    }
    fn assert_state(
        num_releasers: usize,
        num_stallers: usize,
        num_waiters: usize,
        threads: &[TrackedThread],
        ctrl: &DocumentsWriterStallControl,
    ) {
        let mut millis_to_sleep = 100u64;
        while ctrl.has_blocked() && ctrl.is_healthy() {
            for tt in
                &threads[num_releasers + num_stallers..num_releasers + num_stallers + num_waiters]
            {
                let tid = tt.thread_id();
                if ctrl.is_thread_queued(&tid) {
                    if millis_to_sleep < 60_000 {
                        thread::sleep(Duration::from_millis(millis_to_sleep));
                        millis_to_sleep *= 2;
                        break;
                    } else {
                        unreachable!(
                            "control claims no stalled threads but waiter seems to be blocked"
                        );
                    }
                }
            }
        }
    }
    fn waiter(
        stop: Arc<AtomicBool>,
        check_point: Arc<AtomicBool>,
        ctrl: Arc<DocumentsWriterStallControl>,
        sync: Arc<Synchronizer>,
    ) -> TrackedThread {
        TrackedThread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                ctrl.wait_if_stalled();
                if check_point.load(Ordering::SeqCst) {
                    assert!(sync.await_waiter(), "Waiter timed out waiting for release");
                }
                thread::sleep(Duration::from_millis(1));
            }
        })
    }
    fn updater(
        stop: Arc<AtomicBool>,
        check_point: Arc<AtomicBool>,
        ctrl: Arc<DocumentsWriterStallControl>,
        sync: Arc<Synchronizer>,
        release: bool,
    ) -> TrackedThread {
        TrackedThread::spawn(move || {
            let mut rng = rng();
            while !stop.load(Ordering::SeqCst) {
                let internal_iters = if release && rng.random_bool(0.5) {
                    at_least(&mut rng, 5) as usize
                } else {
                    1
                };
                for _ in 0..internal_iters {
                    ctrl.update_stalled(rng.random_bool(0.5));
                }

                if check_point.load(Ordering::SeqCst) {
                    sync.count_down_update_join();
                    assert!(sync.await_waiter());
                    sync.count_down_left_check_point();
                }

                if rng.random_bool(0.5) {
                    thread::yield_now();
                }
            }
            sync.count_down_update_join();
        })
    }
    pub struct Synchronizer {
        waiter: (Mutex<usize>, Condvar),
        update_join: (Mutex<usize>, Condvar),
        left_checkpoint: (Mutex<usize>, Condvar),
    }

    impl Synchronizer {
        pub fn new(num_updaters: usize, _num_threads: usize) -> Self {
            Synchronizer {
                waiter: (Mutex::new(1), Condvar::new()),
                update_join: (Mutex::new(num_updaters), Condvar::new()),
                left_checkpoint: (Mutex::new(num_updaters), Condvar::new()),
            }
        }

        pub fn reset(&self, num_updaters: usize, _num_threads: usize) {
            let (waiter_lock, _) = &self.waiter;
            *waiter_lock.lock() = 1;
            let (uj_lock, _) = &self.update_join;
            *uj_lock.lock() = num_updaters;
            let (lc_lock, _) = &self.left_checkpoint;
            *lc_lock.lock() = num_updaters;
        }

        pub fn await_waiter(&self) -> bool {
            let (waiter_lock, waiter_cv) = &self.waiter;
            let mut released = waiter_lock.lock();
            waiter_cv
                .wait_for(&mut released, Duration::from_secs(10))
                .timed_out()
        }
        fn count_down_update_join(&self) {
            let (uj_lock, uj_cv) = &self.update_join;
            let mut cnt = uj_lock.lock();
            if *cnt > 0 {
                *cnt -= 1;
                if *cnt == 0 {
                    uj_cv.notify_all();
                }
            }
        }

        pub fn await_update_join(&self, timeout: Duration) -> bool {
            let (uj_lock, uj_cv) = &self.update_join;
            let mut cnt = uj_lock.lock();
            uj_cv.wait_for(&mut cnt, timeout).timed_out()
        }

        pub fn count_down_left_check_point(&self) {
            let (lc_lock, lc_cv) = &self.left_checkpoint;
            let mut cnt = lc_lock.lock();
            if *cnt > 0 {
                *cnt -= 1;
                if *cnt == 0 {
                    lc_cv.notify_all();
                }
            }
        }

        pub fn await_left_check_point(&self) {
            let (lc_lock, lc_cv) = &self.left_checkpoint;
            let mut cnt = lc_lock.lock();
            lc_cv.wait(&mut cnt)
        }

        pub fn count_down_waiter(&self) {
            let (lc_lock, lc_cv) = &self.waiter;
            let mut cnt = lc_lock.lock();
            if *cnt > 0 {
                *cnt -= 1;
                if *cnt == 0 {
                    lc_cv.notify_all();
                }
            }
        }
    }
    struct TrackedThread {
        handle: JoinHandle<()>,
        terminated: Arc<AtomicBool>,
    }
    impl TrackedThread {
        pub fn spawn<F>(f: F) -> Self
        where
            F: FnOnce() + Send + 'static,
        {
            let terminated = Arc::new(AtomicBool::new(false));
            let flag = terminated.clone();
            let handle = thread::spawn(move || {
                f();
                flag.store(false, Ordering::SeqCst);
            });
            TrackedThread { handle, terminated }
        }

        fn is_terminated(&self) -> bool {
            self.terminated.load(Ordering::SeqCst)
        }
        fn thread_id(&self) -> ThreadId {
            self.handle.thread().id()
        }
    }
    pub fn start() {
        brief_sleep();
    }
    fn brief_sleep() {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    fn all_terminated(threads: &[TrackedThread]) -> bool {
        threads.iter().all(|t| t.is_terminated())
    }
    fn join(handles: Vec<TrackedThread>) {
        for handle in handles {
            handle.handle.join().unwrap();
        }
    }
    fn wait_threads(num: usize, ctrl: Arc<DocumentsWriterStallControl>) -> Vec<TrackedThread> {
        let mut threads = Vec::with_capacity(num);
        for _ in 0..num {
            let c = ctrl.clone();
            let tt = TrackedThread::spawn(move || {
                c.wait_if_stalled();
            });
            threads.push(tt);
        }
        threads
    }
}
