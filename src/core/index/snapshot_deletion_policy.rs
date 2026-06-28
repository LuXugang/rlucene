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
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

use crate::core::index::index_commit::{IndexCommit, cmp_commit, is_same_commit};
use crate::core::index::index_deletion_policy::{IndexDeletionPolicy, IndexDeletionPolicyEnum};
use crate::core::index::index_file_deleter::CommitPoint;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{LuceneError, Result};

const MISUSE_MESSAGE: &str = "this instance is not being used by IndexWriter; be sure to use the instance returned from writer.getConfig().getIndexDeletionPolicy()";

/// An [`IndexDeletionPolicy`] that wraps any other [`IndexDeletionPolicy`] and adds the ability to
/// hold and later release snapshots of an index. While a snapshot is held, the
/// [`IndexWriter`](crate::core::index::index_writer::IndexWriter) will not remove any files
/// associated with it even if the index is otherwise being actively, arbitrarily changed. Because we
/// wrap another arbitrary [`IndexDeletionPolicy`], this gives you the freedom to continue using
/// whatever [`IndexDeletionPolicy`] you would normally want to use with your index.
///
/// This struct maintains all snapshots in-memory, and so the information is not persisted and not
/// protected against system failures. If persistence is important, you can use
/// `PersistentSnapshotDeletionPolicy`.
///
/// # Experimental
pub struct SnapshotDeletionPolicy<D>
where
  D: Directory,
{
  primary: Arc<IndexDeletionPolicyEnum<D>>,
  inner: Arc<Mutex<Inner<D>>>,
  op_lock: Arc<Mutex<()>>,
}

pub(crate) struct SnapshotDeletionPolicyLock<'a> {
  _guard: MutexGuard<'a, ()>,
}

struct Inner<D>
where
  D: Directory,
{
  /// Records how many snapshots are held against each commit generation.
  ref_counts: HashMap<i64, i32>,

  /// Used to map gen to IndexCommit.
  index_commits: HashMap<i64, Arc<CommitPoint<D>>>,

  /// Most recently committed IndexCommit.
  last_commit: Option<Arc<CommitPoint<D>>>,

  /// Used to detect misuse.
  init_called: bool,
}

impl<D> Clone for SnapshotDeletionPolicy<D>
where
  D: Directory,
{
  fn clone(&self) -> Self {
    Self {
      primary: self.primary.clone(),
      inner: self.inner.clone(),
      op_lock: self.op_lock.clone(),
    }
  }
}

