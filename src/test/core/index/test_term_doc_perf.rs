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
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::analysis::tokenizer::{Tokenizer, TokenizerBase};
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_terms;
use crate::core::index::postings_enum::NONE;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::store::directory::Directory;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer,
  new_log_merge_policy_with_merge_factor, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::prelude::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[allow(dead_code)] // for quick search
struct TestTermDocPerf;
struct RepeatingTokenizer {
  tokenizer_base: TokenizerBase,
  random: StdRng,
  percent_docs: f32,
  max_tf: i32,
  num: i32,
  value: String,
}

impl RepeatingTokenizer {
  pub fn new(value: &str, random: StdRng, percent_docs: f32, max_tf: i32) -> Self {
    Self {
      tokenizer_base: TokenizerBase::new(Attributes::default()),
      random,
      percent_docs,
      max_tf,
      num: 0,
      value: value.to_string(),
    }
  }
}

impl crate::core::util::close::Closeable for RepeatingTokenizer {
  fn close(&mut self) -> Result<()> {
    crate::core::util::close::Closeable::close(&mut self.tokenizer_base)
  }
}

impl TokenStream for RepeatingTokenizer {
  fn increment_token(&mut self) -> Result<bool> {
    self.num -= 1;
    if self.num >= 0 {
      self
        .tokenizer_base
        .token_stream_base
        .att
        .clear_attributes()?;
      self
        .tokenizer_base
        .token_stream_base
        .att
        .append_str(Some(&self.value))?;
      return Ok(true);
    }
    Ok(false)
  }

  fn end(&mut self) -> Result<()> {
    self.tokenizer_base.end()
  }

  fn reset(&mut self) -> Result<()> {
    self.tokenizer_base.reset()?;
    if self.random.random::<f32>() < self.percent_docs {
      self.num = self.random.random_range(0..self.max_tf) + 1;
    } else {
      self.num = 0;
    }
    Ok(())
  }

  fn get_attribute_source(&self) -> &Attributes {
    self.tokenizer_base.get_attribute_source()
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    self.tokenizer_base.get_attribute_source_mut()
  }

  fn set_reader(&mut self, input: ReaderEnum) -> Result<()> {
    self.tokenizer_base.set_reader(input)
  }
}

impl Tokenizer for RepeatingTokenizer {
  fn get_tokenizer_base_mut(&mut self) -> &mut TokenizerBase {
    &mut self.tokenizer_base
  }

  fn get_tokenizer_base(&self) -> &TokenizerBase {
    &self.tokenizer_base
  }
}

struct RepeatingAnalyzer {
  value: String,
  random: Mutex<StdRng>,
  percent_docs: f32,
  max_tf: i32,
  stored_value: AnalyzerStoredValue,
}

impl RepeatingAnalyzer {
  fn new<R>(value: &str, random: &mut R, percent_docs: f32, max_tf: i32) -> Self
  where
    R: Rng + ?Sized,
  {
    Self {
      value: value.to_string(),
      random: Mutex::new(StdRng::seed_from_u64(random.random())),
      percent_docs,
      max_tf,
      stored_value: AnalyzerStoredValue::new(),
    }
  }

  fn next_random(&self) -> StdRng {
    StdRng::seed_from_u64(self.random.lock().expect("random mutex poisoned").random())
  }
}

impl Analyzer for RepeatingAnalyzer {
  fn create_components(&self, _field: &str) -> Result<TokenStreamComponents> {
    Ok(TokenStreamComponents::new(
      Box::new(RepeatingTokenizer::new(
        &self.value,
        self.next_random(),
        self.percent_docs,
        self.max_tf,
      )) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(RepeatingAnalyzer);

impl From<RepeatingAnalyzer> for AnalyzerEnum {
  fn from(analyzer: RepeatingAnalyzer) -> Self {
    AnalyzerEnum::Custom(Box::new(analyzer))
  }
}

fn add_docs<D, R>(
  random: &mut R,
  dir: Arc<D>,
  ndocs: i32,
  field: &str,
  val: &str,
  max_tf: i32,
  percent_docs: f32,
) -> Result<()>
where
  D: Directory + 'static,
  R: Rng + ?Sized,
{
  let analyzer = RepeatingAnalyzer::new(val, random, percent_docs, max_tf);
  let mut doc = Document::new();
  doc.add(StringField::from_string(field, val, Store::No)?);

  let mut iwc = new_index_writer_config_with_analyzer(random, analyzer)?;
  iwc.set_open_mode(OpenMode::Create);
  iwc.set_max_buffered_docs(100);
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(random, 100)?);

  let writer = IndexWriter::new(dir, iwc)?;
  for _ in 0..ndocs {
    writer.add_document(doc.clone())?;
  }
  writer.force_merge(1)?;
  writer.close()
}

pub fn do_test(iter: i32, ndocs: i32, max_tf: i32, percent_docs: f32) -> Result<i32> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let start = Instant::now();
  add_docs(
    &mut random,
    dir.clone(),
    ndocs,
    "foo",
    "val",
    max_tf,
    percent_docs,
  )?;
  let elapsed = start.elapsed();
  if cfg!(feature = "test_log_verbose") {
    println!(
      "milliseconds for creation of {} docs = {}",
      ndocs,
      elapsed.as_millis()
    );
  }

  let reader = directory_reader::open(dir)?;
  let terms = multi_terms::get_terms(&reader, "foo")?.expect("terms should exist");
  let mut terms_enum = terms.iterator()?;

  let start = Instant::now();

  let mut ret: i32 = 0;
  let mut tdocs = None;
  let mut docs_random = StdRng::seed_from_u64(random.random());
  for _ in 0..iter {
    terms_enum.seek_ceil(&BytesRef::from_string("val"))?;
    tdocs = Some(TestUtil::docs(
      &mut docs_random,
      &mut terms_enum,
      tdocs,
      NONE as i32,
    )?);
    let term_docs = tdocs.as_mut().expect("postings enum must exist");
    while term_docs.next_doc()? != NO_MORE_DOCS {
      ret = ret.wrapping_add(term_docs.doc_id());
    }
  }

  let elapsed = start.elapsed();
  if cfg!(feature = "test_log_verbose") {
    println!(
      "milliseconds for {} TermDocs iteration: {}",
      iter,
      elapsed.as_millis()
    );
  }

  reader.close()?;
  Ok(ret)
}

#[test]
fn test_term_doc_perf() -> Result<()> {
  // performance test for 10% of documents containing a term
  do_test(100000, 10000, 3, 0.1)?;
  Ok(())
}
