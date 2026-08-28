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
use crate::core::analysis::token_filter::{TokenFilter, TokenFilterBase};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::index::BytesRef;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_phrase_query::MultiPhraseQuery;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::top_docs::TopDocs;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::transition::Transition;
use crate::core::util::automation::transition_accessor::TransitionAccessor;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use crate::sandbox::search::term_automaton_query::TermAutomatonQuery;
use crate::sandbox::search::token_stream_to_term_automaton_query::TokenStreamToTermAutomatonQuery;
use crate::test_framework::core::analysis::canned_token_stream::CannedTokenStream;
use crate::test_framework::core::analysis::mock_token_filter::{EMPTY_STOPSET, MockTokenFilter};
use crate::test_framework::core::analysis::mock_tokenizer::{MockTokenizer, WHITESPACE};
use crate::test_framework::core::analysis::token::{self, Token};
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::query::RandomQuery;
use crate::test_framework::core::search::test_term_automaton_query::CustomTermAutomatonQuery;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader,
  new_text_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use parking_lot::Mutex;
use rand::prelude::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestTermAutomatonQuery;

// "comes * sun"
#[test]
fn test_basic1() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  // matches
  doc.add(new_text_field(
    &mut random,
    "field",
    "here comes the sun",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  // doesn't match
  doc.add(new_text_field(
    &mut random,
    "field",
    "here comes the other sun",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query = TermAutomatonQuery::new("field");
  let init = query.create_state();
  let s1 = query.create_state();
  query.add_transition(init, s1, "comes")?;
  let s2 = query.create_state();
  query.add_any_transition(s1, s2)?;
  let s3 = query.create_state();
  query.set_accept(s3, true);
  query.add_transition(s2, s3, "sun")?;
  query.finish()?;

  assert_eq!(1, searcher.search(query, 1)?.total_hits.value());

  writer.close(&mut random)?;
  searcher.get_index_reader().close()?;
  CloseableRef::close(dir.as_ref())
}

// "comes * (sun|moon)"
#[test]
fn test_basic_synonym() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  for text in ["here comes the sun", "here comes the moon"] {
    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "field",
      text,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(&mut random, doc)?;
  }
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query = TermAutomatonQuery::new("field");
  let init = query.create_state();
  let s1 = query.create_state();
  query.add_transition(init, s1, "comes")?;
  let s2 = query.create_state();
  query.add_any_transition(s1, s2)?;
  let s3 = query.create_state();
  query.set_accept(s3, true);
  query.add_transition(s2, s3, "sun")?;
  query.add_transition(s2, s3, "moon")?;
  query.finish()?;

  assert_eq!(2, searcher.search(query, 1)?.total_hits.value());

  writer.close(&mut random)?;
  searcher.get_index_reader().close()?;
  CloseableRef::close(dir.as_ref())
}

// "comes sun" or "comes * sun"
#[test]
fn test_basic_slop() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  for text in [
    "here comes the sun",
    "here comes sun",
    "here comes the other sun",
  ] {
    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "field",
      text,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(&mut random, doc)?;
  }
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query = TermAutomatonQuery::new("field");
  let init = query.create_state();
  let s1 = query.create_state();
  query.add_transition(init, s1, "comes")?;
  let s2 = query.create_state();
  query.add_any_transition(s1, s2)?;
  let s3 = query.create_state();
  query.set_accept(s3, true);
  query.add_transition(s1, s3, "sun")?;
  query.add_transition(s2, s3, "sun")?;
  query.finish()?;

  assert_eq!(2, searcher.search(query, 1)?.total_hits.value());

  writer.close(&mut random)?;
  searcher.get_index_reader().close()?;
  CloseableRef::close(dir.as_ref())
}

// Verify posLength is "respected" at query time: index "speedy wifi
// network", search on "fast wi fi network" using (simulated!)
// query-time syn filter to add "wifi" over "wi fi" with posLength=2.
// To make this real we need a version of TS2A that operates on whole
// terms, not characters.
#[test]
fn test_pos_length_at_query_time_mock() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  for text in [
    "speedy wifi network",
    "speedy wi fi network",
    "fast wifi network",
    "fast wi fi network",
    // doesn't match:
    "slow wi fi network",
  ] {
    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "field",
      text,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(&mut random, doc)?;
  }
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query = TermAutomatonQuery::new("field");
  let init = query.create_state();
  let s1 = query.create_state();
  query.add_transition(init, s1, "fast")?;
  query.add_transition(init, s1, "speedy")?;
  let s2 = query.create_state();
  let s3 = query.create_state();
  query.add_transition(s1, s2, "wi")?;
  query.add_transition(s1, s3, "wifi")?;
  query.add_transition(s2, s3, "fi")?;
  let s4 = query.create_state();
  query.add_transition(s3, s4, "network")?;
  query.set_accept(s4, true);
  query.finish()?;

  // println!("DOT:\n{}", query.to_dot()?);
  assert_eq!(4, searcher.search(query, 1)?.total_hits.value());

  writer.close(&mut random)?;
  searcher.get_index_reader().close()?;
  CloseableRef::close(dir.as_ref())
}

