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
use crate::core::index::index_reader::IndexReader;
use crate::core::index::term::Term;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_string_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::RngExt;
use std::collections::{HashMap, HashSet};

#[allow(dead_code)] // for quick search
struct TestMixedCodecs;

#[test]
fn test() -> Result<()> {
  let mut random = random();
  let num_docs = at_least(&mut random, 1000);

  let dir = new_directory_shared(&mut random)?;
  let mut w: Option<RandomIndexWriter<DirEnum>> = None;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
  let mut docs_left_in_this_segment = 0;

  let mut doc_upto = 0;
  while doc_upto < num_docs {
    if cfg!(feature = "test_log_verbose") {
      println!("TEST: {} of {}", doc_upto, num_docs);
    }
    if docs_left_in_this_segment == 0 {
      let mock = MockAnalyzer::new(&mut random);
      let iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
      if random.random_bool(0.5) {
        // TODO set_codec 未实现
      }
      if let Some(writer) = w.take() {
        writer.close(&mut random)?;
      }
      w = Some(RandomIndexWriter::with_config(
        &mut random,
        dir.clone(),
        iwc,
      ));
      docs_left_in_this_segment = TestUtil::next_int(&mut random, 10, 100);
    }
    let mut doc = Document::new();
    doc.add(new_string_field(
      &mut random,
      "id",
      doc_upto.to_string(),
      Store::Yes,
      &mut field_to_type,
    )?);
    w.as_ref().unwrap().add_document(&mut random, doc)?;
    doc_upto += 1;
    docs_left_in_this_segment -= 1;
  }

  if cfg!(feature = "test_log_verbose") {
    println!("\nTEST: now delete...");
  }

  let mut deleted = HashSet::new();
  while deleted.len() < num_docs as usize / 2 {
    let to_delete = random.random_range(0..num_docs);
    if !deleted.contains(&to_delete) {
      deleted.insert(to_delete);
      w.as_ref().unwrap().delete_documents_with_terms(
        &mut random,
        vec![Term::from_text("id", to_delete.to_string())],
      )?;
      if random.random_range(0..17) == 6 {
        let r = w.as_ref().unwrap().get_reader(&mut random)?;
        assert_eq!(num_docs - deleted.len() as i32, r.num_docs()?);
        r.close()?;
      }
    }
  }

  w.unwrap().close(&mut random)?;
  Ok(())
}
