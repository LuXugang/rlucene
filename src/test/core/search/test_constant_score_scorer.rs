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
use crate::core::index::directory_reader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::{BooleanQuery, Builder};
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::Query;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, ScorerEnum2, TwoPhaseState};
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::search::total_hits::Relation::GreaterThanOrEqualTo;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, new_index_writer_config_with_analyzer,
  new_log_merge_policy, new_searcher_with_reader, new_text_field, random,
};
use rand::Rng;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;

#[allow(dead_code)] // for quick search
struct TestConstantScoreScorer;
pub static FIELD: &str = "f";

pub static VALUES: LazyLock<Vec<String>> = LazyLock::new(|| {
  vec![
    "foo".to_string(),
    "bar".to_string(),
    "foo bar".to_string(),
    "bar foo".to_string(),
    "foo not bar".to_string(),
    "bar foo bar".to_string(),
    "azerty".to_string(),
  ]
});

pub static TERM_QUERY: LazyLock<BooleanQuery> = LazyLock::new(|| {
  let mut builder = Builder::new();
  builder
    .add(TermQuery::new(Term::from_text(FIELD, "foo")), Occur::Must)
    .unwrap();
  builder
    .add(TermQuery::new(Term::from_text(FIELD, "bar")), Occur::Must)
    .unwrap();
  builder.build()
});

pub static PHRASE_QUERY: LazyLock<PhraseQuery> =
  LazyLock::new(|| PhraseQuery::from_terms_no_slop(FIELD, &["foo", "bar"]).unwrap());
#[test]
fn test_matching_score_mode_complete() -> Result<()> {
  let mut random = random();
  test_matching(&mut random, ScoreMode::Complete)
}

#[test]
fn test_matching_score_mode_complete_no_scores() -> Result<()> {
  let mut random = random();
  test_matching(&mut random, ScoreMode::CompleteNoScores)
}
fn test_matching<R>(random: &mut R, score_mode: ScoreMode) -> Result<()>
where
  R: Rng + ?Sized,
{
  let mut scorer = constant_score_scorer(random, TERM_QUERY.clone(), 1.0, score_mode)?;

  let mut doc;

  doc = scorer.iterator_mut().next_doc()?;
  assert_eq!(2, doc);
  assert!((scorer.score()? - 1.0).abs() <= 0.0);

  scorer.set_min_competitive_score(2.0)?;
  assert_eq!(doc, scorer.doc_id()?);
  assert_eq!(doc, scorer.iterator().doc_id());
  assert!((scorer.score()? - 1.0).abs() <= 0.0);

  doc = scorer.iterator_mut().next_doc()?;
  assert_eq!(3, doc);
  assert!((scorer.score()? - 1.0).abs() <= 0.0);

  doc = scorer.iterator_mut().next_doc()?;
  assert_eq!(4, doc);
  assert!((scorer.score()? - 1.0).abs() <= 0.0);

  doc = scorer.iterator_mut().next_doc()?;
  assert_eq!(5, doc);
  assert!((scorer.score()? - 1.0).abs() <= 0.0);

  doc = scorer.iterator_mut().next_doc()?;
  assert_eq!(NO_MORE_DOCS, doc);

  Ok(())
}

#[test]
fn test_matching_score_mode_top_scores() -> Result<()> {
  let mut random = random();

  let mut scorer =
    constant_score_scorer(&mut random, TERM_QUERY.clone(), 1.0, ScoreMode::TopScores)?;

  let mut doc;

  doc = scorer.iterator_mut().next_doc()?;
  assert_eq!(2, doc);
  assert_eq!(1.0, scorer.score()?);

  scorer.set_min_competitive_score(2.0)?;
  assert_eq!(doc, scorer.doc_id()?);
  assert_eq!(doc, scorer.iterator().doc_id());
  assert_eq!(1.0, scorer.score()?);

  doc = scorer.iterator_mut().next_doc()?;
  assert_eq!(NO_MORE_DOCS, doc);

  Ok(())
}
#[test]
fn test_two_phase_matching_score_mode_complete() -> Result<()> {
  let mut random = random();
  test_two_phase_matching(&mut random, ScoreMode::Complete)
}

#[test]
fn test_two_phase_matching_score_mode_complete_no_scores() -> Result<()> {
  let mut random = random();
  test_two_phase_matching(&mut random, ScoreMode::CompleteNoScores)
}