impl<D> SnapshotDeletionPolicy<D>
where
  D: Directory,
{
  /// Sole constructor, taking the incoming [`IndexDeletionPolicy`] to wrap.
  pub fn new<T>(primary: T) -> Self
  where
    T: Into<IndexDeletionPolicyEnum<D>>,
  {
    SnapshotDeletionPolicy {
      primary: Arc::new(primary.into()),
      inner: Arc::new(Mutex::new(Inner {
        ref_counts: HashMap::new(),
        index_commits: HashMap::new(),
        last_commit: None,
        init_called: false,
      })),
      op_lock: Arc::new(Mutex::new(())),
    }
  }

  pub(crate) fn lock(&self) -> SnapshotDeletionPolicyLock<'_> {
    SnapshotDeletionPolicyLock {
      _guard: self.op_lock.lock(),
    }
  }

  /// Release a snapshotted commit.
  ///
  /// # Parameters
  ///
  /// * `commit` - the commit previously returned by [`Self::snapshot`].
  pub fn release(&self, commit: &Arc<CommitPoint<D>>) -> Result<()> {
    self.release_with_lock(commit, None)
  }

  pub(crate) fn release_with_lock(
    &self,
    commit: &Arc<CommitPoint<D>>,
    op_lock: Option<&SnapshotDeletionPolicyLock<'_>>,
  ) -> Result<()> {
    let generation = commit.get_generation();
    self.release_gen_with_lock(generation, op_lock)
  }

  /// Release a snapshot by generation.
  pub fn release_gen(&self, generation: i64) -> Result<()> {
    self.release_gen_with_lock(generation, None)
  }

  pub(crate) fn release_gen_with_lock(
    &self,
    generation: i64,
    op_lock: Option<&SnapshotDeletionPolicyLock<'_>>,
  ) -> Result<()> {
    if op_lock.is_none() {
      let op_lock = self.lock();
      return self.release_gen_with_lock(generation, Some(&op_lock));
    }
    self.release_gen_locked(generation)
  }

  fn release_gen_locked(&self, generation: i64) -> Result<()> {
    let mut inner = self.inner.lock();
    if !inner.init_called {
      return Err(LuceneError::illegal_state(MISUSE_MESSAGE));
    }
    let ref_count = inner.ref_counts.get_mut(&generation).ok_or_else(|| {
      LuceneError::illegal_argument(format!(
        "commit gen={generation} is not currently snapshotted"
      ))
    })?;
    debug_assert!(*ref_count > 0);
    *ref_count -= 1;
    if *ref_count == 0 {
      inner.ref_counts.remove(&generation);
      inner.index_commits.remove(&generation);
    }
    Ok(())
  }

  pub(crate) fn inc_ref_with_lock(
    &self,
    ic: &Arc<CommitPoint<D>>,
    op_lock: Option<&SnapshotDeletionPolicyLock<'_>>,
  ) {
    if op_lock.is_none() {
      let op_lock = self.lock();
      self.inc_ref_with_lock(ic, Some(&op_lock));
      return;
    }
    let mut inner = self.inner.lock();
    self.inc_ref_locked(ic, &mut inner);
  }

  fn inc_ref_locked(&self, ic: &Arc<CommitPoint<D>>, inner: &mut Inner<D>) {
    let generation = ic.get_generation();
    let ref_count = inner.ref_counts.get(&generation).copied().unwrap_or(0);
    if ref_count == 0 {
      inner.index_commits.insert(generation, ic.clone());
    }
    inner.ref_counts.insert(generation, ref_count + 1);
  }

  /// Snapshots the last commit and returns it. Once a commit is 'snapshotted,' it is protected from
  /// deletion (as long as this [`IndexDeletionPolicy`] is used). The snapshot can be removed by
  /// calling [`Self::release`] followed by a call to
  /// [`IndexWriter::delete_unused_files`](crate::core::index::index_writer::IndexWriter::delete_unused_files).
  ///
  /// **NOTE:** while the snapshot is held, the files it references will not be deleted, which will
  /// consume additional disk space in your index. If you take a snapshot at a particularly bad time
  /// (say just before you call forceMerge) then in the worst case this could consume an extra 1X of
  /// your total index size, until you release the snapshot.
  ///
  /// # Errors
  ///
  /// Returns an [`IllegalState`](LuceneError::IllegalState) error if this index does not have any
  /// commits yet.
  pub fn snapshot(&self) -> Result<Arc<CommitPoint<D>>> {
    self.snapshot_with_lock(None)
  }

  pub(crate) fn snapshot_with_lock(
    &self,
    op_lock: Option<&SnapshotDeletionPolicyLock<'_>>,
  ) -> Result<Arc<CommitPoint<D>>> {
    if op_lock.is_none() {
      let op_lock = self.lock();
      return self.snapshot_with_lock(Some(&op_lock));
    }
    let mut inner = self.inner.lock();
    if !inner.init_called {
      return Err(LuceneError::illegal_state(MISUSE_MESSAGE));
    }
    let last_commit = inner
      .last_commit
      .clone()
      .ok_or_else(|| LuceneError::illegal_state("No index commit to snapshot"))?;

    self.inc_ref_locked(&last_commit, &mut inner);

    Ok(last_commit)
  }

  /// Returns all IndexCommits held by at least one snapshot.
  pub fn get_snapshots(&self) -> Vec<Arc<CommitPoint<D>>> {
    let op_lock = self.lock();
    self.get_snapshots_with_lock(Some(&op_lock))
  }

  pub(crate) fn get_snapshots_with_lock(
    &self,
    op_lock: Option<&SnapshotDeletionPolicyLock<'_>>,
  ) -> Vec<Arc<CommitPoint<D>>> {
    if op_lock.is_none() {
      let op_lock = self.lock();
      return self.get_snapshots_with_lock(Some(&op_lock));
    }
    let inner = self.inner.lock();
    inner.index_commits.values().cloned().collect()
  }

  /// Returns the total number of snapshots currently held.
  pub fn get_snapshot_count(&self) -> i32 {
    let op_lock = self.lock();
    self.get_snapshot_count_with_lock(Some(&op_lock))
  }

  pub(crate) fn get_snapshot_count_with_lock(
    &self,
    op_lock: Option<&SnapshotDeletionPolicyLock<'_>>,
  ) -> i32 {
    if op_lock.is_none() {
      let op_lock = self.lock();
      return self.get_snapshot_count_with_lock(Some(&op_lock));
    }
    let inner = self.inner.lock();
    inner.ref_counts.values().sum()
  }

  /// Retrieve an [`IndexCommit`] from its generation; returns `None` if this IndexCommit is not
  /// currently snapshotted.
  pub fn get_index_commit(&self, generation: i64) -> Option<Arc<CommitPoint<D>>> {
    let op_lock = self.lock();
    self.get_index_commit_with_lock(generation, Some(&op_lock))
  }

  pub(crate) fn get_index_commit_with_lock(
    &self,
    generation: i64,
    op_lock: Option<&SnapshotDeletionPolicyLock<'_>>,
  ) -> Option<Arc<CommitPoint<D>>> {
    if op_lock.is_none() {
      let op_lock = self.lock();
      return self.get_index_commit_with_lock(generation, Some(&op_lock));
    }
    let inner = self.inner.lock();
    inner.index_commits.get(&generation).cloned()
  }

  pub(crate) fn ref_counts_with_lock(
    &self,
    op_lock: Option<&SnapshotDeletionPolicyLock<'_>>,
  ) -> HashMap<i64, i32> {
    if op_lock.is_none() {
      let op_lock = self.lock();
      return self.ref_counts_with_lock(Some(&op_lock));
    }
    let inner = self.inner.lock();
    inner.ref_counts.clone()
  }

  pub(crate) fn set_ref_counts_with_lock(
    &self,
    ref_counts: HashMap<i64, i32>,
    op_lock: Option<&SnapshotDeletionPolicyLock<'_>>,
  ) {
    if op_lock.is_none() {
      let op_lock = self.lock();
      self.set_ref_counts_with_lock(ref_counts, Some(&op_lock));
      return;
    }
    let mut inner = self.inner.lock();
    inner.ref_counts = ref_counts;
    inner.index_commits.clear();
  }

  /// Wraps each [`IndexCommit`] as a [`SnapshotCommitPoint`].
  fn wrap_commits(&self, commits: &[Arc<CommitPoint<D>>]) -> Vec<SnapshotCommitPoint<D>> {
    let mut wrapped_commits = Vec::with_capacity(commits.len());
    for ic in commits {
      wrapped_commits.push(SnapshotCommitPoint::new(self.inner.clone(), ic.clone()));
    }
    wrapped_commits
  }
}
impl<D> IndexDeletionPolicy<Arc<CommitPoint<D>>> for SnapshotDeletionPolicy<D>
where
  D: Directory,
{
  fn on_init(&self, commits: &[Arc<CommitPoint<D>>]) -> Result<()> {
    let _op_lock = self.op_lock.lock();
    let wrapped_commits = self.wrap_commits(commits);
    {
      let mut inner = self.inner.lock();
      inner.init_called = true;
    }
    self.primary.on_init(&wrapped_commits)?;
    let mut inner = self.inner.lock();
    for commit in commits {
      if inner.ref_counts.contains_key(&commit.get_generation()) {
        inner
          .index_commits
          .insert(commit.get_generation(), commit.clone());
      }
    }
    if let Some(last_commit) = commits.last() {
      inner.last_commit = Some(last_commit.clone());
    }
    Ok(())
  }

  fn on_commit(&self, commits: &[Arc<CommitPoint<D>>]) -> Result<()> {
    let _op_lock = self.op_lock.lock();
    let wrapped_commits = self.wrap_commits(commits);
    self.primary.on_commit(&wrapped_commits)?;
    let mut inner = self.inner.lock();
    inner.last_commit = commits.last().cloned();
    Ok(())
  }
}

