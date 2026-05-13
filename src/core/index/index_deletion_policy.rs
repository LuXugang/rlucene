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
use std::fmt::{Display, Formatter};
/// This [`IndexDeletionPolicy`] implementation keeps only the most recent commit and
/// immediately removes all prior commits after a new commit is done. This is the default deletion
/// policy.
pub trait IndexDeletionPolicy: Display {
  /// Deletes all commits except the most recent one.
  fn on_init<IC>(&self, commits: &mut [IC]) -> Result<()>
  where
    IC: IndexCommit;

  /// Deletes all commits except the most recent one.
  fn on_commit<IC>(&self, commits: &mut [IC]) -> Result<()>
  where
    IC: IndexCommit;
}

pub enum IndexDeletionPolicyEnum {
  KeepOnlyLastCommit(KeepOnlyLastCommitDeletionPolicy),
  No(NoDeletionPolicy),
}

impl_from_for_enum!(
  IndexDeletionPolicyEnum,
  KeepOnlyLastCommitDeletionPolicy => KeepOnlyLastCommit,
  NoDeletionPolicy => No,
);

impl Display for IndexDeletionPolicyEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::KeepOnlyLastCommit(policy) => write!(f, "{policy}"),
      Self::No(policy) => write!(f, "{policy}"),
    }
  }
}

impl IndexDeletionPolicy for IndexDeletionPolicyEnum {
  fn on_init<IC>(&self, commits: &mut [IC]) -> Result<()>
  where
    IC: IndexCommit,
  {
    match self {
      Self::KeepOnlyLastCommit(policy) => policy.on_init(commits),
      Self::No(policy) => policy.on_init(commits),
    }
  }

  fn on_commit<IC>(&self, commits: &mut [IC]) -> Result<()>
  where
    IC: IndexCommit,
  {
    match self {
      Self::KeepOnlyLastCommit(policy) => policy.on_commit(commits),
      Self::No(policy) => policy.on_commit(commits),
    }
  }
}
