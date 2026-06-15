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
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::document::text_field::TextField;
use crate::core::index::BytesRef;
use crate::core::index::impact::Impact;
use crate::core::index::impacts::Impacts;
use crate::core::index::impacts_enum::ImpactsEnum;
use crate::core::index::impacts_source::ImpactsSource;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::exact_phrase_matcher::merge_impacts_from_ie;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::similarities_impl::classic_similarity;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::{
  ENGLISH_STOPSET, MockAnalyzer, SIMPLE, WHITESPACE,
};
use crate::test::core::analysis::mock_tokenizer::MockTokenizer;
use crate::test::core::analysis::{canned_token_stream::CannedTokenStream, token};
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::check_hits::CheckHits;
use crate::test::core::search::query_utils::QueryUtils;
use crate::test::core::util::DefaultIndexSearchCR;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, is_night_mode, new_directory_shared, new_index_writer_config, new_log_merge_policy,
  new_searcher_with_reader, new_text_field, random, random_from_seed,
};
use rand::prelude::SliceRandom;
use rand::{Rng, RngExt};
use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

#[allow(dead_code)]
struct TestPhraseQuery;
pub const SCORE_COMP_THRESH: f32 = 1e-6;

struct PhraseQueryAnalyzer {
  stored_value: AnalyzerStoredValue,
  seed: u64,
}

