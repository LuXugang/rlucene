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
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_file_deleter::CommitPoint;
use crate::core::index::keep_only_last_commit_deletion_policy::KeepOnlyLastCommitDeletionPolicy;
use crate::core::index::no_deletion_policy::NoDeletionPolicy;
use crate::core::index::persistent_snapshot_deletion_policy::PersistentSnapshotDeletionPolicy;
use crate::core::index::snapshot_deletion_policy::{SnapshotCommitPoint, SnapshotDeletionPolicy};
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{LuceneError, Result};
#[cfg(test)]
use crate::test_framework::core::index::deletion_policy::{
  DeleteLastCommitPolicy, KeepAllTransactionDeletionPolicy, RollbackDeletionPolicy,
};
#[cfg(test)]
use crate::test_framework::core::index::deletion_policy::{
  ExpirationTimeDeletionPolicy, KeepAllDeletionPolicy, KeepLastNDeletionPolicy,
  KeepNoneOnInitDeletionPolicy,
};
#[cfg(test)]
use crate::test_framework::core::index::test_check_index::DeleteNothingIndexDeletionPolicy;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// Expert: policy for deletion of stale [`IndexCommit`] index commits.
///
/// Implement this trait, and set it on
/// [`IndexWriterConfig::set_index_deletion_policy`](crate::core::index::index_writer_config::IndexWriterConfig::set_index_deletion_policy)
/// to customize when older [`IndexCommit`] point-in-time commits are deleted from the index
/// directory.
///
/// The default deletion policy is
/// [`KeepOnlyLastCommitDeletionPolicy`], always removes old commits as soon as a new commit is done
/// (this matches the behavior before 2.2).
///
/// One expected use case for this (and the reason why it was first created) is to work around
/// problems with an index directory accessed via filesystems like NFS because NFS does not provide
/// the "delete on last close" semantics that Lucene's "point in time" search normally relies on. By
/// implementing a custom deletion policy, such as "a commit is only removed once it has been stale
/// for more than X minutes", you can give your readers time to refresh to the new commit before
/// [`IndexWriter`](crate::core::index::index_writer::IndexWriter) removes the old commits. Note that
/// doing so will increase the storage requirements of the index. See
/// [LUCENE-710](http://issues.apache.org/jira/browse/LUCENE-710) for details.
pub trait IndexDeletionPolicy<IC>: Display
where
  IC: IndexCommit + Clone,
{
  /// This is called once when a writer is first instantiated to give the policy a chance to remove
  /// old commit points.
  ///
  /// The writer locates all index commits present in the index directory and calls this method. The
  /// policy may choose to delete some of the commit points, doing so by calling method
  /// [`IndexCommit::delete`] on a commit point.
  ///
  /// **Note:** the last commit point is the most recent one, i.e. the "front index state". Be
  /// careful not to delete it, unless you know for sure what you are doing, and unless you can
  /// afford to lose the index content while doing that.
  ///
  /// # Parameters
  ///
  /// * `commits` - Current [`IndexCommit`] point-in-time commits, sorted by age (the 0th
  ///   one is the oldest commit). Note that for a new index this method is invoked with an empty
  ///   list.
  fn on_init(&self, commits: &[IC]) -> Result<()>;

  /// This is called each time the writer completed a commit. This gives the policy a chance to
  /// remove old commit points with each commit.
  ///
  /// The policy may now choose to delete old commit points by calling [`IndexCommit::delete`].
  ///
  /// This method is only called when
  /// [`TwoPhaseCommit::commit`](crate::core::index::two_phase_commit::TwoPhaseCommit::commit) or
  /// [`IndexWriter::close`](crate::core::index::index_writer::IndexWriter::close) is called, or
  /// possibly not at all if the
  /// [`TwoPhaseCommit::rollback`](crate::core::index::two_phase_commit::TwoPhaseCommit::rollback) is
  /// called.
  ///
  /// **Note:** the last commit point is the most recent one, i.e. the "front index state". Be
  /// careful not to delete it, unless you know for sure what you are doing, and unless you can
  /// afford to lose the index content while doing that.
  ///
  /// # Parameters
  ///
  /// * `commits` - [`IndexCommit`] values sorted by age (the 0th one is the oldest commit).
  fn on_commit(&self, commits: &[IC]) -> Result<()>;
}

