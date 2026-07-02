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
use crate::core::document::string_field::StringField;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, new_string_field, new_text_field, random,
};

use crate::core::search::simple_collector::SimpleCollector;
use crate::core::search::term_query::TermQuery;
use crate::core::search::term_range_query::TermRangeQuery;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::query_utils::QueryUtils;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

#[allow(dead_code)] // for quick search
struct TestConstantScoreQuery;

#[test]
fn test_csq() -> Result<()> {
  let q1: Query = ConstantScoreQuery::new(TermQuery::new(Term::from_text("a", "b"))).into();
  let q2: Query = ConstantScoreQuery::new(TermQuery::new(Term::from_text("a", "c"))).into();
  let q3: Query = ConstantScoreQuery::new(TermRangeQuery::new_string_range(
    "a",
    Some("b"),
    Some("c"),
    true,
    true,
  )?)
  .into();

  QueryUtils::check_from_query(&q1);
  QueryUtils::check_from_query(&q2);
  QueryUtils::check_equal(&q1, &q1);
  QueryUtils::check_equal(&q2, &q2);
  QueryUtils::check_equal(&q3, &q3);
  QueryUtils::check_unequal(&q1, &q2);
  QueryUtils::check_unequal(&q2, &q3);
  QueryUtils::check_unequal(&q1, &q3);
  QueryUtils::check_unequal(&q1, &TermQuery::new(Term::from_text("a", "b")).into());

  Ok(())
}
fn check_hits<IRC>(searcher: &IndexSearcher<IRC>, q: Query, expected_score: f32) -> Result<()>
where
  IRC: IndexReaderContext + Sync,
{
  let count = Arc::new(AtomicI32::new(0));
  let manager = CollectorManagerImpl::new(expected_score, count.clone());
  searcher.search_with_collector_manager(q, &manager)?;
  assert_eq!(1, count.load(Ordering::SeqCst));
  Ok(())
}
#[test]
fn test_wrapped_2_times() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, directory)?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("field", "term1", Store::No)?);
  doc.add(StringField::from_string("field", "term2", Store::No)?);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let mut searcher = new_searcher_with_reader(reader)?;
  searcher.set_query_cache(None);

  let csq1 = BoostQuery::new(
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("field", "term1"))),
    2.0,
  )?;
  let csq2 = BoostQuery::new(
    ConstantScoreQuery::new(ConstantScoreQuery::new(TermQuery::new(Term::from_text(
      "field", "term2",
    )))),
    5.0,
  )?;

  let mut bq = Builder::new();
  bq.add(csq1.clone(), Occur::Should)?;
  bq.add(csq2.clone(), Occur::Should)?;

  let csqbq = BoostQuery::new(ConstantScoreQuery::new(bq.build()), 17.0)?;

  check_hits(&searcher, csq1.clone().into(), csq1.get_boost())?;
  check_hits(&searcher, csq2.clone().into(), csq2.get_boost())?;

  check_hits(&searcher, csqbq.clone().into(), csqbq.get_boost())?;

  Ok(())
}

#[test]
fn test_constant_score_query_and_filter() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let w = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "field",
    "a",
    Store::No,
    &mut field_to_type,
  )?);
  w.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "field",
    "b",
    Store::No,
    &mut field_to_type,
  )?);
  w.add_document(&mut random, doc)?;

  let reader = w.get_reader(&mut random)?;
  w.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;

  let filter_b: Query = TermQuery::new(Term::from_text("field", "b")).into();
  let query: Query = ConstantScoreQuery::new(filter_b.clone()).into();

  let mut builder = Builder::new();
  builder
    .add(query, Occur::Must)?
    .add(filter_b.clone(), Occur::Filter)?;
  let mut filtered: Query = builder.build().into();

  assert_eq!(1, searcher.count(filtered)?); // Query for field:b, Filter field:b

  let filter_a: Query = TermQuery::new(Term::from_text("field", "a")).into();
  let query: Query = ConstantScoreQuery::new(filter_a).into();

  builder = Builder::new();
  builder
    .add(query, Occur::Must)?
    .add(filter_b, Occur::Filter)?;
  filtered = builder.build().into();

  assert_eq!(0, searcher.count(filtered)?); // Query field:b, Filter field:a

  Ok(())
}

#[test]
fn test_propagates_approximations() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type = HashMap::new();

  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "a b",
    Store::No,
    &mut field_to_type,
  )?);
  writer.add_document(&mut random, doc)?;
  writer.commit(&mut random)?;

  let reader = writer.get_reader(&mut random)?;
  let mut searcher = new_searcher_with_reader(reader)?;
  searcher.set_query_cache(None); // to still have approximations

  let pq: Query = PhraseQuery::from_terms(0, "field", &["a", "b"])?.into();
  let csq: Query = ConstantScoreQuery::new(pq).into();

  let rewritten = searcher.rewrite(csq)?;
  let weight = rewritten.create_weight(&searcher, &ScoreMode::Complete, 1.0)?;

  let ctx = &searcher.get_leaf_contexts()?[0];
  let scorer = weight.scorer(ctx, &searcher)?.unwrap();

  assert!(scorer.two_phase_iterator().is_some());

  Ok(())
}

#[test]
fn test_rewrite_bubbles_up_match_no_docs_query() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;
  let query: Query = MatchNoDocsQuery::new().into();
  let query = ConstantScoreQuery::new(query);
  let rewritten = searcher.rewrite(query)?;
  assert_eq!(rewritten, Query::MatchNoDocs(MatchNoDocsQuery::new()));
  Ok(())
}

struct CollectorManagerImpl {
  expected_score: f32,
  count: Arc<AtomicI32>,
}
impl CollectorManagerImpl {
  fn new(expected_score: f32, count: Arc<AtomicI32>) -> Self {
    Self {
      expected_score,
      count,
    }
  }
}

impl CollectorManager for CollectorManagerImpl {
  type C = SimpleCollectorImpl;
  type T = ();

  fn new_collector(&self) -> Result<Self::C> {
    Ok(SimpleCollectorImpl {
      expected_score: self.expected_score,
      count: self.count.clone(),
    })
  }

  fn reduce(&self, _collectors: Vec<Self::C>) -> Result<Self::T> {
    Ok(())
  }
}

struct SimpleCollectorImpl {
  expected_score: f32,
  count: Arc<AtomicI32>,
}

impl Collector for SimpleCollectorImpl {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    SimpleCollector::do_set_next_reader(self, context)?;
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::Complete
  }
}

impl LeafCollector for SimpleCollectorImpl {
  fn set_scorer(&mut self, _scorer: &mut dyn Scorable) -> Result<()> {
    Ok(())
  }

  fn collect(&mut self, _doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let score = scorer.score()?;
    assert!(
      (score - self.expected_score).abs() <= 0.00001f32,
      "Score differs from expected: got={}, expected={}",
      score,
      self.expected_score
    );
    self.count.fetch_add(1, Ordering::SeqCst);
    Ok(())
  }
}

impl SimpleCollector for SimpleCollectorImpl {}

impl Display for SimpleCollectorImpl {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}