fn test_two_phase_matching<R>(random: &mut R, score_mode: ScoreMode) -> Result<()>
where
  R: Rng + ?Sized,
{
  let mut scorer = constant_score_scorer(random, PHRASE_QUERY.clone(), 1.0, score_mode)?;

  let mut doc;

  doc = scorer.iterator_mut().next_doc()?;
  assert_eq!(2, doc);
  assert_eq!(1.0, scorer.score()?);

  scorer.set_min_competitive_score(2.0)?;
  assert_eq!(doc, scorer.doc_id()?);
  assert_eq!(doc, scorer.iterator().doc_id());
  assert_eq!(1.0, scorer.score()?);

  doc = scorer.iterator_mut().next_doc()?;
  assert_eq!(5, doc);
  assert_eq!(1.0, scorer.score()?);

  doc = scorer.iterator_mut().next_doc()?;
  assert_eq!(NO_MORE_DOCS, doc);

  Ok(())
}
#[test]
fn test_two_phase_matching_score_mode_top_scores() -> Result<()> {
  let mut random = random();

  let mut scorer =
    constant_score_scorer(&mut random, PHRASE_QUERY.clone(), 1.0, ScoreMode::TopScores)?;

  let mut doc;

  doc = scorer.iterator_mut().next_doc()?;
  assert_eq!(2, doc);
  assert_eq!(1.0, scorer.score()?);

  scorer.set_min_competitive_score(2.0)?;
  assert_eq!(doc, scorer.doc_id()?);
  assert_eq!(doc, scorer.iterator().doc_id());
  assert_eq!(1.0, scorer.score()?);

  doc = scorer.iterator_mut().next_doc()?;
  assert_eq!(NO_MORE_DOCS, doc);

  Ok(())
}
fn constant_score_scorer<R, T>(
  random: &mut R,
  query: T,
  score: f32,
  score_mode: ScoreMode,
) -> Result<Scorers>
where
  R: Rng + ?Sized,
  T: Into<Query>,
{
  let query = query.into();
  let directory = new_directory_shared(random)?;

  let mut iwc = new_index_writer_config(random)?;
  iwc.set_merge_policy(new_log_merge_policy(random)?);

  let writer = RandomIndexWriter::with_config(random, directory.clone(), iwc);
  let mut field_to_type = HashMap::new();

  for value in VALUES.iter() {
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      FIELD,
      value,
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(random, doc)?;
  }

  writer.force_merge(random, 1)?;
  let reader = writer.get_reader(random)?;
  writer.close(random)?;
  let searcher = new_searcher_with_reader(reader)?;
  let weight = searcher.create_weight(ConstantScoreQuery::new(query), score_mode, 1.0)?;

  let leaves = searcher.get_top_reader_context().leaves()?;
  assert_eq!(1, leaves.len());

  let context = &leaves[0];
  let scorer = weight
    .scorer(context, &searcher)?
    .ok_or_else(|| LuceneError::illegal_state("scorer is None"))?;
  let has_tpi = scorer.has_two_phase_iterator() == TwoPhaseState::Yes;
  let v = if has_tpi {
    ScorerEnum2::A(ConstantScoreScorer::from_tpi(
      score,
      score_mode,
      scorer.take_two_phase_iterator().unwrap(),
    ))
  } else {
    ScorerEnum2::B(ConstantScoreScorer::from_disi(
      score,
      score_mode,
      scorer.take_iterator(),
    ))
  };
  Ok(v)
}
#[test]
fn test_early_termination() -> Result<()> {
  let mut random = random();

  let analyzer = MockAnalyzer::new(&mut random);
  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwc.set_max_buffered_docs(2);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

  let iw = IndexWriter::new(dir.clone(), iwc)?;

  let num_docs = 50;
  let mut field_to_type = HashMap::new();

  for i in 0..num_docs {
    let mut doc = Document::new();
    let value = if i % 2 == 0 { "foo bar" } else { "baz" };
    doc.add(new_text_field(
      &mut random,
      "key",
      value,
      Store::Yes,
      &mut field_to_type,
    )?);
    iw.add_document(doc)?;
  }

  let ir = directory_reader::open_from_writer(&iw)?;

  let is = new_searcher_with_reader(ir)?;

  let mut c = TopScoreDocCollectorManager::new(10, 10)?;
  let top_docs = is.search_with_collector_manager(
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("key", "foo"))),
    &c,
  )?;
  assert_eq!(11, top_docs.total_hits.value());
  assert_eq!(GreaterThanOrEqualTo, top_docs.total_hits.relation());

  c = TopScoreDocCollectorManager::new(10, 10)?;
  let mut builder = Builder::new();
  builder.add(
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("key", "foo"))),
    Occur::Should,
  )?;
  builder.add(
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("key", "bar"))),
    Occur::Filter,
  )?;
  let query = builder.build();

  let top_docs = is.search_with_collector_manager(query, &c)?;
  assert_eq!(11, top_docs.total_hits.value());
  assert_eq!(GreaterThanOrEqualTo, top_docs.total_hits.relation());

  iw.close()?;
  Ok(())
}
type Scorers = ScorerEnum2<
  ConstantScoreScorer<DummyDISI, Box<dyn TwoPhaseIterator>>,
  ConstantScoreScorer<Box<dyn DocIdSetIterator>, DummyTwoPhaseIterator>,
>;
