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
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::search::weight::Weight;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random_from_seed;
use rand::Rng;
use rand::RngExt;
use rand::prelude::StdRng;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct RandomApproximationQuery {
  id: Identity,
  query: Box<Query>,
  random_seed: u64,
}
impl RandomApproximationQuery {
  pub(crate) fn new<Q, R>(query: Q, random: &mut R) -> Self
  where
    Q: Into<Box<Query>>,
    R: Rng + ?Sized,
  {
    let query = query.into();
    let random_seed = random.random();
    Self {
      id: Identity::new(),
      query,
      random_seed,
    }
  }
}
impl Hash for RandomApproximationQuery {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.query.hash(state);
  }
}
impl Eq for RandomApproximationQuery {}

impl PartialEq for RandomApproximationQuery {
  fn eq(&self, other: &Self) -> bool {
    self.query.eq(&other.query)
  }
}

impl HasIdentity for RandomApproximationQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for RandomApproximationQuery {
  fn as_string(&self, field: &str) -> Result<String> {
    self.query.as_string(field)
  }

  fn create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    _score_mode: &ScoreMode,
    _boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    todo!()
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    todo!()
  }

  fn visit<QV>(&self, visitor: &QV)
  where
    QV: QueryVisitor,
  {
    self.query.visit(visitor);
  }
}

pub struct RandomApproximationWeight<LR>
where
  LR: IndexReaderContext + 'static,
  IRCLeafReader<LR>: 'static,
{
  query: Arc<Query>,
  random_seed: u64,
  in_: QueryWeight<LR>,
}
impl<LR> RandomApproximationWeight<LR>
where
  LR: IndexReaderContext + 'static,
  IRCLeafReader<LR>: 'static,
{
  fn new(query: RandomApproximationQuery, random_seed: u64, weight: QueryWeight<LR>) -> Self {
    let query = Arc::new(query.into());
    Self {
      query,
      random_seed,
      in_: weight,
    }
  }
}

impl<LR> SegmentCacheable<LR> for RandomApproximationWeight<LR>
where
  LR: IndexReaderContext + 'static,
  IRCLeafReader<LR>: 'static,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<LR>>) -> Result<bool> {
    self.in_.is_cacheable(ctx)
  }
}

impl<LR> Weight<LR> for RandomApproximationWeight<LR>
where
  LR: IndexReaderContext + 'static,
  IRCLeafReader<LR>: 'static,
{
  type Matches = MatchWithNoTerms;

  fn matches(
    &self,
    context: &LeafReaderContext<IRCLeafReader<LR>>,
    doc: i32,
    searcher: &IndexSearcher<LR>,
  ) -> Result<Option<Self::Matches>> {
    self.in_.matches(context, doc, searcher)
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<LR>>,
    doc: i32,
    searcher: &IndexSearcher<LR>,
  ) -> Result<Explanation> {
    self.in_.explain(context, doc, searcher)
  }

  fn get_query(&self) -> Arc<Query> {
    self.in_.get_query()
  }

  type ScorerSupplier = QueryWeightSs<LR>;

  fn scorer_supplier(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<LR>>,
    _searcher: &IndexSearcher<LR>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    todo!()
  }
}

pub struct RandomApproximationScorer<S>
where
  S: Scorer,
{
  random_seed: u64,
  scorer: S,
  two_phase_view: RandomTwoPhaseView<Box<dyn DocIdSetIterator>>,
}
impl<S> RandomApproximationScorer<S>
where
  S: Scorer,
{
  pub fn new(_random_seed: u64, _scorer: S) -> Self {
    todo!()
  }
}

impl<S> Scorable for RandomApproximationScorer<S>
where
  S: Scorer,
{
  fn score(&mut self) -> Result<f32> {
    self.scorer.score()
  }

  fn cost(&self) -> Result<i64> {
    self.iterator().cost()
  }
}

impl<S> crate::core::search::scorable::FixedScore for RandomApproximationScorer<S> where S: Scorer {}

impl<S> Scorer for RandomApproximationScorer<S>
where
  S: Scorer,
{
  fn doc_id(&mut self) -> Result<i32> {
    todo!()
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    todo!()
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    todo!()
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    todo!()
  }

  fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
    todo!()
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    todo!()
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    todo!()
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    todo!()
  }
}

pub struct RandomTwoPhaseView<DISI>
where
  DISI: DocIdSetIterator,
{
  approximation: RandomApproximation<StdRng, DISI>,
  last_doc: i32,
  random_match_cost: f32,
}
impl<DISI> RandomTwoPhaseView<DISI>
where
  DISI: DocIdSetIterator,
{
  pub fn new<R: Rng + ?Sized>(random: &mut R, disi: DISI) -> Self {
    let seed = random.random();
    let new_random = random_from_seed(seed);
    let random_approximation = RandomApproximation::new(new_random, disi);
    Self {
      approximation: random_approximation,
      last_doc: -1,
      random_match_cost: random.random::<f32>() * 200f32,
    }
  }
  pub fn disi(&self) -> &DISI {
    &self.approximation.disi
  }
  pub fn disi_mut(&mut self) -> &mut DISI {
    &mut self.approximation.disi
  }
}
impl<DISI> TwoPhaseIterator for RandomTwoPhaseView<DISI>
where
  DISI: DocIdSetIterator,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.approximation)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.approximation)
  }

  fn matches(&mut self) -> Result<bool> {
    let approx_doc = self.approximation.doc_id();

    if approx_doc == -1 || approx_doc == NO_MORE_DOCS {
      return Err(LuceneError::illegal_state(format!(
        "matches() should not be called on doc ID {}",
        approx_doc
      )));
    }

    if self.last_doc == approx_doc {
      return Err(LuceneError::illegal_state(format!(
        "matches() has been called twice on doc ID {}",
        approx_doc
      )));
    }
    self.last_doc = approx_doc;
    Ok(approx_doc == self.approximation.disi.doc_id())
  }

  fn match_cost(&self) -> f32 {
    self.random_match_cost
  }
}
pub struct RandomApproximation<RNG, DISI>
where
  RNG: Rng,
  DISI: DocIdSetIterator,
{
  random: RNG,
  disi: DISI,
  doc: i32,
}

impl<RNG, DISI> RandomApproximation<RNG, DISI>
where
  RNG: Rng,
  DISI: DocIdSetIterator,
{
  pub fn new(random: RNG, disi: DISI) -> Self {
    Self {
      random,
      disi,
      doc: -1,
    }
  }
}

impl<RNG, DISI> DocIdSetIterator for RandomApproximation<RNG, DISI>
where
  RNG: Rng,
  DISI: DocIdSetIterator,
{
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.advance(self.doc + 1)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    if self.disi.doc_id() < target {
      self.disi.advance(target)?;
    }
    let disi_doc = self.disi.doc_id();
    if disi_doc == NO_MORE_DOCS {
      self.doc = NO_MORE_DOCS;
      return Ok(self.doc);
    }

    let picked = self.random.random_range(target..=disi_doc);
    self.doc = picked;
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    self.disi.cost()
  }
}