pub enum IndexDeletionPolicyEnum<D> {
  KeepOnlyLastCommit(KeepOnlyLastCommitDeletionPolicy),
  No(NoDeletionPolicy),
  Snapshot(Box<SnapshotDeletionPolicy<D>>),
  PersistentSnapshot(Box<PersistentSnapshotDeletionPolicy<D>>),
  #[cfg(test)]
  KeepAll(KeepAllDeletionPolicy),
  #[cfg(test)]
  KeepNoneOnInit(KeepNoneOnInitDeletionPolicy),
  #[cfg(test)]
  KeepLastN(KeepLastNDeletionPolicy),
  #[cfg(test)]
  ExpirationTime(ExpirationTimeDeletionPolicy),
  #[cfg(test)]
  KeepAllTransaction(KeepAllTransactionDeletionPolicy),
  #[cfg(test)]
  Rollback(RollbackDeletionPolicy),
  #[cfg(test)]
  DeleteLastCommit(DeleteLastCommitPolicy),
  #[cfg(test)]
  DeleteNothing(DeleteNothingIndexDeletionPolicy),
}

impl<D> From<KeepOnlyLastCommitDeletionPolicy> for IndexDeletionPolicyEnum<D> {
  fn from(policy: KeepOnlyLastCommitDeletionPolicy) -> Self {
    Self::KeepOnlyLastCommit(policy)
  }
}

impl<D> From<NoDeletionPolicy> for IndexDeletionPolicyEnum<D> {
  fn from(policy: NoDeletionPolicy) -> Self {
    Self::No(policy)
  }
}

impl<D> From<SnapshotDeletionPolicy<D>> for IndexDeletionPolicyEnum<D> {
  fn from(policy: SnapshotDeletionPolicy<D>) -> Self {
    Self::Snapshot(Box::new(policy))
  }
}

impl<D> From<PersistentSnapshotDeletionPolicy<D>> for IndexDeletionPolicyEnum<D> {
  fn from(policy: PersistentSnapshotDeletionPolicy<D>) -> Self {
    Self::PersistentSnapshot(Box::new(policy))
  }
}

#[cfg(test)]
impl<D> From<KeepAllDeletionPolicy> for IndexDeletionPolicyEnum<D> {
  fn from(policy: KeepAllDeletionPolicy) -> Self {
    Self::KeepAll(policy)
  }
}

#[cfg(test)]
impl<D> From<KeepNoneOnInitDeletionPolicy> for IndexDeletionPolicyEnum<D> {
  fn from(policy: KeepNoneOnInitDeletionPolicy) -> Self {
    Self::KeepNoneOnInit(policy)
  }
}

#[cfg(test)]
impl<D> From<KeepLastNDeletionPolicy> for IndexDeletionPolicyEnum<D> {
  fn from(policy: KeepLastNDeletionPolicy) -> Self {
    Self::KeepLastN(policy)
  }
}

#[cfg(test)]
impl<D> From<ExpirationTimeDeletionPolicy> for IndexDeletionPolicyEnum<D> {
  fn from(policy: ExpirationTimeDeletionPolicy) -> Self {
    Self::ExpirationTime(policy)
  }
}

#[cfg(test)]
impl<D> From<KeepAllTransactionDeletionPolicy> for IndexDeletionPolicyEnum<D> {
  fn from(policy: KeepAllTransactionDeletionPolicy) -> Self {
    Self::KeepAllTransaction(policy)
  }
}

#[cfg(test)]
impl<D> From<RollbackDeletionPolicy> for IndexDeletionPolicyEnum<D> {
  fn from(policy: RollbackDeletionPolicy) -> Self {
    Self::Rollback(policy)
  }
}

#[cfg(test)]
impl<D> From<DeleteLastCommitPolicy> for IndexDeletionPolicyEnum<D> {
  fn from(policy: DeleteLastCommitPolicy) -> Self {
    Self::DeleteLastCommit(policy)
  }
}

#[cfg(test)]
impl<D> From<DeleteNothingIndexDeletionPolicy> for IndexDeletionPolicyEnum<D> {
  fn from(policy: DeleteNothingIndexDeletionPolicy) -> Self {
    Self::DeleteNothing(policy)
  }
}

