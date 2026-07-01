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
use crate::core::index::IndexFileNames;
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader;
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::{IndexCommitWrapper, IndexWriter};
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::index::no_merge_policy::NoMergePolicy;
#[cfg(feature = "nightly")]
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::segment_infos::{
  generation_from_segments_file_name, get_last_commit_generation_from_directory,
};
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::standard_directory_reader::ReaderCommit;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::dummy::dummy_comparator::DummyComparator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
#[cfg(feature = "nightly")]
use crate::core::util::version::LATEST;
use crate::test::support::core::analysis::mock_analyzer::MockAnalyzer;
#[cfg(feature = "nightly")]
use crate::test::support::core::index::deletion_policy::ExpirationTimeDeletionPolicy;
pub use crate::test::support::core::index::deletion_policy::{
  KeepAllDeletionPolicy, KeepLastNDeletionPolicy, KeepNoneOnInitDeletionPolicy,
};
use crate::test::support::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer,
  new_log_merge_policy_with_merge_factor, new_searcher_with_reader, new_string_field,
  new_text_field, random,
};
use rand::Rng;
#[cfg(feature = "nightly")]
use rand::RngExt;
use std::collections::HashMap;
#[cfg(feature = "nightly")]
use std::sync::OnceLock;
#[cfg(feature = "nightly")]
use std::time::{Duration, Instant};

#[allow(dead_code)] // for quick search
struct TestDeletionPolicy;

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

#[cfg(feature = "nightly")]
fn current_time_nanos() -> i64 {
  static START: OnceLock<Instant> = OnceLock::new();
  START
    .get_or_init(Instant::now)
    .elapsed()
    .as_nanos()
    .min(i64::MAX as u128) as i64
}

#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_expiration_time_deletion_policy() -> Result<()> {
  const SECONDS: f64 = 2.0;

  let mut random = random();
  let mut field_types = HashMap::new();
  let dir = new_directory_shared(&mut random)?;
  let policy = ExpirationTimeDeletionPolicy::new(dir.clone(), SECONDS);

  let mock = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
  conf.set_index_deletion_policy(policy.clone());
  conf
    .get_merge_policy_mut()
    .get_base_mut()
    .set_no_cfs_ratio(1.0)?;
  let writer = IndexWriter::new(dir.clone(), conf)?;
  let mut commit_data = HashMap::new();
  commit_data.insert("commitTime".to_string(), current_time_nanos().to_string());
  writer.set_live_commit_data(commit_data);
  writer.commit()?;
  writer.close()?;
  drop(writer);

  let mut last_delete_time = 0;
  let target_num_delete = random.random_range(1..=5);
  while policy.num_delete() < target_num_delete {
    // Record last time when writer performed deletes of
    // past commits
    last_delete_time = current_time_nanos();
    let mock = MockAnalyzer::new(&mut random);
    let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
    conf
      .set_open_mode(OpenMode::Append)
      .set_index_deletion_policy(policy.clone());
    conf
      .get_merge_policy_mut()
      .get_base_mut()
      .set_no_cfs_ratio(1.0)?;
    let writer = IndexWriter::new(dir.clone(), conf)?;
    for _ in 0..17 {
      add_doc(&mut random, &writer, &mut field_types)?;
    }
    let mut commit_data = HashMap::new();
    commit_data.insert("commitTime".to_string(), current_time_nanos().to_string());
    writer.set_live_commit_data(commit_data);
    writer.commit()?;
    writer.close()?;
    drop(writer);

    std::thread::sleep(Duration::from_millis((1000.0 * (SECONDS / 5.0)) as u64));
  }

  // Then simplistic check: just verify that the
  // segments_N's that still exist are in fact within SECONDS
  // seconds of the last one's mod time, and, that I can
  // open a reader on each:
  let mut generation = get_last_commit_generation_from_directory(dir.as_ref())?;
  let mut one_second_resolution = true;

  while generation > 0 {
    let reader = match directory_reader::open(dir.clone()) {
      Ok(reader) => reader,
      Err(_) => break,
    };
    if reader.close().is_err() {
      break;
    }

    let file_name =
      IndexFileNames::file_name_from_generation(IndexFileNames::SEGMENTS, "", generation).unwrap();
    let sis = match SegmentInfos::read_commit(dir.clone(), &file_name) {
      Ok(sis) => sis,
      Err(_) => break,
    };
    assert_eq!(Some(&*LATEST), sis.get_commit_lucene_version());
    assert_eq!(Some(&*LATEST), sis.get_min_segment_lucene_version());
    let mod_time = sis
      .get_user_data()
      .get("commitTime")
      .ok_or_else(|| LuceneError::illegal_state("missing commitTime"))?
      .parse::<i64>()?;
    one_second_resolution &= (mod_time % 1000) == 0;
    let leeway = ((SECONDS + if one_second_resolution { 1.0 } else { 0.0 }) * 1000.0) as i64;

    assert!(
      last_delete_time - mod_time <= leeway,
      "commit point was older than {SECONDS} seconds ({} ms) but did not get deleted ",
      last_delete_time - mod_time
    );

    dir.delete_file(
      &IndexFileNames::file_name_from_generation(IndexFileNames::SEGMENTS, "", generation).unwrap(),
    )?;
    generation -= 1;
  }

  Ok(())
}

