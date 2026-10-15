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
use crate::core::search::approximation_disi::ApproximationBitSetAccess;
use crate::core::search::approximation_disi::ApproximationDISI;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_scorer::ConstantScoreTwoPhaseScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::Similarity;
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
  pub fn new<FName, V>(field: FName, numbers: V) -> Result<Self>
  where
    FName: Into<String>,
    V: Into<Vec<i64>>,
  {
    let field = field.into();
    let mut numbers = numbers.into();
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
  type Weight<IRC, SIM>
    = SortedNumericDocValuesSetQueryWeight
  where
    SIM: Similarity,
    IRC: IndexReaderContext + 'static;

  fn to_string(&self, _field: &str) -> Result<String> {
    Ok(format!("{}: {}", self.field, self.numbers))
  }

  fn create_weight<IRC, SIM>(
    self,
    _searcher: &IndexSearcher<IRC, SIM>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<Self::Weight<IRC, SIM>>
  where
    SIM: Similarity,
    IRC: IndexReaderContext + 'static,
    Self: Sized,
    IndexSearcher<IRC, SIM>: Sync,
  {
    Ok(SortedNumericDocValuesSetQueryWeight::new(
      self,
      *score_mode,
      boost,
    ))
  }

  fn rewrite<IRC, SIM>(&self, _searcher: &IndexSearcher<IRC, SIM>) -> Result<Option<Query>>
  where
    SIM: Similarity,
    IRC: IndexReaderContext + 'static,
    Self: Sized,
    IndexSearcher<IRC, SIM>: Sync,
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
  IRC: IndexReaderContext + 'static,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    let field = [self.query.field.as_str()];
    DocValues::is_cacheable(ctx, field)
  }
}

impl<IRC, SIM> Weight<IRC, SIM> for SortedNumericDocValuesSetQueryWeight
where
  SIM: Similarity,
  IRC: IndexReaderContext + 'static,
{
  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Explanation>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    let scorer = self.scorer(context, searcher)?;
    self
      .base
      .explain(scorer, doc, self.parent_query.to_string("")?)
  }

  fn get_query(&self) -> Arc<Query> {
    self.parent_query.clone()
  }

  type ScorerSupplier = DefaultScorerSupplier<ConstantScoreTwoPhaseScorer<TwoPhaseIteratorEnum2<
      TwoPhaseIterator1<<crate::core::index::doc_values::SortedNumeric<IRCLeafReader<IRC>> as SortedNumericDocValues>::NumericDocValues>,
      TwoPhaseIterator2<crate::core::index::doc_values::SortedNumeric<IRCLeafReader<IRC>>>,
    >>>;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<Self::ScorerSupplier>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
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
      TwoPhaseIteratorEnum2::A(TwoPhaseIterator1::new(
        singleton,
        self.query.numbers.clone(),
      ))
    } else {
      TwoPhaseIteratorEnum2::B(TwoPhaseIterator2::new(values, self.query.numbers.clone()))
    };
    let scorer = ConstantScoreScorer::from_tpi(self.base.score(), self.score_mode, iterator);
    Ok(Some(DefaultScorerSupplier::new(scorer)))
  }
}

pub struct TwoPhaseIterator1<N> {
  singleton: N,
  numbers: Arc<DocValuesLongHashSet>,
}
impl<N> TwoPhaseIterator1<N> {
  pub(crate) fn new(singleton: N, numbers: Arc<DocValuesLongHashSet>) -> Self {
    TwoPhaseIterator1 { singleton, numbers }
  }
}
impl<N> ApproximationDISI for TwoPhaseIterator1<N>
where
  N: NumericDocValues,
{
  fn approx_doc_id(&self) -> i32 {
    self.singleton.doc_id()
  }

  fn approx_next_doc(&mut self) -> Result<i32> {
    self.singleton.next_doc()
  }

  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.singleton.advance(target)
  }

  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.singleton.slow_advance(target)
  }

  fn approx_cost(&self) -> Result<i64> {
    self.singleton.cost()
  }
}
impl<N> ApproximationBitSetAccess for TwoPhaseIterator1<N>
where
  N: NumericDocValues,
{
  fn approx_is_bit_iter(&self) -> bool {
    self.singleton.is_bit_iter()
  }

  fn approx_get(&self, index: usize) -> Result<bool> {
    self.singleton.get(index)
  }

  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.singleton.set_doc_id(doc)
  }

  fn approx_bit_set_length(&self) -> Result<usize> {
    self.singleton.bit_set_length()
  }
}

impl<N> TwoPhaseIterator for TwoPhaseIterator1<N>
where
  N: NumericDocValues,
{
  fn matches(&mut self) -> Result<bool> {
    let value = self.singleton.long_value()?;
    let numbers = &self.numbers;
    Ok(value >= numbers.min_value && value <= numbers.max_value && numbers.contains(value))
  }

  fn match_cost(&self) -> f32 {
    5f32
  }
}

pub struct TwoPhaseIterator2<S> {
  value: S,
  numbers: Arc<DocValuesLongHashSet>,
}

impl<S> TwoPhaseIterator2<S> {
  pub(crate) fn new(value: S, numbers: Arc<DocValuesLongHashSet>) -> Self {
    TwoPhaseIterator2 { value, numbers }
  }
}

impl<S> ApproximationDISI for TwoPhaseIterator2<S>
where
  S: SortedNumericDocValues,
{
  fn approx_doc_id(&self) -> i32 {
    self.value.doc_id()
  }

  fn approx_next_doc(&mut self) -> Result<i32> {
    self.value.next_doc()
  }

  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.value.advance(target)
  }

  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.value.slow_advance(target)
  }

  fn approx_cost(&self) -> Result<i64> {
    self.value.cost()
  }
}

impl<S> ApproximationBitSetAccess for TwoPhaseIterator2<S>
where
  S: SortedNumericDocValues,
{
  fn approx_is_bit_iter(&self) -> bool {
    self.value.is_bit_iter()
  }

  fn approx_get(&self, index: usize) -> Result<bool> {
    self.value.get(index)
  }

  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.value.set_doc_id(doc)
  }

  fn approx_bit_set_length(&self) -> Result<usize> {
    self.value.bit_set_length()
  }
}

impl<S> TwoPhaseIterator for TwoPhaseIterator2<S>
where
  S: SortedNumericDocValues,
{
  fn matches(&mut self) -> Result<bool> {
    let numbers = &self.numbers;
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
