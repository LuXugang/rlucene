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
use crate::core::index::concurrent_merge_scheduler::ConcurrentMergeScheduler;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_scheduler::MergeSchedulerEnum;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::{Directory, MockDirWrapper};
use crate::core::store::no_lock_factory::NoLockFactory;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  new_index_writer_config_with_analyzer, new_mock_directory_with_lock_factory, new_text_field,
  random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::prelude::StdRng;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;

type CrashDirectory = MockDirWrapper;
type CrashIndexWriter = IndexWriter<CrashDirectory>;

#[allow(dead_code)] // for quick search
struct TestCrash;

fn init_index(
  random: &mut StdRng,
  initial_commit: bool,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<Arc<CrashIndexWriter>> {
  let dir = Arc::new(new_mock_directory_with_lock_factory(random, NoLockFactory)?);
  init_index_with_directory(random, dir, initial_commit, true, field_to_type)
}

fn init_index_with_directory(
  random: &mut StdRng,
  dir: Arc<CrashDirectory>,
  initial_commit: bool,
  commit_on_close: bool,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<Arc<CrashIndexWriter>> {
  let analyzer = MockAnalyzer::new(random);
  let mut config = new_index_writer_config_with_analyzer(random, analyzer)?;
  config.set_max_buffered_docs(10);
  config.set_merge_scheduler(ConcurrentMergeScheduler::new());
  config.set_commit_on_close(commit_on_close);
  let writer = IndexWriter::new(dir, config)?;
  match writer.get_config().get_merge_scheduler() {
    MergeSchedulerEnum::Concurrent(cms) => cms.set_suppress_exceptions(),
    _ => unreachable!("the test configures ConcurrentMergeScheduler"),
  }
  if initial_commit {
    writer.commit()?;
  }

  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "content",
    "aaa",
    Store::No,
    field_to_type,
  )?);
  doc.add(new_text_field(random, "id", "0", Store::No, field_to_type)?);
  for _ in 0..157 {
    writer.add_document(doc.clone())?;
  }

  Ok(writer)
}

fn crash(writer: &CrashIndexWriter) -> Result<()> {
  let dir = writer.get_directory();
  TestUtil::sync_concurrent_merges(writer)?;
  dir.crash()?;
  TestUtil::sync_concurrent_merges(writer)?;
  dir.clear_crash();
  Ok(())
}

#[test]
fn test_crash_while_indexing() -> Result<()> {
  let mut random = random();
  let mut field_to_type = HashMap::new();
  // This test relies on being able to open a reader before any commit
  // happened, so we must create an initial commit just to allow that, but
  // before any documents were added.
  let writer = init_index(&mut random, true, &mut field_to_type)?;
  let dir = writer.get_directory();

  // We create leftover files because merging could be
  // running when we crash:
  dir.set_assert_no_unrefenced_files_on_close(false);

  crash(writer.as_ref())?;

  let reader = directory_reader::open(dir.clone())?;
  assert!(reader.num_docs()? < 157);
  reader.close()?;

  // Make a new dir, copying from the crashed dir, and
  // open IW on it, to confirm IW "recovers" after a
  // crash:
  let dir2 = TestUtil::ram_copy_of(&mut random, dir.as_ref())?;
  dir.close()?;

  RandomIndexWriter::new(&mut random, dir2.clone())?.close(&mut random)?;
  dir2.close()?;
  Ok(())
}

#[test]
fn test_writer_after_crash() -> Result<()> {
  let mut random = random();
  let mut field_to_type = HashMap::new();
  // This test relies on being able to open a reader before any commit
  // happened, so we must create an initial commit just to allow that, but
  // before any documents were added.
  let mut writer = init_index(&mut random, true, &mut field_to_type)?;
  let dir = writer.get_directory();

  // We create leftover files because merging could be
  // running / store files could be open when we crash:
  dir.set_assert_no_unrefenced_files_on_close(false);

  crash(writer.as_ref())?;
  writer = init_index_with_directory(&mut random, dir.clone(), false, true, &mut field_to_type)?;
  writer.close()?;

  let reader = directory_reader::open(dir.clone())?;
  assert!(reader.num_docs()? < 314);
  reader.close()?;

  // Make a new dir, copying from the crashed dir, and
  // open IW on it, to confirm IW "recovers" after a
  // crash:
  let dir2 = TestUtil::ram_copy_of(&mut random, dir.as_ref())?;
  dir.close()?;

  RandomIndexWriter::new(&mut random, dir2.clone())?.close(&mut random)?;
  dir2.close()?;
  Ok(())
}

#[test]
fn test_crash_after_reopen() -> Result<()> {
  let mut random = random();
  let mut field_to_type = HashMap::new();
  let mut writer = init_index(&mut random, false, &mut field_to_type)?;
  let dir = writer.get_directory();

  // We create leftover files because merging could be
  // running when we crash:
  dir.set_assert_no_unrefenced_files_on_close(false);

  writer.close()?;
  writer = init_index_with_directory(&mut random, dir.clone(), false, true, &mut field_to_type)?;
  assert_eq!(314, writer.get_doc_stats()?.max_doc);
  crash(writer.as_ref())?;

  /*
  println!("\n\nTEST: open reader");
  let mut files = dir.list_all()?;
  files.sort();
  for (i, file) in files.iter().enumerate() {
    println!("file {i} = {file} {} bytes", dir.file_length(file)?);
  }
  */

  let reader = directory_reader::open(dir.clone())?;
  assert!(reader.num_docs()? >= 157);
  reader.close()?;

  // Make a new dir, copying from the crashed dir, and
  // open IW on it, to confirm IW "recovers" after a
  // crash:
  let dir2 = TestUtil::ram_copy_of(&mut random, dir.as_ref())?;
  dir.close()?;

  RandomIndexWriter::new(&mut random, dir2.clone())?.close(&mut random)?;
  dir2.close()?;
  Ok(())
}

#[test]
fn test_crash_after_close() -> Result<()> {
  let mut random = random();
  let mut field_to_type = HashMap::new();
  let writer = init_index(&mut random, false, &mut field_to_type)?;
  let dir = writer.get_directory();

  writer.close()?;
  dir.crash()?;

  /*
  let mut files = dir.list_all()?;
  files.sort();
  for (i, file) in files.iter().enumerate() {
    println!("file {i} = {file} {} bytes", dir.file_length(file)?);
  }
  */

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(157, reader.num_docs()?);
  reader.close()?;
  dir.close()?;
  Ok(())
}

#[test]
fn test_crash_after_close_no_wait() -> Result<()> {
  let mut random = random();
  let mut field_to_type = HashMap::new();
  let dir = Arc::new(new_mock_directory_with_lock_factory(
    &mut random,
    NoLockFactory,
  )?);
  let writer =
    init_index_with_directory(&mut random, dir.clone(), false, false, &mut field_to_type)?;

  let commit_result = catch_unwind(AssertUnwindSafe(|| writer.commit()));
  writer.close()?;
  match commit_result {
    Ok(result) => {
      let _ = result?;
    },
    Err(payload) => resume_unwind(payload),
  }

  dir.crash()?;

  /*
  let mut files = dir.list_all()?;
  files.sort();
  for (i, file) in files.iter().enumerate() {
    println!("file {i} = {file} {} bytes", dir.file_length(file)?);
  }
  */
  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(157, reader.num_docs()?);
  reader.close()?;
  dir.close()?;
  Ok(())
}
