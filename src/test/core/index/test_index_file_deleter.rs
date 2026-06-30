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
use crate::core::document::field::Store;
use crate::core::document::field_type::FieldType;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_io_context,
  new_log_merge_policy_with_merge_factor_cfs, new_mock_directory, new_string_field, new_text_field,
  random, slow_file_exists,
};
use rand::{Rng, RngExt};
use std::collections::HashMap;

use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::{IndexWriterConfig, OpenMode};
use crate::core::index::keep_only_last_commit_deletion_policy::KeepOnlyLastCommitDeletionPolicy;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::snapshot_deletion_policy::SnapshotDeletionPolicy;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::index::{CODEC_FILE_PATTERN, IndexFileNames};
use crate::core::store::directory::Directory;
use crate::core::store::{DataInput, DataOutput, IndexInput};
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::info_stream::{InfoStreamMT, get_default_info_stream};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::store::mock_directory_wrapper::{Failure, MockDirectoryWrapper};
#[allow(dead_code)] // for quick search
struct TestIndexFileDeleter;

use crate::core::index::index_file_deleter::inflate_gens;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::io::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn test_delete_left_over_files() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut merge_policy = new_log_merge_policy_with_merge_factor_cfs(&mut random, true, 10)?;

  // This test expects all of its segments to be in CFS
  merge_policy.get_base_mut().set_no_cfs_ratio(1.0)?;
  merge_policy
    .get_base_mut()
    .set_max_cfs_segment_size_mb(f64::INFINITY)?;

  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  config
    .set_max_buffered_docs(10)
    .set_merge_policy(merge_policy)
    .set_use_compound_file(true);
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut field_types = HashMap::new();
  let mut i = 0;
  while i < 35 {
    add_doc(&mut random, &writer, i, &mut field_types)?;
    i += 1;
  }
  writer
    .get_config_mut()
    .get_merge_policy_mut()
    .get_base_mut()
    .set_no_cfs_ratio(0.0)?;
  writer.get_config_mut().set_use_compound_file(false);
  while i < 45 {
    add_doc(&mut random, &writer, i, &mut field_types)?;
    i += 1;
  }
  writer.close()?;
  drop(writer);

  // Delete one doc so we get a .del file:
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  config
    .set_merge_policy(NoMergePolicy::default())
    .set_use_compound_file(true);
  let writer = IndexWriter::new(dir.clone(), config)?;
  let search_term = Term::from_text("id", "7");
  writer.delete_documents_with_terms(vec![search_term])?;
  writer.close()?;
  drop(writer);

  // read in index to try to not depend on codec-specific filenames so much
  let sis = SegmentInfos::read_latest_commit(dir.clone())?;
  let _si0 = sis.info(0).unwrap().info.clone();
  let _si1 = sis.info(1).unwrap().info.clone();
  let _si3 = sis.info(3).unwrap().info.clone();

  // Now, artificially create an extra .del file & extra
  // .s0 file:
  let mut files = dir.list_all()?;

  // TODO: fix this test better
  let ext = ".liv";

  // Create a bogus separate del file for a
  // segment that already has a separate del file:
  copy_file(
    &mut random,
    dir.as_ref(),
    &format!("_0_1{}", ext),
    &format!("_0_2{}", ext),
  )?;

  // Create a bogus separate del file for a
  // segment that does not yet have a separate del file:
  copy_file(
    &mut random,
    dir.as_ref(),
    &format!("_0_1{}", ext),
    &format!("_1_1{}", ext),
  )?;

  // Create a bogus separate del file for a
  // non-existent segment:
  copy_file(
    &mut random,
    dir.as_ref(),
    &format!("_0_1{}", ext),
    &format!("_188_1{}", ext),
  )?;

  // TODO: SimpleTextCodec is not implemented.
  let cfs_files0 = ["_0.cfs", "_0.cfe"];

  // Create a bogus segment file:
  copy_file(&mut random, dir.as_ref(), cfs_files0[0], "_188.cfs")?;

  // Create a bogus fnm file when the CFS already exists:
  copy_file(&mut random, dir.as_ref(), cfs_files0[0], "_0.fnm")?;

  // Create a bogus cfs file shadowing a non-cfs segment:

  // TODO: assert is bogus (relies upon codec-specific filenames)
  assert!(slow_file_exists(dir.as_ref(), "_3.fdt")? || slow_file_exists(dir.as_ref(), "_3.fld")?);

  // TODO: SimpleTextCodec is not implemented.
  let cfs_files3 = ["_3.cfs", "_3.cfe"];
  for f in cfs_files3 {
    assert!(!slow_file_exists(dir.as_ref(), f)?);
  }

  // TODO: SimpleTextCodec is not implemented.
  let cfs_files1 = ["_1.cfs", "_1.cfe"];
  copy_file(&mut random, dir.as_ref(), cfs_files1[0], "_3.cfs")?;

  let files_pre = dir.list_all()?;

  // Open & close a writer: it should delete the above files and nothing more:
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  config.set_open_mode(OpenMode::Append);
  let writer = IndexWriter::new(dir.clone(), config)?;
  writer.close()?;
  drop(writer);

  let mut files2 = dir.list_all()?;

  files.sort();
  files2.sort();

  let dif = diff_files(&files, &files2);

  if files != files2 {
    panic!(
      "IndexFileDeleter failed to delete unreferenced extra files: should have deleted {} files but only deleted {}; expected files:\n    {}\n  actual files:\n    {}\ndiff: {:?}",
      files_pre.len() - files.len(),
      files_pre.len() - files2.len(),
      to_string(&files),
      to_string(&files2),
      dif
    );
  }

  Ok(())
}

