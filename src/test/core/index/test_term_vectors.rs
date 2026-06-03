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
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::index::directory_reader;
use crate::core::index::fields::Fields;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{DefaultIndexWriterType, IndexWriter};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term_vectors::TermVectors;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config_with_analyzer, random,
};
use rand::Rng;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestTermVectors;
fn create_writer<D, R>(random: &mut R, dir: Arc<D>) -> Result<DefaultIndexWriterType<D>>
where
  D: Directory,
  R: Rng + ?Sized,
{
  let a = MockAnalyzer::new(random);
  let mut conf = new_index_writer_config_with_analyzer(random, a);
  conf.set_max_buffered_docs(2);
  IndexWriter::new(dir, conf)
}
pub fn create_dir<D, R>(random: &mut R, dir: Arc<D>) -> Result<()>
where
  R: Rng + ?Sized,
  D: Directory,
{
  let mock = MockAnalyzer::new(random);
  let mut config = new_index_writer_config_with_analyzer(random, mock);
  config.set_max_buffered_docs(2);
  let writer = IndexWriter::new(dir.clone(), config)?;
  writer.add_document(create_doc()?)?;
  writer.close()
}

fn create_doc() -> Result<Document> {
  let mut doc = Document::new();

  let mut ft = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
  ft.set_store_term_vectors(true)?;
  ft.set_store_term_vector_positions(true)?;
  ft.set_store_term_vector_offsets(true)?;

  doc.add(Field::new("c", "aaa", ft));

  Ok(doc)
}
fn verify_index<D>(dir: Arc<D>) -> Result<()>
where
  D: Directory,
{
  let reader = directory_reader::open(dir)?;

  let mut term_vectors = reader.term_vectors()?;
  let num_docs = reader.num_docs()?;

  for i in 0..num_docs {
    let terms = term_vectors.get(i)?.as_ref().unwrap().terms("c")?;

    assert!(
      terms.is_some(),
      "term vectors should not have been null for document {}",
      i
    );
  }
  Ok(())
}
#[test]
fn test_full_merge_add_docs() -> Result<()> {
  let mut random = random();
  let target = new_directory_shared(&mut random)?;
  let writer = create_writer(&mut random, target.clone())?;
  // with maxBufferedDocs=2, this results in two segments, so that forceMerge
  // actually does something.
  for _ in 0..4 {
    writer.add_document(create_doc()?)?;
  }
  writer.force_merge(1)?;
  writer.close()?;

  verify_index(target.clone())?;
  Ok(())
}
#[test]
fn test_full_merge_add_indexes_dir() -> Result<()> {
  let mut random = random();

  let input = vec![
    new_directory_shared(&mut random)?,
    new_directory_shared(&mut random)?,
  ];
  let target = new_directory_shared(&mut random)?;

  for dir in &input {
    create_dir(&mut random, dir.clone())?;
  }

  let writer = create_writer(&mut random, target.clone())?;
  writer.add_indexes_from_dir(&input)?;
  writer.force_merge(1)?;
  writer.close()?;

  verify_index(target.clone())?;
  Ok(())
}
#[test]
fn test_full_merge_add_indexes_reader() -> Result<()> {
  // TODO add_indexes_slowly未实现
  Ok(())
}
#[test]
fn test_merge_with_payloads() -> Result<()> {
  // TODO token_stream 未实现
  Ok(())
}
