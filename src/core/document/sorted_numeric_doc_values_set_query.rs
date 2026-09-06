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
use crate::core::document::doc_values_long_hash_set::DocValuesLongHashSet;
use crate::core::index::doc_values::DocValues;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::two_phase_iterator::{TwoPhaseIterator, TwoPhaseIteratorEnum2};
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::accountable::Accountable;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::ram_usage_estimator::size_of_string;
use std::hash::{Hash, Hasher};
use std::mem::size_of_val;
use std::sync::Arc;

/// Similar to SortedNumericDocValuesRangeQuery but for a set
#[derive(Debug, Clone)]
pub struct SortedNumericDocValuesSetQuery {
  id: Identity,
  field: String,
  numbers: Arc<DocValuesLongHashSet>,
}
impl SortedNumericDocValuesSetQuery {
  pub fn new(field: String, mut numbers: Vec<i64>) -> Result<Self> {
    numbers.sort_unstable();
    Ok(SortedNumericDocValuesSetQuery {
      id: Identity::new(),
      field,
      numbers: Arc::new(DocValuesLongHashSet::new(numbers.as_slice())?),
    })
  }
}
impl PartialEq for SortedNumericDocValuesSetQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field && self.numbers == other.numbers
  }
}
impl Eq for SortedNumericDocValuesSetQuery {}

impl Hash for SortedNumericDocValuesSetQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.field.hash(state);
    self.numbers.hash(state);
  }
}

impl HasIdentity for SortedNumericDocValuesSetQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}
impl QueryBase for SortedNumericDocValuesSetQuery {
  fn to_string(&self, _field: &str) -> Result<String> {
    Ok(format!("{}: {}", self.field, self.numbers))
  }

  fn create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(Box::new(SortedNumericDocValuesSetQueryWeight::new(
      self,
      *score_mode,
      boost,
    )))
  }

  fn rewrite<IRC>(&self, _searcher: &IndexSearcher<IRC>) -> Result<Option<Query>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    if self.numbers.size() == 0 {
      return Ok(Some(MatchNoDocsQuery::new().into()));
    }
    Ok(None)
  }

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    let query = self.into();
    if visitor.accept_field(&self.field) {
      visitor.visit_leaf(query)?;
    }
    Ok(())
  }
}
impl Accountable for SortedNumericDocValuesSetQuery {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(
      self
        .id
        .ram_bytes_used()?
        .saturating_add(size_of_string(&self.field))
        .saturating_add(size_of_val(self.numbers.as_ref()) as i64)
        .saturating_add(self.numbers.ram_bytes_used()?),
    )
  }
}

pub struct SortedNumericDocValuesSetQueryWeight {
  query: SortedNumericDocValuesSetQuery,
  base: ConstantScoreWeight,
  parent_query: Arc<Query>,
  score_mode: ScoreMode,
}
impl SortedNumericDocValuesSetQueryWeight {
  pub(crate) fn new(
    query: SortedNumericDocValuesSetQuery,
    score_mode: ScoreMode,
    boost: f32,
  ) -> Self {
    let query_clone = query.clone();
    let parent_query = Arc::new(query.into());
    Self {
      query: query_clone,
      base: ConstantScoreWeight::new(boost),
      parent_query,
      score_mode,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for SortedNumericDocValuesSetQueryWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    let field = vec![self.query.field.clone()];
    DocValues::is_cacheable(ctx, field.as_ref())
  }
}

impl<IRC> Weight<IRC> for SortedNumericDocValuesSetQueryWeight
where
  IRC: IndexReaderContext,
{
  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    let scorer = self.scorer(context, searcher)?;
    self
      .base
      .explain(scorer, doc, self.parent_query.to_string("")?)
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
    if context
      .reader()
      .get_field_infos()?
      .field_info_by_name(&self.query.field)?
      .is_none()
    {
      return Ok(None);
    }
    let mut values = DocValues::get_sorted_numeric(context.reader(), &self.query.field)?;
    let iterator = if values.is_single_valued() {
      let singleton = DocValues::unwrap_singleton_numeric(&mut values)?;
      TwoPhaseIteratorEnum2::A(TwoPhaseIterator1::new(singleton, self.query.clone()))
    } else {
      TwoPhaseIteratorEnum2::B(TwoPhaseIterator2::new(values, self.query.clone()))
    };
    let scorer = ConstantScoreScorer::from_tpi(self.base.score(), self.score_mode, iterator);
    Ok(Some(Box::new(DefaultScorerSupplier::new(scorer))))
  }
}

pub struct TwoPhaseIterator1<N> {
  singleton: N,
  query: SortedNumericDocValuesSetQuery,
}
impl<N> TwoPhaseIterator1<N> {
  pub fn new(singleton: N, query: SortedNumericDocValuesSetQuery) -> Self {
    TwoPhaseIterator1 { singleton, query }
  }
}
impl<N> TwoPhaseIterator for TwoPhaseIterator1<N>
where
  N: NumericDocValues,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.singleton)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.singleton)
  }

  fn matches(&mut self) -> Result<bool> {
    let value = self.singleton.long_value()?;
    let numbers = &self.query.numbers;
    Ok(value >= numbers.min_value && value <= numbers.max_value && numbers.contains(value))
  }

  fn match_cost(&self) -> f32 {
    5f32
  }
}
pub struct TwoPhaseIterator2<S> {
  value: S,
  query: SortedNumericDocValuesSetQuery,
}

impl<S> TwoPhaseIterator2<S> {
  pub fn new(value: S, query: SortedNumericDocValuesSetQuery) -> Self {
    TwoPhaseIterator2 { value, query }
  }
}

impl<S> TwoPhaseIterator for TwoPhaseIterator2<S>
where
  S: SortedNumericDocValues,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.value)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.value)
  }

  fn matches(&mut self) -> Result<bool> {
    let numbers = &self.query.numbers;
    let count = self.value.doc_value_count()?;

    for _ in 0..count {
      let value = self.value.next_value()?;

      if value < numbers.min_value {
        continue;
      } else if value > numbers.max_value {
        return Ok(false); // sorted, terminate
      } else if numbers.contains(value) {
        return Ok(true);
      }
    }

    Ok(false)
  }

  fn match_cost(&self) -> f32 {
    5f32
  }
}
