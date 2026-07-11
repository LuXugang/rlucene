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
use std::cell::RefCell;
use std::cmp;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::thread;
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex, MutexGuard};

use crate::core::index::merge_policy::{MergeStat, OneMergeSR};
use crate::core::index::merge_rate_limiter::MergeRateLimiter;
use crate::core::index::merge_scheduler::{MergeScheduler, MergeSource};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::store::directory::Directory;
use crate::core::store::rate_limited_directory::RateLimitedDirectory;
use crate::core::store::rate_limiter::RateLimiter;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};

thread_local! {
  static CURRENT_MERGE_RATE_LIMITER: RefCell<Option<Arc<MergeRateLimiter>>> =
    const { RefCell::new(None) };
}

/**
 * Dynamic default for `max_thread_count` and `max_merge_count`, based on CPU core count.
 * `max_thread_count` is set to `max(1, cpu_core_count/2)`. `max_merge_count`
 * is set to `max_thread_count + 5`.
 */
pub const AUTO_DETECT_MERGES_AND_THREADS: i32 = -1;

/**
 * Used for testing.
 *
 * @lucene.internal
 */
pub const DEFAULT_CPU_CORE_COUNT_PROPERTY: &str = "lucene.cms.override_core_count";

/** Floor for IO write rate limit (we will never go any lower than this) */
const MIN_MERGE_MB_PER_SEC: f64 = 5.0;

/** Ceiling for IO write rate limit (we will never go any higher than this) */
const MAX_MERGE_MB_PER_SEC: f64 = 10240.0;

/** Initial value for IO write rate limit when do_auto_io_throttle is true */
const START_MB_PER_SEC: f64 = 20.0;

/**
 * Merges below this size are not counted in the max_thread_count, i.e. they can freely run in their
 * own thread (up until max_merge_count).
 */
const MIN_BIG_MERGE_MB: f64 = 50.0;

/// A [`MergeScheduler`] that runs each merge using a separate thread.
///
/// Specify the max number of threads that may run at once, and the maximum number of simultaneous
/// merges with [`ConcurrentMergeScheduler::set_max_merges_and_threads`].
///
/// If the number of merges exceeds the max number of threads then the largest merges are paused
/// until one of the smaller merges completes.
///
/// If more than [`ConcurrentMergeScheduler::get_max_merge_count`] merges are requested then this
/// scheduler will forcefully throttle the incoming threads by pausing until one or more merges
/// complete.
///
/// This scheduler sets defaults based on Rust's view of the CPU count, and it assumes a solid state
/// disk (or similar). If you have a spinning disk and want to maximize performance, use
/// [`ConcurrentMergeScheduler::set_default_max_merges_and_threads`].
#[derive(Clone)]
pub struct ConcurrentMergeScheduler {
  inner: Arc<Mutex<Inner>>,
  changed: Arc<Condvar>,
}

struct Inner {
  /** List of currently active merge work. */
  merge_threads: Vec<Arc<MergeThreadState>>,
  // Max number of merge threads allowed to be running at once. When there are more merges then
  // this, we forcefully pause the larger ones, letting the smaller ones run, up until
  // max_merge_count merges at which point we forcefully pause incoming threads (that presumably
  // are the ones causing so much merging).
  max_thread_count: i32,
  // Max number of merges we accept before forcefully throttling the incoming threads.
  max_merge_count: i32,
  /** How many merge threads have kicked off (this is use to name them). */
  merge_thread_count: usize,
  /** Current IO writes throttle rate */
  target_mb_per_sec: f64,
  /** true if we should rate-limit writes for each merge */
  do_auto_io_throttle: bool,
  force_merge_mb_per_sec: f64,
  suppress_exceptions: bool,
  #[cfg(test)]
  stall_on_merge_thread: Option<Arc<AtomicBool>>,
}

impl Default for ConcurrentMergeScheduler {
  fn default() -> Self {
    Self::new()
  }
}

