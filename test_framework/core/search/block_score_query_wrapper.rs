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
use crate::core::search::approximation_disi::ApproximationBitSetAccess;
use crate::core::search::approximation_disi::ApproximationDISI;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::QueryWeightMatches;
use crate::core::search::query::QueryWeightSsBulkScorer;
use crate::core::search::query::QueryWeightSsScorer;
use crate::core::search::query::{IntoBoxQuery, Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{CoreHelper, HasIdentity};
use std::cell::Cell;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Query wrapper that reduces the size of max-score blocks to more easily detect problems with the max-score logic.
#[derive(Clone, Default, Debug)]
pub struct BlockScoreQueryWrapper {
  query: Box<Query>,
  block_length: usize,
  id: Identity,
}
impl BlockScoreQueryWrapper {
  pub fn new<T>(query: T, block_length: usize) -> Self
  where
    T: IntoBoxQuery,
  {
    Self {
      query: query.into_box_query(),
      block_length,
      id: Identity::new(),
    }
  }
}

impl HasIdentity for BlockScoreQueryWrapper {
  fn identity(&self) -> &Identity {
    &self.id
  }
}
impl PartialEq for BlockScoreQueryWrapper {
  fn eq(&self, other: &Self) -> bool {
    self.query == other.query && self.block_length == other.block_length
  }
}
impl Eq for BlockScoreQueryWrapper {}
impl Hash for BlockScoreQueryWrapper {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.query.hash(state);
    self.block_length.hash(state);
  }
}

impl QueryBase for BlockScoreQueryWrapper {
  type Weight<IRC, SIM>
    = BlockScoreQueryWeight<IRC, SIM>
  where
    SIM: crate::core::search::similarities_impl::similarities::Similarity,
    IRC: IndexReaderContext + 'static;

  fn to_string(&self, field: &str) -> Result<String> {
    self.query.to_string(field)
  }

  fn create_weight<IRC, SIM>(
    self,
    searcher: &IndexSearcher<IRC, SIM>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<Self::Weight<IRC, SIM>>
  where
    SIM: crate::core::search::similarities_impl::similarities::Similarity,
    IRC: IndexReaderContext + 'static,
    crate::core::search::index_searcher::IndexSearcher<IRC, SIM>: Sync,
    Self: Sized,
  {
    let query = (*self.query).clone();
    let in_weight = query.create_weight(searcher, score_mode, boost)?;
    if !score_mode.needs_scores() {
      return Ok(BlockScoreQueryWeight::Inner(in_weight));
    }
    Ok(BlockScoreQueryWeight::Scoring(BlockScoreWeight::new(
      self, in_weight,
    )))
  }

  fn rewrite<IRC, SIM>(&self, searcher: &IndexSearcher<IRC, SIM>) -> Result<Option<Query>>
  where
    SIM: crate::core::search::similarities_impl::similarities::Similarity,
    IRC: IndexReaderContext + 'static,
    crate::core::search::index_searcher::IndexSearcher<IRC, SIM>: Sync,
    Self: Sized,
  {
    match self.query.rewrite(searcher)? {
      Some(rewritten) => Ok(Some(
        BlockScoreQueryWrapper::new(rewritten, self.block_length).into(),
      )),
      None => Ok(None),
    }
  }

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    self.query.visit(visitor)
  }
}

pub struct BlockScoreWeight<
  IRC: IndexReaderContext + 'static,
  SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity,
> where
  SIM: crate::core::search::similarities_impl::similarities::Similarity,
  IRC: IndexReaderContext + 'static,
{
  query: Arc<Query>,
  in_weight: QueryWeight<IRC, SIM>,
  block_length: usize,
}
impl<IRC, SIM> BlockScoreWeight<IRC, SIM>
where
  SIM: crate::core::search::similarities_impl::similarities::Similarity,
  IRC: IndexReaderContext + 'static,
{
  fn new(query: BlockScoreQueryWrapper, in_weight: QueryWeight<IRC, SIM>) -> Self {
    Self {
      block_length: query.block_length,
      query: Arc::new(query.into()),
      in_weight,
    }
  }
}