impl<D> Display for IndexDeletionPolicyEnum<D> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::KeepOnlyLastCommit(policy) => write!(f, "{policy}"),
      Self::No(policy) => write!(f, "{policy}"),
      Self::Snapshot(policy) => write!(f, "{policy}"),
      Self::PersistentSnapshot(policy) => write!(f, "{policy}"),
      #[cfg(test)]
      Self::KeepAll(policy) => write!(f, "{policy}"),
      #[cfg(test)]
      Self::KeepNoneOnInit(policy) => write!(f, "{policy}"),
      #[cfg(test)]
      Self::KeepLastN(policy) => write!(f, "{policy}"),
      #[cfg(test)]
      Self::ExpirationTime(policy) => write!(f, "{policy}"),
      #[cfg(test)]
      Self::KeepAllTransaction(policy) => write!(f, "{policy}"),
      #[cfg(test)]
      Self::Rollback(policy) => write!(f, "{policy}"),
      #[cfg(test)]
      Self::DeleteLastCommit(policy) => write!(f, "{policy}"),
      #[cfg(test)]
      Self::DeleteNothing(policy) => write!(f, "{policy}"),
    }
  }
}

impl<D> IndexDeletionPolicy<Arc<CommitPoint<D>>> for IndexDeletionPolicyEnum<D>
where
  D: Directory,
{
  fn on_init(&self, commits: &[Arc<CommitPoint<D>>]) -> Result<()> {
    match self {
      Self::KeepOnlyLastCommit(policy) => policy.on_init(commits),
      Self::No(policy) => policy.on_init(commits),
      Self::Snapshot(policy) => policy.on_init(commits),
      Self::PersistentSnapshot(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::KeepAll(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::KeepNoneOnInit(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::KeepLastN(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::ExpirationTime(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::KeepAllTransaction(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::Rollback(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::DeleteLastCommit(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::DeleteNothing(policy) => policy.on_init(commits),
    }
  }

  fn on_commit(&self, commits: &[Arc<CommitPoint<D>>]) -> Result<()> {
    match self {
      Self::KeepOnlyLastCommit(policy) => policy.on_commit(commits),
      Self::No(policy) => policy.on_commit(commits),
      Self::Snapshot(policy) => policy.on_commit(commits),
      Self::PersistentSnapshot(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::KeepAll(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::KeepNoneOnInit(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::KeepLastN(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::ExpirationTime(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::KeepAllTransaction(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::Rollback(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::DeleteLastCommit(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::DeleteNothing(policy) => policy.on_commit(commits),
    }
  }
}

impl<D> IndexDeletionPolicy<SnapshotCommitPoint<D>> for IndexDeletionPolicyEnum<D>
where
  D: Directory,
{
  fn on_init(&self, commits: &[SnapshotCommitPoint<D>]) -> Result<()> {
    match self {
      Self::KeepOnlyLastCommit(policy) => policy.on_init(commits),
      Self::No(policy) => policy.on_init(commits),
      Self::Snapshot(_) => Err(LuceneError::illegal_argument(
        "SnapshotDeletionPolicy cannot wrap another SnapshotDeletionPolicy",
      )),
      Self::PersistentSnapshot(_) => Err(LuceneError::illegal_argument(
        "SnapshotDeletionPolicy cannot wrap another SnapshotDeletionPolicy",
      )),
      #[cfg(test)]
      Self::KeepAll(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::KeepNoneOnInit(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::KeepLastN(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::ExpirationTime(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::KeepAllTransaction(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::Rollback(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::DeleteLastCommit(policy) => policy.on_init(commits),
      #[cfg(test)]
      Self::DeleteNothing(policy) => policy.on_init(commits),
    }
  }

  fn on_commit(&self, commits: &[SnapshotCommitPoint<D>]) -> Result<()> {
    match self {
      Self::KeepOnlyLastCommit(policy) => policy.on_commit(commits),
      Self::No(policy) => policy.on_commit(commits),
      Self::Snapshot(_) => Err(LuceneError::illegal_argument(
        "SnapshotDeletionPolicy cannot wrap another SnapshotDeletionPolicy",
      )),
      Self::PersistentSnapshot(_) => Err(LuceneError::illegal_argument(
        "SnapshotDeletionPolicy cannot wrap another SnapshotDeletionPolicy",
      )),
      #[cfg(test)]
      Self::KeepAll(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::KeepNoneOnInit(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::KeepLastN(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::ExpirationTime(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::KeepAllTransaction(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::Rollback(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::DeleteLastCommit(policy) => policy.on_commit(commits),
      #[cfg(test)]
      Self::DeleteNothing(policy) => policy.on_commit(commits),
    }
  }
}
