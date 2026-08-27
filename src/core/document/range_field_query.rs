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
use crate::core::document::double_range::DoubleRangeFieldQuery;
use crate::core::document::float_range::FloatRangeFieldQuery;
use crate::core::document::inet_address_range::InetAddressRangeFieldQuery;
use crate::core::document::int_range::IntRangeFieldQuery;
use crate::core::document::long_range::LongRangeFieldQuery;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::point_values::{IntersectVisitor, PointValues, Relation};
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::{AllDISI, DocIdSetIterator};
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{
  Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::Weight;
use crate::core::util::TryIntoInt;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::array_util::{ByteArrayComparator, ByteArrayComparatorEnum};
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::doc_id_set_builder::DocIdSetBuilder;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::impl_from_for_enum;
use crate::sandbox::document::lat_lon_bounding_box::LatLonBoundingBoxFieldQuery;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Query for searching range fields by a defined relation.
#[derive(Clone)]
pub struct RangeFieldQuery {
  id: Identity,
  field: String,
  query_type: QueryType,
  num_dims: usize,
  ranges: Vec<u8>,
  bytes_per_dim: usize,
  comparator: ByteArrayComparatorEnum,
  sub: RangeFieldQueryBaseEnum,
}

impl RangeFieldQuery {
  pub fn new<T>(
    field: String,
    ranges: Vec<u8>,
    num_dims: usize,
    query_type: QueryType,
    sub: T,
  ) -> Result<Self>
  where
    T: Into<RangeFieldQueryBaseEnum>,
  {
    let sub = sub.into();
    Self::check_args(&ranges, num_dims)?;
    let bytes_per_dim = ranges.len() / (2 * num_dims);
    let comparator = ArrayUtil::get_unsigned_comparator(bytes_per_dim);
    Ok(Self {
      id: Identity::new(),
      field,
      query_type,
      num_dims,
      ranges,
      bytes_per_dim,
      comparator,
      sub,
    })
  }

  fn check_args(ranges: &[u8], num_dims: usize) -> Result<()> {
    if num_dims > 4 {
      return Err(LuceneError::illegal_argument(
        "dimension size cannot be greater than 4",
      ));
    }
    if ranges.is_empty() {
      return Err(LuceneError::illegal_argument(
        "encoded ranges cannot be null or empty",
      ));
    }
    Ok(())
  }

  fn check_field_info(&self, field_info: &FieldInfo) -> Result<()> {
    if field_info.get_point_dimension_count() / 2 != self.num_dims {
      return Err(LuceneError::illegal_argument(format!(
        "field=\"{}\" was indexed with numDims={} but this query has numDims={}",
        self.field,
        field_info.get_point_dimension_count() / 2,
        self.num_dims
      )));
    }
    Ok(())
  }

  fn equals_to(&self, other: &RangeFieldQuery) -> bool {
    self.field == other.field
      && self.num_dims == other.num_dims
      && self.ranges == other.ranges
      && self.query_type == other.query_type
  }
}

impl Debug for RangeFieldQuery {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("RangeFieldQuery")
      .field("field", &self.field)
      .field("num_dims", &self.num_dims)
      .field("query_type", &self.query_type)
      .finish()
  }
}

impl PartialEq for RangeFieldQuery {
  fn eq(&self, other: &Self) -> bool {
    self.equals_to(other)
  }
}

impl Eq for RangeFieldQuery {}

impl Hash for RangeFieldQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.field.hash(state);
    self.num_dims.hash(state);
    self.query_type.hash(state);
    self.ranges.hash(state);
  }
}

impl HasIdentity for RangeFieldQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for RangeFieldQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    let mut sb = String::new();
    if self.field != field {
      sb.push_str(&self.field);
      sb.push(':');
    }
    sb.push_str("<ranges:");
    for dim in 0..self.num_dims {
      if dim > 0 {
        sb.push(' ');
      }
      sb.push_str(&self.sub.to_string(&self.ranges, dim)?);
    }
    sb.push('>');
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
    Ok(Box::new(RangeFieldWeight::new(self, *score_mode, boost)))
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
    if visitor.accept_field(&self.field) {
      visitor.visit_leaf(query)?;
    }
    Ok(())
  }
}

pub struct RangeFieldWeight {
  query: RangeFieldQuery,
  base: ConstantScoreWeight,
  parent_query: Arc<Query>,
  score_mode: ScoreMode,
}

impl RangeFieldWeight {
  fn new(query: RangeFieldQuery, score_mode: ScoreMode, boost: f32) -> Self {
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

impl<IRC> SegmentCacheable<IRC> for RangeFieldWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(true)
  }
}

