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
use crate::core::index::directory_reader;
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_deletion_policy::IndexDeletionPolicy;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexCommitWrapper, IndexWriter};
use crate::core::index::multi_bits::get_live_docs;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::DirEnum;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::dummy::dummy_comparator::DummyComparator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_text_field, random,
};
use rand_chacha::rand_core::Rng;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestTransactionRollback;

const FIELD_RECORD_ID: &str = "record_id";

fn roll_back_last<R>(random: &mut R, dir: Arc<DirEnum>, id: i32) -> Result<()>
where
  R: Rng + ?Sized,
{
  let ids = format!("-{id}");
  let mut last = None;
  let commits = directory_reader::list_commits(dir.clone())?;
  for commit in commits {
    let user_data = commit.get_user_data();
    if !user_data.is_empty()
      && user_data
        .get("index")
        .is_some_and(|index| index.ends_with(&ids))
    {
      last = Some(commit);
    }
  }

  let last =
    last.ok_or_else(|| LuceneError::illegal_state(format!("Couldn't find commit point {id}")))?;

  let mock = MockAnalyzer::new(random);
  let mut config = new_index_writer_config_with_analyzer(random, mock)?;
  config.set_index_deletion_policy(RollbackDeletionPolicy::new(id));
  let writer = IndexWriter::with_index_commit(
    dir,
    config,
    IndexCommitWrapper::<_, DummyComparator, _>::new(Some(last), None, None)?,
  )?;
  writer.set_live_commit_data(HashMap::from([(
    "index".to_string(),
    format!("Rolled back to 1-{id}"),
  )]));
  writer.close()?;
  Ok(())
}

#[test]
fn test_repeated_roll_backs() -> Result<()> {
  let mut random = random();
  let dir = set_up(&mut random)?;
  let mut expected_last_record_id = 100;
  while expected_last_record_id > 10 {
    expected_last_record_id -= 10;
    roll_back_last(&mut random, dir.clone(), expected_last_record_id)?;

    let mut expecteds = FixedBitSet::new(101);
    expecteds.set_with_range(1, expected_last_record_id as usize + 1);
    check_expecteds(dir.clone(), &mut expecteds)?;
  }
  Ok(())
}

fn check_expecteds(dir: Arc<DirEnum>, expecteds: &mut FixedBitSet) -> Result<()> {
  let reader = directory_reader::open(dir)?;
  let live_docs = get_live_docs(&reader)?;
  let mut stored_fields = reader.stored_fields()?;
  for i in 0..reader.max_doc()? {
    let is_live = match &live_docs {
      Some(live_docs) => live_docs.get(i as usize)?,
      None => true,
    };
    if is_live && let Some(value) = stored_fields.document(i)?.get(FIELD_RECORD_ID)? {
      let value = value.parse::<usize>()?;
      assert!(expecteds.get(value)?, "Did not expect document #{value}");
      expecteds.clear_with_index(value);
    }
  }
  reader.close()?;
  assert_eq!(0, expecteds.cardinality(), "Should have 0 docs remaining");
  Ok(())
}

fn set_up<R>(random: &mut R) -> Result<Arc<DirEnum>>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let mut field_types: HashMap<String, FieldType> = HashMap::new();

  let mock = MockAnalyzer::new(random);
  let mut config = new_index_writer_config_with_analyzer(random, mock)?;
  config.set_index_deletion_policy(KeepAllTransactionDeletionPolicy);
  let writer = IndexWriter::new(dir.clone(), config)?;

  for current_record_id in 1..=100 {
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      FIELD_RECORD_ID,
      current_record_id.to_string(),
      Store::Yes,
      &mut field_types,
    )?);
    writer.add_document(doc)?;

    if current_record_id % 10 == 0 {
      writer.set_live_commit_data(HashMap::from([(
        "index".to_string(),
        format!("records 1-{current_record_id}"),
      )]));
      writer.commit()?;
    }
  }

  writer.close()?;
  Ok(dir)
}

#[derive(Clone)]
pub struct RollbackDeletionPolicy {
  rollback_point: i32,
}

impl RollbackDeletionPolicy {
  fn new(rollback_point: i32) -> Self {
    Self { rollback_point }
  }
}

impl Display for RollbackDeletionPolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl IndexDeletionPolicy for RollbackDeletionPolicy {
  fn on_init<IC>(&self, commits: &[IC]) -> Result<()>
  where
    IC: IndexCommit + Clone,
  {
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

  fn on_commit<IC>(&self, _commits: &[IC]) -> Result<()>
  where
    IC: IndexCommit + Clone,
  {
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

impl IndexDeletionPolicy for DeleteLastCommitPolicy {
  fn on_init<IC>(&self, commits: &[IC]) -> Result<()>
  where
    IC: IndexCommit + Clone,
  {
    commits.last().unwrap().delete()
  }

  fn on_commit<IC>(&self, _commits: &[IC]) -> Result<()>
  where
    IC: IndexCommit + Clone,
  {
    Ok(())
  }
}

#[test]
fn test_rollback_deletion_policy() -> Result<()> {
  let mut random = random();
  let dir = set_up(&mut random)?;

  for _ in 0..2 {
    let mock = MockAnalyzer::new(&mut random);
    let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
    config.set_index_deletion_policy(DeleteLastCommitPolicy);
    IndexWriter::new(dir.clone(), config)?.close()?;

    let reader = directory_reader::open(dir.clone())?;
    assert_eq!(100, reader.num_docs()?);
    reader.close()?;
  }
  Ok(())
}

#[derive(Clone)]
pub struct KeepAllTransactionDeletionPolicy;

impl Display for KeepAllTransactionDeletionPolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl IndexDeletionPolicy for KeepAllTransactionDeletionPolicy {
  fn on_init<IC>(&self, _commits: &[IC]) -> Result<()>
  where
    IC: IndexCommit + Clone,
  {
    Ok(())
  }

  fn on_commit<IC>(&self, _commits: &[IC]) -> Result<()>
  where
    IC: IndexCommit + Clone,
  {
    Ok(())
  }
}
