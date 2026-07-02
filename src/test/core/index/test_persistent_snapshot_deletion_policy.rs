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
use std::io::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::test_snapshot_deletion_policy::{assert_snapshot_exists, prepare_index_and_snapshots};
use crate::core::document::document::Document;
use crate::core::index::directory_reader;
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_deletion_policy::IndexDeletionPolicyEnum;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::{IndexWriterConfig, OpenMode};
use crate::core::index::keep_only_last_commit_deletion_policy::KeepOnlyLastCommitDeletionPolicy;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::persistent_snapshot_deletion_policy::{
  PersistentSnapshotDeletionPolicy, SNAPSHOTS_PREFIX,
};
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::store::mock_directory_wrapper::{Failure, MockDirectoryWrapper};
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_mock_directory, random,
};

#[allow(dead_code)] // for quick search
struct TestPersistentSnapshotDeletionPolicy;

struct FailOnPersist {
  do_fail: bool,
  fail_on_persist: Arc<AtomicBool>,
  thrown: bool,
}

impl FailOnPersist {
  fn new(fail_on_persist: Arc<AtomicBool>) -> Self {
    Self {
      do_fail: true,
      fail_on_persist,
      thrown: false,
    }
  }
}

impl<D> Failure<D> for FailOnPersist
where
  D: Directory,
{
  fn eval(&mut self, _dir: &MockDirectoryWrapper<D>) -> Result<()> {
    if self.do_fail && self.fail_on_persist.load(Ordering::SeqCst) && !self.thrown {
      self.thrown = true;
      return Err(LuceneError::io(Error::other("now fail on purpose")));
    }
    Ok(())
  }

  fn do_fail_mut(&mut self) -> &mut bool {
    &mut self.do_fail
  }
}

fn get_config<D, T>(random: &mut impl rand::Rng, deletion_policy: T) -> Result<IndexWriterConfig<D>>
where
  D: Directory,
  T: Into<IndexDeletionPolicyEnum<D>>,
{
  let mock = MockAnalyzer::new(random);
  let mut conf = new_index_writer_config_with_analyzer(random, mock)?;
  conf.set_index_deletion_policy(deletion_policy);
  Ok(conf)
}

fn get_deletion_policy<D>(dir: Arc<D>) -> Result<PersistentSnapshotDeletionPolicy<D>>
where
  D: Directory,
{
  PersistentSnapshotDeletionPolicy::with_open_mode(
    KeepOnlyLastCommitDeletionPolicy,
    dir,
    OpenMode::Create,
  )
}

fn persistent_policy<D>(writer: &IndexWriter<D>) -> &PersistentSnapshotDeletionPolicy<D>
where
  D: Directory,
{
  match writer.get_config().get_index_deletion_policy() {
    IndexDeletionPolicyEnum::PersistentSnapshot(policy) => policy.as_ref(),
    policy => panic!("expected PersistentSnapshotDeletionPolicy but got {policy}"),
  }
}

#[test]
fn test_existing_snapshots() -> Result<()> {
  let mut random = random();
  let num_snapshots = 3;
  let dir = Arc::new(new_mock_directory(&mut random)?);
  let writer = IndexWriter::new(
    dir.clone(),
    get_config(&mut random, get_deletion_policy(dir.clone())?)?,
  )?;
  let psdp = persistent_policy(&writer);
  assert!(psdp.get_last_save_file().is_none());
  let mut snapshots = prepare_index_and_snapshots(|| psdp.snapshot(), &writer, num_snapshots)?;
  assert!(psdp.get_last_save_file().is_some());
  writer.close()?;

  // Make sure only 1 save file exists:
  let count = dir
    .list_all()?
    .iter()
    .filter(|file| file.starts_with(SNAPSHOTS_PREFIX))
    .count();
  assert_eq!(1, count);

  // Make sure we fsync:
  dir.crash()?;
  dir.clear_crash();

  // Re-initialize and verify snapshots were persisted
  let psdp = PersistentSnapshotDeletionPolicy::with_open_mode(
    KeepOnlyLastCommitDeletionPolicy,
    dir.clone(),
    OpenMode::Append,
  )?;

  let writer = IndexWriter::new(dir.clone(), get_config(&mut random, psdp)?)?;
  let psdp = persistent_policy(&writer);

  assert_eq!(num_snapshots, psdp.get_snapshots().len());
  assert_eq!(num_snapshots as i32, psdp.get_snapshot_count());
  assert_snapshot_exists(
    &dir,
    |generation| psdp.get_index_commit(generation),
    &snapshots,
    num_snapshots,
    false,
  )?;

  writer.add_document(Document::new())?;
  writer.commit()?;
  snapshots.push(psdp.snapshot()?);
  assert_eq!(num_snapshots + 1, psdp.get_snapshots().len());
  assert_eq!(num_snapshots as i32 + 1, psdp.get_snapshot_count());
  assert_snapshot_exists(
    &dir,
    |generation| psdp.get_index_commit(generation),
    &snapshots,
    num_snapshots + 1,
    false,
  )?;

  writer.close()?;
  Ok(())
}