impl Analyzer for PhraseQueryAnalyzer {
  fn create_components(&self, _field_name: &str) -> Result<TokenStreamComponents> {
    let tokenizer = MockTokenizer::with_default_max_token_length(
      random_from_seed(self.seed),
      WHITESPACE.clone(),
      false,
    );
    Ok(TokenStreamComponents::new(
      Box::new(tokenizer) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn get_position_increment_gap(&self, _field_name: &str) -> i32 {
    100
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let analyzer = PhraseQueryAnalyzer {
    stored_value: AnalyzerStoredValue::new(),
    seed: random.random(),
  };
  let writer =
    RandomIndexWriter::with_analyzer(random, dir.clone(), Box::new(analyzer) as Box<dyn Analyzer>);
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "field",
    "one two three four five",
    Store::Yes,
    &mut field_to_type,
  )?);
  doc.add(new_text_field(
    random,
    "repeated",
    "this is a repeated field - first part",
    Store::Yes,
    &mut field_to_type,
  )?);
  let repeated_field = new_text_field(
    random,
    "repeated",
    "second part of a repeated field",
    Store::Yes,
    &mut field_to_type,
  )?;
  doc.add(repeated_field);
  doc.add(new_text_field(
    random,
    "palindrome",
    "one two three two one",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(doc)?;

  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "nonexist",
    "phrase exist notexist exist found",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(doc)?;

  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "nonexist",
    "phrase exist notexist exist found",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(doc)?;

  let reader = writer.get_reader()?;
  writer.close()?;

  let searcher = new_searcher_with_reader(reader)?;

  Ok(searcher)
}
#[test]
fn test_not_close_enough() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;
  let query = PhraseQuery::from_terms(2, "field", &["one", "five"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(0, hits.len());
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  Ok(())
}

#[test]
fn test_barely_close_enough() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;
  let query = PhraseQuery::from_terms(3, "field", &["one", "five"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len());

  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;
  Ok(())
}

/// Ensures slop of 0 works for exact matches, but not reversed
#[test]
fn test_exact() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;
  // slop is zero by default
  let query = PhraseQuery::from_terms(0, "field", &["four", "five"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "exact match");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  let query = PhraseQuery::from_terms(0, "field", &["two", "one"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(0, hits.len(), "reverse not exact");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  Ok(())
}

#[test]
fn test_slop1() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  // Ensures slop of 1 works with terms in order.
  let query = PhraseQuery::from_terms(1, "field", &["one", "two"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "in order");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  // Ensures slop of 1 does not work for phrases out of order;
  // must be at least 2.
  let query = PhraseQuery::from_terms(1, "field", &["two", "one"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(0, hits.len(), "reversed, slop not 2 or more");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  Ok(())
}

/// As long as slop is at least 2, terms can be reversed
#[test]
fn test_order_doesnt_matter() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  // must be at least two for reverse order match
  let query = PhraseQuery::from_terms(2, "field", &["two", "one"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "just sloppy enough");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  let query = PhraseQuery::from_terms(2, "field", &["three", "one"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(0, hits.len(), "not sloppy enough");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  Ok(())
}

/// slop is the total number of positional moves allowed to line up a phrase
#[test]
fn test_multiple_terms() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let query = PhraseQuery::from_terms(2, "field", &["one", "three", "five"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "two total moves");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  // it takes six moves to match this phrase
  let query = PhraseQuery::from_terms(5, "field", &["five", "three", "one"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(0, hits.len(), "slop of 5 not close enough");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  let query = PhraseQuery::from_terms(6, "field", &["five", "three", "one"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "slop of 6 just right");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  Ok(())
}
#[test]
fn test_phrase_query_with_stop_analyzer() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let stop_analyzer =
    MockAnalyzer::with_filter(&mut random, SIMPLE.clone(), true, ENGLISH_STOPSET.clone());
  let writer = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), stop_analyzer);
  let mut field_to_type = HashMap::new();

  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "the stop words are here",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(doc)?;

  let reader = writer.get_reader()?;
  writer.close()?;

  let searcher = new_searcher_with_reader(reader)?;

  // valid exact phrase query
  let query = PhraseQuery::from_terms(0, "field", &["stop", "words"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len());

  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  Ok(())
}

#[test]
fn test_phrase_query_in_conjunction_scorer() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_to_type = HashMap::new();
  {
    let writer = RandomIndexWriter::new(&mut random, dir.clone());

    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "source",
      "marketing info",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "contents",
      "foobar",
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(new_text_field(
      &mut random,
      "source",
      "marketing info",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;

    let reader = writer.get_reader()?;
    writer.close()?;

    let searcher = new_searcher_with_reader(reader)?;

    let phrase_query = PhraseQuery::from_terms(0, "source", &["marketing", "info"])?;
    let top_docs = searcher.search(phrase_query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(2, hits.len());
    QueryUtils::check_from_searcher(&mut random, phrase_query.clone(), &searcher)?;

    let term_query: Query = TermQuery::new(Term::from_text("contents", "foobar")).into();

    let mut b = Builder::new();
    b.add(term_query.clone(), Occur::Must)?;
    b.add(phrase_query.clone(), Occur::Must)?;
    let boolean_query: Query = b.build().into();

    let top_docs = searcher.search(boolean_query, 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(1, hits.len());
    QueryUtils::check_from_searcher(&mut random, term_query, &searcher)?;
  }

  {
    let writer = RandomIndexWriter::new(&mut random, dir.clone());

    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "contents",
      "map entry woo",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "contents",
      "woo map entry",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "contents",
      "map foobarword entry woo",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;

    let reader = writer.get_reader()?;
    writer.close()?;

    let searcher = new_searcher_with_reader(reader)?;

    let term_query: Query = TermQuery::new(Term::from_text("contents", "woo")).into();
    let phrase_query = PhraseQuery::from_terms(0, "contents", &["map", "entry"])?;

    let top_docs = searcher.search(term_query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(3, hits.len());

    let top_docs = searcher.search(phrase_query.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(2, hits.len());

    let mut b = Builder::new();
    b.add(term_query.clone(), Occur::Must)?;
    b.add(phrase_query.clone(), Occur::Must)?;
    let boolean_query1: Query = b.build().into();
    let top_docs = searcher.search(boolean_query1, 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(2, hits.len());

    let mut b = Builder::new();
    b.add(phrase_query.clone(), Occur::Must)?;
    b.add(term_query.clone(), Occur::Must)?;
    let boolean_query2: Query = b.build().into();
    let top_docs = searcher.search(boolean_query2.clone(), 1000)?;
    let hits = top_docs.score_docs();
    assert_eq!(2, hits.len());

    QueryUtils::check_from_searcher(&mut random, boolean_query2, &searcher)?;
  }

  Ok(())
}
#[test]
fn test_slop_scoring() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let writer = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let mut field_to_type = HashMap::new();

  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "foo firstname lastname foo",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(doc)?;

  let mut doc2 = Document::new();
  doc2.add(new_text_field(
    &mut random,
    "field",
    "foo firstname zzz lastname foo",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(doc2)?;

  let mut doc3 = Document::new();
  doc3.add(new_text_field(
    &mut random,
    "field",
    "foo firstname zzz yyy lastname foo",
    Store::Yes,
    &mut field_to_type,
  )?);
  writer.add_document(doc3)?;

  let reader = writer.get_reader()?;
  writer.close()?;

  let mut searcher = new_searcher_with_reader(reader)?;
  searcher.set_similarity(classic_similarity::new());

  let query = PhraseQuery::from_terms(i32::MAX as usize, "field", &["firstname", "lastname"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(3, hits.len());

  assert!((hits[0].score - 1.0).abs() <= 0.01);
  assert_eq!(0, hits[0].doc);

  assert!((hits[1].score - 0.63).abs() <= 0.01);
  assert_eq!(1, hits[1].doc);

  assert!((hits[2].score - 0.47).abs() <= 0.01);
  assert_eq!(2, hits[2].doc);

  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;
  Ok(())
}
#[test]
fn test_to_string() -> Result<()> {
  let q = PhraseQuery::from_terms(0, "field", &[])?;
  assert_eq!("\"\"", q.to_string("")?);

  // single term at position 1
  let mut builder = crate::core::search::phrase_query::Builder::new();
  builder.add(Term::from_text("field", "hi"), 1)?;
  let q = builder.build()?;
  assert_eq!("field:\"? hi\"", q.to_string("")?);

  // two terms with gap
  let mut builder = crate::core::search::phrase_query::Builder::new();
  builder.add(Term::from_text("field", "hi"), 1)?;
  builder.add(Term::from_text("field", "test"), 5)?;
  let q = builder.build()?;
  assert_eq!("field:\"? hi ? ? ? test\"", q.to_string("")?);

  // multi-term at same position
  let mut builder = crate::core::search::phrase_query::Builder::new();
  builder.add(Term::from_text("field", "hi"), 1)?;
  builder.add(Term::from_text("field", "hello"), 1)?;
  builder.add(Term::from_text("field", "test"), 5)?;
  let q = builder.build()?;
  assert_eq!("field:\"? hi|hello ? ? ? test\"", q.to_string("")?);

  // with slop
  let mut builder = crate::core::search::phrase_query::Builder::new();
  builder.add(Term::from_text("field", "hi"), 1)?;
  builder.add(Term::from_text("field", "hello"), 1)?;
  builder.add(Term::from_text("field", "test"), 5)?;
  builder.set_slop(5);
  let q = builder.build()?;
  assert_eq!("field:\"? hi|hello ? ? ? test\"~5", q.to_string("")?);

  Ok(())
}
#[test]
fn test_wrapped_phrase() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let query = PhraseQuery::from_terms(100, "repeated", &["first", "part", "second", "part"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "slop of 100 just right");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  let query = PhraseQuery::from_terms(99, "repeated", &["first", "part", "second", "part"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(0, hits.len(), "slop of 99 not enough");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  Ok(())
}
#[test]
fn test_non_existing_phrase() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  // phrase without repetitions that exists in 2 docs
  let query = PhraseQuery::from_terms(2, "nonexist", &["phrase", "notexist", "found"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(2, hits.len(), "phrase without repetitions exists in 2 docs");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  // phrase with repetitions that exists in 2 docs
  let query = PhraseQuery::from_terms(1, "nonexist", &["phrase", "exist", "exist"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(2, hits.len(), "phrase with repetitions exists in two docs");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  // phrase I with repetitions that does not exist in any doc
  let query = PhraseQuery::from_terms(1000, "nonexist", &["phrase", "notexist", "phrase"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(
    0,
    hits.len(),
    "nonexisting phrase with repetitions does not exist in any doc"
  );
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  // phrase II with repetitions that does not exist in any doc
  let query = PhraseQuery::from_terms(1000, "nonexist", &["phrase", "exist", "exist", "exist"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(
    0,
    hits.len(),
    "nonexisting phrase with repetitions does not exist in any doc"
  );
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  Ok(())
}
#[test]
fn test_palyndrome2() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  // search on non palyndrome, find phrase with no slop, using exact phrase scorer
  let query = PhraseQuery::from_terms(0, "field", &["two", "three"])?; // to use exact phrase scorer
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "phrase found with exact phrase scorer");
  let score0 = hits[0].score;
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  // search on non palyndrome, find phrase with slop 2, though no slop required here.
  let query = PhraseQuery::from_terms(2, "field", &["two", "three"])?; // to use sloppy scorer
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "just sloppy enough");
  let score1 = hits[0].score;
  assert!(
    (score0 - score1).abs() <= SCORE_COMP_THRESH,
    "exact scorer and sloppy scorer score the same when slop does not matter"
  );
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  // search ordered in palyndrome, find it twice
  let query = PhraseQuery::from_terms(2, "palindrome", &["two", "three"])?; // must be at least two for both ordered and reversed to match
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "just sloppy enough");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  // search reveresed in palyndrome, find it twice
  let query = PhraseQuery::from_terms(2, "palindrome", &["three", "two"])?; // must be at least two for both ordered and reversed to match
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "just sloppy enough");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  Ok(())
}
#[test]
fn test_palyndrome3() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  // search on non palyndrome, find phrase with no slop, using exact phrase scorer
  // slop=0 to use exact phrase scorer
  let query = PhraseQuery::from_terms(0, "field", &["one", "two", "three"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "phrase found with exact phrase scorer");
  let score0 = hits[0].score;
  QueryUtils::check_from_searcher(&mut random, query.clone(), &searcher)?;

  // just make sure no exc:
  searcher.explain(query.clone(), 0)?;

  // search on non palyndrome, find phrase with slop 3, though no slop required here.
  // slop=4 to use sloppy scorer
  let query = PhraseQuery::from_terms(4, "field", &["one", "two", "three"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "just sloppy enough");
  let score1 = hits[0].score;
  assert!(
    (score0 - score1).abs() <= SCORE_COMP_THRESH,
    "exact scorer and sloppy scorer score the same when slop does not matter"
  );
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  // search ordered in palyndrome, find it twice
  // slop must be at least four for both ordered and reversed to match
  let query = PhraseQuery::from_terms(4, "palindrome", &["one", "two", "three"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();

  // just make sure no exc:
  let _ = searcher.explain(query.clone(), 0)?;

  assert_eq!(1, hits.len(), "just sloppy enough");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  // search reveresed in palyndrome, find it twice
  // must be at least four for both ordered and reversed to match
  let query = PhraseQuery::from_terms(4, "palindrome", &["three", "two", "one"])?;
  let top_docs = searcher.search(query.clone(), 1000)?;
  let hits = top_docs.score_docs();
  assert_eq!(1, hits.len(), "just sloppy enough");
  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  Ok(())
}
#[test]
fn test_empty_phrase_query() -> Result<()> {
  let mut b = Builder::new();
  b.add(PhraseQuery::from_terms(0, "field", &[])?, Occur::Must)?;
  let q: Query = b.build().into();
  let _ = q.to_string("");
  Ok(())
}

#[test]
fn test_rewrite() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let pq: Query = PhraseQuery::from_terms(0, "foo", &["bar"])?.into();
  let rewritten = pq.rewrite(&searcher)?;

  assert!(matches!(rewritten, Query::Term(_)));
  Ok(())
}
#[test]
fn test_zero_pos_incr() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let tokens = vec![
    token::with_pos_inc("a", 1, 0, 1)?,
    token::with_pos_inc("aa", 0, 0, 2)?,
    token::with_pos_inc("b", 1, 3, 4)?,
  ];

  let writer = RandomIndexWriter::new(&mut random, dir.clone());
  let mut doc = Document::new();
  doc.add(TextField::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(CannedTokenStream::new(tokens)),
  )?);
  writer.add_document(doc)?;

  let reader = writer.get_reader()?;
  writer.close()?;
  let searcher = new_searcher_with_reader(reader)?;

  // Sanity check; simple "a b" phrase.
  let mut pq_builder = crate::core::search::phrase_query::Builder::new();
  pq_builder.add(Term::from_text("field", "a"), 0)?;
  pq_builder.add(Term::from_text("field", "b"), 1)?;
  assert_eq!(1, searcher.count(pq_builder.build()?)?);

  // Now with "a|aa b".
  let mut pq_builder = crate::core::search::phrase_query::Builder::new();
  pq_builder.add(Term::from_text("field", "a"), 0)?;
  pq_builder.add(Term::from_text("field", "aa"), 0)?;
  pq_builder.add(Term::from_text("field", "b"), 1)?;
  assert_eq!(1, searcher.count(pq_builder.build()?)?);

  // Now with "a|z b" which should not match; this isn't a MultiPhraseQuery.
  let mut pq_builder = crate::core::search::phrase_query::Builder::new();
  pq_builder.add(Term::from_text("field", "a"), 0)?;
  pq_builder.add(Term::from_text("field", "z"), 0)?;
  pq_builder.add(Term::from_text("field", "b"), 1)?;
  assert_eq!(0, searcher.count(pq_builder.build()?)?);

  Ok(())
}
#[test]
fn test_random_phrases() -> Result<()> {
  // TODO IMPORTANT
  Ok(())
}
#[test]
fn test_negative_slop() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_negative_position() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_backward_positions() -> Result<()> {
  let mut builder = crate::core::search::phrase_query::Builder::new();
  builder.add(Term::from_text("field", "one"), 1)?;
  builder.add(Term::from_text("field", "two"), 5)?;

  let result = builder.add(Term::from_text("field", "three"), 4);

  assert!(result.is_err());
  Ok(())
}
static DOCS: [&str; 6] = [
  "a b c d e f g h",
  "b c b",
  "c d d d e f g b",
  "c b a b c",
  "a a b b c c d d",
  "a b c d a b c d a b c d",
];

#[test]
fn test_top_phrases() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone());
  let mut field_to_type = HashMap::new();

  let mut docs = DOCS.to_vec();
  docs.shuffle(&mut random);

  for value in docs {
    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "f",
      value,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;
  }

  let reader = writer.get_reader()?;
  writer.close()?;

  let searcher = new_searcher_with_reader(reader)?;

  let queries: Vec<Query> = vec![
    PhraseQuery::from_terms(0, "f", &["b", "c"])?.into(), // common phrase
    PhraseQuery::from_terms(0, "f", &["e", "f"])?.into(), // always appear next to each other
    PhraseQuery::from_terms(0, "f", &["d", "d"])?.into(), // repeated term
  ];

  for query in queries {
    for top_n in 1..=2 {
      let collector_manager = TopScoreDocCollectorManager::new(top_n, i32::MAX as usize)?;
      let top_docs1 = searcher.search_with_collector_manager(query.clone(), &collector_manager)?;
      let hits1 = top_docs1.score_docs();

      let collector_manager = TopScoreDocCollectorManager::new(top_n, 1)?;
      let top_docs2 = searcher.search_with_collector_manager(query.clone(), &collector_manager)?;
      let hits2 = top_docs2.score_docs();

      assert!(!hits1.is_empty(), "{}", query.to_string("")?);
      CheckHits::check_equal(&query, hits1, hits2)?;
    }
  }

  Ok(())
}

#[test]
fn test_merge_impacts() -> Result<()> {
  let impacts1 = DummyImpactsEnum::new(1000);
  let impacts2 = DummyImpactsEnum::new(2000);

  let mut merged_impacts = merge_impacts_from_ie(vec![impacts1, impacts2])?;

  merged_impacts.impacts_enums.all_disi[0].reset(
    vec![
      vec![Impact::new(3, 10), Impact::new(5, 12), Impact::new(8, 13)],
      vec![
        Impact::new(3, 10),
        Impact::new(5, 11),
        Impact::new(8, 13),
        Impact::new(12, 14),
      ],
    ],
    vec![110, 945],
  );

  // Merge with empty impacts
  merged_impacts.impacts_enums.all_disi[1].reset(vec![], vec![]);
  assert_impacts_eq(
    vec![
      vec![Impact::new(3, 10), Impact::new(5, 12), Impact::new(8, 13)],
      vec![
        Impact::new(3, 10),
        Impact::new(5, 11),
        Impact::new(8, 13),
        Impact::new(12, 14),
      ],
    ],
    vec![110, 945],
    &merged_impacts.get_impacts()?,
  )?;

  // Merge with dummy impacts
  merged_impacts.impacts_enums.all_disi[1].reset(vec![vec![Impact::new(i32::MAX, 1)]], vec![5000]);
  assert_impacts_eq(
    vec![
      vec![Impact::new(3, 10), Impact::new(5, 12), Impact::new(8, 13)],
      vec![
        Impact::new(3, 10),
        Impact::new(5, 11),
        Impact::new(8, 13),
        Impact::new(12, 14),
      ],
    ],
    vec![110, 945],
    &merged_impacts.get_impacts()?,
  )?;

  // Merge with dummy impacts that we don't special case
  merged_impacts.impacts_enums.all_disi[1].reset(vec![vec![Impact::new(i32::MAX, 2)]], vec![5000]);
  assert_impacts_eq(
    vec![
      vec![Impact::new(3, 10), Impact::new(5, 12), Impact::new(8, 13)],
      vec![
        Impact::new(3, 10),
        Impact::new(5, 11),
        Impact::new(8, 13),
        Impact::new(12, 14),
      ],
    ],
    vec![110, 945],
    &merged_impacts.get_impacts()?,
  )?;

  // First level of impacts2 doesn't cover the first level of impacts1
  merged_impacts.impacts_enums.all_disi[1].reset(
    vec![
      vec![Impact::new(2, 10), Impact::new(6, 13)],
      vec![Impact::new(3, 9), Impact::new(5, 11), Impact::new(7, 13)],
    ],
    vec![90, 1000],
  );
  assert_impacts_eq(
    vec![
      vec![Impact::new(3, 10), Impact::new(5, 12), Impact::new(7, 13)],
      vec![Impact::new(3, 10), Impact::new(5, 11), Impact::new(7, 13)],
    ],
    vec![110, 945],
    &merged_impacts.get_impacts()?,
  )?;

  // First level of impacts2 doesn't cover the first level of impacts1
  merged_impacts.impacts_enums.all_disi[1].reset(
    vec![
      vec![Impact::new(2, 10), Impact::new(6, 11)],
      vec![Impact::new(3, 9), Impact::new(5, 11), Impact::new(7, 13)],
    ],
    vec![150, 900],
  );
  assert_impacts_eq(
    vec![
      vec![
        Impact::new(2, 10),
        Impact::new(3, 11),
        Impact::new(5, 12),
        Impact::new(6, 13),
      ],
      vec![
        Impact::new(3, 10),
        Impact::new(5, 11),
        Impact::new(8, 13),
        Impact::new(12, 14),
      ],
    ],
    vec![110, 945],
    &merged_impacts.get_impacts()?,
  )?;

  merged_impacts.impacts_enums.all_disi[1].reset(
    vec![
      vec![Impact::new(4, 10), Impact::new(9, 13)],
      vec![
        Impact::new(1, 1),
        Impact::new(4, 10),
        Impact::new(5, 11),
        Impact::new(8, 13),
        Impact::new(12, 14),
        Impact::new(13, 15),
      ],
    ],
    vec![113, 950],
  );
  assert_impacts_eq(
    vec![
      vec![Impact::new(3, 10), Impact::new(4, 12), Impact::new(8, 13)],
      vec![
        Impact::new(3, 10),
        Impact::new(5, 11),
        Impact::new(8, 13),
        Impact::new(12, 14),
      ],
    ],
    vec![110, 945],
    &merged_impacts.get_impacts()?,
  )?;

  // Make sure negative norms are treated as unsigned
  merged_impacts.impacts_enums.all_disi[0].reset(
    vec![
      vec![Impact::new(3, 10), Impact::new(5, -10), Impact::new(8, -5)],
      vec![
        Impact::new(3, 10),
        Impact::new(5, -15),
        Impact::new(8, -5),
        Impact::new(12, -3),
      ],
    ],
    vec![110, 945],
  );

  merged_impacts.impacts_enums.all_disi[1].reset(
    vec![
      vec![Impact::new(2, 10), Impact::new(12, -4)],
      vec![Impact::new(3, 9), Impact::new(12, -4), Impact::new(20, -1)],
    ],
    vec![150, 960],
  );

  assert_impacts_eq(
    vec![
      vec![Impact::new(2, 10), Impact::new(8, -4)],
      vec![Impact::new(3, 10), Impact::new(8, -4), Impact::new(12, -3)],
    ],
    vec![110, 945],
    &merged_impacts.get_impacts()?,
  )?;

  Ok(())
}
fn assert_impacts_eq(
  impacts: Vec<Vec<Impact>>,
  doc_id_upto: Vec<i32>,
  actual: &impl Impacts,
) -> Result<()> {
  assert_eq!(impacts.len(), actual.num_levels() as usize);

  for i in 0..impacts.len() {
    assert_eq!(doc_id_upto[i], actual.get_doc_id_upto(i as i32));

    let actual_impacts = actual.get_impacts(i as i32)?;
    let expect = impacts[i].as_slice();
    assert_eq!(expect, actual_impacts.as_slice());
  }
  Ok(())
}

struct DummyImpactsEnum {
  cost: i64,
  impacts: Rc<Vec<Vec<Impact>>>,
  doc_id_upto: Rc<Vec<i32>>,
}
impl DummyImpactsEnum {
  fn new(cost: i64) -> Self {
    Self {
      cost,
      impacts: Rc::new(vec![vec![]]),
      doc_id_upto: Rc::new(vec![]),
    }
  }

  fn reset(&mut self, impacts: Vec<Vec<Impact>>, doc_id_upto: Vec<i32>) {
    self.impacts = Rc::new(impacts);
    self.doc_id_upto = Rc::new(doc_id_upto);
  }
}

impl PostingsEnum for DummyImpactsEnum {
  fn freq(&mut self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn next_position(&mut self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn start_offset(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn end_offset(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl DocIdSetIterator for DummyImpactsEnum {
  fn doc_id(&self) -> i32 {
    unreachable!("")
  }

  fn next_doc(&mut self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.cost)
  }
}

impl ImpactsSource for DummyImpactsEnum {
  fn advance_shallow(&mut self, _target: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  type Impacts<'a>
    = ImpactsImpl
  where
    Self: 'a;

  fn get_impacts(&self) -> Result<Self::Impacts<'_>> {
    Ok(ImpactsImpl::new(
      self.impacts.clone(),
      self.doc_id_upto.clone(),
    ))
  }
}

impl ImpactsEnum for DummyImpactsEnum {}
struct ImpactsImpl {
  impacts: Rc<Vec<Vec<Impact>>>,
  doc_id_upto: Rc<Vec<i32>>,
}
impl ImpactsImpl {
  fn new(impacts: Rc<Vec<Vec<Impact>>>, doc_id_upto: Rc<Vec<i32>>) -> Self {
    Self {
      impacts,
      doc_id_upto,
    }
  }
}
impl Impacts for ImpactsImpl {
  fn num_levels(&self) -> i32 {
    self.impacts.len() as i32
  }

  fn get_doc_id_upto(&self, level: i32) -> i32 {
    self.doc_id_upto[level as usize]
  }

  fn get_impacts(&self, level: i32) -> Result<Vec<Impact>> {
    Ok(self.impacts[level as usize].clone())
  }
}

#[test]
fn test_random_top_docs() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let config = new_index_writer_config(&mut random);
  let writer = RandomIndexWriter::with_config(&mut random, dir.clone(), config);

  let num_docs = if is_night_mode() {
    at_least(&mut random, 128 * 8 * 8 * 3)
  } else {
    at_least(&mut random, 100)
  };
  for _ in 0..num_docs {
    let mut doc = Document::new();
    let shift = random.random_range(0..5);
    let num_terms = random.random_range(0..(1 << shift));
    let mut text = String::new();
    for i in 0..num_terms {
      if i > 0 {
        text.push(' ');
      }
      if random.random_range(0..2) == 0 {
        text.push('a');
      } else if random.random_range(0..2) == 0 {
        text.push('b');
      } else {
        text.push('c');
      }
    }
    doc.add(TextField::from_string("foo", &text, Store::No)?);
    writer.add_document(doc)?;
  }

  let reader = writer.get_reader()?;
  writer.close()?;
  let searcher = new_searcher_with_reader(reader)?;

  for first_term in &["a", "b", "c"] {
    for second_term in &["a", "b", "c"] {
      let query: Query = PhraseQuery::from_terms(0, "foo", &[first_term, second_term])?.into();

      let complete_manager = TopScoreDocCollectorManager::new(10, i32::MAX as usize)?;
      let top_scores_manager = TopScoreDocCollectorManager::new(10, 10)?;

      let complete = searcher.search_with_collector_manager(query.clone(), &complete_manager)?;
      let top_scores =
        searcher.search_with_collector_manager(query.clone(), &top_scores_manager)?;
      CheckHits::check_equal(&query, complete.score_docs(), top_scores.score_docs())?;

      let mut filtered_builder = Builder::new();
      filtered_builder.add(query.clone(), Occur::Must)?;
      filtered_builder.add(TermQuery::new(Term::from_text("foo", "b")), Occur::Filter)?;
      let filtered_query: Query = filtered_builder.build().into();

      let complete_manager = TopScoreDocCollectorManager::new(10, i32::MAX as usize)?;
      let top_scores_manager = TopScoreDocCollectorManager::new(10, 10)?;

      let complete =
        searcher.search_with_collector_manager(filtered_query.clone(), &complete_manager)?;
      let top_scores =
        searcher.search_with_collector_manager(filtered_query.clone(), &top_scores_manager)?;
      CheckHits::check_equal(&query, complete.score_docs(), top_scores.score_docs())?;
    }
  }

  Ok(())
}

#[test]
fn test_null_term() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
