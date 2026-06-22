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
use crate::core::analysis::analyzer::{
  Analyzer, AnalyzerEnum, AnalyzerStoredValue, TokenStreamComponents,
};
use crate::core::analysis::reader::{ReaderEnum, StringReader};
use crate::core::analysis::token_attributes::payload_attribute;
use crate::core::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::core::analysis::token_filter::{TokenFilter, TokenFilterBase};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::field::{Field, Store};
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::document::text_field::TextField;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_terms::{get_term_postings_enum_with_flag, get_terms};
use crate::core::index::postings_enum::{PAYLOADS, PostingsEnum};
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::store::directory::Directory;
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::canned_token_stream::CannedTokenStream;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::analysis::mock_tokenizer::MockTokenizer;
use crate::test::core::analysis::{mock_tokenizer, token};
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  at_least, get_only_leaf_reader, new_directory_shared, new_index_writer_config,
  new_index_writer_config_with_analyzer, new_log_merge_policy, random, random_from_seed,
};
use crate::test::core::util::test_util::TestUtil;
use parking_lot::Mutex;
use rand::RngExt;
use rand_chacha::rand_core::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

#[allow(dead_code)] // for quick search
pub struct TestPayloads;

#[test]
fn test_payload() -> Result<()> {
  let payload: BytesRef<Vec<u8>> = BytesRef::from_string("This is a test!");
  assert_eq!("This is a test!".len(), payload.length);

  let clone = payload.clone();
  assert_eq!(payload.length, clone.length);
  for i in 0..payload.length {
    assert_eq!(
      payload.bytes[i + payload.offset],
      clone.bytes[i + clone.offset]
    );
  }

  Ok(())
}
// Tests whether the DocumentWriter and SegmentMerger correctly enable the
// payload bit in the FieldInfo
#[test]
fn test_payload_field_bit() -> Result<()> {
  let mut random = random();
  let ram = new_directory_shared(&mut random)?;
  let analyzer = PayloadAnalyzer::new();
  analyzer.set_payload_data("f2", "somedata".as_bytes().to_vec(), 0, 1);
  let writer = IndexWriter::new(
    ram.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;

  let mut d = Document::new();
  d.add(TextField::from_string(
    "f1",
    "This field has no payloads",
    Store::No,
  )?);
  // this field will have payloads in all docs, however not for all term positions,
  // so this field is used to check if the DocumentWriter correctly enables the payloads bit
  // even if only some term positions have payloads
  d.add(TextField::from_string(
    "f2",
    "This field has payloads in all docs",
    Store::No,
  )?);
  d.add(TextField::from_string(
    "f2",
    "This field has payloads in all docs NO PAYLOAD",
    Store::No,
  )?);
  // this field is used to verify if the SegmentMerger enables payloads for a field if it has
  // payloads
  // enabled in only some documents
  d.add(TextField::from_string(
    "f3",
    "This field has payloads in some docs",
    Store::No,
  )?);
  writer.add_document(d)?;
  writer.close()?;
  drop(writer);

  let reader = directory_reader::open(ram.clone())?;
  let leaf = get_only_leaf_reader(&reader)?;
  let fi = leaf.get_field_infos()?;
  assert!(
    !fi
      .field_info_by_name("f1")
      .ok_or_else(|| LuceneError::illegal_state("field f1 not found"))?
      .has_payloads()
  );
  assert!(
    fi.field_info_by_name("f2")
      .ok_or_else(|| LuceneError::illegal_state("field f2 not found"))?
      .has_payloads()
  );
  assert!(
    !fi
      .field_info_by_name("f3")
      .ok_or_else(|| LuceneError::illegal_state("field f3 not found"))?
      .has_payloads()
  );

  let analyzer = PayloadAnalyzer::new();
  analyzer.set_payload_data("f2", "somedata".as_bytes().to_vec(), 0, 1);
  analyzer.set_payload_data("f3", "somedata".as_bytes().to_vec(), 0, 3);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_open_mode(OpenMode::Create);
  let writer = IndexWriter::new(ram.clone(), iwc)?;
  let mut d = Document::new();
  d.add(TextField::from_string(
    "f1",
    "This field has no payloads",
    Store::No,
  )?);
  d.add(TextField::from_string(
    "f2",
    "This field has payloads in all docs",
    Store::No,
  )?);
  d.add(TextField::from_string(
    "f2",
    "This field has payloads in all docs",
    Store::No,
  )?);
  d.add(TextField::from_string(
    "f3",
    "This field has payloads in some docs",
    Store::No,
  )?);
  writer.add_document(d)?;
  writer.force_merge(1)?;
  writer.close()?;

  let reader = directory_reader::open(ram)?;
  let leaf = get_only_leaf_reader(&reader)?;
  let fi = leaf.get_field_infos()?;
  assert!(
    !fi
      .field_info_by_name("f1")
      .ok_or_else(|| LuceneError::illegal_state("field f1 not found"))?
      .has_payloads()
  );
  assert!(
    fi.field_info_by_name("f2")
      .ok_or_else(|| LuceneError::illegal_state("field f2 not found"))?
      .has_payloads()
  );
  assert!(
    fi.field_info_by_name("f3")
      .ok_or_else(|| LuceneError::illegal_state("field f3 not found"))?
      .has_payloads()
  );

  Ok(())
}

// #[test]
fn test_payloads_encoding() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  perform_test(dir)
}