#[test]
fn test_no_snapshot_infos() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  PersistentSnapshotDeletionPolicy::with_open_mode(
    KeepOnlyLastCommitDeletionPolicy,
    dir,
    OpenMode::Create,
  )?;
  Ok(())
}

#[test]
fn test_missing_snapshots() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  assert!(
    PersistentSnapshotDeletionPolicy::with_open_mode(
      KeepOnlyLastCommitDeletionPolicy,
      dir,
      OpenMode::Append,
    )
    .is_err()
  );

  Ok(())
}

#[test]
fn test_exception_during_save() -> Result<()> {
  let mut random = random();
  let dir = Arc::new(new_mock_directory(&mut random)?);
  let fail_on_persist = Arc::new(AtomicBool::new(false));
  dir.fail_on(Box::new(FailOnPersist::new(fail_on_persist.clone())));
  let writer = IndexWriter::new(
    dir.clone(),
    get_config(
      &mut random,
      PersistentSnapshotDeletionPolicy::with_open_mode(
        KeepOnlyLastCommitDeletionPolicy,
        dir.clone(),
        OpenMode::CreateOrAppend,
      )?,
    )?,
  )?;
  writer.add_document(Document::new())?;
  writer.commit()?;

  let psdp = persistent_policy(&writer);
  fail_on_persist.store(true, Ordering::SeqCst);
  let error = match psdp.snapshot() {
    Ok(_) => panic!("snapshot save should fail on purpose"),
    Err(error) => error,
  };
  assert!(
    error.to_string().contains("now fail on purpose"),
    "unexpected error: {error}"
  );
  fail_on_persist.store(false, Ordering::SeqCst);
  assert_eq!(0, psdp.get_snapshot_count());
  writer.close()?;
  assert_eq!(1, directory_reader::list_commits(dir.clone())?.len());
  Ok(())
}

#[test]
fn test_snapshot_release() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(
    dir.clone(),
    get_config(&mut random, get_deletion_policy(dir.clone())?)?,
  )?;
  let psdp = persistent_policy(&writer);
  let snapshots = prepare_index_and_snapshots(|| psdp.snapshot(), &writer, 1)?;
  writer.close()?;

  psdp.release(&snapshots[0])?;

  let psdp = PersistentSnapshotDeletionPolicy::with_open_mode(
    KeepOnlyLastCommitDeletionPolicy,
    dir,
    OpenMode::Append,
  )?;
  assert_eq!(0, psdp.get_snapshot_count(), "Should have no snapshots !");
  Ok(())
}

#[test]
fn test_snapshot_release_by_generation() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(
    dir.clone(),
    get_config(&mut random, get_deletion_policy(dir.clone())?)?,
  )?;
  let psdp = persistent_policy(&writer);
  let snapshots = prepare_index_and_snapshots(|| psdp.snapshot(), &writer, 1)?;
  writer.close()?;

  psdp.release_gen(snapshots[0].get_generation())?;

  let psdp = PersistentSnapshotDeletionPolicy::with_open_mode(
    KeepOnlyLastCommitDeletionPolicy,
    dir,
    OpenMode::Append,
  )?;
  assert_eq!(0, psdp.get_snapshot_count(), "Should have no snapshots !");
  Ok(())
}