#[test]
fn test_pos_length_at_query_time_trueish() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  for text in [
    "speedy wifi network",
    "speedy wi fi network",
    "fast wifi network",
    "fast wi fi network",
    // doesn't match:
    "slow wi fi network",
  ] {
    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "field",
      text,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(&mut random, doc)?;
  }
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut token_stream = CannedTokenStream::new(vec![
    token("fast", 1, 1)?,
    token("speedy", 0, 1)?,
    token("wi", 1, 1)?,
    token("wifi", 0, 2)?,
    token("fi", 1, 1)?,
    token("network", 1, 1)?,
  ]);

  let query = TokenStreamToTermAutomatonQuery::new().to_query("field", &mut token_stream)?;
  // println!("DOT: {}", query.to_dot()?);
  assert_eq!(4, searcher.search(query, 1)?.total_hits.value());

  writer.close(&mut random)?;
  searcher.get_index_reader().close()?;
  CloseableRef::close(dir.as_ref())
}

#[test]
fn test_segs_missing_terms() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "here comes the sun",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  writer.commit(&mut random)?;

  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "here comes the moon",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query = TermAutomatonQuery::new("field");
  let init = query.create_state();
  let s1 = query.create_state();
  query.add_transition(init, s1, "comes")?;
  let s2 = query.create_state();
  query.add_any_transition(s1, s2)?;
  let s3 = query.create_state();
  query.set_accept(s3, true);
  query.add_transition(s2, s3, "sun")?;
  query.add_transition(s2, s3, "moon")?;
  query.finish()?;

  assert_eq!(2, searcher.search(query, 1)?.total_hits.value());
  writer.close(&mut random)?;
  searcher.get_index_reader().close()?;
  CloseableRef::close(dir.as_ref())
}

#[test]
fn test_invalid_lead_with_any() -> Result<()> {
  let mut query = TermAutomatonQuery::new("field");
  let s0 = query.create_state();
  let s1 = query.create_state();
  let s2 = query.create_state();
  query.set_accept(s2, true);
  query.add_any_transition(s0, s1)?;
  query.add_transition(s1, s2, "b")?;
  assert!(matches!(
    query.finish(),
    Err(error) if error.is_illegal_state_error()
  ));
  Ok(())
}

#[test]
fn test_invalid_trail_with_any() -> Result<()> {
  let mut query = TermAutomatonQuery::new("field");
  let s0 = query.create_state();
  let s1 = query.create_state();
  let s2 = query.create_state();
  query.set_accept(s2, true);
  query.add_transition(s0, s1, "b")?;
  query.add_any_transition(s1, s2)?;
  assert!(matches!(
    query.finish(),
    Err(error) if error.is_illegal_state_error()
  ));
  Ok(())
}