fn diff_files(files1: &[String], files2: &[String]) -> HashSet<String> {
  let set1: HashSet<String> = files1.iter().cloned().collect();
  let set2: HashSet<String> = files2.iter().cloned().collect();
  let mut extra = HashSet::new();

  for item in &set1 {
    if !set2.contains(item) {
      extra.insert(item.clone());
    }
  }
  for item in &set2 {
    if !set1.contains(item) {
      extra.insert(item.clone());
    }
  }

  extra
}

fn to_string(list: &[String]) -> String {
  let mut s = String::new();
  for (i, item) in list.iter().enumerate() {
    if i > 0 {
      s.push_str("\n    ");
    }
    s.push_str(item);
  }
  s
}
fn copy_file<D, R>(random: &mut R, dir: &D, src: &str, dest: &str) -> Result<()>
where
  D: Directory,
  R: Rng + ?Sized,
{
  let mut input = dir.open_input(src, &new_io_context(random)?)?;
  let mut output = dir.create_output(dest, &new_io_context(random)?)?;
  let mut buffer = [0u8; 1024];
  let mut remainder = input.length()? as i64;

  while remainder > 0 {
    let len = usize::min(buffer.len(), remainder as usize);
    input.read_bytes(&mut buffer, 0, len)?;
    output.write_bytes_with_len(&buffer, len)?;
    remainder -= len as i64;
  }
  output.close()?;

  Ok(())
}
fn add_doc<D, R>(
  random: &mut R,
  writer: &IndexWriter<D>,
  id: i32,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  R: Rng + ?Sized,
  D: Directory + 'static,
{
  let mut doc = Document::new();

  doc.add(new_text_field(
    random,
    "content",
    "aaa",
    Store::No,
    field_types,
  )?);

  doc.add(new_string_field(
    random,
    "id",
    id.to_string(),
    Store::No,
    field_types,
  )?);
  writer.add_document(doc)?;
  Ok(())
}
#[test]
fn test_virus_scanner_doesnt_corrupt_index() -> Result<()> {
  // TODO addVirusChecker is not implemented.
  Ok(())
}

#[test]
fn test_no_segments_dot_gen_inflation() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  // empty commit
  let writer = IndexWriter::new(dir.clone(), IndexWriterConfig::new()?)?;
  writer.close()?;
  drop(writer);

  let mut sis = SegmentInfos::read_latest_commit(dir.clone())?;
  assert_eq!(1, sis.get_generation());

  // no inflation
  inflate_gens_test(&mut sis, dir.list_all()?, &get_default_info_stream())?;
  assert_eq!(1, sis.get_generation());

  Ok(())
}

#[test]
fn test_segments_inflation() -> Result<()> {
  let mut random = random();
  let dir = new_mock_directory(&mut random)?;
  dir.set_check_index_on_close(false); // TODO: allow falling back more than one commit

  // empty commit
  let writer = IndexWriter::new(Arc::new(dir.clone()), IndexWriterConfig::new()?)?;
  writer.close()?;
  drop(writer);

  let mut sis = SegmentInfos::read_latest_commit(Arc::new(dir.clone()))?;
  assert_eq!(1, sis.get_generation());

  // add trash commit
  let mut output = dir.create_output(
    &format!("{}{}", IndexFileNames::SEGMENTS, "_2"),
    &new_io_context(&mut random)?,
  )?;
  output.close()?;

  // ensure inflation
  inflate_gens_test(&mut sis, dir.list_all()?, &get_default_info_stream())?;
  assert_eq!(2, sis.get_generation());

  // add another trash commit
  let mut output = dir.create_output(
    &format!("{}{}", IndexFileNames::SEGMENTS, "_4"),
    &new_io_context(&mut random)?,
  )?;
  output.close()?;
  inflate_gens_test(&mut sis, dir.list_all()?, &get_default_info_stream())?;
  assert_eq!(4, sis.get_generation());

  Ok(())
}