fn perform_test<D>(dir: Arc<D>) -> Result<()>
where
  D: Directory,
{
  let mut random = random();
  let analyzer = PayloadAnalyzer::new();
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer.clone());
  iwc.set_open_mode(OpenMode::Create);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let skip_interval = 16;
  let num_terms = 5;
  let field_name = "f1";
  let num_docs = skip_interval + 1;
  let terms = generate_terms(field_name, num_terms);
  let mut content = String::new();
  for term in &terms {
    content.push_str(&term.text()?);
    content.push(' ');
  }

  let payload_data_length = num_terms * num_docs * 2 + num_terms * num_docs * (num_docs - 1) / 2;
  let mut payload_data = generate_random_data_with_len(&mut random, payload_data_length);

  let mut d = Document::new();
  d.add(TextField::from_string(field_name, &content, Store::No)?);
  let mut offset = 0;
  for _ in 0..2 * num_docs {
    analyzer.set_payload_data(field_name, payload_data.clone(), offset, 1);
    offset += num_terms;
    writer.add_document(d.clone())?;
  }

  writer.commit()?;

  for i in 0..num_docs {
    analyzer.set_payload_data(field_name, payload_data.clone(), offset, i);
    offset += i * num_terms;
    writer.add_document(d.clone())?;
  }

  writer.force_merge(1)?;
  writer.close()?;

  let reader = directory_reader::open(dir.clone())?;

  let mut verify_payload_data = vec![0; payload_data_length];
  offset = 0;
  let mut tps = Vec::new();
  for term in &terms {
    tps.push(
      get_term_postings_enum_with_flag(
        &reader,
        &term.field,
        &BytesRef::from_string(&term.text()?),
        PAYLOADS as i32,
      )?
      .unwrap(),
    );
  }

  while tps[0].next_doc()? != NO_MORE_DOCS {
    for postings in tps.iter_mut().skip(1) {
      postings.next_doc()?;
    }
    let freq = tps[0].freq()?;

    for _ in 0..freq {
      for postings in &mut tps {
        postings.next_position()?;
        if let Some(payload) = postings.get_payload()? {
          let payload = payload.as_ref();
          let end = offset + payload.length;
          verify_payload_data[offset..end]
            .copy_from_slice(&payload.bytes[payload.offset..payload.offset + payload.length]);
          offset = end;
        }
      }
    }
  }
  assert_byte_array_equals(payload_data.as_ref(), verify_payload_data.as_ref());

  let mut tp = get_term_postings_enum_with_flag(
    &reader,
    terms[0].field(),
    &BytesRef::from_string(&terms[0].text()?),
    PAYLOADS as i32,
  )?
  .ok_or_else(|| LuceneError::illegal_state("term postings not found"))?;
  tp.next_doc()?;
  tp.next_position()?;
  tp.next_doc()?;
  tp.next_position()?;
  let payload = tp
    .get_payload()?
    .ok_or_else(|| LuceneError::illegal_state("payload missing"))?;
  assert_eq!(1, payload.length);
  assert_eq!(payload.bytes[payload.offset], payload_data[num_terms]);
  tp.next_doc()?;
  tp.next_position()?;

  tp.advance(5)?;
  tp.next_position()?;
  let payload = tp
    .get_payload()?
    .ok_or_else(|| LuceneError::illegal_state("payload missing"))?;
  assert_eq!(1, payload.length);
  assert_eq!(payload.bytes[payload.offset], payload_data[5 * num_terms]);

  let mut tp = get_term_postings_enum_with_flag(
    &reader,
    terms[1].field(),
    &BytesRef::from_string(&terms[1].text()?),
    PAYLOADS as i32,
  )?
  .ok_or_else(|| LuceneError::illegal_state("term postings not found"))?;
  tp.next_doc()?;
  tp.next_position()?;
  assert_eq!(1, tp.get_payload()?.unwrap().length);
  tp.advance(skip_interval as i32 - 1)?;
  tp.next_position()?;
  assert_eq!(1, tp.get_payload()?.unwrap().length);
  tp.advance(2 * skip_interval as i32 - 1)?;
  tp.next_position()?;
  assert_eq!(1, tp.get_payload()?.unwrap().length);
  tp.advance(3 * skip_interval as i32 - 1)?;
  tp.next_position()?;
  assert_eq!(
    3 * skip_interval - 2 * num_docs - 1,
    tp.get_payload()?.unwrap().length
  );

  let analyzer = PayloadAnalyzer::new();
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer.clone());
  iwc.set_open_mode(OpenMode::Create);
  let writer = IndexWriter::new(dir.clone(), iwc)?;
  let single_term = "lucene";

  let mut d = Document::new();
  d.add(TextField::from_string(field_name, single_term, Store::No)?);
  payload_data = generate_random_data_with_len(&mut random, 2000);
  analyzer.set_payload_data(field_name, payload_data.clone(), 100, 1500);
  writer.add_document(d)?;

  writer.force_merge(1)?;
  writer.close()?;

  let reader = directory_reader::open(dir)?;
  let mut tp = get_term_postings_enum_with_flag(
    &reader,
    field_name,
    &BytesRef::from_string(single_term),
    PAYLOADS as i32,
  )?
  .ok_or_else(|| LuceneError::illegal_state("term postings not found"))?;
  tp.next_doc()?;
  tp.next_position()?;

  let br = tp
    .get_payload()?
    .ok_or_else(|| LuceneError::illegal_state("payload missing"))?;
  assert_byte_array_equals_range(
    payload_data.as_ref(),
    br.bytes.as_slice(),
    br.offset,
    br.length,
  );
  Ok(())
}