impl ConcurrentMergeScheduler {
  /** Sole constructor, with all settings set to default values. */
  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(Inner {
        merge_threads: Vec::new(),
        max_thread_count: AUTO_DETECT_MERGES_AND_THREADS,
        max_merge_count: AUTO_DETECT_MERGES_AND_THREADS,
        merge_thread_count: 0,
        target_mb_per_sec: START_MB_PER_SEC,
        do_auto_io_throttle: false,
        force_merge_mb_per_sec: f64::INFINITY,
        suppress_exceptions: false,
        #[cfg(test)]
        stall_on_merge_thread: None,
      })),
      changed: Arc::new(Condvar::new()),
    }
  }

  /**
   * Expert: directly set the maximum number of merge threads and simultaneous merges allowed.
   *
   * @param max_merge_count the max # simultaneous merges that are allowed. If a merge is necessary
   *     yet we already have this many threads running, the incoming thread (that is calling
   *     add/updateDocument) will block until a merge thread has completed. Note that we will only
   *     run the smallest `max_thread_count` merges at a time.
   * @param max_thread_count the max # simultaneous merge threads that should be running at once.
   *     This must be <= `max_merge_count`.
   */
  pub fn set_max_merges_and_threads(
    &self,
    max_merge_count: i32,
    max_thread_count: i32,
  ) -> Result<()> {
    let mut inner = self.inner.lock();
    if max_merge_count == AUTO_DETECT_MERGES_AND_THREADS
      && max_thread_count == AUTO_DETECT_MERGES_AND_THREADS
    {
      inner.max_merge_count = AUTO_DETECT_MERGES_AND_THREADS;
      inner.max_thread_count = AUTO_DETECT_MERGES_AND_THREADS;
    } else if max_merge_count == AUTO_DETECT_MERGES_AND_THREADS
      || max_thread_count == AUTO_DETECT_MERGES_AND_THREADS
    {
      return Err(LuceneError::illegal_argument(
        "both max_merge_count and max_thread_count must be AUTO_DETECT_MERGES_AND_THREADS",
      ));
    } else {
      if max_thread_count < 1 {
        return Err(LuceneError::illegal_argument(
          "max_thread_count should be at least 1",
        ));
      }
      if max_merge_count < 1 {
        return Err(LuceneError::illegal_argument(
          "max_merge_count should be at least 1",
        ));
      }
      if max_thread_count > max_merge_count {
        return Err(LuceneError::illegal_argument(format!(
          "max_thread_count should be <= max_merge_count (= {max_merge_count})"
        )));
      }
      inner.max_thread_count = max_thread_count;
      inner.max_merge_count = max_merge_count;
    }
    Ok(())
  }

  /**
   * Sets max merges and threads to proper defaults for rotational or non-rotational storage.
   *
   * @param spins true to set defaults best for traditional rotatational storage (spinning disks),
   *     else false (e.g. for solid-state disks)
   */
  pub fn set_default_max_merges_and_threads(&self, spins: bool) {
    let mut inner = self.inner.lock();
    if spins {
      inner.max_thread_count = 1;
      inner.max_merge_count = 6;
    } else {
      let mut core_count = thread::available_parallelism()
        .map(|count| count.get() as i32)
        .unwrap_or(1);

      // Let tests override this to help reproducing a failure on a machine that has a different
      // core count than the one where the test originally failed:
      if let Ok(value) = std::env::var(DEFAULT_CPU_CORE_COUNT_PROPERTY)
        && let Ok(parsed) = value.parse::<i32>()
      {
        core_count = parsed;
      }

      // If you are indexing at full throttle, how many merge threads do you need to keep up? It
      // depends: for most data structures, merging is cheaper than indexing/flushing, but for knn
      // vectors, merges can require about as much work as the initial indexing/flushing. Plus
      // documents are indexed/flushed only once, but may be merged multiple times.
      // Here, we assume an intermediate scenario where merging requires about as much work as
      // indexing/flushing overall, so we give half the core count to merges.
      inner.max_thread_count = cmp::max(1, core_count / 2);
      inner.max_merge_count = inner.max_thread_count + 5;
    }
  }

  /**
   * Set the per-merge IO throttle rate for forced merges (default: `f64::INFINITY`).
   */
  pub fn set_force_merge_mb_per_sec(&self, v: f64) -> Result<()> {
    let mut inner = self.inner.lock();
    inner.force_merge_mb_per_sec = v;
    Self::update_merge_threads(&mut inner)
  }

  /** Get the per-merge IO throttle rate for forced merges. */
  pub fn get_force_merge_mb_per_sec(&self) -> f64 {
    self.inner.lock().force_merge_mb_per_sec
  }

  /**
   * Turn on dynamic IO throttling, to adaptively rate limit writes bytes/sec to the minimal rate
   * necessary so merges do not fall behind. By default this is disabled and writes are not
   * rate-limited.
   */
  pub fn enable_auto_io_throttle(&self) -> Result<()> {
    let mut inner = self.inner.lock();
    inner.do_auto_io_throttle = true;
    inner.target_mb_per_sec = START_MB_PER_SEC;
    Self::update_merge_threads(&mut inner)
  }

  /**
   * Turn off auto IO throttling.
   *
   * @see #enableAutoIOThrottle
   */
  pub fn disable_auto_io_throttle(&self) -> Result<()> {
    let mut inner = self.inner.lock();
    inner.do_auto_io_throttle = false;
    Self::update_merge_threads(&mut inner)
  }

  /** Returns true if auto IO throttling is currently enabled. */
  pub fn get_auto_io_throttle(&self) -> bool {
    self.inner.lock().do_auto_io_throttle
  }

  /**
   * Returns the currently set per-merge IO writes rate limit, if `enable_auto_io_throttle` was
   * called, else `f64::INFINITY`.
   */
  pub fn get_io_rate_limit_mb_per_sec(&self) -> f64 {
    let inner = self.inner.lock();
    if inner.do_auto_io_throttle {
      inner.target_mb_per_sec
    } else {
      f64::INFINITY
    }
  }

  /**
   * Returns `max_thread_count`.
   *
   * @see #setMaxMergesAndThreads
   */
  pub fn get_max_thread_count(&self) -> i32 {
    self.inner.lock().max_thread_count
  }

  /** See `set_max_merges_and_threads`. */
  pub fn get_max_merge_count(&self) -> i32 {
    self.inner.lock().max_merge_count
  }

  /** Removes the calling thread from the active merge threads. */
  fn remove_merge_thread(inner: &mut Inner) {
    let removed = if let Some(index) = inner
      .merge_threads
      .iter()
      .position(|merge_thread| merge_thread.is_current_thread())
    {
      inner.merge_threads.remove(index);
      true
    } else {
      false
    };
    debug_assert!(removed, "merge thread was not found");
  }

  /**
   * Called whenever the running merges have changed, to set merge IO limits. This method sorts the
   * merge threads by their merge size in descending order and then pauses/unpauses threads from
   * first to last -- that way, smaller merges are guaranteed to run before larger ones.
   */
  fn update_merge_threads(inner: &mut Inner) -> Result<()> {
    // Only look at threads that are alive and not in the process of stopping (i.e. have an active
    // merge):
    let mut thread_idx = 0;
    while thread_idx < inner.merge_threads.len() {
      if !inner.merge_threads[thread_idx].is_alive() {
        // Prune any dead threads
        inner.merge_threads.remove(thread_idx);
        continue;
      }
      thread_idx += 1;
    }

    let mut active_merges: Vec<usize> = (0..inner.merge_threads.len()).collect();

    // Sort the merge threads, largest first:
    active_merges.sort_by(|left, right| {
      inner.merge_threads[*right]
        .estimated_merge_bytes
        .load(Ordering::SeqCst)
        .cmp(
          &inner.merge_threads[*left]
            .estimated_merge_bytes
            .load(Ordering::SeqCst),
        )
    });

    let active_merge_count = active_merges.len();

    let mut big_merge_count = 0;

    for thread_idx in (0..active_merge_count).rev() {
      let merge_thread = &inner.merge_threads[active_merges[thread_idx]];
      if merge_thread.estimated_merge_bytes.load(Ordering::SeqCst)
        > (MIN_BIG_MERGE_MB as i64) * 1024 * 1024
      {
        big_merge_count = 1 + thread_idx;
        break;
      }
    }

    for (thread_idx, merge_idx) in active_merges.into_iter().enumerate() {
      let merge_thread = &inner.merge_threads[merge_idx];

      // pause the thread if max_thread_count is smaller than the number of merge threads.
      let max_thread_count = cmp::max(0, inner.max_thread_count) as usize;
      let do_pause = thread_idx < big_merge_count.saturating_sub(max_thread_count);

      let new_mb_per_sec = if do_pause {
        0.0
      } else if merge_thread.merge_stat.max_num_segments() != -1 {
        inner.force_merge_mb_per_sec
      } else if !inner.do_auto_io_throttle {
        f64::INFINITY
      } else if merge_thread.estimated_merge_bytes.load(Ordering::SeqCst)
        < (MIN_BIG_MERGE_MB as i64) * 1024 * 1024
      {
        // Don't rate limit small merges:
        f64::INFINITY
      } else {
        inner.target_mb_per_sec
      };

      merge_thread.rate_limiter.set_mb_per_sec(new_mb_per_sec)?;
    }

    Ok(())
  }

  fn init_dynamic_defaults<D>(&self, _directory: &D)
  where
    D: Directory,
  {
    let should_init = self.inner.lock().max_thread_count == AUTO_DETECT_MERGES_AND_THREADS;
    if should_init {
      self.set_default_max_merges_and_threads(false);
    }
  }

  fn rate_to_string(mb_per_sec: f64) -> String {
    if mb_per_sec == 0.0 {
      "stopped".to_string()
    } else if mb_per_sec == f64::INFINITY {
      "unlimited".to_string()
    } else {
      format!("{mb_per_sec:.1} MB/sec")
    }
  }
}

