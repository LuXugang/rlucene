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
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader;
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_deletion_policy::IndexDeletionPolicy;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::segment_infos::generation_from_segments_file_name;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

fn verify_commit_order<IC>(commits: &[IC])
where
  IC: IndexCommit,
{
  if commits.is_empty() {
    return;
  }

  let first_commit = &commits[0];
  let mut last = generation_from_segments_file_name(first_commit.get_segments_file_name()).unwrap();
  assert_eq!(last, first_commit.get_generation());
  for commit in commits.iter().skip(1) {
    let now = generation_from_segments_file_name(commit.get_segments_file_name()).unwrap();
    assert!(now > last, "SegmentInfos commits are out-of-order");
    assert_eq!(now, commit.get_generation());
    last = now;
  }
}

#[derive(Clone)]
pub struct KeepAllDeletionPolicy {
  num_on_init: Arc<AtomicUsize>,
  num_on_commit: Arc<AtomicUsize>,
  dir: Arc<DirEnum>,
}

impl KeepAllDeletionPolicy {
  pub(crate) fn new(dir: Arc<DirEnum>) -> Self {
    Self {
      num_on_init: Arc::new(AtomicUsize::new(0)),
      num_on_commit: Arc::new(AtomicUsize::new(0)),
      dir,
    }
  }

  pub(crate) fn num_on_init(&self) -> usize {
    self.num_on_init.load(Ordering::SeqCst)
  }

  pub(crate) fn num_on_commit(&self) -> usize {
    self.num_on_commit.load(Ordering::SeqCst)
  }
}

impl Display for KeepAllDeletionPolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl<IC> IndexDeletionPolicy<IC> for KeepAllDeletionPolicy
where
  IC: IndexCommit + Clone,
{
  fn on_init(&self, commits: &[IC]) -> Result<()> {
    verify_commit_order(commits);
    self.num_on_init.fetch_add(1, Ordering::SeqCst);
    Ok(())
  }

  fn on_commit(&self, commits: &[IC]) -> Result<()> {
    let last_commit = commits.last().unwrap();
    let r = directory_reader::open(self.dir.clone())?;
    let reader_segment_count = get_context(&r)?.leaves()?.len();
    let last_commit_segment_count = last_commit.get_segment_count();
    assert_eq!(
      reader_segment_count, last_commit_segment_count,
      "lastCommit.segmentCount()={} vs IndexReader.segmentCount={}",
      last_commit_segment_count, reader_segment_count
    );
    r.close()?;
    verify_commit_order(commits);
    self.num_on_commit.fetch_add(1, Ordering::SeqCst);
    Ok(())
  }
}

#[derive(Clone)]
pub struct KeepNoneOnInitDeletionPolicy {
  num_on_init: Arc<AtomicUsize>,
  num_on_commit: Arc<AtomicUsize>,
}

impl KeepNoneOnInitDeletionPolicy {
  pub(crate) fn new() -> Self {
    Self {
      num_on_init: Arc::new(AtomicUsize::new(0)),
      num_on_commit: Arc::new(AtomicUsize::new(0)),
    }
  }

  pub(crate) fn num_on_init(&self) -> usize {
    self.num_on_init.load(Ordering::SeqCst)
  }

  pub(crate) fn num_on_commit(&self) -> usize {
    self.num_on_commit.load(Ordering::SeqCst)
  }
}

impl Display for KeepNoneOnInitDeletionPolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl<IC> IndexDeletionPolicy<IC> for KeepNoneOnInitDeletionPolicy
where
  IC: IndexCommit + Clone,
{
  fn on_init(&self, commits: &[IC]) -> Result<()> {
    verify_commit_order(commits);
    self.num_on_init.fetch_add(1, Ordering::SeqCst);
    for commit in commits {
      commit.delete()?;
      assert!(commit.is_deleted());
    }
    Ok(())
  }

  fn on_commit(&self, commits: &[IC]) -> Result<()> {
    verify_commit_order(commits);
    let size = commits.len();
    for commit in commits.iter().take(size.saturating_sub(1)) {
      commit.delete()?;
    }
    self.num_on_commit.fetch_add(1, Ordering::SeqCst);
    Ok(())
  }
}

#[derive(Clone)]
pub struct KeepLastNDeletionPolicy {
  num_on_init: Arc<AtomicUsize>,
  num_on_commit: Arc<AtomicUsize>,
  num_to_keep: usize,
  num_delete: Arc<AtomicUsize>,
  seen: Arc<Mutex<HashSet<String>>>,
}

impl KeepLastNDeletionPolicy {
  pub(crate) fn new(num_to_keep: usize) -> Self {
    Self {
      num_on_init: Arc::new(AtomicUsize::new(0)),
      num_on_commit: Arc::new(AtomicUsize::new(0)),
      num_to_keep,
      num_delete: Arc::new(AtomicUsize::new(0)),
      seen: Arc::new(Mutex::new(HashSet::new())),
    }
  }

  pub(crate) fn num_on_init(&self) -> usize {
    self.num_on_init.load(Ordering::SeqCst)
  }

  pub(crate) fn num_on_commit(&self) -> usize {
    self.num_on_commit.load(Ordering::SeqCst)
  }

  pub(crate) fn num_delete(&self) -> usize {
    self.num_delete.load(Ordering::SeqCst)
  }

