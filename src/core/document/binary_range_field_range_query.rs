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
use crate::core::document::binary_range_doc_values::BinaryRangeDocValues;
use crate::core::document::double_range_slow_range_query::DoubleRangeSlowRangeQuery;
use crate::core::document::float_range_slow_range_query::FloatRangeSlowRangeQuery;
use crate::core::document::int_range_slow_range_query::IntRangeSlowRangeQuery;
use crate::core::document::long_range_slow_range_query::LongRangeSlowRangeQuery;
use crate::core::document::range_field_query::QueryType;
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::core::index::doc_values::DocValues;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::array_util::{ArrayUtil, ByteArrayComparatorEnum};
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::impl_from_for_enum;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Clone)]
pub struct BinaryRangeFieldRangeQuery {
  id: Identity,
  query_packed_value: Vec<u8>,
  num_dims: usize,
  num_bytes_per_dimension: usize,
  query_type: QueryType,
  comparator: ByteArrayComparatorEnum,
  sub: BinaryRangeFieldRangeQueryEnum,
}

impl BinaryRangeFieldRangeQuery {
  pub fn new<T>(
    query_packed_value: Vec<u8>,
    num_bytes_per_dimension: usize,
    num_dims: usize,
    query_type: QueryType,
    sub: T,
  ) -> Result<Self>
  where
    T: Into<BinaryRangeFieldRangeQueryEnum>,
  {
    if query_type != QueryType::Intersects {
      return Err(LuceneError::unsupported_operation(
        "INTERSECTS is the only query type supported for this field type right now",
      ));
    }
    let comparator = ArrayUtil::get_unsigned_comparator(num_bytes_per_dimension);
    let sub = sub.into();
    Ok(Self {
      id: Identity::new(),
      query_packed_value,
      num_dims,
      num_bytes_per_dimension,
      query_type,
      comparator,
      sub,
    })
  }
}

impl Debug for BinaryRangeFieldRangeQuery {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("BinaryRangeFieldRangeQuery")
      .field("field", &self.sub.field())
      .finish()
  }
}

impl PartialEq for BinaryRangeFieldRangeQuery {
  fn eq(&self, other: &Self) -> bool {
    self.sub == other.sub
  }
}

impl Eq for BinaryRangeFieldRangeQuery {}

impl Hash for BinaryRangeFieldRangeQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.sub.hash(state);
  }
}

impl HasIdentity for BinaryRangeFieldRangeQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for BinaryRangeFieldRangeQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    let mut sb = String::new();
    if self.sub.field() != field {
      sb.push_str(self.sub.field());
      sb.push(':');
    }
    sb.push_str(&self.sub.range_string());
    Ok(sb)
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
    Ok(Box::new(BinaryRangeFieldRangeWeight::new(
      self,
      *score_mode,
      boost,
    )))
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(self.into())
  }

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    let query = self.into();
    if visitor.accept_field(self.sub.field()) {
      visitor.visit_leaf(query)?;
    }
    Ok(())
  }
}

pub struct BinaryRangeFieldRangeWeight {
  query: BinaryRangeFieldRangeQuery,
  base: ConstantScoreWeight,
  parent_query: Arc<Query>,
  score_mode: ScoreMode,
  boost: f32,
}

impl BinaryRangeFieldRangeWeight {
  fn new(query: BinaryRangeFieldRangeQuery, score_mode: ScoreMode, boost: f32) -> Self {
    let query_clone = query.clone();
    let parent_query = Arc::new(query.into());
    Self {
      query: query_clone,
      base: ConstantScoreWeight::new(boost),
      parent_query,
      score_mode,
      boost,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for BinaryRangeFieldRangeWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    let field = vec![self.query.sub.field().to_string()];
    DocValues::is_cacheable(ctx, field.as_ref())
  }
}

impl<IRC> Weight<IRC> for BinaryRangeFieldRangeWeight
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
    let field_info = context
      .reader()
      .get_field_infos()?
      .field_info_by_name(self.query.sub.field())?;
    if field_info.is_none() {
      return Ok(None);
    }
    let binary_doc_values = DocValues::get_binary(context.reader(), self.query.sub.field())?;
    let brdv = BinaryRangeDocValues::new(
      binary_doc_values,
      self.query.num_dims,
      self.query.num_bytes_per_dimension,
    );
    let tpi = BinaryRangeFieldRangeTPI::new(
      brdv,
      self.query.query_packed_value.clone(),
      self.query.num_dims,
      self.query.num_bytes_per_dimension,
      self.query.query_type,
      self.query.comparator.clone(),
    );
    let scorer = ConstantScoreScorer::from_tpi(self.boost, self.score_mode, tpi);
    Ok(Some(Box::new(DefaultScorerSupplier::new(scorer))))
  }
}

