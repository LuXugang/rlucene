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
use crate::core::analysis::reader::StringReader;
use crate::core::analysis::token_attributes::payload_attribute;
use crate::core::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::index::BytesRef;
use crate::core::index::fields::Fields;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::postings_enum::{ALL, PostingsEnum};
use crate::core::index::term_vectors::TermVectors;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::util::attribute::Attribute;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::canned_token_stream::CannedTokenStream;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::analysis::mock_tokenizer::MockTokenizer;
use crate::test::core::analysis::{mock_tokenizer, token};
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy, random,
  random_from_seed,
};
use rand::RngExt;

#[allow(dead_code)] // for quick search
pub struct TestPayloadsOnVectors;

/// some docs have payload att, some not
#[test]
fn test_mixup_docs() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let writer = RandomIndexWriter::with_config(&mut random, dir, iwc);

  let mut doc = Document::new();
  let mut custom_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  custom_type.set_store_term_vectors(true)?;
  custom_type.set_store_term_vector_positions(true)?;
  custom_type.set_store_term_vector_payloads(true)?;
  custom_type.set_store_term_vector_offsets(random.random_bool(0.5))?;
  let mut ts = MockTokenizer::with_default_max_token_length(
    random_from_seed(random.random()),
    mock_tokenizer::WHITESPACE.clone(),
    true,
  );
  ts.set_reader(StringReader::new("here we go").into())?;
  let field = Field::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(ts),
    custom_type.clone(),
  )?;
  doc.add(field);
  writer.add_document(doc)?;

  let mut with_payload = token::with_range(Some("withPayload"), 0, 11)?;
  with_payload
    .sub
    .token
    .set_payload(Some(BytesRef::from_string("test")));
  assert!(
    with_payload
      .get_attribute_name()?
      .contains(payload_attribute::NAME)
  );
  let mut doc = Document::new();
  let field = Field::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(CannedTokenStream::new(vec![with_payload])),
    custom_type.clone(),
  )?;
  doc.add(field);
  writer.add_document(doc)?;

  let mut ts = MockTokenizer::with_default_max_token_length(
    random_from_seed(random.random()),
    mock_tokenizer::WHITESPACE.clone(),
    true,
  );
  ts.set_reader(StringReader::new("another").into())?;
  let mut doc = Document::new();
  let field = Field::from_token_stream("field", FieldTokenStreamEnum::custom(ts), custom_type)?;
  doc.add(field);
  writer.add_document(doc)?;

  let reader = writer.get_reader()?;
  let mut term_vectors = reader.term_vectors()?;
  let fields = term_vectors
    .get(1)?
    .ok_or_else(|| LuceneError::illegal_state("term vectors missing"))?;
  let terms = fields
    .terms("field")?
    .ok_or_else(|| LuceneError::illegal_state("field term vectors missing"))?;
  let mut terms_enum = terms.iterator()?;
  assert!(terms_enum.seek_exact(&BytesRef::from_string("withPayload"))?);
  let mut de = terms_enum.postings_with_flags(None, ALL as i32)?;
  assert_eq!(0, de.next_doc()?);
  assert_eq!(0, de.next_position()?);
  assert_eq!(
    &BytesRef::from_string("test"),
    de.get_payload()?
      .ok_or_else(|| LuceneError::illegal_state("payload missing"))?
      .as_ref()
  );
  assert_eq!(NO_MORE_DOCS, de.next_doc()?);
  writer.close()?;

  Ok(())
}

/// some field instances have payload att, some not
#[test]
fn test_mixup_multi_valued() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir);
  let mut doc = Document::new();
  let mut custom_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  custom_type.set_store_term_vectors(true)?;
  custom_type.set_store_term_vector_positions(true)?;
  custom_type.set_store_term_vector_payloads(true)?;
  custom_type.set_store_term_vector_offsets(random.random_bool(0.5))?;

  let mut ts = MockTokenizer::with_default_max_token_length(
    random_from_seed(random.random()),
    mock_tokenizer::WHITESPACE.clone(),
    true,
  );
  ts.set_reader(StringReader::new("here we go").into())?;
  let field = Field::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(ts),
    custom_type.clone(),
  )?;
  doc.add(field);

  let mut with_payload = token::with_range(Some("withPayload"), 0, 11)?;
  with_payload
    .sub
    .token
    .set_payload(Some(BytesRef::from_string("test")));
  assert!(
    with_payload
      .get_attribute_name()?
      .contains(payload_attribute::NAME)
  );
  let field2 = Field::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(CannedTokenStream::new(vec![with_payload])),
    custom_type.clone(),
  )?;
  doc.add(field2);

  let mut ts = MockTokenizer::with_default_max_token_length(
    random_from_seed(random.random()),
    mock_tokenizer::WHITESPACE.clone(),
    true,
  );
  ts.set_reader(StringReader::new("nopayload").into())?;
  let field3 = Field::from_token_stream("field", FieldTokenStreamEnum::custom(ts), custom_type)?;
  doc.add(field3);
  writer.add_document(doc)?;

  let reader = writer.get_reader()?;
  let mut term_vectors = reader.term_vectors()?;
  let fields = term_vectors
    .get(0)?
    .ok_or_else(|| LuceneError::illegal_state("term vectors missing"))?;
  let terms = fields
    .terms("field")?
    .ok_or_else(|| LuceneError::illegal_state("field term vectors missing"))?;
  let mut terms_enum = terms.iterator()?;
  assert!(terms_enum.seek_exact(&BytesRef::from_string("withPayload"))?);
  let mut de = terms_enum.postings_with_flags(None, ALL as i32)?;
  assert_eq!(0, de.next_doc()?);
  assert_eq!(3, de.next_position()?);
  assert_eq!(
    &BytesRef::from_string("test"),
    de.get_payload()?
      .ok_or_else(|| LuceneError::illegal_state("payload missing"))?
      .as_ref()
  );
  assert_eq!(NO_MORE_DOCS, de.next_doc()?);
  writer.close()?;

  Ok(())
}

#[test]
fn test_payloads_without_positions() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir);
  let mut doc = Document::new();
  let mut custom_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  custom_type.set_store_term_vectors(true)?;
  custom_type.set_store_term_vector_positions(false)?;
  custom_type.set_store_term_vector_payloads(true)?;
  custom_type.set_store_term_vector_offsets(random.random_bool(0.5))?;
  doc.add(Field::new("field", "foo", custom_type));

  assert!(matches!(
    writer.add_document(doc),
    Err(LuceneError::IllegalArgument(_))
  ));
  writer.close()?;

  Ok(())
}
