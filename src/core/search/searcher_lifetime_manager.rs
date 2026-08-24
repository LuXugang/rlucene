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
use crate::core::index::directory_reader::DirectoryReader;
use crate::core::index::index_reader::{
  IndexReader, IndexReaderContextKind, IndexReaderContextType,
};
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use parking_lot::{Mutex, RwLock};
use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

type ManagedSearcher<DR> = IndexSearcher<IndexReaderContextType<Arc<DR>>>;

struct SearcherTracker<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>>,
  IndexReaderContextType<Arc<DR>>: 'static,
{
  searcher: Arc<ManagedSearcher<DR>>,
  record_time: Instant,
  version: i64,
  close_lock: Mutex<()>,
}

impl<DR> SearcherTracker<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>>,
  IndexReaderContextType<Arc<DR>>: 'static,
{
  fn new(searcher: Arc<ManagedSearcher<DR>>) -> Result<Self> {
    let version = searcher.get_index_reader().get_version()?;
    searcher.get_index_reader().inc_ref()?;
    // Use a monotonic clock to reduce the risk from wall-clock shifts.
    let record_time = Instant::now();
    Ok(Self {
      searcher,
      record_time,
      version,
      close_lock: Mutex::new(()),
    })
  }

  fn close(&self) -> Result<()> {
    let _close_lock = self.close_lock.lock();
    self.searcher.get_index_reader().dec_ref()
  }
}

/// Keeps track of current plus old [`IndexSearcher`]s, closing the old ones once they have timed
/// out.
///
/// Keeping many searchers around uses more open files and RAM than keeping a single searcher.
/// However, when [`DirectoryReader`] reopening is used, the searchers usually share almost all
/// segments and the additional resource usage is contained.
///
/// @lucene.experimental
pub struct SearcherLifetimeManager<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>>,
  IndexReaderContextType<Arc<DR>>: 'static,
{
  closed: AtomicBool,
  searchers: RwLock<HashMap<i64, Arc<SearcherTracker<DR>>>>,
  operation_lock: Mutex<()>,
}