fn generate_random_data<R>(random: &mut R, data: &mut [u8])
where
  R: Rng + ?Sized,
{
  let s = TestUtil::random_fixed_byte_length_unicode_string(random, data.len());
  let b = s.as_bytes();

  debug_assert_eq!(b.len(), data.len());
  data.copy_from_slice(b)
}
fn generate_random_data_with_len<R>(random: &mut R, n: usize) -> Vec<u8>
where
  R: Rng + ?Sized,
{
  let mut data = vec![0; n];
  generate_random_data(random, &mut data);
  data
}

fn generate_terms(field_name: &str, n: usize) -> Vec<Term> {
  let max_digits = (n as f64).log10() as i32;
  let mut terms = Vec::with_capacity(n);

  for i in 0..n {
    let mut s = String::from("t");
    let zeros = max_digits - (i as f64).log10() as i32;

    for _ in 0..zeros {
      s.push('0');
    }

    s.push_str(&i.to_string());
    terms.push(Term::from_text(field_name, &s));
  }

  terms
}
fn assert_byte_array_equals(b1: &[u8], b2: &[u8]) {
  assert_eq!(
    b1.len(),
    b2.len(),
    "Byte arrays have different lengths: {}, {}",
    b1.len(),
    b2.len()
  );

  for (i, (&v1, &v2)) in b1.iter().zip(b2.iter()).enumerate() {
    assert_eq!(
      v1, v2,
      "Byte arrays different at index {}: {}, {}",
      i, v1, v2
    );
  }
}

fn assert_byte_array_equals_range(b1: &[u8], b2: &[u8], b2_offset: usize, b2_length: usize) {
  assert_eq!(
    b1.len(),
    b2_length,
    "Byte arrays have different lengths: {}, {}",
    b1.len(),
    b2_length
  );

  for (i, &v1) in b1.iter().enumerate() {
    let v2 = b2[b2_offset + i];
    assert_eq!(
      v1, v2,
      "Byte arrays different at index {}: {}, {}",
      i, v1, v2
    );
  }
}
#[derive(Clone)]
struct PayloadData {
  data: Vec<u8>,
  offset: usize,
  length: usize,
}