#[test]
fn test_keep_all_deletion_policy() -> Result<()> {
  let mut random = random();
  let mut field_types = HashMap::new();
  for pass in 0..2 {
    let use_compound_file = (pass % 2) != 0;

    let dir = new_directory_shared(&mut random)?;

    let policy = KeepAllDeletionPolicy::new(dir.clone());
    let mock = MockAnalyzer::new(&mut random);
    let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
    conf
      .set_index_deletion_policy(policy.clone())
      .set_max_buffered_docs(10)
      .set_merge_scheduler(SerialMergeScheduler::new());
    conf
      .get_merge_policy_mut()
      .get_base_mut()
      .set_no_cfs_ratio(if use_compound_file { 1.0 } else { 0.0 })?;

    let writer = IndexWriter::new(dir.clone(), conf)?;
    for _ in 0..107 {
      add_doc(&mut random, &writer, &mut field_types)?;
    }
    writer.close()?;
    drop(writer);

    let needs_merging;
    {
      let r = directory_reader::open(dir.clone())?;
      needs_merging = get_context(&r)?.leaves()?.len() != 1;
      r.close()?;
    }

    if needs_merging {
      let mock = MockAnalyzer::new(&mut random);
      let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
      conf
        .set_open_mode(OpenMode::Append)
        .set_index_deletion_policy(policy.clone());
      conf
        .get_merge_policy_mut()
        .get_base_mut()
        .set_no_cfs_ratio(if use_compound_file { 1.0 } else { 0.0 })?;

      let writer = IndexWriter::new(dir.clone(), conf)?;
      writer.force_merge(1)?;
      writer.close()?;
    }

    assert_eq!(if needs_merging { 2 } else { 1 }, policy.num_on_init());
    assert_eq!(1 + usize::from(needs_merging), policy.num_on_commit());

    let commits = directory_reader::list_commits(dir.clone())?;
    assert_eq!(1 + usize::from(needs_merging), commits.len());

    for commit in &commits {
      let r = directory_reader::open_from_commit::<_, DummyComparator, _>(commit)?;
      r.close()?;
    }

    let mut generation = get_last_commit_generation_from_directory(dir.as_ref())?;
    while generation > 0 {
      let reader = directory_reader::open(dir.clone())?;
      reader.close()?;
      dir.delete_file(
        &IndexFileNames::file_name_from_generation(IndexFileNames::SEGMENTS, "", generation)
          .unwrap(),
      )?;
      generation -= 1;

      if generation > 0 {
        let pre_count = dir.list_all()?.len();
        let mock = MockAnalyzer::new(&mut random);
        let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
        conf
          .set_open_mode(OpenMode::Append)
          .set_index_deletion_policy(policy.clone());
        let writer = IndexWriter::new(dir.clone(), conf)?;
        writer.close()?;
        let post_count = dir.list_all()?.len();
        assert!(post_count < pre_count);
      }
    }
  }
  Ok(())
}

