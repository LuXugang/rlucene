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
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::fields::Fields;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::{DefaultIndexWriter, IndexWriter};
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term_vectors::TermVectors;
use crate::core::store::directory::Directory;
use crate::core::util::attribute_source::Attributes;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::base_term_vectors_format_test_case::RandomTokenStream;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestTermVectors;
fn create_writer<D, R>(random: &mut R, dir: Arc<D>) -> Result<DefaultIndexWriter<D>>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  let a = MockAnalyzer::new(random);
  let mut conf = new_index_writer_config_with_analyzer(random, a)?;
  conf.set_max_buffered_docs(2);
  IndexWriter::new(dir, conf)
}
pub fn create_dir<D, R>(random: &mut R, dir: Arc<D>) -> Result<()>
where
  R: Rng + ?Sized,
  D: Directory + 'static,
{
  let mock = MockAnalyzer::new(random);
  let mut config = new_index_writer_config_with_analyzer(random, mock)?;
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
  D: Directory + 'static,
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
  writer.add_indexes_from_directory(&input)?;
  writer.force_merge(1)?;
  writer.close()?;

  verify_index(target.clone())?;
  Ok(())
}
#[test]
fn test_full_merge_add_indexes_reader() -> Result<()> {
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
  for dir in &input {
    let reader = directory_reader::open(dir.clone())?;
    TestUtil::add_indexes_slowly(&writer, &[&reader])?;
    reader.close()?;
  }
  writer.force_merge(1)?;
  writer.close()?;

  verify_index(target.clone())?;
  Ok(())
}
/// Assert that a merged segment has payloads set up in field info, if at least 1 segment has
/// payloads for this field.
#[test]
fn test_merge_with_payloads() -> Result<()> {
  let mut random = random();
  let mut ft1 = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  ft1.set_store_term_vectors(true)?;
  ft1.set_store_term_vector_offsets(true)?;
  ft1.set_store_term_vector_positions(true)?;
  ft1.set_store_term_vector_payloads(true)?;
  ft1.freeze();

  let num_docs_in_segment = 10;
  for has_payloads in [false, true] {
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let mut index_writer_config = IndexWriterConfig::with_analyzer(mock)?;
    index_writer_config.set_max_buffered_docs(num_docs_in_segment);
    let writer = IndexWriter::new(dir.clone(), index_writer_config)?;
    let tkg1 = TokenStreamGenerator::new(&mut random, has_payloads);
    let tkg2 = TokenStreamGenerator::new(&mut random, !has_payloads);

    // create one segment with payloads, and another without payloads
    for _ in 0..num_docs_in_segment {
      let mut doc = Document::new();
      doc.add(Field::from_token_stream(
        "c",
        FieldTokenStreamEnum::custom(tkg1.new_token_stream(&mut random)?),
        ft1.clone(),
      )?);
      writer.add_document(doc)?;
    }
    for _ in 0..num_docs_in_segment {
      let mut doc = Document::new();
      doc.add(Field::from_token_stream(
        "c",
        FieldTokenStreamEnum::custom(tkg2.new_token_stream(&mut random)?),
        ft1.clone(),
      )?);
      writer.add_document(doc)?;
    }

    let reader1 = directory_reader::open_from_writer(&writer)?;
    {
      let context = (&reader1).get_context()?;
      let leaves = context.leaves()?;
      assert_eq!(2, leaves.len());
      assert_eq!(
        has_payloads,
        leaves[0]
          .reader()
          .get_field_infos()?
          .field_info_by_name("c")?
          .expect("field c must exist")
          .has_payloads()
      );
      assert_ne!(
        has_payloads,
        leaves[1]
          .reader()
          .get_field_infos()?
          .field_info_by_name("c")?
          .expect("field c must exist")
          .has_payloads()
      );
    }

    writer.force_merge(1)?;
    let reader2 = directory_reader::open_from_writer(&writer)?;
    {
      let context = (&reader2).get_context()?;
      let leaves = context.leaves()?;
      assert_eq!(1, leaves.len());
      // assert that in the merged segments payloads set up for the field
      assert!(
        leaves[0]
          .reader()
          .get_field_infos()?
          .field_info_by_name("c")?
          .expect("field c must exist")
          .has_payloads()
      );
    }

    let mut close_result = writer.close();
    close_result = IOUtils::use_or_suppress_result(close_result, reader1.close());
    close_result = IOUtils::use_or_suppress_result(close_result, reader2.close());
    drop(writer);
    drop(reader1);
    drop(reader2);
    let dir_close_result = match Arc::try_unwrap(dir) {
      Ok(dir) => dir.close(),
      Err(_) => Err(LuceneError::illegal_state(
        "directory still has outstanding references",
      )),
    };
    IOUtils::use_or_suppress_result(close_result, dir_close_result)?;
  }

  Ok(())
}

/// A generator for token streams with optional payloads.
struct TokenStreamGenerator {
  terms: Vec<String>,
  term_bytes: Vec<BytesRef<Vec<u8>>>,
  has_payloads: bool,
}

impl TokenStreamGenerator {
  fn new<R>(random: &mut R, has_payloads: bool) -> Self
  where
    R: Rng + ?Sized,
  {
    let terms_count = 10;
    let mut terms = Vec::with_capacity(terms_count);
    let mut term_bytes = Vec::with_capacity(terms_count);
    for _ in 0..terms_count {
      let term = TestUtil::random_realistic_unicode_string(random);
      term_bytes.push(BytesRef::from_string(&term));
      terms.push(term);
    }
    Self {
      terms,
      term_bytes,
      has_payloads,
    }
  }

  fn new_token_stream<R>(&self, random: &mut R) -> Result<OptionalNullPayloadTokenStream>
  where
    R: Rng + ?Sized,
  {
    let len = TestUtil::next_int(random, 1, 5) as usize;
    OptionalNullPayloadTokenStream::new(
      random,
      len,
      &self.terms,
      &self.term_bytes,
      self.has_payloads,
    )
  }
}

#[derive(Clone)]
struct OptionalNullPayloadTokenStream {
  delegate: RandomTokenStream,
}

impl OptionalNullPayloadTokenStream {
  fn new<R>(
    random: &mut R,
    len: usize,
    sample_terms: &[String],
    sample_term_bytes: &[BytesRef<Vec<u8>>],
    has_payloads: bool,
  ) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    Ok(Self {
      delegate: RandomTokenStream::new_with_random_payload(
        random,
        len,
        sample_terms,
        sample_term_bytes,
        |random| Self::random_payload(random, has_payloads),
      )?,
    })
  }

  fn random_payload<R>(random: &mut R, has_payloads: bool) -> Option<BytesRef<Vec<u8>>>
  where
    R: Rng + ?Sized,
  {
    if !has_payloads {
      return None;
    }
    let len = TestUtil::next_int(random, 1, 5) as usize;
    let mut bytes = vec![0; len];
    random.fill_bytes(&mut bytes);
    Some(BytesRef::from_bytes(bytes))
  }
}

impl Closeable for OptionalNullPayloadTokenStream {
  fn close(&mut self) -> Result<()> {
    self.delegate.close()
  }
}

impl TokenStream for OptionalNullPayloadTokenStream {
  fn increment_token(&mut self) -> Result<bool> {
    self.delegate.increment_token()
  }

  fn end(&mut self) -> Result<()> {
    self.delegate.end()
  }

  fn reset(&mut self) -> Result<()> {
    self.delegate.reset()
  }

  fn get_attribute_source(&self) -> &Attributes {
    self.delegate.get_attribute_source()
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    self.delegate.get_attribute_source_mut()
  }
}