impl<IRC, SIM> SegmentCacheable<IRC> for BlockScoreWeight<IRC, SIM>
where
  SIM: crate::core::search::similarities_impl::similarities::Similarity,
  IRC: IndexReaderContext + 'static,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    self.in_weight.is_cacheable(ctx)
  }
}

impl<IRC, SIM> Weight<IRC, SIM> for BlockScoreWeight<IRC, SIM>
where
  SIM: crate::core::search::similarities_impl::similarities::Similarity,
  IRC: IndexReaderContext + 'static,
{
  fn matches<'a>(
    &'a self,
    context: &'a LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &'a IndexSearcher<IRC, SIM>,
  ) -> Result<Option<crate::core::search::query::QueryWeightMatches<'a, IRC, SIM>>>
  where
    crate::core::search::index_searcher::IndexSearcher<IRC, SIM>: Sync,
  {
    self.in_weight.matches(context, doc, searcher)
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Explanation>
  where
    crate::core::search::index_searcher::IndexSearcher<IRC, SIM>: Sync,
  {
    self.in_weight.explain(context, doc, searcher)
  }

  fn get_query(&self) -> Arc<Query> {
    self.query.clone()
  }

  type ScorerSupplier = DefaultScorerSupplier<BlockScoreScorer>;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<Self::ScorerSupplier>>
  where
    crate::core::search::index_searcher::IndexSearcher<IRC, SIM>: Sync,
  {
    let in_scorer = self.in_weight.scorer(context, searcher)?;
    let Some(mut in_scorer) = in_scorer else {
      return Ok(None);
    };

    let mut tmp_docs = Vec::with_capacity(2);
    let mut tmp_scores = Vec::with_capacity(2);
    tmp_docs.push(-1);
    tmp_scores.push(0.0);

    loop {
      let doc = in_scorer.next_doc()?;

      let size = tmp_docs.len() + 1;
      ArrayUtil::grow_capacity(&mut tmp_docs, size)?;
      ArrayUtil::grow_capacity(&mut tmp_scores, size)?;
      tmp_docs.push(doc);

      if doc == NO_MORE_DOCS {
        tmp_scores.push(0.0);
        break;
      }

      tmp_scores.push(in_scorer.score()?);
    }

    let ss = BlockScoreScorer::new(tmp_docs, tmp_scores, self.block_length);
    Ok(Some(DefaultScorerSupplier::new(ss)))
  }
}

pub struct BlockScoreScorer {
  docs: Vec<i32>,
  scores: Vec<f32>,
  block_length: usize,
  i: Cell<usize>,
  last_shallow_target: Cell<i32>,
}
impl BlockScoreScorer {
  fn new(docs: Vec<i32>, scores: Vec<f32>, block_length: usize) -> Self {
    Self {
      docs,
      scores,
      block_length,
      i: Cell::new(0),
      last_shallow_target: Cell::new(-1),
    }
  }

  fn index_for_target(&self, target: i32) -> usize {
    match self.docs.binary_search(&target) {
      Ok(i) | Err(i) => i,
    }
  }

  fn start_of_block(&self, target: i32) -> usize {
    let i = self.index_for_target(target);
    i - i % self.block_length
  }

  fn end_of_block(&self, target: i32) -> usize {
    std::cmp::min(
      self.start_of_block(target) + self.block_length,
      self.docs.len() - 1,
    )
  }
}

impl Scorable for BlockScoreScorer {
  type Child<'a>
    = crate::core::search::dummy::dummy_scorable::DummyScorable
  where
    Self: 'a;

  fn score(&mut self) -> Result<f32> {
    Ok(self.scores[self.i.get()])
  }

