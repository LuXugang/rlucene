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
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{IntoBoxQuery, Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::HasIdentity;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::error::lucene_error::Result;
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
  fn to_string(&self, field: &str) -> Result<String> {
    self.query.to_string(field)
  }

  fn create_weight<IRC>(
    self,
    searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let query = (*self.query).clone();
    let in_weight = query.create_weight(searcher, score_mode, boost)?;
    if !score_mode.needs_scores() {
      return Ok(in_weight);
    }
    Ok(Box::new(BlockScoreWeight::new(self, in_weight)))
  }

  fn rewrite<IRC>(mut self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let query_id = self.query.identity().clone();
    let rewritten = self.query.rewrite(searcher)?;
    if rewritten.identity() != &query_id {
      return Ok(BlockScoreQueryWrapper::new(rewritten, self.block_length).into());
    }
    self.query = Box::new(rewritten);
    Ok(self.into())
  }

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    self.query.visit(visitor)
  }
}

struct BlockScoreWeight<IRC>
where
  IRC: IndexReaderContext,
{
  query: Arc<Query>,
  in_weight: QueryWeight<IRC>,
  block_length: usize,
}
impl<IRC> BlockScoreWeight<IRC>
where
  IRC: IndexReaderContext,
{
  fn new(query: BlockScoreQueryWrapper, in_weight: QueryWeight<IRC>) -> Self {
    Self {
      block_length: query.block_length,
      query: Arc::new(query.into()),
      in_weight,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for BlockScoreWeight<IRC>
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    self.in_weight.is_cacheable(ctx)
  }
}

impl<IRC> Weight<IRC> for BlockScoreWeight<IRC>
where
  IRC: IndexReaderContext,
{
  fn matches<'a>(
    &'a self,
    context: &'a LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &'a IndexSearcher<IRC>,
  ) -> Result<Option<crate::core::search::query::QueryWeightMatches<'a>>> {
    self.in_weight.matches(context, doc, searcher)
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    self.in_weight.explain(context, doc, searcher)
  }

  fn get_query(&self) -> Arc<Query> {
    self.query.clone()
  }

  type ScorerSupplier = QueryWeightSs<IRC>;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    let in_scorer = self.in_weight.scorer(context, searcher)?;
    let Some(mut in_scorer) = in_scorer else {
      return Ok(None);
    };

    let mut tmp_docs = vec![0i32; 2];
    let mut tmp_scores = vec![0f32; 2];
    tmp_docs[0] = -1;

    let mut i = 1usize;

    loop {
      let doc = in_scorer.iterator_mut().next_doc()?;

      ArrayUtil::grow_with_len(&mut tmp_docs, i + 1)?;
      ArrayUtil::grow_with_len(&mut tmp_scores, i + 1)?;
      tmp_docs[i] = doc;

      if doc == NO_MORE_DOCS {
        i += 1;
        break;
      }

      tmp_scores[i] = in_scorer.score()?;
      i += 1;
    }

    let docs = tmp_docs[0..i].to_vec();
    let scores = tmp_scores[0..i].to_vec();
    let ss = BlockScoreScorer::new(docs, scores, self.block_length);
    Ok(Some(Box::new(DefaultScorerSupplier::new(ss))))
  }
}

struct BlockScoreScorer {
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
  fn score(&mut self) -> Result<f32> {
    Ok(self.scores[self.i.get()])
  }

  fn cost(&self) -> Result<i64> {
    Ok((self.docs.len() as i64) - 2)
  }
}

impl FixedScore for BlockScoreScorer {}

impl Scorer for BlockScoreScorer {
  fn doc_id(&mut self) -> Result<i32> {
    Ok(self.docs[self.i.get()])
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(BlockScoreIterator { scorer: self })
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(BlockScoreIterator { scorer: self })
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let BlockScoreScorer { docs, i, .. } = *self;
    Box::new(BlockScoreOwnedIterator { docs, i: i.get() })
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
      max_score = max_score.max(self.scores[j]);
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

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(BlockScoreIterator { scorer: self })
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(BlockScoreIterator { scorer: self })
  }
}

struct BlockScoreIterator<'a> {
  scorer: &'a BlockScoreScorer,
}
impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for BlockScoreIterator<'_>
{
}
impl DocIdSetIterator for BlockScoreIterator<'_> {
  fn doc_id(&self) -> i32 {
    self.scorer.docs[self.scorer.i.get()]
  }

  fn next_doc(&mut self) -> Result<i32> {
    let i = self.scorer.i.get();
    assert!(self.scorer.docs[i] != NO_MORE_DOCS);
    self.scorer.i.set(i + 1);
    Ok(self.scorer.docs[i + 1])
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let i = self.scorer.index_for_target(target);
    self.scorer.i.set(i);
    Ok(self.scorer.docs[i])
  }

  fn cost(&self) -> Result<i64> {
    Ok((self.scorer.docs.len() as i64) - 2)
  }
}

struct BlockScoreOwnedIterator {
  docs: Vec<i32>,
  i: usize,
}
impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for BlockScoreOwnedIterator
{
}
impl DocIdSetIterator for BlockScoreOwnedIterator {
  fn doc_id(&self) -> i32 {
    self.docs[self.i]
  }

  fn next_doc(&mut self) -> Result<i32> {
    assert!(self.docs[self.i] != NO_MORE_DOCS);
    self.i += 1;
    Ok(self.docs[self.i])
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.i = match self.docs.binary_search(&target) {
      Ok(i) | Err(i) => i,
    };
    assert!(self.docs[self.i] >= target);
    Ok(self.docs[self.i])
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