/// TwoPhaseIterator for BinaryRangeFieldRangeQuery.
///
/// Wraps a BinaryRangeDocValues (which itself wraps BinaryDocValues) and
/// uses the QueryType's matches method to check if each document's packed
/// value matches the query's packed value.
pub struct BinaryRangeFieldRangeTPI<T>
where
  T: BinaryDocValues,
{
  values: BinaryRangeDocValues<T>,
  query_packed_value: Vec<u8>,
  num_dims: usize,
  num_bytes_per_dimension: usize,
  query_type: QueryType,
  comparator: ByteArrayComparatorEnum,
}

impl<T> BinaryRangeFieldRangeTPI<T>
where
  T: BinaryDocValues,
{
  fn new(
    values: BinaryRangeDocValues<T>,
    query_packed_value: Vec<u8>,
    num_dims: usize,
    num_bytes_per_dimension: usize,
    query_type: QueryType,
    comparator: ByteArrayComparatorEnum,
  ) -> Self {
    Self {
      values,
      query_packed_value,
      num_dims,
      num_bytes_per_dimension,
      query_type,
      comparator,
    }
  }
}

impl<T> TwoPhaseIterator for BinaryRangeFieldRangeTPI<T>
where
  T: BinaryDocValues,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.values)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.values)
  }

  fn matches(&mut self) -> Result<bool> {
    let packed_value = self.values.get_packed_value();
    self.query_type.matches(
      &self.query_packed_value,
      packed_value,
      self.num_dims,
      self.num_bytes_per_dimension,
      &self.comparator,
    )
  }

  fn match_cost(&self) -> f32 {
    self.query_packed_value.len() as f32
  }
}
#[derive(Clone)]
pub enum BinaryRangeFieldRangeQueryEnum {
  Double(DoubleRangeSlowRangeQuery),
  Int(IntRangeSlowRangeQuery),
  Float(FloatRangeSlowRangeQuery),
  Long(LongRangeSlowRangeQuery),
}
impl BinaryRangeFieldRangeQueryEnum {
  fn field(&self) -> &str {
    match self {
      BinaryRangeFieldRangeQueryEnum::Double(v) => v.field(),
      BinaryRangeFieldRangeQueryEnum::Int(v) => v.field(),
      BinaryRangeFieldRangeQueryEnum::Float(v) => v.field(),
      BinaryRangeFieldRangeQueryEnum::Long(v) => v.field(),
    }
  }

  fn range_string(&self) -> String {
    match self {
      BinaryRangeFieldRangeQueryEnum::Double(v) => {
        format!("[{:?} TO {:?}]", v.min(), v.max())
      },
      BinaryRangeFieldRangeQueryEnum::Int(v) => {
        format!("[{:?} TO {:?}]", v.min(), v.max())
      },
      BinaryRangeFieldRangeQueryEnum::Float(v) => {
        format!("[{:?} TO {:?}]", v.min(), v.max())
      },
      BinaryRangeFieldRangeQueryEnum::Long(v) => {
        format!("[{:?} TO {:?}]", v.min(), v.max())
      },
    }
  }
}
impl Hash for BinaryRangeFieldRangeQueryEnum {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      BinaryRangeFieldRangeQueryEnum::Double(v) => v.hash(state),
      BinaryRangeFieldRangeQueryEnum::Int(v) => v.hash(state),
      BinaryRangeFieldRangeQueryEnum::Float(v) => v.hash(state),
      BinaryRangeFieldRangeQueryEnum::Long(v) => v.hash(state),
    }
  }
}
impl PartialEq for BinaryRangeFieldRangeQueryEnum {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (BinaryRangeFieldRangeQueryEnum::Double(v1), BinaryRangeFieldRangeQueryEnum::Double(v2)) => {
        v1 == v2
      },
      (BinaryRangeFieldRangeQueryEnum::Int(v1), BinaryRangeFieldRangeQueryEnum::Int(v2)) => {
        v1 == v2
      },
      (BinaryRangeFieldRangeQueryEnum::Float(v1), BinaryRangeFieldRangeQueryEnum::Float(v2)) => {
        v1 == v2
      },
      (BinaryRangeFieldRangeQueryEnum::Long(v1), BinaryRangeFieldRangeQueryEnum::Long(v2)) => {
        v1 == v2
      },
      _ => false,
    }
  }
}
impl Eq for BinaryRangeFieldRangeQueryEnum {}
impl_from_for_enum!(
    BinaryRangeFieldRangeQueryEnum,
    DoubleRangeSlowRangeQuery=> Double,
    IntRangeSlowRangeQuery=> Int,
    FloatRangeSlowRangeQuery=> Float,
  LongRangeSlowRangeQuery => Long,
);

impl crate::core::util::accountable::Accountable for BinaryRangeFieldRangeQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}