  fn cost(&self) -> Result<i64> {
    Ok((self.docs.len() as i64) - 2)
  }
}

impl FixedScore for BlockScoreScorer {}

impl ApproximationDISI for BlockScoreScorer {
  fn approx_doc_id(&self) -> i32 {
    DocIdSetIterator::doc_id(self)
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    DocIdSetIterator::next_doc(self)
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    DocIdSetIterator::advance(self, target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    DocIdSetIterator::slow_advance(self, target)
  }
  fn approx_cost(&self) -> Result<i64> {
    DocIdSetIterator::cost(self)
  }
}

impl ApproximationBitSetAccess for BlockScoreScorer {
  fn approx_is_bit_iter(&self) -> bool {
    crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::is_bit_iter(self)
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::get(self, index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::set_doc_id(self, doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::bit_set_length(self)
  }
}

impl TwoPhaseIterator for BlockScoreScorer {
  fn matches(&mut self) -> Result<bool> {
    Ok(true)
  }
  fn match_cost(&self) -> f32 {
    0.0
  }
}

impl Scorer for BlockScoreScorer {
  fn scoring_doc_id(&mut self) -> Result<i32> {
    Ok(self.docs[self.i.get()])
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.last_shallow_target.set(target);
    if target == NO_MORE_DOCS {
      return Ok(NO_MORE_DOCS);
    }
    Ok(self.docs[self.end_of_block(target)] - 1)
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    let start_target = std::cmp::max(self.docs[self.i.get()], self.last_shallow_target.get());
    let mut max_score = 0.0f32;
    let mut j = self.start_of_block(start_target);
    loop {
      if self.docs[j] > upto {
        break;
      }
      max_score = CoreHelper::max_f32(max_score, self.scores[j]);
      if j == self.docs.len() - 1 {
        break;
      }
      j += 1;
    }
    Ok(max_score)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }
}

impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for BlockScoreScorer {}
impl crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for BlockScoreScorer {}

impl DocIdSetIterator for BlockScoreScorer {
  fn doc_id(&self) -> i32 {
    self.docs[self.i.get()]
  }

  fn next_doc(&mut self) -> Result<i32> {
    let i = self.i.get();
    assert!(self.docs[i] != NO_MORE_DOCS);
    self.i.set(i + 1);
    Ok(self.docs[i + 1])
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let i = self.index_for_target(target);
    self.i.set(i);
    Ok(self.docs[i])
  }

  fn cost(&self) -> Result<i64> {
    Ok((self.docs.len() as i64) - 2)
  }
}

impl crate::core::util::accountable::Accountable for BlockScoreQueryWrapper {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}

/// Weight returned by the actual branches of this query factory.
pub enum BlockScoreQueryWeight<
  IRC: IndexReaderContext + 'static,
  SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity,
> where
  SIM: crate::core::search::similarities_impl::similarities::Similarity,
{
  Scoring(BlockScoreWeight<IRC, SIM>),
  Inner(QueryWeight<IRC, SIM>),
}
impl<IRC: IndexReaderContext + 'static, SIM> SegmentCacheable<IRC>
  for BlockScoreQueryWeight<IRC, SIM>
where
  SIM: crate::core::search::similarities_impl::similarities::Similarity,
{
  fn is_cacheable(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    match self {
      Self::Scoring(inner) => SegmentCacheable::<IRC>::is_cacheable(inner, context),
      Self::Inner(inner) => SegmentCacheable::<IRC>::is_cacheable(inner, context),
    }
  }
}
impl<IRC: IndexReaderContext + 'static, SIM> Weight<IRC, SIM> for BlockScoreQueryWeight<IRC, SIM>
where
  SIM: crate::core::search::similarities_impl::similarities::Similarity,
{
  type ScorerSupplier = QueryWeightSs<IRC, SIM>;
  fn matches<'a>(
    &'a self,
    context: &'a LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &'a IndexSearcher<IRC, SIM>,
  ) -> Result<Option<QueryWeightMatches<'a, IRC, SIM>>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    match self {
      Self::Scoring(inner) => Weight::<IRC, SIM>::matches(inner, context, doc, searcher),
      Self::Inner(inner) => Weight::<IRC, SIM>::matches(inner, context, doc, searcher),
    }
  }
  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Explanation>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    match self {
      Self::Scoring(inner) => Weight::<IRC, SIM>::explain(inner, context, doc, searcher),
      Self::Inner(inner) => Weight::<IRC, SIM>::explain(inner, context, doc, searcher),
    }
  }
  fn get_query(&self) -> Arc<Query> {
    match self {
      Self::Scoring(inner) => Weight::<IRC, SIM>::get_query(inner),
      Self::Inner(inner) => Weight::<IRC, SIM>::get_query(inner),
    }
  }
  fn scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<QueryWeightSsScorer<IRC, SIM>>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    match self {
      Self::Scoring(inner) => Weight::<IRC, SIM>::scorer(inner, context, searcher)
        .map(|scorer| scorer.map(crate::core::search::query::QueryWeightSsScorer::<IRC, SIM>::new)),
      Self::Inner(inner) => Weight::<IRC, SIM>::scorer(inner, context, searcher),
    }
  }
  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<Self::ScorerSupplier>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    match self {
      Self::Scoring(inner) => Weight::<IRC, SIM>::scorer_supplier(inner, context, searcher)
        .map(|supplier| supplier.map(Into::into)),
      Self::Inner(inner) => Weight::<IRC, SIM>::scorer_supplier(inner, context, searcher),
    }
  }
  fn bulk_scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<QueryWeightSsBulkScorer<IRC, SIM>>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    match self {
      Self::Scoring(inner) => Weight::<IRC, SIM>::bulk_scorer(inner, context, searcher)
        .map(|scorer| scorer.map(QueryWeightSsBulkScorer::<IRC, SIM>::new)),
      Self::Inner(inner) => Weight::<IRC, SIM>::bulk_scorer(inner, context, searcher),
    }
  }
  fn count(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<i32>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    match self {
      Self::Scoring(inner) => Weight::<IRC, SIM>::count(inner, context, searcher),
      Self::Inner(inner) => Weight::<IRC, SIM>::count(inner, context, searcher),
    }
  }
  fn default_count(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    match self {
      Self::Scoring(inner) => Weight::<IRC, SIM>::default_count(inner, context),
      Self::Inner(inner) => Weight::<IRC, SIM>::default_count(inner, context),
    }
  }
  fn is_cache_wrapper(&self) -> bool {
    match self {
      Self::Scoring(inner) => Weight::<IRC, SIM>::is_cache_wrapper(inner),
      Self::Inner(inner) => Weight::<IRC, SIM>::is_cache_wrapper(inner),
    }
  }
  fn into_inner_weight(self: Box<Self>) -> Option<QueryWeight<IRC, SIM>> {
    match *self {
      Self::Scoring(_) => None,
      Self::Inner(inner) => inner.into_inner_weight(),
    }
  }
  #[cfg(test)]
  fn as_any(&mut self) -> &mut dyn std::any::Any {
    match self {
      Self::Scoring(inner) => Weight::<IRC, SIM>::as_any(inner),
      Self::Inner(inner) => Weight::<IRC, SIM>::as_any(inner),
    }
  }
}
impl<IRC: IndexReaderContext + 'static, SIM> From<BlockScoreQueryWeight<IRC, SIM>>
  for QueryWeight<IRC, SIM>
where
  SIM: crate::core::search::similarities_impl::similarities::Similarity,
{
  fn from(value: BlockScoreQueryWeight<IRC, SIM>) -> Self {
    match value {
      BlockScoreQueryWeight::Scoring(inner) => inner.into(),
      BlockScoreQueryWeight::Inner(inner) => inner,
    }
  }
}