impl<DR> Default for SearcherLifetimeManager<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>>,
  IndexReaderContextType<Arc<DR>>: 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<DR> SearcherLifetimeManager<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>>,
  IndexReaderContextType<Arc<DR>>: 'static,
{
  pub fn new() -> Self {
    Self {
      closed: AtomicBool::new(false),
      searchers: RwLock::new(HashMap::new()),
      operation_lock: Mutex::new(()),
    }
  }

  fn ensure_open(&self) -> Result<()> {
    if self.closed.load(Ordering::SeqCst) {
      return Err(LuceneError::already_closed(
        "this SearcherLifetimeManager instance is closed",
      ));
    }
    Ok(())
  }

  /// Records that the provided searcher is now in use. It is fine to pass the same searcher more
  /// than once.
  ///
  /// The returned token can later be passed to [`acquire`](Self::acquire) to retrieve the same
  /// searcher.
  pub fn record(&self, searcher: &Arc<ManagedSearcher<DR>>) -> Result<i64> {
    self.ensure_open()?;
    let version = searcher.get_index_reader().get_version()?;
    if let Some(tracker) = self.searchers.read().get(&version).cloned() {
      if !Arc::ptr_eq(&tracker.searcher, searcher) {
        return Err(LuceneError::illegal_argument(format!(
          "the provided searcher has the same underlying reader version yet the searcher instance differs from before (new={:p} vs old={:p})",
          Arc::as_ptr(searcher),
          Arc::as_ptr(&tracker.searcher)
        )));
      }
    } else {
      let tracker = Arc::new(SearcherTracker::new(searcher.clone())?);
      let existing = {
        let mut searchers = self.searchers.write();
        if let Some(existing) = searchers.get(&version) {
          Some(existing.clone())
        } else {
          searchers.insert(version, tracker.clone());
          None
        }
      };
      if existing.is_some() {
        // Another thread beat us: undo the incRef performed by SearcherTracker::new.
        tracker.close()?;
      }
    }
    Ok(version)
  }

  /// Retrieves a previously recorded searcher if it has not yet been closed.
  ///
  /// If this returns a searcher, it must later be passed to [`release`](Self::release), preferably
  /// from a finally-equivalent path.
  pub fn acquire(&self, version: i64) -> Result<Option<Arc<ManagedSearcher<DR>>>> {
    self.ensure_open()?;
    let tracker = self.searchers.read().get(&version).cloned();
    if let Some(tracker) = tracker
      && tracker.searcher.get_index_reader().try_inc_ref()
    {
      return Ok(Some(tracker.searcher.clone()));
    }
    Ok(None)
  }

  /// Releases a searcher previously obtained from [`acquire`](Self::acquire).
  ///
  /// It is safe to call this after [`close`](Self::close).
  pub fn release(&self, searcher: Arc<ManagedSearcher<DR>>) -> Result<()> {
    searcher.get_index_reader().dec_ref()
  }

  /// Calls the provided [`Pruner`] on entries in newest-to-oldest order.
  ///
  /// This should be called periodically, ideally from the same background thread that opens new
  /// searchers.
  pub fn prune<P>(&self, pruner: &P) -> Result<()>
  where
    P: Pruner<DR>,
  {
    let _operation_lock = self.operation_lock.lock();
    // Copy one entry at a time because the map can change while the snapshot is being built.
    let mut trackers = Vec::new();
    for tracker in self.searchers.read().values() {
      trackers.push(tracker.clone());
    }
    // Newer searchers sort before older searchers.
    trackers.sort_by_key(|tracker| Reverse(tracker.record_time));
    let mut last_record_time = None;
    let now = Instant::now();
    for tracker in trackers {
      let age_sec = last_record_time
        .map(|record_time| now.duration_since(record_time).as_secs_f64())
        .unwrap_or(0.0);
      if pruner.do_prune(age_sec, tracker.searcher.as_ref()) {
        self.searchers.write().remove(&tracker.version);
        tracker.close()?;
      }
      last_record_time = Some(tracker.record_time);
    }
    Ok(())
  }

  /// Closes this manager to future searching. Searches already in progress are unaffected and
  /// should still call [`release`](Self::release) when finished.
  ///
  /// No other thread should call [`record`](Self::record) while this method runs.
  pub fn close(&self) -> Result<()> {
    let _operation_lock = self.operation_lock.lock();
    self.closed.store(true, Ordering::SeqCst);
    let to_close = self.searchers.read().values().cloned().collect::<Vec<_>>();

    // Remove up front in case closing fails, so a second close does not over-decRef.
    {
      let mut searchers = self.searchers.write();
      for tracker in &to_close {
        searchers.remove(&tracker.version);
      }
    }

    IOUtils::close_with(&to_close, |tracker| tracker.close())?;

    // Make some effort to catch misuse.
    if !self.searchers.read().is_empty() {
      return Err(LuceneError::illegal_state(
        "another thread called record while this SearcherLifetimeManager instance was being closed; not all searchers were closed",
      ));
    }
    Ok(())
  }
}

/// Decides whether a searcher should be removed by [`SearcherLifetimeManager::prune`].
pub trait Pruner<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>>,
  IndexReaderContextType<Arc<DR>>: 'static,
{
  /// Returns `true` if this searcher should be removed.
  ///
  /// `age_sec` is how much time has passed since this searcher was the current live searcher.
  fn do_prune(&self, age_sec: f64, searcher: &ManagedSearcher<DR>) -> bool;
}

/// A simple pruner that drops searchers older by more than the specified number of seconds than
/// the newest searcher.
pub struct PruneByAge {
  max_age_sec: f64,
}

impl PruneByAge {
  pub fn new(max_age_sec: f64) -> Result<Self> {
    if max_age_sec < 0.0 {
      return Err(LuceneError::illegal_argument(format!(
        "maxAgeSec must be > 0 (got {max_age_sec})"
      )));
    }
    Ok(Self { max_age_sec })
  }
}

impl<DR> Pruner<DR> for PruneByAge
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>>,
  IndexReaderContextType<Arc<DR>>: 'static,
{
  fn do_prune(&self, age_sec: f64, _searcher: &ManagedSearcher<DR>) -> bool {
    age_sec > self.max_age_sec
  }
}

impl<DR> Closeable for SearcherLifetimeManager<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>>,
  IndexReaderContextType<Arc<DR>>: 'static,
{
  fn close(&mut self) -> Result<()> {
    SearcherLifetimeManager::close(self)
  }
}

impl<DR> CloseableRef for SearcherLifetimeManager<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>>,
  IndexReaderContextType<Arc<DR>>: 'static,
{
  fn close(&self) -> Result<()> {
    SearcherLifetimeManager::close(self)
  }
}