impl CloseableRef for ConcurrentMergeScheduler {
  fn close(&self) -> Result<()> {
    self.sync()
  }
}

impl ConcurrentMergeScheduler {
  /**
   * Wait for any running merge threads to finish. This call is not interruptible as used by
   * `close`.
   */
  pub fn sync(&self) -> Result<()> {
    let mut inner = self.inner.lock();
    while inner
      .merge_threads
      .iter()
      .any(|merge_thread| merge_thread.is_alive() && !merge_thread.is_current_thread())
    {
      self.changed.wait(&mut inner);
    }
    Ok(())
  }

  /**
   * Returns the number of merge threads that are alive, ignoring the calling thread if it is a
   * merge thread. Note that this number is <= `merge_threads` size.
   *
   * @lucene.internal
   */
  pub fn merge_thread_count(&self) -> usize {
    let inner = self.inner.lock();
    Self::merge_thread_count_locked(&inner)
  }

  fn merge_thread_count_locked(inner: &Inner) -> usize {
    inner
      .merge_threads
      .iter()
      .filter(|merge_thread| {
        merge_thread.is_alive()
          && !merge_thread.is_current_thread()
          && !merge_thread.rate_limiter.is_aborted()
      })
      .count()
  }
}

impl MergeScheduler for ConcurrentMergeScheduler {
  type Directory<D>
    = RateLimitedDirectory<D, Arc<MergeRateLimiter>>
  where
    D: Directory;

