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
use crate::core::index::fields::Fields;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term_vectors::TermVectors;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::iterator::IteratorExt;
use crate::test::support::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::support::core::util::english::English;
use crate::test::support::core::util::lucene_test_case::{
  is_night_mode, new_directory_shared, new_field, new_index_writer_config_with_analyzer,
  new_log_merge_policy, random,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
#[allow(dead_code)] // for quick search
struct TestMultiThreadTermVectors;

fn verify_vectors<F: Fields>(vectors: &F, num: i32) -> Result<()> {
  let mut fields_iter = vectors.iterator()?;
  while fields_iter.has_next()? {
    let field_name = fields_iter.next()?.unwrap();
    let terms = vectors.terms(field_name)?;
    assert!(terms.is_some());
    verify_vector(&mut terms.unwrap().iterator()?, num)?;
  }
  Ok(())
}

fn verify_vector<TE: TermsEnum>(terms_enum: &mut TE, num: i32) -> Result<()> {
  let mut temp = String::new();
  while terms_enum.next()?.is_some() {
    temp.push_str(&terms_enum.term()?.utf8_to_string()?);
  }
  assert_eq!(English::int_to_english(num).trim(), temp.trim());
  Ok(())
}
#[test]
fn test() -> Result<()> {
  let mut random = random();
  let num_docs = if is_night_mode() { 1000 } else { 50 };
  let num_threads = if is_night_mode() { 3 } else { 2 };
  let num_iterations = if is_night_mode() { 100 } else { 50 };

  let directory = new_directory_shared(&mut random)?;
  let a = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, a)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

  let writer = IndexWriter::new(directory.clone(), iwc)?;

  let mut custom_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
  custom_type.set_tokenized(false)?;
  custom_type.set_store_term_vectors(true)?;

  let mut field_to_type = HashMap::new();

  for i in 0..num_docs {
    let mut doc = Document::new();
    let fld = new_field(
      &mut random,
      "field",
      English::int_to_english(i),
      &custom_type,
      &mut field_to_type,
    )?;
    doc.add(fld);
    writer.add_document(doc)?;
  }
  writer.close()?;

  let reader = directory_reader::open(directory)?;
  let reader = Arc::new(reader);

  let mut handles = Vec::with_capacity(num_threads as usize);

  for _ in 0..num_threads {
    let reader = reader.clone();
    handles.push(thread::spawn(move || -> Result<()> {
      for _ in 0..num_iterations {
        let num_docs = reader.num_docs()?;
        let mut term_vectors = reader.term_vectors()?;
        for doc_id in 0..num_docs {
          let vectors = term_vectors.get(doc_id)?.expect("vectors should exist");
          verify_vectors(&vectors, doc_id)?;
          let vector = term_vectors
            .get_field_terms(doc_id, "field")?
            .expect("field terms should exist");
          verify_vector(&mut vector.iterator()?, doc_id)?;
        }
      }
      Ok(())
    }));
  }

  for handle in handles {
    handle.join().unwrap()?;
  }

  Ok(())
}