impl PayloadData {
  fn new(data: Vec<u8>, offset: usize, length: usize) -> Self {
    Self {
      data,
      offset,
      length,
    }
  }
}

struct PayloadAnalyzer {
  field_to_data: Arc<Mutex<HashMap<String, PayloadData>>>,
  stored_value: AnalyzerStoredValue,
}

impl Clone for PayloadAnalyzer {
  fn clone(&self) -> Self {
    Self {
      field_to_data: self.field_to_data.clone(),
      stored_value: AnalyzerStoredValue::per_field(),
    }
  }
}

impl PayloadAnalyzer {
  fn new() -> Self {
    Self {
      field_to_data: Arc::new(Mutex::new(HashMap::new())),
      stored_value: AnalyzerStoredValue::per_field(),
    }
  }

  fn set_payload_data(&self, field: &str, data: Vec<u8>, offset: usize, length: usize) {
    self
      .field_to_data
      .lock()
      .insert(field.to_string(), PayloadData::new(data, offset, length));
  }
}

impl Analyzer for PayloadAnalyzer {
  fn create_components(&self, field_name: &str) -> Result<TokenStreamComponents> {
    let tokenizer = MockTokenizer::new(random());
    let token_stream: Box<dyn TokenStream + Send + Sync> =
      if self.field_to_data.lock().contains_key(field_name) {
        Box::new(PayloadFilter::new(
          tokenizer,
          field_name.to_string(),
          self.field_to_data.clone(),
        ))
      } else {
        Box::new(tokenizer)
      };

    Ok(TokenStreamComponents::new(token_stream, None))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(PayloadAnalyzer);

impl From<PayloadAnalyzer> for AnalyzerEnum {
  fn from(analyzer: PayloadAnalyzer) -> Self {
    AnalyzerEnum::Custom(Box::new(analyzer))
  }
}
struct PayloadFilter<TS>
where
  TS: TokenStream,
{
  token_filter_base: TokenFilterBase<TS>,
  field_name: String,
  field_to_data: Arc<Mutex<HashMap<String, PayloadData>>>,
  payload_data: Option<PayloadData>,
  offset: usize,
}

impl<TS> PayloadFilter<TS>
where
  TS: TokenStream,
{
  fn new(
    input: TS,
    field_name: String,
    field_to_data: Arc<Mutex<HashMap<String, PayloadData>>>,
  ) -> Self {
    Self {
      token_filter_base: TokenFilterBase::new(input),
      field_name,
      field_to_data,
      payload_data: None,
      offset: 0,
    }
  }
}

impl<TS> TokenStream for PayloadFilter<TS>
where
  TS: TokenStream,
{
  fn increment_token(&mut self) -> Result<bool> {
    if !self.token_filter_base.input.increment_token()? {
      return Ok(false);
    }

    let payload_data = self
      .payload_data
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("payload data is not set"))?;
    let attr = self.token_filter_base.input.get_attribute_source_mut();
    let len = attr.length()?;
    let term = attr.buffer()?[..len].iter().collect::<String>();

    if self.offset + payload_data.length <= payload_data.data.len() && !term.ends_with("NO PAYLOAD")
    {
      let payload =
        BytesRef::from_slice(payload_data.data.clone(), self.offset, payload_data.length);
      attr.set_payload(Some(payload))?;
      self.offset += payload_data.length;
    } else {
      attr.set_payload(None)?;
    }

    Ok(true)
  }

  fn end(&mut self) -> Result<()> {
    self.token_filter_base.end()
  }

  fn reset(&mut self) -> Result<()> {
    self.token_filter_base.reset()?;
    self.payload_data = self.field_to_data.lock().get(&self.field_name).cloned();
    self.offset = self
      .payload_data
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("payload data is not set"))?
      .offset;
    Ok(())
  }

  fn close(&mut self) -> Result<()> {
    self.token_filter_base.close()
  }

  fn set_reader(&mut self, input: ReaderEnum) -> Result<()> {
    self.token_filter_base.input.set_reader(input)
  }

  fn set_reader_test_point(&mut self) -> Result<()> {
    self.token_filter_base.input.set_reader_test_point()
  }

  fn get_attribute_source(&self) -> &Attributes {
    self.token_filter_base.input.get_attribute_source()
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    self.token_filter_base.input.get_attribute_source_mut()
  }
}

