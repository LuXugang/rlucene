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
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub trait AbstractKnnVectorQuery {}
/// Caches the results of a KnnVector search: a list of docs and their scores
#[derive(Clone, Debug)]
pub struct DocAndScoreQuery {
  docs: Arc<Vec<i32>>,
  scores: Arc<Vec<f32>>,
  max_score: f32,
  segment_starts: Arc<Vec<usize>>,
  context_identity: Identity,
  id: Identity,
}
impl DocAndScoreQuery {
  /// Constructor
  ///
  /// # Arguments
  ///
  /// * `docs` - the global docids of documents that match, in ascending order
  /// * `scores` - the scores of the matching documents
  /// * `max_score` - the maximum score
  /// * `segment_starts` - the indexes in docs and scores corresponding to the first matching
  ///   document in each segment. If a segment has no matching documents, it should be assigned
  ///   the index of the next segment that does. There should be a final entry that is always
  ///   docs.length-1.
  /// * `context_identity` - an object identifying the reader context that was used to build this
  ///   query
  pub fn new(
    docs: Arc<Vec<i32>>,
    scores: Arc<Vec<f32>>,
    max_score: f32,
    segment_starts: Arc<Vec<usize>>,
    context_identity: Identity,
  ) -> Self {
    Self {
      docs,
      scores,
      max_score,
      segment_starts,
      context_identity,
      id: Identity::new(),
    }
  }
}

impl HasIdentity for DocAndScoreQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for DocAndScoreQuery {
  fn as_string(&self, _field: &str) -> Result<String> {
    Ok(format!(
      "DocAndScoreQuery[{},...][{},...],{}",
      self.docs[0], self.scores[0], self.max_score
    ))
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
    Ok(Box::new(DocAndScoreQueryWeight::new(self, _boost)))
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(self.into())
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}
impl Eq for DocAndScoreQuery {}
impl PartialEq for DocAndScoreQuery {
  fn eq(&self, other: &Self) -> bool {
    self.context_identity == other.context_identity
      && self.docs == other.docs
      && self
        .scores
        .iter()
        .zip(other.scores.iter())
        .all(|(a, b)| a.to_bits() == b.to_bits())
  }
}
impl Hash for DocAndScoreQuery {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.context_identity.hash(state);
    self.docs.hash(state);
    for f in self.scores.iter() {
      state.write_u32(f.to_bits());
    }
  }
}

pub struct DocAndScoreQueryWeight {
  parent_query: Arc<Query>,
  query: DocAndScoreQuery,
  boost: f32,
}
impl DocAndScoreQueryWeight {
  pub fn new(query: DocAndScoreQuery, boost: f32) -> Self {
    let parent_query = Arc::new(query.clone().into());
    Self {
      parent_query,
      query,
      boost,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for DocAndScoreQueryWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(true)
  }
}

impl<IRC> Weight<IRC> for DocAndScoreQueryWeight
where
  IRC: IndexReaderContext,
{
  type Matches = MatchWithNoTerms;

  fn matches(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::Matches>> {
    self.default_matches(context, doc, searcher)
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    let target = doc + context.doc_base as i32;

    match self.query.docs.binary_search(&target) {
      Ok(found) => Ok(Explanation::match_(
        self.query.scores[found] * self.boost,
        format!("within top {} docs", self.query.docs.len()),
        vec![],
      )),
      Err(_) => Ok(Explanation::no_match_no_details(format!(
        "not in top {} docs",
        self.query.docs.len()
      ))),
    }
  }

  fn get_query(&self) -> Arc<Query> {
    self.parent_query.clone()
  }

  type ScorerSupplier = QueryWeightSs<IRC>;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    if self.query.segment_starts[context.ord] == self.query.segment_starts[context.ord + 1] {
      return Ok(None);
    }
    let disi = DocIdSetIteratorImpl::new(
      self.query.segment_starts[context.ord],
      self.query.segment_starts[context.ord + 1],
      self.query.docs.clone(),
      context.doc_base,
    );
    let scorer = ScorerImpl::new(
      disi,
      self.query.max_score,
      self.boost,
      self.query.scores.clone(),
    );
    Ok(Some(Box::new(DefaultScorerSupplier::new(scorer))))
  }

  fn count(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    Ok((self.query.segment_starts[context.ord + 1] - self.query.segment_starts[context.ord]) as i32)
  }
}

pub struct DocIdSetIteratorImpl {
  lower: usize,
  upper: usize,
  upto: Option<usize>,
  docs: Arc<Vec<i32>>,
  doc_base: usize,
}
impl DocIdSetIteratorImpl {
  pub fn new(lower: usize, upper: usize, docs: Arc<Vec<i32>>, doc_base: usize) -> Self {
    Self {
      lower,
      upper,
      upto: None,
      docs,
      doc_base,
    }
  }
}
impl DocIdSetIterator for DocIdSetIteratorImpl {
  fn doc_id(&self) -> i32 {
    doc_id_no_shadow(self.upto, self.upper, self.docs.as_ref(), self.doc_base)
  }

  fn next_doc(&mut self) -> Result<i32> {
    match self.upto {
      Some(ref mut v) => {
        *v += 1;
      },
      None => self.upto = Some(self.lower),
    }
    Ok(doc_id_no_shadow(
      self.upto,
      self.upper,
      self.docs.as_ref(),
      self.doc_base,
    ))
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    Ok((self.upper - self.lower) as i64)
  }
}
fn doc_id_no_shadow(upto: Option<usize>, upper: usize, docs: &[i32], doc_base: usize) -> i32 {
  match upto {
    Some(v) => {
      if v >= upper {
        return NO_MORE_DOCS;
      }
      docs[v] - doc_base as i32
    },
    None => -1,
  }
}

pub struct ScorerImpl {
  disi: DocIdSetIteratorImpl,
  max_score: f32,
  boost: f32,
  scorers: Arc<Vec<f32>>,
}

impl ScorerImpl {
  fn new(disi: DocIdSetIteratorImpl, max_score: f32, boost: f32, scorers: Arc<Vec<f32>>) -> Self {
    Self {
      disi,
      max_score,
      boost,
      scorers,
    }
  }
}

impl Scorable for ScorerImpl {
  fn score(&mut self) -> Result<f32> {
    let upto = self
      .disi
      .upto
      .ok_or_else(|| LuceneError::illegal_state("upto is None"))?;
    Ok(self.scorers[upto] * self.boost)
  }
}

impl FixedScore for ScorerImpl {}

impl Scorer for ScorerImpl {
  fn doc_id(&mut self) -> Result<i32> {
    Ok(doc_id_no_shadow(
      self.disi.upto,
      self.disi.upper,
      self.disi.docs.as_ref(),
      self.disi.doc_base,
    ))
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.disi)
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.disi)
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let ScorerImpl { disi, .. } = *self;
    Box::new(disi)
  }

  fn get_max_score(&mut self, _upto: i32) -> Result<f32> {
    Ok(self.max_score * self.boost)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.disi)
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.disi)
  }
}
