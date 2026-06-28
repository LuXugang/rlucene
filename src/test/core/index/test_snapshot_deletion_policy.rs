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
use crate::core::document::document::Document;
use crate::core::document::field_type::FieldType;
use crate::core::index::directory_reader;
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_file_deleter::CommitPoint;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexCommitWrapper, IndexWriter};
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::keep_only_last_commit_deletion_policy::KeepOnlyLastCommitDeletionPolicy;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::snapshot_deletion_policy::SnapshotDeletionPolicy;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::data_input::DataInput;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::store::io_context::IOContext;
use crate::core::util::close::Closeable;
use crate::core::util::dummy::dummy_comparator::DummyComparator;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::test_index_writer::assert_no_unreferenced_files;
use crate::test::core::util::lucene_test_case::{
  is_night_mode, new_directory_shared, new_field, new_index_writer_config_with_analyzer, random,
  slow_file_exists,
};
use std::collections::HashMap;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;

#[allow(dead_code)] // for quick search
struct TestSnapshotDeletionPolicy;

fn get_config(
  random: &mut impl rand::Rng,
  dp: Option<SnapshotDeletionPolicy<DirEnum>>,
) -> Result<crate::core::index::index_writer_config::IndexWriterConfig<DirEnum>> {
  let mock = MockAnalyzer::new(random);
  let mut conf = new_index_writer_config_with_analyzer(random, mock)?;
  if let Some(dp) = dp {
    conf.set_index_deletion_policy(dp);
  }
  Ok(conf)
}

pub(crate) fn check_snapshot_exists(dir: &impl Directory, c: &impl IndexCommit) -> Result<()> {
  let seg_file_name = c.get_segments_file_name();
  assert!(
    slow_file_exists(dir, seg_file_name)?,
    "segments file not found in directory: {seg_file_name}"
  );
  Ok(())
}

pub(crate) fn check_max_doc<D>(commit: &Arc<CommitPoint<D>>, expected_max_doc: i32) -> Result<()>
where
  D: Directory + 'static,
{
  let reader = directory_reader::open_from_commit::<_, DummyComparator, _>(commit)?;
  assert_eq!(expected_max_doc, reader.max_doc()?);
  reader.close()
}

pub(crate) fn prepare_index_and_snapshots<D, F>(
  mut snapshot: F,
  writer: &IndexWriter<D>,
  num_snapshots: usize,
) -> Result<Vec<Arc<CommitPoint<D>>>>
where
  D: Directory + 'static,
  F: FnMut() -> Result<Arc<CommitPoint<D>>>,
{
  let mut snapshots = Vec::new();
  for _ in 0..num_snapshots {
    // create dummy document to trigger commit.
    writer.add_document(Document::new())?;
    writer.commit()?;
    snapshots.push(snapshot()?);
  }
  Ok(snapshots)
}

fn get_deletion_policy() -> SnapshotDeletionPolicy<DirEnum> {
  SnapshotDeletionPolicy::new(KeepOnlyLastCommitDeletionPolicy)
}

pub(crate) fn assert_snapshot_exists<D, F>(
  dir: &Arc<D>,
  get_index_commit: F,
  snapshots: &[Arc<CommitPoint<D>>],
  num_snapshots: usize,
  check_index_commit_same: bool,
) -> Result<()>
where
  D: Directory + 'static,
  F: Fn(i64) -> Option<Arc<CommitPoint<D>>>,
{
  for (i, snapshot) in snapshots.iter().take(num_snapshots).enumerate() {
    check_max_doc(snapshot, i as i32 + 1)?;
    check_snapshot_exists(dir.as_ref(), snapshot)?;
    let index_commit =
      get_index_commit(snapshot.get_generation()).expect("snapshot generation should be held");
    if check_index_commit_same {
      assert!(Arc::ptr_eq(snapshot, &index_commit));
    } else {
      assert_eq!(snapshot.get_generation(), index_commit.get_generation());
    }
  }
  Ok(())
}