#[test]
fn test_segment_name_inflation() -> Result<()> {
  let mut random = random();
  let dir = new_mock_directory(&mut random)?;

  // empty commit
  let writer = IndexWriter::new(Arc::new(dir.clone()), IndexWriterConfig::new()?)?;
  writer.close()?;
  drop(writer);

  let mut sis = SegmentInfos::read_latest_commit(Arc::new(dir.clone()))?;
  assert_eq!(0, sis.counter);

  // no inflation
  inflate_gens_test(&mut sis, dir.list_all()?, &get_default_info_stream())?;
  assert_eq!(0, sis.counter);

  // add trash per-segment file
  let mut output = dir.create_output(
    &IndexFileNames::segment_file_name("_0", "", "foo"),
    &new_io_context(&mut random)?,
  )?;
  output.close()?;

  // ensure inflation
  inflate_gens_test(&mut sis, dir.list_all()?, &get_default_info_stream())?;
  assert_eq!(1, sis.counter);

  // add trash per-segment file
  let mut output = dir.create_output(
    &IndexFileNames::segment_file_name("_3", "", "foo"),
    &new_io_context(&mut random)?,
  )?;
  output.close()?;
  inflate_gens_test(&mut sis, dir.list_all()?, &get_default_info_stream())?;
  assert_eq!(4, sis.counter);

  // ensure we write _4 segment next
  let writer = IndexWriter::new(Arc::new(dir.clone()), IndexWriterConfig::new()?)?;
  writer.add_document(Document::new())?;
  writer.commit()?;
  writer.close()?;
  drop(writer);
  sis = SegmentInfos::read_latest_commit(Arc::new(dir.clone()))?;
  assert_eq!("_4", sis.info(0).unwrap().info.name);
  assert_eq!(5, sis.counter);

  Ok(())
}

#[test]
fn test_generation_inflation() -> Result<()> {
  let mut random = random();
  let dir = new_mock_directory(&mut random)?;

  // initial commit
  let writer = IndexWriter::new(Arc::new(dir.clone()), IndexWriterConfig::new()?)?;
  writer.add_document(Document::new())?;
  writer.commit()?;
  writer.close()?;
  drop(writer);

  // no deletes: start at 1
  let mut sis = SegmentInfos::read_latest_commit(Arc::new(dir.clone()))?;
  assert_eq!(1, sis.info(0).unwrap().get_next_del_gen());

  // no inflation
  inflate_gens_test(&mut sis, dir.list_all()?, &get_default_info_stream())?;
  assert_eq!(1, sis.info(0).unwrap().get_next_del_gen());

  // add trash per-segment deletes file
  let mut output = dir.create_output(
    &IndexFileNames::file_name_from_generation("_0", "del", 2).unwrap(),
    &new_io_context(&mut random)?,
  )?;
  output.close()?;

  // ensure inflation
  inflate_gens_test(&mut sis, dir.list_all()?, &get_default_info_stream())?;
  assert_eq!(3, sis.info(0).unwrap().get_next_del_gen());

  Ok(())
}

#[test]
fn test_trashy_file() -> Result<()> {
  let mut random = random();
  let dir = new_mock_directory(&mut random)?;
  dir.set_check_index_on_close(false); // TODO: maybe handle such trash better elsewhere...

  // empty commit
  let writer = IndexWriter::new(Arc::new(dir.clone()), IndexWriterConfig::new()?)?;
  writer.close()?;
  drop(writer);

  let mut sis = SegmentInfos::read_latest_commit(Arc::new(dir.clone()))?;
  assert_eq!(1, sis.get_generation());

  // add trash file
  let mut output = dir.create_output(
    &format!("{}{}", IndexFileNames::SEGMENTS, "_"),
    &new_io_context(&mut random)?,
  )?;
  output.close()?;

  // no inflation
  inflate_gens_test(&mut sis, dir.list_all()?, &get_default_info_stream())?;
  assert_eq!(1, sis.get_generation());

  Ok(())
}