  fn wrap_for_merge<D>(&self, in_: D) -> Result<Self::Directory<D>>
  where
    D: Directory,
  {
    let rate_limiter = CURRENT_MERGE_RATE_LIMITER
      .with(|slot| slot.borrow().clone())
      .ok_or_else(|| {
        LuceneError::illegal_state(format!(
          "wrap_for_merge should be called from MergeThread. Current thread: {:?}",
          thread::current().id()
        ))
      })?;

    // Return a wrapped Directory which has rate-limited output.
    // Note: the rate limiter is only per thread. So, if there are multiple merge threads running
    // and throttling is required, each thread will be throttled independently.
    // The implication of this, is that the total IO rate could be higher than the target rate.
    Ok(RateLimitedDirectory::new(in_, rate_limiter))
  }

  fn initialize<D>(&mut self, directory: &D) -> Result<()>
  where
    D: Directory,
  {
    self.init_dynamic_defaults(directory);
    Ok(())
  }

  fn merge<MS, D>(&self, merge_source: MS, trigger: MergeTrigger) -> Result<()>
  where
    MS: MergeSource<D> + Clone + 'static,
    D: Directory + 'static,
    OneMergeSR<D>: Send + 'static,
  {
    let mut inner = self.inner.lock();
    self.merge_locked(&mut inner, merge_source, trigger)
  }
}