impl<IRC> Weight<IRC> for RangeFieldWeight
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
    let reader = context.reader();
    let Some(values) = reader.get_point_values(&self.query.field)? else {
      return Ok(None);
    };
    let Some(field_info) = reader
      .get_field_infos()?
      .field_info_by_name(&self.query.field)?
    else {
      return Ok(None);
    };
    self.query.check_field_info(field_info.as_ref())?;

    let all_docs_match = values.get_doc_count()? == reader.max_doc()?
      && self.query.query_type.compare(
        &self.query.ranges,
        values
          .get_min_packed_value()?
          .ok_or_else(|| LuceneError::illegal_state("min_packed_value is None"))?
          .as_ref(),
        values
          .get_max_packed_value()?
          .ok_or_else(|| LuceneError::illegal_state("max_packed_value is None"))?
          .as_ref(),
        self.query.num_dims,
        self.query.bytes_per_dim,
        &self.query.comparator,
      )? == Relation::CellInsideQuery;

    if all_docs_match {
      Ok(Some(Box::new(RangeFieldAllScorerSupplier::new(
        self.base.score(),
        self.score_mode,
        reader.max_doc()?,
      ))))
    } else {
      let result =
        DocIdSetBuilder::from_point_values(reader.max_doc()?, &values, &self.query.field)?;
      let visitor = RangeFieldIntersectVisitor::new(result, self.query.clone());
      Ok(Some(Box::new(RangeFieldScorerSupplier::new(
        self.base.score(),
        self.score_mode,
        values,
        visitor,
      ))))
    }
  }
}

pub struct RangeFieldScorerSupplier<PV> {
  score: f32,
  score_mode: ScoreMode,
  values: PV,
  visitor: RangeFieldIntersectVisitor,
  cost: i64,
}

impl<PV> RangeFieldScorerSupplier<PV> {
  fn new(
    score: f32,
    score_mode: ScoreMode,
    values: PV,
    visitor: RangeFieldIntersectVisitor,
  ) -> Self {
    Self {
      score,
      score_mode,
      values,
      visitor,
      cost: -1,
    }
  }
}

impl<IRC> ScorerSupplier<IRC>
  for RangeFieldScorerSupplier<<IRCLeafReader<IRC> as LeafReader>::PointValues>