#[test]
fn test_open_prior_snapshot() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let policy = KeepAllDeletionPolicy::new(dir.clone());
  let mock = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
  conf
    .set_index_deletion_policy(policy.clone())
    .set_max_buffered_docs(2)
    .set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let writer = IndexWriter::new(dir.clone(), conf)?;
  for i in 0..10 {
    add_doc(&mut random, &writer, &mut field_types)?;
    if (1 + i) % 2 == 0 {
      writer.commit()?;
    }
  }
  writer.close()?;
  drop(writer);

  let commits = directory_reader::list_commits(dir.clone())?;
  assert_eq!(5, commits.len());
  let mut last_commit: Option<ReaderCommit<DirEnum>> = None;
  for commit in commits {
    if match &last_commit {
      None => true,
      Some(last_commit) => commit.get_generation() > last_commit.get_generation(),
    } {
      last_commit = Some(commit);
    }
  }
  assert!(last_commit.is_some());
  let last_commit = last_commit.unwrap();

  let mock = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
  conf.set_index_deletion_policy(policy.clone());
  let writer = IndexWriter::new(dir.clone(), conf)?;
  add_doc(&mut random, &writer, &mut field_types)?;
  assert_eq!(11, writer.get_doc_stats()?.num_docs);
  writer.force_merge(1)?;
  writer.close()?;
  drop(writer);

  assert_eq!(6, directory_reader::list_commits(dir.clone())?.len());

  let mock = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
  conf
    .set_index_deletion_policy(policy.clone())
    .set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let writer = IndexWriter::with_index_commit(
    dir.clone(),
    conf,
    IndexCommitWrapper::<_, DummyComparator, _>::new(Some(last_commit.clone()), None, None)?,
  )?;
  assert_eq!(10, writer.get_doc_stats()?.num_docs);

  writer.rollback()?;
  drop(writer);
  let r = directory_reader::open(dir.clone())?;
  assert_eq!(1, get_context(&r)?.leaves()?.len());
  assert_eq!(11, r.num_docs()?);
  r.close()?;

  let mock = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
  conf
    .set_index_deletion_policy(policy.clone())
    .set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let writer = IndexWriter::with_index_commit(
    dir.clone(),
    conf,
    IndexCommitWrapper::<_, DummyComparator, _>::new(Some(last_commit.clone()), None, None)?,
  )?;
  assert_eq!(10, writer.get_doc_stats()?.num_docs);
  writer.close()?;
  drop(writer);
  assert_eq!(7, directory_reader::list_commits(dir.clone())?.len());

  let r = directory_reader::open(dir.clone())?;
  assert!(get_context(&r)?.leaves()?.len() > 1);
  assert_eq!(10, r.num_docs()?);
  r.close()?;

  let mock = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
  conf.set_index_deletion_policy(policy.clone());
  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;
  drop(writer);

  let r = directory_reader::open(dir.clone())?;
  assert_eq!(1, get_context(&r)?.leaves()?.len());
  assert_eq!(10, r.num_docs()?);
  r.close()?;

  let mock = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
  conf.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 10)?);
  let writer = IndexWriter::with_index_commit(
    dir.clone(),
    conf,
    IndexCommitWrapper::<_, DummyComparator, _>::new(Some(last_commit.clone()), None, None)?,
  )?;
  assert_eq!(10, writer.get_doc_stats()?.num_docs);

  let r = directory_reader::open(dir.clone())?;
  assert_eq!(1, get_context(&r)?.leaves()?.len());
  assert_eq!(10, r.num_docs()?);
  r.close()?;

  writer.close()?;
  drop(writer);

  let r = directory_reader::open(dir.clone())?;
  assert!(get_context(&r)?.leaves()?.len() > 1);
  assert_eq!(10, r.num_docs()?);
  r.close()?;

  Ok(())
}