impl ConcurrentMergeScheduler {
  fn merge_locked<MS, D>(
    &self,
    inner: &mut MutexGuard<'_, Inner>,
    merge_source: MS,
    trigger: MergeTrigger,
  ) -> Result<()>
  where
    MS: MergeSource<D> + Clone + 'static,
    D: Directory + 'static,
    OneMergeSR<D>: Send + 'static,
  {
    if trigger == MergeTrigger::Closing {
      // Disable throttling on close:
      inner.target_mb_per_sec = MAX_MERGE_MB_PER_SEC;
      Self::update_merge_threads(inner)?;
    }

    // Iterate, pulling from the IndexWriter's queue of pending merges, until it's empty:
    loop {
      if !self.maybe_stall(inner, &merge_source)? {
        break;
      }

      let merge = match merge_source.get_next_merge()? {
        Some(merge) => merge,
        None => return Ok(()),
      };

      let merge_stat = merge.stat.clone();

      let setup_result = (|| -> Result<()> {
        let new_merge_thread = Self::get_merge_thread(inner, merge_source.clone(), merge);
        let merge_thread_state = new_merge_thread.state.clone();
        inner.merge_threads.push(merge_thread_state.clone());
        Self::update_io_throttle(inner, &merge_thread_state)?;
        new_merge_thread.start(self.clone())?;
        Self::update_merge_threads(inner)?;
        Ok(())
      })();

      if setup_result.is_err() {
        merge_source.on_merge_finished(&merge_stat, None);
      }

      setup_result?;
    }

    Ok(())
  }

