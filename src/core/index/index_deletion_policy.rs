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
use crate::core::index::keep_only_last_commit_deletion_policy::KeepOnlyLastCommitDeletionPolicy;
use crate::core::index::no_deletion_policy::NoDeletionPolicy;
use crate::core::util::error::lucene_error::Result;
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test::core::index::test_deletion_policy::{
  ExpirationTimeDeletionPolicy, KeepAllDeletionPolicy, KeepLastNDeletionPolicy,
  KeepNoneOnInitDeletionPolicy,
};
#[cfg(test)]
use crate::test::core::index::test_transaction_rollback::{
  DeleteLastCommitPolicy, KeepAllTransactionDeletionPolicy, RollbackDeletionPolicy,
};
use std::fmt::{Display, Formatter};

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
  /// [`IndexCommit::delete`] of [`IndexCommit`].
  ///
  /// **Note:** the last commit point is the most recent one, i.e. the "front index state". Be
  /// careful not to delete it, unless you know for sure what you are doing, and unless you can
  /// afford to lose the index content while doing that.
  ///
  /// # Parameters
  ///
  /// * `commits` - List of current [`IndexCommit`] point-in-time commits, sorted by age (the 0th
  ///   one is the oldest commit). Note that for a new index this method is invoked with an empty
  ///   list.
  fn on_init(&self, commits: &[IC]) -> Result<()>;

  /// This is called each time the writer completed a commit. This gives the policy a chance to
  /// remove old commit points with each commit.
  ///
  /// The policy may now choose to delete old commit points by calling method [`IndexCommit::delete`]
  /// of [`IndexCommit`].
  ///
  /// This method is only called when
  /// [`IndexWriter::commit`](crate::core::index::index_writer::IndexWriter::commit) or
  /// [`IndexWriter::close`](crate::core::index::index_writer::IndexWriter::close) is called, or
  /// possibly not at all if the
  /// [`IndexWriter::rollback`](crate::core::index::index_writer::IndexWriter::rollback) is called.
  ///
  /// **Note:** the last commit point is the most recent one, i.e. the "front index state". Be
  /// careful not to delete it, unless you know for sure what you are doing, and unless you can
  /// afford to lose the index content while doing that.
  ///
  /// # Parameters
  ///
  /// * `commits` - List of [`IndexCommit`], sorted by age (the 0th one is the oldest commit).
  fn on_commit(&self, commits: &[IC]) -> Result<()>;
}

pub enum IndexDeletionPolicyEnum {
  KeepOnlyLastCommit(KeepOnlyLastCommitDeletionPolicy),
  No(NoDeletionPolicy),
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
}

impl_from_for_enum!(
  IndexDeletionPolicyEnum,
  KeepOnlyLastCommitDeletionPolicy => KeepOnlyLastCommit,
  NoDeletionPolicy => No,
);

#[cfg(test)]
impl_from_for_enum!(
  IndexDeletionPolicyEnum,
  KeepAllDeletionPolicy => KeepAll,
  KeepNoneOnInitDeletionPolicy => KeepNoneOnInit,
  KeepLastNDeletionPolicy => KeepLastN,
  ExpirationTimeDeletionPolicy => ExpirationTime,
  KeepAllTransactionDeletionPolicy => KeepAllTransaction,
  RollbackDeletionPolicy => Rollback,
  DeleteLastCommitPolicy => DeleteLastCommit,
);

impl Display for IndexDeletionPolicyEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::KeepOnlyLastCommit(policy) => write!(f, "{policy}"),
      Self::No(policy) => write!(f, "{policy}"),
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
    }
  }
}

impl<IC> IndexDeletionPolicy<IC> for IndexDeletionPolicyEnum
where
  IC: IndexCommit + Clone,
{
  fn on_init(&self, commits: &[IC]) -> Result<()> {
    match self {
      Self::KeepOnlyLastCommit(policy) => policy.on_init(commits),
      Self::No(policy) => policy.on_init(commits),
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
    }
  }

  fn on_commit(&self, commits: &[IC]) -> Result<()> {
    match self {
      Self::KeepOnlyLastCommit(policy) => policy.on_commit(commits),
      Self::No(policy) => policy.on_commit(commits),
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
    }
  }
}