#[test]
fn test_trashy_gen_file() -> Result<()> {
  let mut random = random();
  let dir = new_mock_directory(&mut random)?;

  // initial commit
  let writer = IndexWriter::new(Arc::new(dir.clone()), IndexWriterConfig::new()?)?;
  writer.add_document(Document::new())?;
  writer.commit()?;
  writer.close()?;
  drop(writer);

  // no deletes: start at 1
  let mut sis = SegmentInfos::read_latest_commit(Arc::new(dir.clone()))?;
  assert_eq!(1, sis.info(0).unwrap().get_next_del_gen());

  // add trash file
  let mut output = dir.create_output("_1_A", &new_io_context(&mut random)?)?;
  output.close()?;

  // no inflation
  inflate_gens_test(&mut sis, dir.list_all()?, &get_default_info_stream())?;
  assert_eq!(1, sis.info(0).unwrap().get_next_del_gen());

  Ok(())
}

fn inflate_gens_test<D>(
  sis: &mut SegmentInfos<D>,
  files: Vec<String>,
  stream: &InfoStreamMT,
) -> Result<()>
where
  D: Directory,
{
  let mut filtered = Vec::new();
  for file in files {
    if CODEC_FILE_PATTERN.is_match(&file)
      || file.starts_with(IndexFileNames::SEGMENTS)
      || file.starts_with(IndexFileNames::PENDING_SEGMENTS)
    {
      filtered.push(file);
    }
  }
  inflate_gens(sis, filtered.iter(), stream)
}

#[test]
fn test_exc_in_dec_ref() -> Result<()> {
  // TODO IMPORTANT ConcurrentMergeScheduler未实现
  Ok(())
}

#[test]
fn test_exc_in_delete_file() -> Result<()> {
  // TODO IMPORTANT callStackContains未实现
  Ok(())
}

#[test]
fn test_throw_exception_while_delete_commits() -> Result<()> {
  let mut random = random();
  let dir = new_mock_directory(&mut random)?;
  let fail_on_delete_commits = Arc::new(AtomicBool::new(false));
  dir.fail_on(Box::new(FailOnDeleteCommits::new(
    fail_on_delete_commits.clone(),
  )));

  let snapshot_deletion_policy = SnapshotDeletionPolicy::new(KeepOnlyLastCommitDeletionPolicy);
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  config.set_index_deletion_policy(snapshot_deletion_policy.clone());

  let writer = IndexWriter::new(Arc::new(dir.clone()), config)?;
  writer.add_document(Document::new())?;
  writer.commit()?;

  let snapshot_commit = snapshot_deletion_policy.snapshot()?;
  let commits = random.random_range(1..=3);
  for _ in 0..commits {
    writer.add_document(Document::new())?;
    writer.commit()?;
  }
  snapshot_deletion_policy.release(&snapshot_commit)?;
  fail_on_delete_commits.store(true, Ordering::SeqCst);
  if let Err(error) = writer.delete_unused_files() {
    match error {
      LuceneError::Io { source, .. } | LuceneError::IoWithPath { source, .. } => {
        assert!(
          source
            .get_ref()
            .is_some_and(|source| source.is::<FakeDeleteCommitsIOException>()),
          "expected FakeDeleteCommitsIOException, got {source}"
        );
      },
      other => return Err(other),
    }
  }
  fail_on_delete_commits.store(false, Ordering::SeqCst);
  for _ in 0..commits {
    writer.add_document(Document::new())?;
    writer.commit()?;
  }
  writer.close()?;
  Ok(())
}
#[derive(Debug)]
struct FakeDeleteCommitsIOException;

impl Display for FakeDeleteCommitsIOException {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "fake delete commits IO exception")
  }
}

impl std::error::Error for FakeDeleteCommitsIOException {}

struct FailOnDeleteCommits {
  do_fail: bool,
  fail_on_delete_commits: Arc<AtomicBool>,
  thrown: bool,
}

impl FailOnDeleteCommits {
  fn new(fail_on_delete_commits: Arc<AtomicBool>) -> Self {
    Self {
      do_fail: true,
      fail_on_delete_commits,
      thrown: false,
    }
  }
}

impl<D> Failure<D> for FailOnDeleteCommits
where
  D: Directory,
{
  fn eval(&mut self, _dir: &MockDirectoryWrapper<D>) -> Result<()> {
    if self.do_fail && self.fail_on_delete_commits.load(Ordering::SeqCst) && !self.thrown {
      self.thrown = true;
      return Err(LuceneError::io(Error::other(FakeDeleteCommitsIOException)));
    }
    Ok(())
  }

  fn do_fail_mut(&mut self) -> &mut bool {
    &mut self.do_fail
  }
}