  /**
   * This is invoked by `merge` to possibly stall the incoming thread when there are too many
   * merges running or pending. The default behavior is to force this thread, which is producing too
   * many segments for merging to keep up, to wait until merges catch up. Applications that can take
   * other less drastic measures, such as limiting how many threads are allowed to index, can do
   * nothing here and throttle elsewhere.
   *
   * If this method wants to stall but the calling thread is a merge thread, it should return
   * false to tell caller not to kick off any new merges.
   */
  fn maybe_stall<MS, D>(&self, inner: &mut MutexGuard<'_, Inner>, merge_source: &MS) -> Result<bool>
  where
    MS: MergeSource<D>,
    D: Directory,
  {
    let mut start_stall_time = None;

    while merge_source.has_pending_merges(None)?
      && Self::merge_thread_count_locked(inner) >= inner.max_merge_count as usize
    {
      // This means merging has fallen too far behind: we have already created max_merge_count
      // threads, and now there's at least one more merge pending. Note that only max_thread_count
      // of those created merge threads will actually be running; the rest will be paused (see
      // update_merge_threads). We stall this producer thread to prevent creation of new segments,
      // until merging has caught up:
      if inner
        .merge_threads
        .iter()
        .any(|merge_thread| merge_thread.is_current_thread())
      {
        // Never stall a merge thread since this blocks the thread from finishing and calling
        // update_merge_threads, and blocking it accomplishes nothing anyway (it's not really a
        // segment producer):
        return Ok(false);
      }

      if start_stall_time.is_none() {
        start_stall_time = Some(Instant::now());
      }
      self.do_stall(inner);
    }

    Ok(true)
  }

  /** Called from `maybe_stall` to pause the calling thread for a bit. */
  fn do_stall(&self, inner: &mut MutexGuard<'_, Inner>) {
    #[cfg(test)]
    if let Some(stall_on_merge_thread) = &inner.stall_on_merge_thread
      && inner
        .merge_threads
        .iter()
        .any(|merge_thread| merge_thread.is_current_thread())
    {
      stall_on_merge_thread.store(true, Ordering::SeqCst);
    }

    // Defensively wait for only .25 seconds in case we are missing a notify/all somewhere:
    self.changed.wait_for(inner, Duration::from_millis(250));
  }

  /**
   * Does the actual merge, by calling `MergeSource::merge`.
   */
  fn do_merge<MS, D>(&self, merge_source: &MS, merge: OneMergeSR<D>) -> Result<()>
  where
    MS: MergeSource<D>,
    D: Directory + 'static,
  {
    merge_source.merge(merge)
  }

  /** Create and return a new MergeThread */
  fn get_merge_thread<MS, D>(
    inner: &mut Inner,
    merge_source: MS,
    merge: OneMergeSR<D>,
  ) -> MergeThread<MS, D>
  where
    MS: MergeSource<D> + Clone + 'static,
    D: Directory + 'static,
    OneMergeSR<D>: Send + 'static,
  {
    let name = format!("Lucene Merge Thread #{}", inner.merge_thread_count);
    inner.merge_thread_count += 1;
    let state = Arc::new(MergeThreadState::new(name, &merge));
    MergeThread::new(state, merge_source, merge)
  }

  fn run_on_merge_finished<MS, D>(&self, merge_source: MS) -> Result<()>
  where
    MS: MergeSource<D> + Clone + 'static,
    D: Directory + 'static,
    OneMergeSR<D>: Send + 'static,
  {
    let mut inner = self.inner.lock();
    // The merge call as well as the merge thread handling in the finally
    // block must be sync'd on CMS otherwise stalling decisions might cause
    // us to miss pending merges.
    debug_assert!(
      inner
        .merge_threads
        .iter()
        .any(|merge_thread| merge_thread.is_current_thread()),
      "caller is not a merge thread"
    );

    // Let CMS run new merges if necessary:
    let merge_result =
      match self.merge_locked(&mut inner, merge_source, MergeTrigger::MergeFinished) {
        Ok(()) | Err(LuceneError::AlreadyClosed(_)) => Ok(()),
        Err(err) => Err(LuceneError::unchecked_io_error(err.to_string())),
      };
    let finish_result = {
      Self::remove_merge_thread(&mut inner);
      match Self::update_merge_threads(&mut inner) {
        Ok(()) => {
          // In case we had stalled indexing, we can now wake up
          // and possibly unstall:
          self.changed.notify_all();
          Ok(())
        },
        Err(err) => Err(err),
      }
    };
    match finish_result {
      Err(err) => Err(err),
      Ok(()) => merge_result,
    }
  }
}

/** State for a running merge thread tracked by the scheduler. */
struct MergeThreadState {
  name: String,
  owner_id: Mutex<Option<ThreadId>>,
  estimated_merge_bytes: Arc<AtomicI64>,
  merge_start_ns: Arc<Mutex<Instant>>,
  merge_stat: MergeStat,
  rate_limiter: Arc<MergeRateLimiter>,
  alive: AtomicBool,
}

impl MergeThreadState {
  /** Sole constructor. */
  fn new<D>(name: String, merge: &OneMergeSR<D>) -> Self
  where
    D: Directory,
  {
    Self {
      name,
      owner_id: Mutex::new(None),
      estimated_merge_bytes: merge.estimated_merge_bytes.clone(),
      merge_start_ns: merge.merge_start_ns.clone(),
      merge_stat: merge.stat.clone(),
      rate_limiter: Arc::new(MergeRateLimiter::new(merge.get_merge_progress())),
      alive: AtomicBool::new(false),
    }
  }

  fn set_owner_to_current_thread(&self) {
    *self.owner_id.lock() = Some(thread::current().id());
  }

  fn is_current_thread(&self) -> bool {
    self
      .owner_id
      .lock()
      .map(|owner_id| owner_id == thread::current().id())
      .unwrap_or(false)
  }

  fn set_alive(&self, alive: bool) {
    self.alive.store(alive, Ordering::SeqCst);
  }