impl<TS> TokenFilter for PayloadFilter<TS> where TS: TokenStream {}

#[test]
fn test_thread_safety() -> Result<()> {
  let mut random = random();
  let num_threads = 5;
  let num_docs = at_least(&mut random, 50);
  let pool = Arc::new(ByteArrayPool::new(num_threads, 5));

  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;
  let field = "test";
  let random = Mutex::new(random);
  thread::scope(|scope| -> Result<()> {
    let mut ingesters = Vec::new();
    for _ in 0..num_threads {
      let writer = &writer;
      let pool = pool.clone();
      let random = &random;
      ingesters.push(scope.spawn(move || -> Result<()> {
        for _ in 0..num_docs {
          let mut d = Document::new();
          let mut random = random.lock();
          d.add(TextField::from_token_stream(
            field,
            FieldTokenStreamEnum::custom(PoolingPayloadTokenStream::new(
              &mut *random,
              pool.clone(),
            )),
          )?);
          writer.add_document(d)?;
        }
        Ok(())
      }));
    }

    for ingester in ingesters {
      ingester.join().expect("thread panicked")?;
    }
    Ok(())
  })?;

  writer.close()?;
  let reader = directory_reader::open(dir)?;
  let terms =
    get_terms(&reader, field)?.ok_or_else(|| LuceneError::illegal_state("terms missing"))?;
  let mut terms_enum = terms.iterator()?;
  while let Some(term) = terms_enum.next()? {
    let term_text = term.utf8_to_string()?;
    let mut tp = terms_enum.postings_with_flags(None, PAYLOADS as i32)?;
    while tp.next_doc()? != NO_MORE_DOCS {
      let freq = tp.freq()?;
      for _ in 0..freq {
        tp.next_position()?;
        let payload = tp
          .get_payload()?
          .ok_or_else(|| LuceneError::illegal_state("payload missing"))?;
        assert_eq!(term_text, payload.utf8_to_string()?);
      }
    }
  }
  drop(reader);
  assert_eq!(pool.size(), num_threads);
  Ok(())
}

struct PoolingPayloadTokenStream {
  payload: Option<Vec<u8>>,
  first: bool,
  pool: Arc<ByteArrayPool>,
  term: String,
  att: Attributes,
}

impl PoolingPayloadTokenStream {
  fn new<R>(random: &mut R, pool: Arc<ByteArrayPool>) -> Self
  where
    R: Rng + ?Sized,
  {
    let mut payload = pool.get();
    generate_random_data(random, &mut payload);
    let term = String::from_utf8_lossy(&payload).into_owned();
    Self {
      payload: Some(payload),
      first: true,
      pool,
      term,
      att: Attributes::default(),
    }
  }
}

impl TokenStream for PoolingPayloadTokenStream {
  fn increment_token(&mut self) -> Result<bool> {
    if !self.first {
      return Ok(false);
    }
    self.first = false;
    self.att.clear_attributes();
    self.att.append_str(Some(&self.term))?;
    self.att.set_payload(Some(BytesRef::from_bytes(
      self.payload.clone().unwrap_or_default(),
    )))?;
    Ok(true)
  }

  fn end(&mut self) -> Result<()> {
    self.default_end()
  }

  fn close(&mut self) -> Result<()> {
    if let Some(payload) = self.payload.take() {
      self.pool.release(payload);
    }
    Ok(())
  }

  fn get_attribute_source(&self) -> &Attributes {
    &self.att
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    &mut self.att
  }
}

struct ByteArrayPool {
  pool: Mutex<Vec<Vec<u8>>>,
}

impl ByteArrayPool {
  fn new(capacity: usize, size: usize) -> Self {
    let mut pool = Vec::new();
    for _ in 0..capacity {
      pool.push(vec![0; size]);
    }
    Self {
      pool: Mutex::new(pool),
    }
  }

  fn get(&self) -> Vec<u8> {
    self.pool.lock().remove(0)
  }

  fn release(&self, bytes: Vec<u8>) {
    self.pool.lock().push(bytes);
  }

  fn size(&self) -> usize {
    self.pool.lock().len()
  }
}