#[test]
fn test_any_from_token_stream() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  for text in [
    "here comes the sun",
    "here comes the moon",
    "here comes sun",
    // Should not match:
    "here comes the other sun",
  ] {
    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "field",
      text,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(&mut random, doc)?;
  }
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut token_stream = CannedTokenStream::new(vec![
    token("comes", 1, 1)?,
    token("comes", 0, 2)?,
    token("*", 1, 1)?,
    token("sun", 1, 1)?,
    token("moon", 0, 1)?,
  ]);

  let query = TokenStreamToTermAutomatonQuery::new().to_query("field", &mut token_stream)?;
  // println!("DOT: {}", query.to_dot()?);
  assert_eq!(3, searcher.search(query, 1)?.total_hits.value());

  writer.close(&mut random)?;
  searcher.get_index_reader().close()?;
  CloseableRef::close(dir.as_ref())
}

fn token(term: &str, pos_inc: i32, pos_length: i32) -> Result<Token> {
  token::with_all(term, pos_inc, 0, term.len() as i32, pos_length)
}

struct RandomSynonymFilter<TS>
where
  TS: TokenStream,
{
  syn_next: bool,
  random: Arc<Mutex<StdRng>>,
  base: TokenFilterBase<TS>,
}

impl<TS> RandomSynonymFilter<TS>
where
  TS: TokenStream,
{
  fn new(input: TS, random: Arc<Mutex<StdRng>>) -> Self {
    Self {
      syn_next: false,
      random,
      base: TokenFilterBase::new(input),
    }
  }
}

impl<TS> crate::core::util::close::Closeable for RandomSynonymFilter<TS>
where
  TS: TokenStream,
{
  fn close(&mut self) -> Result<()> {
    crate::core::util::close::Closeable::close(&mut self.base)
  }
}

impl<TS> TokenStream for RandomSynonymFilter<TS>
where
  TS: TokenStream,
{
  fn increment_token(&mut self) -> Result<bool> {
    if self.syn_next {
      let attr = self.base.input.get_attribute_source_mut();
      attr.set_position_increment(0)?;
      attr.append_char((b'a' + self.random.lock().random_range(0..3)) as char)?;
      self.syn_next = false;
      return Ok(true);
    }

    if self.base.input.increment_token()? {
      if self.random.lock().random_range(0..10) == 8 {
        self.syn_next = true;
      }
      Ok(true)
    } else {
      Ok(false)
    }
  }

  fn end(&mut self) -> Result<()> {
    self.base.end()
  }

  fn reset(&mut self) -> Result<()> {
    self.base.reset()?;
    self.syn_next = false;
    Ok(())
  }

  fn set_reader(&mut self, input: ReaderEnum) -> Result<()> {
    self.base.input.set_reader(input)
  }

  fn set_reader_test_point(&mut self) -> Result<()> {
    self.base.input.set_reader_test_point()
  }

  fn get_attribute_source(&self) -> &Attributes {
    self.base.input.get_attribute_source()
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    self.base.input.get_attribute_source_mut()
  }
}

impl<TS> TokenFilter for RandomSynonymFilter<TS> where TS: TokenStream {}

struct RandomSynonymAnalyzer {
  random: Arc<Mutex<StdRng>>,
  stored_value: AnalyzerStoredValue,
}

impl RandomSynonymAnalyzer {
  fn new(seed: u64) -> Self {
    Self {
      random: Arc::new(Mutex::new(StdRng::seed_from_u64(seed))),
      stored_value: AnalyzerStoredValue::new(),
    }
  }
}