  fn is_alive(&self) -> bool {
    self.alive.load(Ordering::SeqCst)
  }
}

/** Runs a merge to execute a single merge, then exits. */
struct MergeThread<MS, D>
where
  D: Directory,
{
  state: Arc<MergeThreadState>,
  merge_source: MS,
  merge: OneMergeSR<D>,
}

impl<MS, D> MergeThread<MS, D>
where
  MS: MergeSource<D> + Clone + 'static,
  D: Directory + 'static,
  OneMergeSR<D>: Send + 'static,
{
  /** Sole constructor. */
  fn new(state: Arc<MergeThreadState>, merge_source: MS, merge: OneMergeSR<D>) -> Self {
    Self {
      state,
      merge_source,
      merge,
    }
  }

  fn run(self, merge_scheduler: ConcurrentMergeScheduler) -> Result<()> {
    let MergeThread {
      state,
      merge_source,
      merge,
    } = self;
    let merge_stat = merge.stat.clone();

    state.set_owner_to_current_thread();
    let previous =
      CURRENT_MERGE_RATE_LIMITER.with(|slot| slot.borrow_mut().replace(state.rate_limiter.clone()));
    let merge_result = merge_scheduler.do_merge(&merge_source, merge);
    CURRENT_MERGE_RATE_LIMITER.with(|slot| {
      *slot.borrow_mut() = previous;
    });

    let merge_aborted = merge_stat.is_aborted();

    if let Err(exc) = merge_result {
      let mut inner = merge_scheduler.inner.lock();
      ConcurrentMergeScheduler::remove_merge_thread(&mut inner);
      ConcurrentMergeScheduler::update_merge_threads(&mut inner)?;
      merge_scheduler.changed.notify_all();
      drop(inner);
      if matches!(exc, LuceneError::MergeAborted(_)) || merge_aborted {
        // OK to ignore.
        Ok(())
      } else if !merge_scheduler.inner.lock().suppress_exceptions {
        Err(ConcurrentMergeScheduler::handle_merge_exception(exc))
      } else {
        Ok(())
      }
    } else {
      merge_scheduler.run_on_merge_finished(merge_source)
    }
  }

  fn start(self, merge_scheduler: ConcurrentMergeScheduler) -> Result<()> {
    let state = self.state.clone();
    let thread_name = self.state.name.clone();
    let thread_state = state.clone();
    // WARNING: this must be set before spawning. After spawn succeeds, the caller immediately
    // runs update_merge_threads; if the new thread has not entered its closure yet, leaving this
    // false would let the scheduler prune a live merge thread.
    state.set_alive(true);
    match thread::Builder::new()
      .name(thread_name)
      .spawn(move || -> Result<()> {
        // Guard used as a Rust finally block: whenever the merge thread closure exits, mark the
        // tracked thread as no longer alive so update_merge_threads can prune it.
        struct AliveOnExit {
          state: Arc<MergeThreadState>,
        }

        impl Drop for AliveOnExit {
          fn drop(&mut self) {
            self.state.set_alive(false);
          }
        }

        let _alive_on_exit = AliveOnExit {
          state: thread_state,
        };
        self.run(merge_scheduler)
      }) {
      Ok(_handle) => Ok(()),
      Err(err) => {
        state.set_alive(false);
        Err(LuceneError::io(err))
      },
    }
  }
}

impl ConcurrentMergeScheduler {
  /** Called when an exception is hit in a background merge thread. */
  fn handle_merge_exception(exc: LuceneError) -> LuceneError {
    let mut merge_error = crate::core::util::error::MergeError::new(format!("merge failed: {exc}"));
    merge_error.add_suppressed(exc);
    LuceneError::Merge(merge_error)
  }
  /** Used for testing */
  pub(crate) fn set_suppress_exceptions(&self) {
    self.inner.lock().suppress_exceptions = true;
  }

  /** Used for testing */
  pub(crate) fn clear_suppress_exceptions(&self) {
    self.inner.lock().suppress_exceptions = false;
  }

