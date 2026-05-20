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
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_string_field, random,
};
use std::collections::HashMap;
#[allow(dead_code)] // for quick search
struct TestRollback;

#[test]
fn test_rollback_integrity_with_buffer_flush() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let rw = RandomIndexWriter::new(&mut random, dir.clone());
  let mut field_to_type = HashMap::new();

  for i in 0..5 {
    let mut doc = Document::new();
    doc.add(new_string_field(
      &mut random,
      "pk",
      i.to_string(),
      Store::Yes,
      &mut field_to_type,
    )?);
    rw.add_document(doc)?;
  }
  rw.close()?;
  drop(rw);
  // If buffer size is small enough to cause a flush, errors ensue...
  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_max_buffered_docs(2);
  iwc.set_open_mode(OpenMode::Append);

  let w = IndexWriter::new(dir.clone(), iwc)?;

  for i in 0..3 {
    let mut doc = Document::new();
    let value = i.to_string();

    doc.add(new_string_field(
      &mut random,
      "pk",
      &value,
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(new_string_field(
      &mut random,
      "text",
      "foo",
      Store::Yes,
      &mut field_to_type,
    )?);

    w.update_document_with_term(Term::from_text("pk", &value), doc)?;
  }

  w.rollback()?;
  let r = directory_reader::open(dir)?;
  assert_eq!(
    5,
    r.num_docs()?,
    "index should contain same number of docs post rollback"
  );

  Ok(())
}