impl Analyzer for RandomSynonymAnalyzer {
  fn create_components(&self, _field_name: &str) -> Result<TokenStreamComponents> {
    let tokenizer_random = StdRng::seed_from_u64(self.random.lock().random());
    let mut tokenizer =
      MockTokenizer::with_automaton(tokenizer_random, WHITESPACE.clone(), true, 100);
    tokenizer.set_enable_checks(true);
    let filter = MockTokenFilter::new(tokenizer, EMPTY_STOPSET.clone());
    let filter = RandomSynonymFilter::new(filter, self.random.clone());
    Ok(TokenStreamComponents::new(
      Box::new(filter) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(RandomSynonymAnalyzer);

#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let num_docs = at_least(&mut random, 50);
  let dir = new_directory_shared(&mut random)?;

  // Adds occasional random synonyms:
  let analyzer: Box<dyn Analyzer> = Box::new(RandomSynonymAnalyzer::new(random.random::<u64>()));
  let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let writer = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);
  let mut field_to_type = HashMap::new();
  let mut doc_values = Vec::with_capacity(num_docs as usize);

  for i in 0..num_docs {
    let mut doc = Document::new();
    let num_tokens = at_least(&mut random, 10);

    let mut contents = String::new();
    for _ in 0..num_tokens {
      contents.push(' ');
      contents.push((b'a' + random.random_range(0..3)) as char);
    }
    doc.add(new_text_field(
      &mut random,
      "field",
      &contents,
      Store::No,
      &mut field_to_type,
    )?);
    doc.add(StoredField::from_string("id", i.to_string())?);
    doc.add(NumericDocValuesField::new("id", i as i64));
    doc_values.push(Some(BytesRef::from(i.to_string().as_str())));
    if cfg!(feature = "test_log_verbose") {
      println!("  doc {i} -> {contents}");
    }
    writer.add_document(&mut random, doc)?;
  }

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;
  let doc_values = Arc::new(doc_values);

  // Used to match ANY using MultiPhraseQuery:
  let all_terms = [
    Term::from_text("field", "a"),
    Term::from_text("field", "b"),
    Term::from_text("field", "c"),
  ];
  let num_iters = at_least(&mut random, 1000);
  for iter in 0..num_iters {
    // Build the (finite, no any transitions) TermAutomatonQuery and
    // also the "equivalent" BooleanQuery and make sure they match the
    // same docs:
    let mut boolean_query = BooleanQueryBuilder::new();
    let count = TestUtil::next_int(&mut random, 1, 5);
    let mut strings = HashSet::new();
    for _ in 0..count {
      let mut string = String::new();
      let num_tokens = TestUtil::next_int(&mut random, 1, 5);
      for j in 0..num_tokens {
        if j > 0 && j < num_tokens - 1 && random.random_range(0..5) == 3 {
          string.push('*');
        } else {
          string.push((b'a' + random.random_range(0..3)) as char);
        }
      }
      let mut multi_phrase_query = MultiPhraseQuery::builder();
      for character in string.chars() {
        if character == '*' {
          multi_phrase_query.add_terms(&all_terms)?;
        } else {
          multi_phrase_query.add_term(Term::from_text("field", character.to_string()))?;
        }
      }
      boolean_query.add(multi_phrase_query.build(), Occur::Should)?;
      strings.insert(BytesRef::from(string.as_str()));
    }

    let mut strings_list = strings.into_iter().collect::<Vec<_>>();
    strings_list.sort();
    let automaton = Automata::make_string_union(&strings_list)?;

    // Translate automaton to query:
    let mut query = TermAutomatonQuery::new("field");
    let num_states = automaton.get_num_states();
    for i in 0..num_states {
      query.create_state();
      query.set_accept(i, automaton.is_accept(i));
    }

    let mut transition = Transition::default();
    for i in 0..num_states {
      let trans_count = automaton.init_transition(i, &mut transition);
      for _ in 0..trans_count {
        automaton.get_next_transition(&mut transition);
        for label in transition.min..=transition.max {
          if label == '*' as i32 {
            query.add_any_transition(transition.source, transition.dest)?;
          } else {
            query.add_transition(
              transition.source,
              transition.dest,
              &char::from_u32(label as u32)
                .ok_or_else(|| LuceneError::illegal_state("invalid automaton label"))?
                .to_string(),
            )?;
          }
        }
      }
    }
    query.finish()?;

    if cfg!(feature = "test_log_verbose") {
      println!("TEST: iter={iter}");
      for string in &strings_list {
        println!("  string: {}", string.utf8_to_string()?);
      }
      println!("{}", query.to_dot()?);
    }

    let mut query1: Query = query.into();
    let mut query2: Query = boolean_query.build().into();
    if random.random_range(0..5) == 1 {
      if cfg!(feature = "test_log_verbose") {
        println!("  use random filter");
      }
      let filter = RandomQuery::new(
        random.random::<u64>(),
        random.random::<f32>(),
        doc_values.clone(),
      );
      let mut builder = BooleanQueryBuilder::new();
      builder.add(query1, Occur::Must)?;
      builder.add(filter.clone(), Occur::Filter)?;
      query1 = builder.build().into();
      let mut builder = BooleanQueryBuilder::new();
      builder.add(query2, Occur::Must)?;
      builder.add(filter, Occur::Filter)?;
      query2 = builder.build().into();
    }

    let hits1 = searcher.search(query1, num_docs as usize)?;
    let hits2 = searcher.search(query2, num_docs as usize)?;
    let hits1_docs = to_doc_ids(&searcher, &hits1)?;
    let hits2_docs = to_doc_ids(&searcher, &hits2)?;

    if hits1.total_hits.value() != hits2.total_hits.value() || hits1_docs != hits2_docs {
      println!("FAILED:");
      for id in hits1_docs.difference(&hits2_docs) {
        println!("  id={id:>3} matched but should not have");
      }
      for id in hits2_docs.difference(&hits1_docs) {
        println!("  id={id:>3} did not match but should have");
      }
    }
    assert_eq!(hits2.total_hits.value(), hits1.total_hits.value());
    assert_eq!(hits2_docs, hits1_docs);
  }

  let close_result = searcher.get_index_reader().close();
  IOUtils::use_or_suppress_result(close_result, CloseableRef::close(dir.as_ref()))
}

fn to_doc_ids<IRC>(
  searcher: &IndexSearcher<IRC>,
  hits: &TopDocs<ScoreDoc>,
) -> Result<HashSet<String>>
where
  IRC: IndexReaderContext,
{
  let mut result = HashSet::new();
  let mut stored_fields = searcher.stored_fields()?;
  for hit in &hits.score_docs {
    let document = stored_fields.document(hit.doc)?;
    result.insert(
      document
        .get("id")?
        .ok_or_else(|| LuceneError::illegal_state("stored id field is missing"))?
        .into_owned(),
    );
  }
  Ok(result)
}

/// See if we can create a TAQ with cycles.
#[test]
fn test_with_cycles1() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  for text in ["here comes here comes", "comes foo"] {
    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "field",
      text,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(&mut random, doc)?;
  }
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query = TermAutomatonQuery::new("field");
  let init = query.create_state();
  let s1 = query.create_state();
  let s2 = query.create_state();
  query.add_transition(init, s1, "here")?;
  query.add_transition(s1, s2, "comes")?;
  query.add_transition(s2, s1, "here")?;
  query.set_accept(s1, true);
  query.finish()?;

  assert_eq!(1, searcher.search(query, 1)?.total_hits.value());
  writer.close(&mut random)?;
  searcher.get_index_reader().close()?;
  CloseableRef::close(dir.as_ref())
}

/// See if we can create a TAQ with cycles.
#[test]
fn test_with_cycles2() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  for text in ["here comes kaoma", "here comes sun sun sun sun kaoma"] {
    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "field",
      text,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(&mut random, doc)?;
  }
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query = TermAutomatonQuery::new("field");
  let init = query.create_state();
  let s1 = query.create_state();
  query.add_transition(init, s1, "here")?;
  let s2 = query.create_state();
  query.add_transition(s1, s2, "comes")?;
  let s3 = query.create_state();
  query.add_transition(s2, s3, "sun")?;
  query.add_transition(s3, s3, "sun")?;
  let s4 = query.create_state();
  query.add_transition(s3, s4, "kaoma")?;
  query.set_accept(s4, true);
  query.finish()?;