#[test]
fn test_keep_none_on_init_deletion_policy() -> Result<()> {
  let mut random = random();
  let mut field_types = HashMap::new();
  for pass in 0..2 {
    let use_compound_file = (pass % 2) != 0;

    let dir = new_directory_shared(&mut random)?;

    let policy = KeepNoneOnInitDeletionPolicy::new();
    let mock = MockAnalyzer::new(&mut random);
    let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
    conf
      .set_open_mode(OpenMode::Create)
      .set_index_deletion_policy(policy.clone())
      .set_max_buffered_docs(10);
    conf
      .get_merge_policy_mut()
      .get_base_mut()
      .set_no_cfs_ratio(if use_compound_file { 1.0 } else { 0.0 })?;
    let writer = IndexWriter::new(dir.clone(), conf)?;
    for _ in 0..107 {
      add_doc(&mut random, &writer, &mut field_types)?;
    }
    writer.close()?;
    drop(writer);

    let mock = MockAnalyzer::new(&mut random);
    let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
    conf
      .set_open_mode(OpenMode::Append)
      .set_index_deletion_policy(policy.clone());
    conf
      .get_merge_policy_mut()
      .get_base_mut()
      .set_no_cfs_ratio(1.0)?;
    let writer = IndexWriter::new(dir.clone(), conf)?;
    writer.force_merge(1)?;
    writer.close()?;
    drop(writer);

    assert_eq!(2, policy.num_on_init());
    assert_eq!(2, policy.num_on_commit());

    let reader = directory_reader::open(dir.clone())?;
    reader.close()?;
  }
  Ok(())
}

#[test]
fn test_keep_last_n_deletion_policy() -> Result<()> {
  let mut random = random();
  let mut field_types = HashMap::new();
  const N: usize = 5;

  for pass in 0..2 {
    let use_compound_file = (pass % 2) != 0;

    let dir = new_directory_shared(&mut random)?;

    let policy = KeepLastNDeletionPolicy::new(N);
    for _ in 0..(N + 1) {
      let mock = MockAnalyzer::new(&mut random);
      let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
      conf
        .set_open_mode(OpenMode::Create)
        .set_index_deletion_policy(policy.clone())
        .set_max_buffered_docs(10);
      conf
        .get_merge_policy_mut()
        .get_base_mut()
        .set_no_cfs_ratio(if use_compound_file { 1.0 } else { 0.0 })?;
      let writer = IndexWriter::new(dir.clone(), conf)?;
      for _ in 0..17 {
        add_doc(&mut random, &writer, &mut field_types)?;
      }
      writer.force_merge(1)?;
      writer.close()?;
      drop(writer);
    }

    assert!(policy.num_delete() > 0);
    assert_eq!(N + 1, policy.num_on_init());
    assert_eq!(N + 1, policy.num_on_commit());

    let mut generation = get_last_commit_generation_from_directory(dir.as_ref())?;
    for i in 0..(N + 1) {
      match directory_reader::open(dir.clone()) {
        Ok(reader) => {
          reader.close()?;
          assert!(i != N, "should have failed on commits prior to last {N}");
        },
        Err(err) => {
          if i != N {
            return Err(err);
          }
        },
      }
      if i < N {
        dir.delete_file(
          &IndexFileNames::file_name_from_generation(IndexFileNames::SEGMENTS, "", generation)
            .unwrap(),
        )?;
      }
      generation -= 1;
    }
  }
  Ok(())
}