/// Example showing how to use the [`SnapshotDeletionPolicy`] to take a backup. This method does not
/// really do a backup; instead, it reads every byte of every file just to test that the files indeed
/// exist and are readable even while the index is changing.
fn backup_index(
  dir: &Arc<DirEnum>,
  dp: &SnapshotDeletionPolicy<DirEnum>,
  buffer: &mut [u8],
) -> Result<()> {
  // To backup an index we first take a snapshot:
  let snapshot = dp.snapshot()?;
  let result = copy_files(dir.as_ref(), snapshot.as_ref(), buffer);
  let release_result = dp.release(&snapshot);
  // Make sure to release the snapshot, otherwise these
  // files will never be deleted during this IndexWriter
  // session:
  release_result?;
  result
}

fn copy_files(dir: &impl Directory, cp: &impl IndexCommit, buffer: &mut [u8]) -> Result<()> {
  // While we hold the snapshot, and nomatter how long
  // we take to do the backup, the IndexWriter will
  // never delete the files in the snapshot:
  for file in cp.get_file_names()? {
    // NOTE: in a real backup you would not use
    // readFile; you would need to use something else
    // that copies the file to a backup location.  This
    // could even be a spawned shell process (eg "tar",
    // "zip") that takes the list of files and builds a
    // backup.
    read_file(dir, file, buffer)?;
  }
  Ok(())
}

fn read_file(dir: &impl Directory, name: &str, buffer: &mut [u8]) -> Result<()> {
  let mut input = dir.open_input(name, &IOContext::read_once_io_context()?)?;
  let result = (|| {
    let mut bytes_left = dir.file_length(name)?;
    while bytes_left > 0 {
      let num_to_read = usize::min(bytes_left, buffer.len());
      input.read_bytes_with_buffer(buffer, 0, num_to_read, false)?;
      bytes_left -= num_to_read;
    }
    // Don't do this in your real backups!  This is just
    // to force a backup to take a somewhat long time, to
    // make sure we are exercising the fact that the
    // IndexWriter should not delete this file even when I
    // take my time reading it.
    thread::sleep(Duration::from_millis(1));
    Ok(())
  })();
  let close_result = input.close();
  close_result?;
  result
}

#[test]
fn test_snapshot_deletion_policy() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  run_test(&mut random, dir)
}

fn run_test(rng: &mut impl rand::Rng, dir: Arc<DirEnum>) -> Result<()> {
  let max_iterations = if is_night_mode() { 100 } else { 10 };

  let dp = get_deletion_policy();
  let mock = MockAnalyzer::new(&mut *rng);
  let mut config = new_index_writer_config_with_analyzer(rng, mock)?;
  config
    .set_index_deletion_policy(dp.clone())
    .set_max_buffered_docs(2);
  let writer = Arc::new(IndexWriter::new(dir.clone(), config)?);

  // Verify we catch misuse:
  assert!(
    dp.snapshot().is_err(),
    "snapshot should not succeed before commit"
  );
  writer.commit()?;

  let index_writer = writer.clone();
  let field_to_type = Arc::new(Mutex::new(HashMap::new()));
  let index_thread_field_to_type = field_to_type.clone();
  let handle = thread::spawn(move || -> Result<()> {
    let mut random = random();
    let mut doc = Document::new();
    let mut custom_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
    custom_type.set_store_term_vectors(true)?;
    custom_type.set_store_term_vector_positions(true)?;
    custom_type.set_store_term_vector_offsets(true)?;
    {
      let mut field_to_type = index_thread_field_to_type
        .lock()
        .expect("field_to_type mutex should not poison");
      doc.add(new_field(
        &mut random,
        "content",
        "aaa",
        &custom_type,
        &mut field_to_type,
      )?);
    }
    let mut iterations = 0;
    loop {
      for i in 0..27 {
        index_writer.add_document(doc.clone())?;
        if i % 2 == 0 {
          index_writer.commit()?;
        }
      }
      thread::sleep(Duration::from_millis(1));
      iterations += 1;
      if iterations >= max_iterations {
        break;
      }
    }
    Ok(())
  });

  // While the above indexing thread is running, take many
  // backups:
  let mut buffer = vec![0; 4096];
  loop {
    backup_index(&dir, &dp, &mut buffer)?;
    thread::sleep(Duration::from_millis(20));
    if handle.is_finished() {
      break;
    }
  }
  handle.join().expect("indexing thread should not panic")?;

  // Add one more document to force writer to commit a
  // final segment, so deletion policy has a chance to
  // delete again:
  let mut doc = Document::new();
  let mut custom_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
  custom_type.set_store_term_vectors(true)?;
  custom_type.set_store_term_vector_positions(true)?;
  custom_type.set_store_term_vector_offsets(true)?;
  {
    let mut field_to_type = field_to_type
      .lock()
      .expect("field_to_type mutex should not poison");
    doc.add(new_field(
      rng,
      "content",
      "aaa",
      &custom_type,
      &mut field_to_type,
    )?);
  }
  writer.add_document(doc)?;

  // Make sure we don't have any leftover files in the
  // directory:
  writer.close()?;

  assert_no_unreferenced_files(
    dir.clone(),
    "some files were not deleted but should have been",
  )?;
  Ok(())
}