  assert_eq!(1, searcher.search(query, 1)?.total_hits.value());
  writer.close(&mut random)?;
  searcher.get_index_reader().close()?;
  CloseableRef::close(dir.as_ref())
}

#[test]
fn test_term_does_not_exist() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "x y z",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut token_stream = CannedTokenStream::new(vec![token("a", 1, 1)?]);
  let query = TokenStreamToTermAutomatonQuery::new().to_query("field", &mut token_stream)?;
  // println!("DOT: {}", query.to_dot()?);
  assert_eq!(0, searcher.search(query, 1)?.total_hits.value());

  writer.close(&mut random)?;
  searcher.get_index_reader().close()?;
  CloseableRef::close(dir.as_ref())
}

#[test]
fn test_one_term_does_not_exist() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "x y z",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut token_stream = CannedTokenStream::new(vec![token("a", 1, 1)?, token("x", 1, 1)?]);
  let query = TokenStreamToTermAutomatonQuery::new().to_query("field", &mut token_stream)?;
  // println!("DOT: {}", query.to_dot()?);
  assert_eq!(0, searcher.search(query, 1)?.total_hits.value());

  let close_result = IOUtils::use_or_suppress_result(
    writer.close(&mut random),
    searcher.get_index_reader().close(),
  );
  IOUtils::use_or_suppress_result(close_result, CloseableRef::close(dir.as_ref()))
}

