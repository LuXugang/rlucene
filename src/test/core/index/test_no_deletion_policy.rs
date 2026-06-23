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
use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::no_deletion_policy::NoDeletionPolicy;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_text_field, random,
};
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
struct TestNoDeletionPolicy;
#[test]
fn test_no_deletion_policy() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_final_singleton() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_methods_overridden() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_all_commits_remain() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_index_deletion_policy(NoDeletionPolicy);

  let writer = IndexWriter::new(dir.clone(), iwc)?;
  let mut field_types = HashMap::new();

  for i in 0..10 {
    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "c",
      format!("a{i}"),
      Store::Yes,
      &mut field_types,
    )?);
    writer.add_document(doc)?;
    writer.commit()?;

    assert_eq!(
      i + 1,
      directory_reader::list_commits(dir.clone())?.len(),
      "wrong number of commits !"
    );
  }

  writer.close()?;
  Ok(())
}