#[test]
fn test_basic_snapshots() -> Result<()> {
  let mut random = random();
  let num_snapshots = 3;

  // Create 3 snapshots: snapshot0, snapshot1, snapshot2
  let dir = new_directory_shared(&mut random)?;
  let sdp = get_deletion_policy();
  let writer = IndexWriter::new(dir.clone(), get_config(&mut random, Some(sdp.clone()))?)?;
  let snapshots = prepare_index_and_snapshots(|| sdp.snapshot(), &writer, num_snapshots)?;
  writer.close()?;

  assert_eq!(num_snapshots, sdp.get_snapshots().len());
  assert_eq!(num_snapshots as i32, sdp.get_snapshot_count());
  assert_snapshot_exists(
    &dir,
    |generation| sdp.get_index_commit(generation),
    &snapshots,
    num_snapshots,
    true,
  )?;

  // open a reader on a snapshot - should succeed.
  directory_reader::open_from_commit::<_, DummyComparator, _>(&snapshots[0])?.close()?;

  // open a new IndexWriter w/ no snapshots to keep and assert that all snapshots are gone.
  let writer = IndexWriter::new(dir.clone(), get_config(&mut random, None)?)?;
  writer.delete_unused_files()?;
  writer.close()?;
  assert_eq!(
    1,
    directory_reader::list_commits(dir.clone())?.len(),
    "no snapshots should exist"
  );
  Ok(())
}

#[test]
fn test_multi_threaded_snapshotting() -> Result<()> {
  const NUM_THREADS: usize = 10;

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let sdp = get_deletion_policy();
  let writer = Arc::new(IndexWriter::new(
    dir.clone(),
    get_config(&mut random, Some(sdp.clone()))?,
  )?);

  let snapshots = Arc::new(Mutex::new(vec![None; NUM_THREADS]));
  let starting_gun = Arc::new(Barrier::new(1 + NUM_THREADS));
  let mut handles = Vec::new();
  for i in 0..NUM_THREADS {
    let sdp = sdp.clone();
    let writer = writer.clone();
    let snapshots = snapshots.clone();
    let starting_gun = starting_gun.clone();
    handles.push(thread::spawn(move || -> Result<()> {
      starting_gun.wait();
      writer.add_document(Document::new())?;
      writer.commit()?;
      let snapshot = sdp.snapshot()?;
      snapshots.lock().expect("snapshots mutex should not poison")[i] = Some(snapshot);
      Ok(())
    }));
  }

  starting_gun.wait();
  for handle in handles {
    handle.join().expect("snapshot thread should not panic")?;
  }

  // Do one last commit, so that after we release all snapshots, we stay w/ one
  // commit
  writer.add_document(Document::new())?;
  writer.commit()?;

  let snapshots = snapshots.lock().expect("snapshots mutex should not poison");
  for snapshot in snapshots.iter() {
    let snapshot = snapshot
      .as_ref()
      .expect("snapshotting thread should record a snapshot");
    sdp.release(snapshot)?;
    writer.delete_unused_files()?;
  }

  assert_eq!(1, directory_reader::list_commits(dir.clone())?.len());
  writer.close()?;
  Ok(())
}