#[test]
fn test_empty_string() -> Result<()> {
  let mut query = TermAutomatonQuery::new("field");
  let init_state = query.create_state();
  query.set_accept(init_state, true);
  assert!(matches!(
    query.finish(),
    Err(error) if error.is_illegal_state_error()
  ));
  Ok(())
}

#[test]
fn test_rewrite_no_match() -> Result<()> {
  let mut query = TermAutomatonQuery::new("field");
  query.create_state(); // initState
  query.finish()?;

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "x y z",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  assert!(matches!(query.rewrite(&searcher)?, Query::MatchNoDocs(_)));
  let close_result = IOUtils::use_or_suppress_result(
    writer.close(&mut random),
    searcher.get_index_reader().close(),
  );
  IOUtils::use_or_suppress_result(close_result, CloseableRef::close(dir.as_ref()))
}

#[test]
fn test_rewrite_term() -> Result<()> {
  let mut query = TermAutomatonQuery::new("field");
  let init_state = query.create_state();
  let s1 = query.create_state();
  query.add_transition(init_state, s1, "foo")?;
  query.set_accept(s1, true);
  query.finish()?;

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "x y z",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let rewrite = query.rewrite(&searcher)?;
  let Query::Term(rewrite) = rewrite else {
    panic!("rewrite should be a TermQuery");
  };
  assert_eq!(Term::from_text("field", "foo"), *rewrite.get_term());
  let close_result = IOUtils::use_or_suppress_result(
    writer.close(&mut random),
    searcher.get_index_reader().close(),
  );
  IOUtils::use_or_suppress_result(close_result, CloseableRef::close(dir.as_ref()))
}

#[test]
fn test_rewrite_simple_phrase() -> Result<()> {
  let mut query = TermAutomatonQuery::new("field");
  let init_state = query.create_state();
  let s1 = query.create_state();
  let s2 = query.create_state();
  query.add_transition(init_state, s1, "foo")?;
  query.add_transition(s1, s2, "bar")?;
  query.set_accept(s2, true);
  query.finish()?;

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "x y z",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let rewrite = query.rewrite(&searcher)?;
  let Query::Phrase(rewrite) = rewrite else {
    panic!("rewrite should be a PhraseQuery");
  };
  let terms = rewrite.get_terms();
  assert_eq!(Term::from_text("field", "foo"), terms[0]);
  assert_eq!(Term::from_text("field", "bar"), terms[1]);
  let positions = rewrite.get_positions();
  assert_eq!(0, positions[0]);
  assert_eq!(1, positions[1]);

  let close_result = IOUtils::use_or_suppress_result(
    writer.close(&mut random),
    searcher.get_index_reader().close(),
  );
  IOUtils::use_or_suppress_result(close_result, CloseableRef::close(dir.as_ref()))
}

/* Implement a custom term automaton query to ensure that rewritten queries
 *  do not get rewritten to primitive queries. The custom extension will allow
 *  the following explain tests to evaluate Explain for the query we intend to
 *  test, TermAutomatonQuery.
 * */