  fn do_deletes<IC>(&self, commits: &[IC], is_commit: bool) -> Result<()>
  where
    IC: IndexCommit,
  {
    if is_commit {
      let file_name = commits.last().unwrap().get_segments_file_name().to_string();
      let mut seen = self.seen.lock().unwrap();
      if seen.contains(&file_name) {
        return Err(LuceneError::illegal_state(format!(
          "onCommit was called twice on the same commit point: {file_name}"
        )));
      }
      seen.insert(file_name);
      self.num_on_commit.fetch_add(1, Ordering::SeqCst);
    }

    let num_to_delete = commits.len().saturating_sub(self.num_to_keep);
    for commit in commits.iter().take(num_to_delete) {
      commit.delete()?;
      self.num_delete.fetch_add(1, Ordering::SeqCst);
    }
    Ok(())
  }
}

impl Display for KeepLastNDeletionPolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl<IC> IndexDeletionPolicy<IC> for KeepLastNDeletionPolicy
where
  IC: IndexCommit + Clone,
{
  fn on_init(&self, commits: &[IC]) -> Result<()> {
    verify_commit_order(commits);
    self.num_on_init.fetch_add(1, Ordering::SeqCst);
    self.do_deletes(commits, false)
  }

  fn on_commit(&self, commits: &[IC]) -> Result<()> {
    verify_commit_order(commits);
    self.do_deletes(commits, true)
  }
}

fn get_commit_time<IC>(commit: &IC) -> Result<i64>
where
  IC: IndexCommit,
{
  Ok(
    commit
      .get_user_data()
      .get("commitTime")
      .ok_or_else(|| LuceneError::illegal_state("missing commitTime"))?
      .parse::<i64>()?,
  )
}

#[derive(Clone)]
pub struct ExpirationTimeDeletionPolicy {
  #[allow(dead_code)]
  dir: Arc<DirEnum>,
  expiration_time_seconds: f64,
  num_delete: Arc<AtomicUsize>,
}

impl ExpirationTimeDeletionPolicy {
  pub(crate) fn new(dir: Arc<DirEnum>, seconds: f64) -> Self {
    Self {
      dir,
      expiration_time_seconds: seconds,
      num_delete: Arc::new(AtomicUsize::new(0)),
    }
  }

  pub(crate) fn num_delete(&self) -> usize {
    self.num_delete.load(Ordering::SeqCst)
  }
}

impl Display for ExpirationTimeDeletionPolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl<IC> IndexDeletionPolicy<IC> for ExpirationTimeDeletionPolicy
where
  IC: IndexCommit + Clone,
{
  fn on_init(&self, commits: &[IC]) -> Result<()> {
    if commits.is_empty() {
      return Ok(());
    }
    verify_commit_order(commits);
    self.on_commit(commits)
  }

  fn on_commit(&self, commits: &[IC]) -> Result<()> {
    verify_commit_order(commits);

    let last_commit = commits.last().unwrap();

    let expire_time = get_commit_time(last_commit)? as f64 / 1000.0 - self.expiration_time_seconds;

    for commit in commits {
      let mod_time = get_commit_time(commit)? as f64 / 1000.0;
      if commit != last_commit && mod_time < expire_time {
        commit.delete()?;
        self.num_delete.fetch_add(1, Ordering::SeqCst);
      }
    }
    Ok(())
  }
}

#[derive(Clone)]
pub struct RollbackDeletionPolicy {
  rollback_point: i32,
}

impl RollbackDeletionPolicy {
  pub(crate) fn new(rollback_point: i32) -> Self {
    Self { rollback_point }
  }
}

impl Display for RollbackDeletionPolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl<IC> IndexDeletionPolicy<IC> for RollbackDeletionPolicy
where
  IC: IndexCommit + Clone,
{
  fn on_init(&self, commits: &[IC]) -> Result<()> {
    for commit in commits {
      let user_data = commit.get_user_data();
      if !user_data.is_empty() {
        let index = user_data.get("index").unwrap();
        let last = index.rsplit('-').next().unwrap().parse::<i32>()?;
        if last > self.rollback_point {
          commit.delete()?;
        }
      }
    }
    Ok(())
  }

  fn on_commit(&self, _commits: &[IC]) -> Result<()> {
    Ok(())
  }
}

#[derive(Clone)]
pub struct DeleteLastCommitPolicy;

impl Display for DeleteLastCommitPolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl<IC> IndexDeletionPolicy<IC> for DeleteLastCommitPolicy
where
  IC: IndexCommit + Clone,
{
  fn on_init(&self, commits: &[IC]) -> Result<()> {
    commits.last().unwrap().delete()
  }

  fn on_commit(&self, _commits: &[IC]) -> Result<()> {
    Ok(())
  }
}

#[derive(Clone)]
pub struct KeepAllTransactionDeletionPolicy;

impl Display for KeepAllTransactionDeletionPolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl<IC> IndexDeletionPolicy<IC> for KeepAllTransactionDeletionPolicy
where
  IC: IndexCommit + Clone,
{
  fn on_init(&self, _commits: &[IC]) -> Result<()> {
    Ok(())
  }

  fn on_commit(&self, _commits: &[IC]) -> Result<()> {
    Ok(())
  }
}