#[test]
fn test_rollback_to_old_snapshot() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let sdp = get_deletion_policy();
  let writer = IndexWriter::new(dir.clone(), get_config(&mut random, Some(sdp.clone()))?)?;
  let snapshots = prepare_index_and_snapshots(|| sdp.snapshot(), &writer, 2)?;
  writer.close()?;

  // now open the writer on "snapshot0" - make sure it succeeds
  let mut config = get_config(&mut random, Some(sdp.clone()))?;
  config.set_open_mode(OpenMode::CreateOrAppend);
  let index_commit =
    IndexCommitWrapper::<Arc<CommitPoint<DirEnum>>, DummyComparator, DirEnum>::new(
      Some(snapshots[0].clone()),
      None,
      None,
    )?;
  let writer = IndexWriter::with_index_commit(dir.clone(), config, index_commit)?;
  // this does the actual rollback.
  writer.commit()?;
  writer.delete_unused_files()?;

  // but 'snapshot1' files will still exist, since it was snapshotted.
  assert_snapshot_exists(
    &dir,
    |generation| sdp.get_index_commit(generation),
    &snapshots,
    1,
    false,
  )?;
  check_snapshot_exists(dir.as_ref(), snapshots[1].as_ref())?;

  writer.close()?;
  Ok(())
}

#[test]
fn test_release_snapshot() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let sdp = get_deletion_policy();
  let writer = IndexWriter::new(dir.clone(), get_config(&mut random, Some(sdp.clone()))?)?;
  let snapshots = prepare_index_and_snapshots(|| sdp.snapshot(), &writer, 1)?;

  // Create another commit - we must do that, because otherwise the "snapshot"
  // files will still remain in the index, since it's the last commit.
  writer.add_document(Document::new())?;
  writer.commit()?;

  // Release
  let seg_file_name = snapshots[0].get_segments_file_name().to_string();
  sdp.release(&snapshots[0])?;
  writer.delete_unused_files()?;
  writer.close()?;
  assert!(
    !slow_file_exists(dir.as_ref(), &seg_file_name)?,
    "segments file should not be found in directory: {seg_file_name}"
  );
  Ok(())
}

#[test]
fn test_snapshot_last_commit_twice() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let sdp = get_deletion_policy();
  let writer = IndexWriter::new(dir.clone(), get_config(&mut random, Some(sdp.clone()))?)?;
  writer.add_document(Document::new())?;
  writer.commit()?;

  let s1 = sdp.snapshot()?;
  let s2 = sdp.snapshot()?;
  assert!(Arc::ptr_eq(&s1, &s2)); // should be the same instance

  // create another commit
  writer.add_document(Document::new())?;
  writer.commit()?;

  // release "s1" should not delete "s2"
  sdp.release(&s1)?;
  writer.delete_unused_files()?;
  check_snapshot_exists(dir.as_ref(), s2.as_ref())?;

  writer.close()?;
  Ok(())
}

#[test]
fn test_missing_commits() -> Result<()> {
  // Tests the behavior of SDP when commits that are given at ctor are missing
  // on onInit().
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let sdp = get_deletion_policy();
  let writer = IndexWriter::new(dir.clone(), get_config(&mut random, Some(sdp.clone()))?)?;
  writer.add_document(Document::new())?;
  writer.commit()?;
  let s1 = sdp.snapshot()?;

  // create another commit, not snapshotted.
  writer.add_document(Document::new())?;
  writer.close()?;

  // open a new writer w/ KeepOnlyLastCommit policy, so it will delete "s1"
  // commit.
  IndexWriter::new(dir.clone(), get_config(&mut random, None)?)?.close()?;

  assert!(
    !slow_file_exists(dir.as_ref(), s1.get_segments_file_name())?,
    "snapshotted commit should not exist"
  );
  Ok(())
}