#[test]
fn test_explain_no_matching_document() -> Result<()> {
  let mut query = TermAutomatonQuery::with_hook("field", CustomTermAutomatonQuery);
  let init_state = query.create_state();
  let s1 = query.create_state();
  query.add_transition(init_state, s1, "xml")?;
  query.set_accept(s1, true);
  query.finish()?;

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "protobuf",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let rewritten_query = searcher.rewrite(query.clone())?;
  assert!(matches!(rewritten_query, Query::TermAutomaton(_)));
  let top_docs = searcher.search(query.clone(), 10)?;
  assert_eq!(0, top_docs.total_hits.value());
  let explanation = searcher.explain(query, 0)?;
  assert!(
    !explanation.is_match(),
    "Explanation should indicate no match"
  );

  let close_result = IOUtils::use_or_suppress_result(
    writer.close(&mut random),
    searcher.get_index_reader().close(),
  );
  IOUtils::use_or_suppress_result(close_result, CloseableRef::close(dir.as_ref()))
}

#[test]
fn test_explain_matching_documents() -> Result<()> {
  let mut query = TermAutomatonQuery::with_hook("field", CustomTermAutomatonQuery);
  let init_state = query.create_state();
  let s1 = query.create_state();
  let s2 = query.create_state();
  query.add_transition(init_state, s1, "xml")?;
  query.add_transition(s1, s2, "json")?;
  query.add_transition(s1, s2, "protobuf")?;
  query.set_accept(s2, true);
  query.finish()?;

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  for text in ["xml json", "xml protobuf", "xml qux"] {
    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "field",
      text,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(&mut random, doc)?;
  }
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let rewritten_query = searcher.rewrite(query.clone())?;
  assert!(
    matches!(rewritten_query, Query::TermAutomaton(_)),
    "Rewritten query should be an instance of TermAutomatonQuery"
  );
  let top_docs = searcher.search(query.clone(), 10)?;
  assert_eq!(2, top_docs.total_hits.value());
  for score_doc in top_docs.score_docs {
    let explanation = searcher.explain(query.clone(), score_doc.doc)?;
    assert!(
      explanation.is_match(),
      "Explanation should indicate a match"
    );
  }

  let close_result = IOUtils::use_or_suppress_result(
    writer.close(&mut random),
    searcher.get_index_reader().close(),
  );
  IOUtils::use_or_suppress_result(close_result, CloseableRef::close(dir.as_ref()))
}

#[test]
fn test_rewrite_phrase_with_any() -> Result<()> {
  let mut query = TermAutomatonQuery::new("field");
  let init_state = query.create_state();
  let s1 = query.create_state();
  let s2 = query.create_state();
  let s3 = query.create_state();
  query.add_transition(init_state, s1, "foo")?;
  query.add_any_transition(s1, s2)?;
  query.add_transition(s2, s3, "bar")?;
  query.set_accept(s3, true);
  query.finish()?;

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "x y z",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let rewrite = query.rewrite(&searcher)?;
  let Query::Phrase(rewrite) = rewrite else {
    panic!("rewrite should be a PhraseQuery");
  };
  let terms = rewrite.get_terms();
  assert_eq!(Term::from_text("field", "foo"), terms[0]);
  assert_eq!(Term::from_text("field", "bar"), terms[1]);
  let positions = rewrite.get_positions();
  assert_eq!(0, positions[0]);
  assert_eq!(2, positions[1]);

  let close_result = IOUtils::use_or_suppress_result(
    writer.close(&mut random),
    searcher.get_index_reader().close(),
  );
  IOUtils::use_or_suppress_result(close_result, CloseableRef::close(dir.as_ref()))
}