impl<D> Display for SnapshotDeletionPolicy<D>
where
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

/// Wraps a provided [`IndexCommit`] and prevents it from being deleted.
pub struct SnapshotCommitPoint<D>
where
  D: Directory,
{
  /// The [`IndexCommit`] we are preventing from deletion.
  cp: Arc<CommitPoint<D>>,
  snapshot_policy: Arc<Mutex<Inner<D>>>,
}

impl<D> SnapshotCommitPoint<D>
where
  D: Directory,
{
  /// Creates a [`SnapshotCommitPoint`] wrapping the provided [`IndexCommit`].
  fn new(policy: Arc<Mutex<Inner<D>>>, cp: Arc<CommitPoint<D>>) -> Self {
    SnapshotCommitPoint {
      cp,
      snapshot_policy: policy,
    }
  }
}

impl<D> Clone for SnapshotCommitPoint<D>
where
  D: Directory,
{
  fn clone(&self) -> Self {
    SnapshotCommitPoint {
      cp: self.cp.clone(),
      snapshot_policy: self.snapshot_policy.clone(),
    }
  }
}

impl<D> PartialEq for SnapshotCommitPoint<D>
where
  D: Directory,
{
  fn eq(&self, other: &Self) -> bool {
    is_same_commit(self, other)
  }
}

impl<D> Eq for SnapshotCommitPoint<D> where D: Directory {}

impl<D> PartialOrd for SnapshotCommitPoint<D>
where
  D: Directory,
{
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl<D> Ord for SnapshotCommitPoint<D>
where
  D: Directory,
{
  fn cmp(&self, other: &Self) -> Ordering {
    cmp_commit(self, other)
  }
}

impl<D> Display for SnapshotCommitPoint<D>
where
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "SnapshotDeletionPolicy.SnapshotCommitPoint({})", self.cp)
  }
}

impl<D> IndexCommit for SnapshotCommitPoint<D>
where
  D: Directory,
{
  fn get_segments_file_name(&self) -> &str {
    self.cp.get_segments_file_name()
  }

  fn get_file_names(&self) -> Result<&[String]> {
    self.cp.get_file_names()
  }

  type Directory = Arc<D>;

  fn get_directory(&self) -> Self::Directory {
    self.cp.get_directory()
  }

  fn delete(&self) -> Result<()> {
    let inner = self.snapshot_policy.lock();
    // Suppress the delete request if this commit point is
    // currently snapshotted.
    if !inner.ref_counts.contains_key(&self.cp.get_generation()) {
      self.cp.delete()?;
    }
    Ok(())
  }

  fn is_deleted(&self) -> bool {
    self.cp.is_deleted()
  }

  fn get_segment_count(&self) -> usize {
    self.cp.get_segment_count()
  }

  fn get_generation(&self) -> i64 {
    self.cp.get_generation()
  }

  fn get_user_data(&self) -> &HashMap<String, String> {
    self.cp.get_user_data()
  }
}