#[test]
fn test_keep_last_n_deletion_policy_with_creates() -> Result<()> {
  let mut random = random();
  let mut field_types = HashMap::new();
  const N: usize = 10;

  for pass in 0..2 {
    let use_compound_file = (pass % 2) != 0;

    let dir = new_directory_shared(&mut random)?;
    let policy = KeepLastNDeletionPolicy::new(N);
    let mock = MockAnalyzer::new(&mut random);
    let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
    conf
      .set_open_mode(OpenMode::Create)
      .set_index_deletion_policy(policy.clone())
      .set_max_buffered_docs(10);
    conf
      .get_merge_policy_mut()
      .get_base_mut()
      .set_no_cfs_ratio(if use_compound_file { 1.0 } else { 0.0 })?;
    let writer = IndexWriter::new(dir.clone(), conf)?;
    writer.close()?;
    drop(writer);
    let query = TermQuery::new(Term::from_text("content", "aaa"));

    for i in 0..(N + 1) {
      let mock = MockAnalyzer::new(&mut random);
      let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
      conf
        .set_open_mode(OpenMode::Append)
        .set_index_deletion_policy(policy.clone())
        .set_max_buffered_docs(10);
      conf
        .get_merge_policy_mut()
        .get_base_mut()
        .set_no_cfs_ratio(if use_compound_file { 1.0 } else { 0.0 })?;
      let writer = IndexWriter::new(dir.clone(), conf)?;
      for j in 0..17 {
        add_doc_with_id(
          &mut random,
          &writer,
          (i * (N + 1) + j) as i32,
          &mut field_types,
        )?;
      }
      writer.close()?;
      drop(writer);

      let mock = MockAnalyzer::new(&mut random);
      let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
      conf
        .set_index_deletion_policy(policy.clone())
        .set_merge_policy(NoMergePolicy::default());
      let writer = IndexWriter::new(dir.clone(), conf)?;
      writer
        .delete_documents_with_terms(vec![Term::from_text("id", (i * (N + 1) + 3).to_string())])?;
      writer.close()?;
      drop(writer);

      let reader = directory_reader::open(dir.clone())?;
      let searcher = new_searcher_with_reader(reader)?;
      let hits = searcher.search(query.clone(), 1000)?.score_docs;
      assert_eq!(16, hits.len());

      let mock = MockAnalyzer::new(&mut random);
      let mut conf = new_index_writer_config_with_analyzer(&mut random, mock)?;
      conf
        .set_open_mode(OpenMode::Create)
        .set_index_deletion_policy(policy.clone());
      let writer = IndexWriter::new(dir.clone(), conf)?;
      writer.close()?;
      drop(writer);
    }

    assert_eq!(3 * (N + 1) + 1, policy.num_on_init());
    assert_eq!(3 * (N + 1) + 1, policy.num_on_commit());

    let rw_reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(rw_reader)?;
    let hits = searcher.search(query.clone(), 1000)?.score_docs;
    assert_eq!(0, hits.len());

    let mut generation = get_last_commit_generation_from_directory(dir.as_ref())?;

    let mut expected_count = 0;
    for i in 0..(N + 1) {
      match directory_reader::open(dir.clone()) {
        Ok(reader) => {
          let searcher = new_searcher_with_reader(reader)?;
          let hits = searcher.search(query.clone(), 1000)?.score_docs;
          assert_eq!(expected_count, hits.len());
          if expected_count == 0 {
            expected_count = 16;
          } else if expected_count == 16 {
            expected_count = 17;
          } else if expected_count == 17 {
            expected_count = 0;
          }
          assert!(i != N, "should have failed on commits before last {N}");
        },
        Err(err) => {
          if i != N {
            return Err(err);
          }
        },
      }
      if i < N {
        dir.delete_file(
          &IndexFileNames::file_name_from_generation(IndexFileNames::SEGMENTS, "", generation)
            .unwrap(),
        )?;
      }
      generation -= 1;
    }
  }
  Ok(())
}
fn add_doc_with_id<R>(
  random: &mut R,
  writer: &IndexWriter<DirEnum>,
  id: i32,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  R: Rng + ?Sized,
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
fn add_doc<R>(
  random: &mut R,
  writer: &IndexWriter<DirEnum>,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "content",
    "aaa",
    Store::No,
    field_types,
  )?);
  writer.add_document(doc)?;
  Ok(())
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