#[test]
fn test_rewrite_simple_multi_phrase() -> Result<()> {
  let mut query = TermAutomatonQuery::new("field");
  let init_state = query.create_state();
  let s1 = query.create_state();
  query.add_transition(init_state, s1, "foo")?;
  query.add_transition(init_state, s1, "bar")?;
  query.set_accept(s1, true);
  query.finish()?;

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "x y z",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let rewrite = query.rewrite(&searcher)?;
  let Query::MultiPhrase(rewrite) = rewrite else {
    panic!("rewrite should be a MultiPhraseQuery");
  };
  let terms = rewrite.get_term_arrays();
  assert_eq!(1, terms.len());
  assert_eq!(2, terms[0].len());
  assert_eq!(Term::from_text("field", "foo"), terms[0][0]);
  assert_eq!(Term::from_text("field", "bar"), terms[0][1]);
  let positions = rewrite.get_positions();
  assert_eq!(1, positions.len());
  assert_eq!(0, positions[0]);

  let close_result = IOUtils::use_or_suppress_result(
    writer.close(&mut random),
    searcher.get_index_reader().close(),
  );
  IOUtils::use_or_suppress_result(close_result, CloseableRef::close(dir.as_ref()))
}

#[test]
fn test_rewrite_multi_phrase_with_any() -> Result<()> {
  let mut query = TermAutomatonQuery::new("field");
  let init_state = query.create_state();
  let s1 = query.create_state();
  let s2 = query.create_state();
  let s3 = query.create_state();
  query.add_transition(init_state, s1, "foo")?;
  query.add_transition(init_state, s1, "bar")?;
  query.add_any_transition(s1, s2)?;
  query.add_transition(s2, s3, "baz")?;
  query.set_accept(s3, true);
  query.finish()?;

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "x y z",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let rewrite = query.rewrite(&searcher)?;
  let Query::MultiPhrase(rewrite) = rewrite else {
    panic!("rewrite should be a MultiPhraseQuery");
  };
  let terms = rewrite.get_term_arrays();
  assert_eq!(2, terms.len());
  assert_eq!(2, terms[0].len());
  assert_eq!(Term::from_text("field", "foo"), terms[0][0]);
  assert_eq!(Term::from_text("field", "bar"), terms[0][1]);
  assert_eq!(1, terms[1].len());
  assert_eq!(Term::from_text("field", "baz"), terms[1][0]);
  let positions = rewrite.get_positions();
  assert_eq!(2, positions.len());
  assert_eq!(0, positions[0]);
  assert_eq!(2, positions[1]);

  let close_result = IOUtils::use_or_suppress_result(
    writer.close(&mut random),
    searcher.get_index_reader().close(),
  );
  IOUtils::use_or_suppress_result(close_result, CloseableRef::close(dir.as_ref()))
}

// we query with sun|moon but moon doesn't exist
#[test]
fn test_one_term_missing() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "here comes the sun",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query = TermAutomatonQuery::new("field");
  let init = query.create_state();
  let s1 = query.create_state();
  query.add_transition(init, s1, "comes")?;
  let s2 = query.create_state();
  query.add_any_transition(s1, s2)?;
  let s3 = query.create_state();
  query.set_accept(s3, true);
  query.add_transition(s2, s3, "sun")?;
  query.add_transition(s2, s3, "moon")?;
  query.finish()?;
  assert_eq!(1, searcher.search(query, 1)?.total_hits.value());

  writer.close(&mut random)?;
  searcher.get_index_reader().close()?;
  CloseableRef::close(dir.as_ref())
}

// we query with sun|moon but no terms exist for the field
#[test]
fn test_field_missing() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "here comes the sun",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query = TermAutomatonQuery::new("bogusfield");
  let init = query.create_state();
  let s1 = query.create_state();
  query.add_transition(init, s1, "comes")?;
  let s2 = query.create_state();
  query.add_any_transition(s1, s2)?;
  let s3 = query.create_state();
  query.set_accept(s3, true);
  query.add_transition(s2, s3, "sun")?;
  query.add_transition(s2, s3, "moon")?;
  query.finish()?;
  assert_eq!(0, searcher.search(query, 1)?.total_hits.value());

  writer.close(&mut random)?;
  searcher.get_index_reader().close()?;
  CloseableRef::close(dir.as_ref())
}
