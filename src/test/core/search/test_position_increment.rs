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
use crate::core::analysis::analyzer::{Analyzer, AnalyzerStoredValue, TokenStreamComponents};
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_stream::{TokenStream, default_attribute};
use crate::core::analysis::tokenizer::{Tokenizer, TokenizerBase};
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::index::BytesRef;
use crate::core::index::multi_terms::get_term_postings_enum;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::term::Term;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::multi_phrase_query::MultiPhraseQuery;
use crate::core::search::phrase_query::{Builder as PhraseQueryBuilder, PhraseQuery};
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, new_text_field, random,
};
use std::collections::HashMap;

/** Term position unit test. */
#[allow(dead_code)] // for quick search
struct TestPositionIncrement;

#[allow(dead_code)]
const VERBOSE: bool = false;

#[test]
fn test_set_position() -> Result<()> {
  let mut random = random();
  let analyzer = Box::new(PositionIncrementAnalyzer {
    stored_value: AnalyzerStoredValue::new(),
  }) as Box<dyn Analyzer>;
  let store = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::with_analyzer(&mut random, store, analyzer);
  let mut d = Document::new();
  let mut field_to_type = HashMap::new();
  d.add(new_text_field(
    &mut random,
    "field",
    "bogus",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(d)?;
  let reader = writer.get_reader()?;
  writer.close()?;

  let searcher = new_searcher_with_reader(reader)?;

  let mut pos = get_term_postings_enum(
    searcher.get_index_reader(),
    "field",
    &BytesRef::from_string("1"),
  )?
  .expect("postings for term 1 should exist");
  pos.next_doc()?;
  // first token should be at position 0
  assert_eq!(0, pos.next_position()?);

  let mut pos = get_term_postings_enum(
    searcher.get_index_reader(),
    "field",
    &BytesRef::from_string("2"),
  )?
  .expect("postings for term 2 should exist");
  pos.next_doc()?;
  // second token should be at position 2
  assert_eq!(2, pos.next_position()?);

  let mut q = PhraseQuery::from_terms_no_slop("field", &["1", "2"])?;
  let mut hits = searcher.search(q, 1000)?.score_docs;
  assert_eq!(0, hits.len());

  // same as previous, using the builder with implicit positions
  let mut builder = PhraseQueryBuilder::new();
  builder.add_term(Term::from_text("field", "1"))?;
  builder.add_term(Term::from_text("field", "2"))?;
  q = builder.build()?;
  hits = searcher.search(q, 1000)?.score_docs;
  assert_eq!(0, hits.len());

  // same as previous, just specify positions explicitely.
  builder = PhraseQueryBuilder::new();
  builder.add(Term::from_text("field", "1"), 0)?;
  builder.add(Term::from_text("field", "2"), 1)?;
  q = builder.build()?;
  hits = searcher.search(q, 1000)?.score_docs;
  assert_eq!(0, hits.len());

  // specifying correct positions should find the phrase.
  builder = PhraseQueryBuilder::new();
  builder.add(Term::from_text("field", "1"), 0)?;
  builder.add(Term::from_text("field", "2"), 2)?;
  q = builder.build()?;
  hits = searcher.search(q, 1000)?.score_docs;
  assert_eq!(1, hits.len());

  q = PhraseQuery::from_terms_no_slop("field", &["2", "3"])?;
  hits = searcher.search(q, 1000)?.score_docs;
  assert_eq!(1, hits.len());

  q = PhraseQuery::from_terms_no_slop("field", &["3", "4"])?;
  hits = searcher.search(q, 1000)?.score_docs;
  assert_eq!(0, hits.len());

  // phrase query would find it when correct positions are specified.
  builder = PhraseQueryBuilder::new();
  builder.add(Term::from_text("field", "3"), 0)?;
  builder.add(Term::from_text("field", "4"), 0)?;
  q = builder.build()?;
  hits = searcher.search(q, 1000)?.score_docs;
  assert_eq!(1, hits.len());

  // phrase query should fail for non existing searched term
  // even if there exist another searched terms in the same searched position.
  builder = PhraseQueryBuilder::new();
  builder.add(Term::from_text("field", "3"), 0)?;
  builder.add(Term::from_text("field", "9"), 0)?;
  q = builder.build()?;
  hits = searcher.search(q, 1000)?.score_docs;
  assert_eq!(0, hits.len());

  // multi-phrase query should succed for non existing searched term
  // because there exist another searched terms in the same searched position.
  let mut mqb = MultiPhraseQuery::builder();
  mqb.add_terms_with_position(
    &[Term::from_text("field", "3"), Term::from_text("field", "9")],
    0,
  )?;
  hits = searcher.search(mqb.build(), 1000)?.score_docs;
  assert_eq!(1, hits.len());

  q = PhraseQuery::from_terms_no_slop("field", &["2", "4"])?;
  hits = searcher.search(q, 1000)?.score_docs;
  assert_eq!(1, hits.len());

  q = PhraseQuery::from_terms_no_slop("field", &["3", "5"])?;
  hits = searcher.search(q, 1000)?.score_docs;
  assert_eq!(1, hits.len());

  q = PhraseQuery::from_terms_no_slop("field", &["4", "5"])?;
  hits = searcher.search(q, 1000)?.score_docs;
  assert_eq!(1, hits.len());

  q = PhraseQuery::from_terms_no_slop("field", &["2", "5"])?;
  hits = searcher.search(q, 1000)?.score_docs;
  assert_eq!(0, hits.len());

  Ok(())
}

struct PositionIncrementAnalyzer {
  stored_value: AnalyzerStoredValue,
}

impl Analyzer for PositionIncrementAnalyzer {
  fn create_components(&self, _field_name: &str) -> Result<TokenStreamComponents> {
    Ok(TokenStreamComponents::new(
      Box::new(PositionIncrementTokenizer::new()) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(PositionIncrementAnalyzer);

struct PositionIncrementTokenizer {
  i: usize,
  tokenizer_base: TokenizerBase,
}

impl PositionIncrementTokenizer {
  const TOKENS: [&'static str; 5] = ["1", "2", "3", "4", "5"];
  const INCREMENTS: [i32; 5] = [1, 2, 1, 0, 1];

  fn new() -> Self {
    Self {
      i: 0,
      tokenizer_base: TokenizerBase::new(default_attribute()),
    }
  }
}

impl TokenStream for PositionIncrementTokenizer {
  fn increment_token(&mut self) -> Result<bool> {
    if self.i == Self::TOKENS.len() {
      return Ok(false);
    }

    let att = &mut self.tokenizer_base.token_stream_base.att;
    att.clear_attributes();
    att.append_str(Some(Self::TOKENS[self.i]))?;
    att.set_offset(self.i as i32, self.i as i32)?;
    att.set_position_increment(Self::INCREMENTS[self.i])?;
    self.i += 1;
    Ok(true)
  }

  fn end(&mut self) -> Result<()> {
    self.tokenizer_base.end()
  }

  fn reset(&mut self) -> Result<()> {
    self.tokenizer_base.reset()?;
    self.i = 0;
    Ok(())
  }

  fn close(&mut self) -> Result<()> {
    self.tokenizer_base.close()
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

impl Tokenizer for PositionIncrementTokenizer {
  fn get_tokenizer_base_mut(&mut self) -> &mut TokenizerBase {
    &mut self.tokenizer_base
  }

  fn get_tokenizer_base(&self) -> &TokenizerBase {
    &self.tokenizer_base
  }
}
