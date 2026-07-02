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
use crate::core::document::string_field::StringField;
use crate::core::index::index_writer::IndexWriter;
use crate::core::store::directory::Directory;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::util::lucene_test_case::{
  new_field, new_index_writer_config_with_analyzer, new_text_field, random,
};
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
pub static STORED_TEXT_TYPE: LazyLock<FieldType> = LazyLock::new(|| {
  FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)
    .expect("should not fail")
});
#[allow(dead_code)]
struct TestIndexWriter;

pub(crate) fn add_doc<D, R>(
  random: &mut R,
  writer: &IndexWriter<D>,
  field_types: &mut HashMap<String, FieldType>,
) -> crate::core::util::error::lucene_error::Result<()>
where
  D: Directory + 'static,
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
  let _ = writer.add_document(doc)?;
  Ok(())
}
pub(crate) fn add_doc_with_index<D, R>(
  random: &mut R,
  writer: &IndexWriter<D>,
  index: i32,
  field_types: &mut HashMap<String, FieldType>,
) -> crate::core::util::error::lucene_error::Result<()>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(new_field(
    random,
    "content",
    format!("aaa {}", index),
    &STORED_TEXT_TYPE,
    field_types,
  )?);
  doc.add(StringField::from_string(
    "id",
    index.to_string(),
    Store::No,
  )?);

  match writer.add_document(doc) {
    Ok(_) => Ok(()),
    Err(e) => Err(e),
  }
}

pub(crate) fn assert_no_unreferenced_files<D>(
  dir: Arc<D>,
  message: &str,
) -> crate::core::util::error::lucene_error::Result<()>
where
  D: Directory + 'static,
{
  let mut start_files = dir.list_all()?;
  let mut random = random();
  let mock = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, mock)?,
  )?;
  writer.close()?;
  let mut end_files = dir.list_all()?;

  start_files.sort();
  end_files.sort();

  assert_eq!(
    start_files,
    end_files,
    "{}: before delete:\n    {}\n  after delete:\n    {}",
    message,
    start_files.join("\n    "),
    end_files.join("\n    ")
  );

  Ok(())
}