where
  IRC: IndexReaderContext,
  IRCLeafReader<IRC>: LeafReader,
{
  type Scorer = QueryWeightSsScorer;
  type BulkScorer = QueryWeightSsBulkScorer;

  fn get(
    &mut self,
    _lead_cost: i64,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::Scorer> {
    self.values.intersect(&mut self.visitor)?;
    let iterator = self.visitor.result.build()?.iterator()?;
    Ok(Box::new(ConstantScoreScorer::from_disi(
      self.score,
      self.score_mode,
      iterator,
    )))
  }

  fn bulk_scorer(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::BulkScorer>> {
    Ok(Some(Box::new(self.default_bulk_scorer(context, searcher)?)))
  }

  fn cost(
    &mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<i64> {
    if self.cost == -1 {
      self.cost = self.values.estimate_doc_count(&self.visitor)?;
      debug_assert!(self.cost >= 0);
    }
    Ok(self.cost)
  }
}

pub struct RangeFieldAllScorerSupplier {
  score: f32,
  score_mode: ScoreMode,
  max_doc: i32,
}

impl RangeFieldAllScorerSupplier {
  fn new(score: f32, score_mode: ScoreMode, max_doc: i32) -> Self {
    Self {
      score,
      score_mode,
      max_doc,
    }
  }
}

impl<IRC> ScorerSupplier<IRC> for RangeFieldAllScorerSupplier
where
  IRC: IndexReaderContext,
  IRCLeafReader<IRC>: LeafReader,
{
  type Scorer = QueryWeightSsScorer;
  type BulkScorer = QueryWeightSsBulkScorer;

  fn get(
    &mut self,
    _lead_cost: i64,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::Scorer> {
    debug_assert!(context.reader().max_doc()? == self.max_doc);
    Ok(Box::new(ConstantScoreScorer::from_disi(
      self.score,
      self.score_mode,
      AllDISI::new(self.max_doc),
    )))
  }

  fn bulk_scorer(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::BulkScorer>> {
    Ok(Some(Box::new(self.default_bulk_scorer(context, searcher)?)))
  }

  fn cost(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<i64> {
    debug_assert!(context.reader().max_doc()? == self.max_doc);
    Ok(self.max_doc as i64)
  }
}

pub struct RangeFieldIntersectVisitor {
  result: DocIdSetBuilder,
  query: RangeFieldQuery,
}

impl RangeFieldIntersectVisitor {
  fn new(result: DocIdSetBuilder, query: RangeFieldQuery) -> Self {
    Self { result, query }
  }
}

impl IntersectVisitor for RangeFieldIntersectVisitor {
  fn grow(&mut self, count: usize) -> Result<()> {
    self.result.grow(count.try_convert()?)?;
    Ok(())
  }

  fn visit(&mut self, doc_id: i32) -> Result<()> {
    self.result.add_doc(doc_id)?;
    Ok(())
  }

  fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
    self.result.add_disi(iterator)?;
    Ok(())
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, leaf: &[u8]) -> Result<()> {
    if self.query.query_type.matches(
      &self.query.ranges,
      leaf,
      self.query.num_dims,
      self.query.bytes_per_dim,
      &self.query.comparator,
    )? {
      self.visit(doc_id)?;
    }
    Ok(())
  }

  fn visit_iterator_with_packed_value(
    &mut self,
    iterator: &mut impl DocIdSetIterator,
    leaf: &[u8],
  ) -> Result<()> {
    if self.query.query_type.matches(
      &self.query.ranges,
      leaf,
      self.query.num_dims,
      self.query.bytes_per_dim,
      &self.query.comparator,
    )? {
      self.result.add_disi(iterator)?;
    }
    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    self.query.query_type.compare(
      &self.query.ranges,
      min_packed_value,
      max_packed_value,
      self.query.num_dims,
      self.query.bytes_per_dim,
      &self.query.comparator,
    )
  }
}
pub trait RangeFieldQueryBase {
  fn to_string(&self, value: &[u8], dimension: usize) -> Result<String>;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RangeFieldQueryBaseEnum {
  Double(DoubleRangeFieldQuery),
  Float(FloatRangeFieldQuery),
  InetAddress(InetAddressRangeFieldQuery),
  Int(IntRangeFieldQuery),
  LatLonBoundingBox(LatLonBoundingBoxFieldQuery),
  Long(LongRangeFieldQuery),
}

impl_from_for_enum!(
  RangeFieldQueryBaseEnum,
  DoubleRangeFieldQuery => Double,
  FloatRangeFieldQuery => Float,
  InetAddressRangeFieldQuery => InetAddress,
  IntRangeFieldQuery => Int,
  LatLonBoundingBoxFieldQuery => LatLonBoundingBox,
  LongRangeFieldQuery => Long,
);

impl RangeFieldQueryBase for RangeFieldQueryBaseEnum {
  fn to_string(&self, value: &[u8], dimension: usize) -> Result<String> {
    match self {
      RangeFieldQueryBaseEnum::Double(i) => i.to_string(value, dimension),
      RangeFieldQueryBaseEnum::Float(i) => i.to_string(value, dimension),
      RangeFieldQueryBaseEnum::InetAddress(i) => i.to_string(value, dimension),
      RangeFieldQueryBaseEnum::Int(i) => i.to_string(value, dimension),
      RangeFieldQueryBaseEnum::LatLonBoundingBox(i) => i.to_string(value, dimension),
      RangeFieldQueryBaseEnum::Long(i) => i.to_string(value, dimension),
    }
  }
}
/// Used by [`RangeFieldQuery`] to check how each internal or leaf node relates to the query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryType {
  /// Use this for intersects queries.
  Intersects,
  /// Use this for within queries.
  Within,
  /// Use this for contains queries.
  Contains,
  /// Use this for crosses queries.
  Crosses,
}

impl QueryType {
  #[allow(clippy::too_many_arguments)]
  fn compare_dim(
    &self,
    query_packed_value: &[u8],
    min_packed_value: &[u8],
    max_packed_value: &[u8],
    num_dims: usize,
    bytes_per_dim: usize,
    dim: usize,
    comparator: &ByteArrayComparatorEnum,
  ) -> Result<Relation> {
    let min_offset = dim * bytes_per_dim;
    let max_offset = min_offset + bytes_per_dim * num_dims;

    match self {
      QueryType::Intersects => {
        if comparator.compare(query_packed_value, max_offset, min_packed_value, min_offset) < 0
          || comparator.compare(query_packed_value, min_offset, max_packed_value, max_offset) > 0
        {
          return Ok(Relation::CellOutsideQuery);
        }

        if comparator.compare(query_packed_value, max_offset, max_packed_value, min_offset) >= 0
          && comparator.compare(query_packed_value, min_offset, min_packed_value, max_offset) <= 0
        {
          return Ok(Relation::CellInsideQuery);
        }

        Ok(Relation::CellCrossesQuery)
      },
      QueryType::Within => {
        if comparator.compare(query_packed_value, max_offset, min_packed_value, max_offset) < 0
          || comparator.compare(query_packed_value, min_offset, max_packed_value, min_offset) > 0
        {
          return Ok(Relation::CellOutsideQuery);
        }

        if comparator.compare(query_packed_value, max_offset, max_packed_value, max_offset) >= 0
          && comparator.compare(query_packed_value, min_offset, min_packed_value, min_offset) <= 0
        {
          return Ok(Relation::CellInsideQuery);
        }

        Ok(Relation::CellCrossesQuery)
      },
      QueryType::Contains => {
        if comparator.compare(query_packed_value, max_offset, max_packed_value, max_offset) > 0
          || comparator.compare(query_packed_value, min_offset, min_packed_value, min_offset) < 0
        {
          return Ok(Relation::CellOutsideQuery);
        }

        if comparator.compare(query_packed_value, max_offset, min_packed_value, max_offset) <= 0
          && comparator.compare(query_packed_value, min_offset, max_packed_value, min_offset) >= 0
        {
          return Ok(Relation::CellInsideQuery);
        }

        Ok(Relation::CellCrossesQuery)
      },
      QueryType::Crosses => Err(LuceneError::unsupported_operation("")),
    }
  }

  pub fn compare(
    &self,
    query_packed_value: &[u8],
    min_packed_value: &[u8],
    max_packed_value: &[u8],
    num_dims: usize,
    bytes_per_dim: usize,
    comparator: &ByteArrayComparatorEnum,
  ) -> Result<Relation> {
    if self == &QueryType::Crosses {
      let intersect_relation = QueryType::Intersects.compare(
        query_packed_value,
        min_packed_value,
        max_packed_value,
        num_dims,
        bytes_per_dim,
        comparator,
      )?;
      if intersect_relation == Relation::CellOutsideQuery {
        return Ok(Relation::CellOutsideQuery);
      }

      let within_relation = QueryType::Within.compare(
        query_packed_value,
        min_packed_value,
        max_packed_value,
        num_dims,
        bytes_per_dim,
        comparator,
      )?;
      if within_relation == Relation::CellInsideQuery {
        return Ok(Relation::CellOutsideQuery);
      }

      if intersect_relation == Relation::CellInsideQuery
        && within_relation == Relation::CellOutsideQuery
      {
        return Ok(Relation::CellInsideQuery);
      }

      return Ok(Relation::CellCrossesQuery);
    }

    let mut inside = true;
    for dim in 0..num_dims {
      let relation = self.compare_dim(
        query_packed_value,
        min_packed_value,
        max_packed_value,
        num_dims,
        bytes_per_dim,
        dim,
        comparator,
      )?;
      if relation == Relation::CellOutsideQuery {
        return Ok(Relation::CellOutsideQuery);
      } else if relation != Relation::CellInsideQuery {
        inside = false;
      }
    }
    if inside {
      Ok(Relation::CellInsideQuery)
    } else {
      Ok(Relation::CellCrossesQuery)
    }
  }

  fn matches_dim(
    &self,
    query_packed_value: &[u8],
    packed_value: &[u8],
    num_dims: usize,
    bytes_per_dim: usize,
    dim: usize,
    comparator: &ByteArrayComparatorEnum,
  ) -> Result<bool> {
    let min_offset = dim * bytes_per_dim;
    let max_offset = min_offset + bytes_per_dim * num_dims;

    match self {
      QueryType::Intersects => Ok(
        comparator.compare(query_packed_value, max_offset, packed_value, min_offset) >= 0
          && comparator.compare(query_packed_value, min_offset, packed_value, max_offset) <= 0,
      ),
      QueryType::Within => Ok(
        comparator.compare(query_packed_value, min_offset, packed_value, min_offset) <= 0
          && comparator.compare(query_packed_value, max_offset, packed_value, max_offset) >= 0,
      ),
      QueryType::Contains => Ok(
        comparator.compare(query_packed_value, min_offset, packed_value, min_offset) >= 0
          && comparator.compare(query_packed_value, max_offset, packed_value, max_offset) <= 0,
      ),
      QueryType::Crosses => Err(LuceneError::unsupported_operation("")),
    }
  }

  /// Compares every dim for 2 encoded ranges and returns true if all dims match.
  /// Matching implementation is based on the [`QueryType`].
  pub fn matches(
    &self,
    query_packed_value: &[u8],
    packed_value: &[u8],
    num_dims: usize,
    bytes_per_dim: usize,
    comparator: &ByteArrayComparatorEnum,
  ) -> Result<bool> {
    if self == &QueryType::Crosses {
      return Ok(
        QueryType::Intersects.matches(
          query_packed_value,
          packed_value,
          num_dims,
          bytes_per_dim,
          comparator,
        )? && !QueryType::Within.matches(
          query_packed_value,
          packed_value,
          num_dims,
          bytes_per_dim,
          comparator,
        )?,
      );
    }

    for dim in 0..num_dims {
      if !self.matches_dim(
        query_packed_value,
        packed_value,
        num_dims,
        bytes_per_dim,
        dim,
        comparator,
      )? {
        return Ok(false);
      }
    }
    Ok(true)
  }
}

impl crate::core::util::accountable::Accountable for RangeFieldQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}