  /** Used for testing */
  #[cfg(test)]
  pub(crate) fn set_stall_on_merge_thread(&self, stall_on_merge_thread: Arc<AtomicBool>) {
    self.inner.lock().stall_on_merge_thread = Some(stall_on_merge_thread);
  }
}

impl Display for ConcurrentMergeScheduler {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let inner = self.inner.lock();
    write!(
      f,
      "ConcurrentMergeScheduler: maxThreadCount={}, maxMergeCount={}, ioThrottle={}",
      inner.max_thread_count, inner.max_merge_count, inner.do_auto_io_throttle
    )
  }
}

impl ConcurrentMergeScheduler {
  fn is_backlog(inner: &Inner, now: Instant, merge_thread: &Arc<MergeThreadState>) -> bool {
    let merge_mb = Self::bytes_to_mb(merge_thread.estimated_merge_bytes.load(Ordering::SeqCst));
    for other in &inner.merge_threads {
      if other.is_alive()
        && !Arc::ptr_eq(other, merge_thread)
        && other.estimated_merge_bytes.load(Ordering::SeqCst)
          >= (MIN_BIG_MERGE_MB as i64) * 1024 * 1024
        && now
          .saturating_duration_since(*other.merge_start_ns.lock())
          .as_secs_f64()
          > 3.0
      {
        let other_merge_mb = Self::bytes_to_mb(other.estimated_merge_bytes.load(Ordering::SeqCst));
        let ratio = other_merge_mb / merge_mb;
        if ratio > 0.3 && ratio < 3.0 {
          return true;
        }
      }
    }

    false
  }

  /** Tunes IO throttle when a new merge starts. */
  fn update_io_throttle(inner: &mut Inner, new_merge_thread: &Arc<MergeThreadState>) -> Result<()> {
    if !inner.do_auto_io_throttle {
      return Ok(());
    }

    let merge_mb = Self::bytes_to_mb(
      new_merge_thread
        .estimated_merge_bytes
        .load(Ordering::SeqCst),
    );
    if merge_mb < MIN_BIG_MERGE_MB {
      // Only watch non-trivial merges for throttling; this is safe because the MP must eventually
      // have to do larger merges:
      return Ok(());
    }

    let now = Instant::now();

    // Simplistic closed-loop feedback control: if we find any other similarly
    // sized merges running, then we are falling behind, so we bump up the
    // IO throttle, else we lower it:
    let new_backlog = Self::is_backlog(inner, now, new_merge_thread);

    let mut cur_backlog = false;

    if !new_backlog {
      if inner.merge_threads.len() > inner.max_thread_count as usize {
        // If there are already more than the maximum merge threads allowed, count that as backlog:
        cur_backlog = true;
      } else {
        // Now see if any still-running merges are backlog'd:
        for merge_thread in &inner.merge_threads {
          if Self::is_backlog(inner, now, merge_thread) {
            cur_backlog = true;
            break;
          }
        }
      }
    }

    if new_backlog {
      // This new merge adds to the backlog: increase IO throttle by 20%
      inner.target_mb_per_sec *= 1.20;
      if inner.target_mb_per_sec > MAX_MERGE_MB_PER_SEC {
        inner.target_mb_per_sec = MAX_MERGE_MB_PER_SEC;
      }
    } else if cur_backlog {
      // We still have an existing backlog; leave the rate as is.
    } else {
      // We are not falling behind: decrease IO throttle by 10%
      inner.target_mb_per_sec /= 1.10;
      if inner.target_mb_per_sec < MIN_MERGE_MB_PER_SEC {
        inner.target_mb_per_sec = MIN_MERGE_MB_PER_SEC;
      }
    }

    let rate = if new_merge_thread.merge_stat.max_num_segments() != -1 {
      inner.force_merge_mb_per_sec
    } else {
      inner.target_mb_per_sec
    };
    new_merge_thread.rate_limiter.set_mb_per_sec(rate)?;
    Self::target_mb_per_sec_changed();
    Ok(())
  }

  /** Subtype can override to tweak target_mb_per_sec. */
  fn target_mb_per_sec_changed() {}

  fn bytes_to_mb(bytes: i64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
  }
}