#[test]
fn test_across_fields() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer =
    MockAnalyzer::with_automaton(&mut random, mock_tokenizer::WHITESPACE.clone(), false);
  let writer = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), analyzer);
  let mut doc = Document::new();
  doc.add(TextField::from_string(
    "hasMaybepayload",
    "here we go",
    Store::Yes,
  )?);
  writer.add_document(&mut random, doc)?;
  writer.close(&mut random)?;
  drop(writer);

  let analyzer =
    MockAnalyzer::with_automaton(&mut random, mock_tokenizer::WHITESPACE.clone(), true);
  let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let writer = RandomIndexWriter::with_config(&mut random, dir, iwc);
  let mut doc = Document::new();
  doc.add(TextField::from_string(
    "hasMaybepayload2",
    "here we go",
    Store::Yes,
  )?);
  writer.add_document(&mut random, doc.clone())?;
  writer.add_document(&mut random, doc)?;
  writer.force_merge(&mut random, 1)?;
  writer.close(&mut random)?;

  Ok(())
}
/// some docs have payload att, some not
#[test]
fn test_mixup_docs() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let writer = RandomIndexWriter::with_config(&mut random, dir, iwc);

  let mut doc = Document::new();
  let v = random_from_seed(random.random());
  let mut ts = MockTokenizer::new(v);
  ts.set_reader(StringReader::new("here we go").into())?;
  let field = Field::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(ts),
    crate::core::document::text_field::TYPE_NOT_STORED.clone(),
  )?;
  doc.add(field);
  writer.add_document(&mut random, doc)?;

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
  let field = Field::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(CannedTokenStream::new(vec![with_payload])),
    crate::core::document::text_field::TYPE_NOT_STORED.clone(),
  )?;
  let mut doc = Document::new();
  doc.add(field);
  writer.add_document(&mut random, doc)?;

  let v = random_from_seed(random.random());
  let mut ts =
    MockTokenizer::with_default_max_token_length(v, mock_tokenizer::WHITESPACE.clone(), true);
  ts.set_reader(StringReader::new("another").into())?;
  let field = Field::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(ts),
    crate::core::document::text_field::TYPE_NOT_STORED.clone(),
  )?;
  let mut doc = Document::new();
  doc.add(field);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  let terms = get_terms(&reader, "field")?.unwrap();
  let mut te = terms.iterator()?;
  assert!(te.seek_exact(&BytesRef::from_string("withPayload"))?);
  let mut de = te.postings_with_flags(None, PAYLOADS as i32)?;
  de.next_doc()?;
  de.next_position()?;
  assert_eq!(
    &BytesRef::from_string("test"),
    de.get_payload()?
      .ok_or_else(|| LuceneError::illegal_state("payload missing"))?
      .as_ref()
  );
  writer.close(&mut random)?;

  Ok(())
}

/// some field instances have payload att, some not
#[test]
fn test_mixup_multi_valued() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir);
  let mut doc = Document::new();
  let v = random_from_seed(random.random());
  let mut ts = MockTokenizer::new(v);
  ts.set_reader(StringReader::new("here we go").into())?;
  let field = Field::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(ts),
    crate::core::document::text_field::TYPE_NOT_STORED.clone(),
  )?;
  doc.add(field);

  let mut t = token::with_range(Some("withPayload"), 0, 11)?;
  t.sub.token.set_payload(Some(BytesRef::from_string("test")));
  assert!(t.get_attribute_name()?.contains(payload_attribute::NAME));
  let fields2 = Field::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(CannedTokenStream::new(vec![t])),
    crate::core::document::text_field::TYPE_NOT_STORED.clone(),
  )?;
  doc.add(fields2);

  let v = random_from_seed(random.random());
  let mut ts =
    MockTokenizer::with_default_max_token_length(v, mock_tokenizer::WHITESPACE.clone(), true);
  ts.set_reader(StringReader::new("nopayload").into())?;
  let field3 = Field::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(ts),
    crate::core::document::text_field::TYPE_NOT_STORED.clone(),
  )?;
  doc.add(field3);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  let leaf = get_only_leaf_reader(&reader)?;
  let mut de = leaf
    .postings_with_flag(&Term::from_text("field", "withPayload"), PAYLOADS as i32)?
    .ok_or_else(|| LuceneError::illegal_state("withPayload postings not found"))?;
  de.next_doc()?;
  de.next_position()?;
  assert_eq!(
    &BytesRef::from_string("test"),
    de.get_payload()?
      .ok_or_else(|| LuceneError::illegal_state("payload missing"))?
      .as_ref()
  );
  writer.close(&mut random)?;

  Ok(())
}
