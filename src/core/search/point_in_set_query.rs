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
use crate::core::document::big_integer_point::BigIntegerPointInSetQuery;
use crate::core::document::binary_point::BinaryPointInSetQuery;
use crate::core::document::double_point::DoublePointInSetQuery;
use crate::core::document::float_point::FloatPointInSetQuery;
use crate::core::document::inet_address_point::InetAddressPointInSetQuery;
use crate::core::document::int_point::IntPointInSetQuery;
use crate::core::document::long_point::LongPointInSetQuery;
use crate::core::index::BytesRef;
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::point_values::{
  IntersectVisitor, MAX_INDEX_DIMENSIONS, MAX_NUM_BYTES, PointValues, Relation,
};
use crate::core::index::prefix_coded_terms::{
  PrefixCodedTermsArc, PrefixCodedTermsBuilder, TermIteratorArc,
};
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{
  Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::Weight;
use crate::core::util::TryIntoInt;
use crate::core::util::accountable::Accountable;
use crate::core::util::array_util::{ArrayUtil, ByteArrayComparator, ByteArrayComparatorEnum};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::doc_id_set_builder::DocIdSetBuilder;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::sandbox::document::half_float_point::HalfFloatPointInSetQuery;
use std::cell::RefCell;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Abstract query class to find all documents whose indexed point values are
/// contained in the specified set.
///
/// This works on the underlying binary encoding. Point field types should add
/// factory methods that pack typed values and pass a matching
/// [`PointInSetBase`] implementation for display.
#[derive(Debug, Clone)]
pub struct PointInSetQuery {
  id: Identity,
  sorted_packed_points: PrefixCodedTermsArc,
  sorted_packed_points_hash_code: u64,
  field: String,
  num_dims: usize,
  bytes_per_dim: usize,
  ram_bytes_used: i64,
  sub: PointInSetBaseEnum,
}

impl PointInSetQuery {
  /// The `packed_points` iterator must be in sorted order.
  pub fn new<S, I>(
    field: String,
    num_dims: usize,
    bytes_per_dim: usize,
    mut packed_points: I,
    sub: S,
  ) -> Result<Self>
  where
    S: Into<PointInSetBaseEnum>,
    I: BytesRefIterator,
  {
    if !(1..=MAX_NUM_BYTES).contains(&bytes_per_dim) {
      return Err(LuceneError::illegal_argument(format!(
        "bytesPerDim must be > 0 and <= {}; got {}",
        MAX_NUM_BYTES, bytes_per_dim
      )));
    }
    if !(1..=MAX_INDEX_DIMENSIONS).contains(&num_dims) {
      return Err(LuceneError::illegal_argument(format!(
        "numDims must be > 0 and <= {}; got {}",
        MAX_INDEX_DIMENSIONS, num_dims
      )));
    }

    let mut builder = PrefixCodedTermsBuilder::new();
    let mut previous: Option<BytesRef<Vec<u8>>> = None;

    while let Some(current) = packed_points.next()? {
      let current = current.as_ref();
      let packed_length = num_dims * bytes_per_dim;
      if current.length != packed_length {
        return Err(LuceneError::illegal_argument(format!(
          "packed point length should be {} but got {}; field=\"{}\" numDims={} bytesPerDim={}",
          packed_length, current.length, field, num_dims, bytes_per_dim
        )));
      }

      if let Some(prev) = previous.as_ref() {
        match prev.cmp(current) {
          std::cmp::Ordering::Equal => continue,
          std::cmp::Ordering::Greater => {
            return Err(LuceneError::illegal_argument(format!(
              "values are out of order: saw {:?} before {:?}",
              prev, current
            )));
          },
          std::cmp::Ordering::Less => {},
        }
      }

      builder.add(field.clone(), current)?;
      previous = Some(BytesRef::deep_copy_of(current));
    }

    let sorted_packed_points: PrefixCodedTermsArc = builder.finish().into();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    sorted_packed_points.hash(&mut hasher);
    let sorted_packed_points_hash_code = hasher.finish();
    let ram_bytes_used =
      field.len() as i64 + sorted_packed_points.ram_bytes_used().unwrap_or_default();

    Ok(Self {
      id: Identity::new(),
      sorted_packed_points,
      sorted_packed_points_hash_code,
      field,
      num_dims,
      bytes_per_dim,
      ram_bytes_used,
      sub: sub.into(),
    })
  }

  pub fn get_packed_points(&self) -> Result<Vec<Vec<u8>>> {
    packed_points_as_vec(&self.sorted_packed_points)
  }

  pub fn field(&self) -> &str {
    &self.field
  }

  pub fn num_dims(&self) -> usize {
    self.num_dims
  }

  pub fn bytes_per_dim(&self) -> usize {
    self.bytes_per_dim
  }

  fn equals_to(&self, other: &PointInSetQuery) -> bool {
    self.field == other.field
      && self.num_dims == other.num_dims
      && self.bytes_per_dim == other.bytes_per_dim
      && self.sorted_packed_points_hash_code == other.sorted_packed_points_hash_code
      && self.sorted_packed_points == other.sorted_packed_points
  }

  pub fn to_string(&self, field: &str) -> Result<String> {
    let mut sb = String::new();
    if self.field != field {
      sb.push_str(&self.field);
      sb.push(':');
    }

    sb.push('{');
    let mut first = true;
    let mut iterator = self.sorted_packed_points.iterator()?;
    while let Some(point) = iterator.next()? {
      if !first {
        sb.push(' ');
      }
      first = false;
      let value = point.as_ref();
      sb.push_str(
        &self
          .sub
          .to_string(&value.bytes[value.offset..value.offset + value.length])?,
      );
    }
    sb.push('}');
    Ok(sb)
  }
}

impl Accountable for PointInSetQuery {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(self.ram_bytes_used)
  }
}

impl Eq for PointInSetQuery {}

impl PartialEq<Self> for PointInSetQuery {
  fn eq(&self, other: &Self) -> bool {
    self.equals_to(other)
  }
}

impl Hash for PointInSetQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.field.hash(state);
    self.sorted_packed_points_hash_code.hash(state);
    self.num_dims.hash(state);
    self.bytes_per_dim.hash(state);
  }
}

impl HasIdentity for PointInSetQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for PointInSetQuery {
  fn as_string(&self, field: &str) -> Result<String> {
    self.to_string(field)
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
    Ok(Box::new(PointInSetWeight::new(boost, self, *score_mode)))
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

pub struct PointInSetWeight {
  base: ConstantScoreWeight,
  parent_query: Arc<Query>,
  query: Arc<PointInSetQuery>,
  score_mode: ScoreMode,
}

impl PointInSetWeight {
  pub fn new(score: f32, query: PointInSetQuery, score_mode: ScoreMode) -> Self {
    let point_in_set_query = Arc::new(query.clone());
    let parent_query = Arc::new(query.into());
    Self {
      base: ConstantScoreWeight::new(score),
      parent_query,
      query: point_in_set_query,
      score_mode,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for PointInSetWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(true)
  }
}

impl<IRC> Weight<IRC> for PointInSetWeight
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
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    let scorer = self.scorer(context, searcher)?;
    self
      .base
      .explain(scorer, doc, self.parent_query.as_string("")?)
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
    let values = match reader.get_point_values(&self.query.field)? {
      Some(v) => v,
      None => return Ok(None),
    };

    if values.get_num_index_dimensions()? != self.query.num_dims {
      return Err(LuceneError::illegal_argument(format!(
        "field=\"{}\" was indexed with numIndexDims={} but this query has numIndexDims={}",
        self.query.field,
        values.get_num_index_dimensions()?,
        self.query.num_dims
      )));
    }
    if values.get_bytes_per_dimension()? != self.query.bytes_per_dim {
      return Err(LuceneError::illegal_argument(format!(
        "field=\"{}\" was indexed with bytesPerDim={} but this query has bytesPerDim={}",
        self.query.field,
        values.get_bytes_per_dimension()?,
        self.query.bytes_per_dim
      )));
    }

    let max_doc = reader.max_doc()?;
    if self.query.num_dims == 1 {
      Ok(Some(Box::new(MergePointScorerSupplier::new(
        self.base.score(),
        self.score_mode,
        values,
        max_doc,
        self.query.field.clone(),
        self.query.sorted_packed_points.clone(),
        self.query.bytes_per_dim,
      ))))
    } else {
      Ok(Some(Box::new(SinglePointScorerSupplier::new(
        self.base.score(),
        self.score_mode,
        values,
        max_doc,
        self.query.field.clone(),
        self.query.sorted_packed_points.clone(),
        self.query.num_dims,
        self.query.bytes_per_dim,
      ))))
    }
  }
}

pub struct MergePointScorerSupplier<PV>
where
  PV: PointValues,
{
  score: f32,
  score_mode: ScoreMode,
  values: PV,
  max_doc: i32,
  field: String,
  sorted_packed_points: PrefixCodedTermsArc,
  bytes_per_dim: usize,
  cost: i64,
}

impl<PV> MergePointScorerSupplier<PV>
where
  PV: PointValues,
{
  pub fn new(
    score: f32,
    score_mode: ScoreMode,
    values: PV,
    max_doc: i32,
    field: String,
    sorted_packed_points: PrefixCodedTermsArc,
    bytes_per_dim: usize,
  ) -> Self {
    Self {
      score,
      score_mode,
      values,
      max_doc,
      field,
      sorted_packed_points,
      bytes_per_dim,
      cost: -1,
    }
  }
}

impl<IRC> ScorerSupplier<IRC>
  for MergePointScorerSupplier<<IRCLeafReader<IRC> as LeafReader>::PointValues>
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
    let mut visitor = MergePointVisitor::new(
      self.sorted_packed_points.clone(),
      DocIdSetBuilder::from_point_values(self.max_doc, &self.values, &self.field)?,
      self.bytes_per_dim,
    )?;
    self.values.intersect(&mut visitor)?;
    let iterator = visitor.result.build()?.iterator()?;
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
      let visitor = MergePointVisitor::new(
        self.sorted_packed_points.clone(),
        DocIdSetBuilder::from_point_values(self.max_doc, &self.values, &self.field)?,
        self.bytes_per_dim,
      )?;
      self.cost = self.values.estimate_doc_count(&visitor)?;
      debug_assert!(self.cost >= 0);
    }
    Ok(self.cost)
  }
}

pub struct SinglePointScorerSupplier<PV>
where
  PV: PointValues,
{
  score: f32,
  score_mode: ScoreMode,
  values: PV,
  max_doc: i32,
  field: String,
  sorted_packed_points: PrefixCodedTermsArc,
  num_dims: usize,
  bytes_per_dim: usize,
  cost: i64,
}

impl<PV> SinglePointScorerSupplier<PV>
where
  PV: PointValues,
{
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    score: f32,
    score_mode: ScoreMode,
    values: PV,
    max_doc: i32,
    field: String,
    sorted_packed_points: PrefixCodedTermsArc,
    num_dims: usize,
    bytes_per_dim: usize,
  ) -> Self {
    Self {
      score,
      score_mode,
      values,
      max_doc,
      field,
      sorted_packed_points,
      num_dims,
      bytes_per_dim,
      cost: -1,
    }
  }
}

impl<IRC> ScorerSupplier<IRC>
  for SinglePointScorerSupplier<<IRCLeafReader<IRC> as LeafReader>::PointValues>
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
    let result = DocIdSetBuilder::from_point_values(self.max_doc, &self.values, &self.field)?;
    let mut visitor = SinglePointVisitor::new(result, self.num_dims, self.bytes_per_dim);
    let mut iterator = self.sorted_packed_points.iterator()?;
    while let Some(point) = iterator.next()? {
      let point = point.as_ref();
      visitor.set_point(&point.bytes[point.offset..point.offset + point.length]);
      self.values.intersect(&mut visitor)?;
    }
    let iterator = visitor.result.build()?.iterator()?;
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
      let result = DocIdSetBuilder::from_point_values(self.max_doc, &self.values, &self.field)?;
      let mut visitor = SinglePointVisitor::new(result, self.num_dims, self.bytes_per_dim);
      let mut cost = 0;
      let mut iterator = self.sorted_packed_points.iterator()?;
      while let Some(point) = iterator.next()? {
        let point = point.as_ref();
        visitor.set_point(&point.bytes[point.offset..point.offset + point.length]);
        cost += self.values.estimate_doc_count(&visitor)?;
      }
      self.cost = cost;
      debug_assert!(self.cost >= 0);
    }
    Ok(self.cost)
  }
}

/// Essentially does a merge sort, collecting hits when the indexed point and
/// query point are the same. This is an optimization for the 1D case.
pub struct MergePointVisitor {
  result: DocIdSetBuilder,
  iterator: RefCell<TermIteratorArc>,
  next_query_point: RefCell<Option<BytesRef<Vec<u8>>>>,
  comparator: ByteArrayComparatorEnum,
}

impl MergePointVisitor {
  pub fn new(
    sorted_packed_points: PrefixCodedTermsArc,
    result: DocIdSetBuilder,
    bytes_per_dim: usize,
  ) -> Result<Self> {
    let mut iterator = sorted_packed_points.iterator()?;
    let next_query_point = iterator.next()?.map(|point| point.into_owned());
    Ok(Self {
      result,
      iterator: RefCell::new(iterator),
      next_query_point: RefCell::new(next_query_point),
      comparator: ArrayUtil::get_unsigned_comparator(bytes_per_dim),
    })
  }

  fn next_query_point(&self) -> Result<()> {
    let mut iterator = self.iterator.borrow_mut();
    let next_query_point = iterator.next()?.map(|point| point.into_owned());
    *self.next_query_point.borrow_mut() = next_query_point;
    Ok(())
  }

  fn matches(&mut self, packed_value: &[u8]) -> Result<bool> {
    loop {
      let cmp = match self.next_query_point.borrow().as_ref() {
        Some(next_query_point) => self.comparator.compare(
          &next_query_point.bytes,
          next_query_point.offset,
          packed_value,
          0,
        ),
        None => return Ok(false),
      };
      if cmp == 0 {
        return Ok(true);
      } else if cmp < 0 {
        self.next_query_point()?;
      } else {
        break;
      }
    }
    Ok(false)
  }
}

impl IntersectVisitor for MergePointVisitor {
  fn grow(&mut self, count: usize) -> Result<()> {
    self.result.grow(count.try_convert()?);
    Ok(())
  }

  fn visit(&mut self, doc_id: i32) -> Result<()> {
    self.result.add_doc(doc_id);
    Ok(())
  }

  fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
    self.result.add_disi(iterator)?;
    Ok(())
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
    if self.matches(packed_value)? {
      self.visit(doc_id)?;
    }
    Ok(())
  }

  fn visit_iterator_with_packed_value(
    &mut self,
    iterator: &mut impl DocIdSetIterator,
    packed_value: &[u8],
  ) -> Result<()> {
    if self.matches(packed_value)? {
      self.result.add_disi(iterator)?;
    }
    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    loop {
      let cmp_min = match self.next_query_point.borrow().as_ref() {
        Some(next_query_point) => self.comparator.compare(
          &next_query_point.bytes,
          next_query_point.offset,
          min_packed_value,
          0,
        ),
        None => return Ok(Relation::CellOutsideQuery),
      };
      if cmp_min < 0 {
        self.next_query_point()?;
        continue;
      }
      let cmp_max = {
        let next_query_point = self.next_query_point.borrow();
        let next_query_point = next_query_point.as_ref().unwrap();
        self.comparator.compare(
          &next_query_point.bytes,
          next_query_point.offset,
          max_packed_value,
          0,
        )
      };
      if cmp_max > 0 {
        return Ok(Relation::CellOutsideQuery);
      }

      if cmp_min == 0 && cmp_max == 0 {
        return Ok(Relation::CellInsideQuery);
      } else {
        return Ok(Relation::CellCrossesQuery);
      }
    }
  }
}

/// IntersectVisitor that queries against a single point, used in the > 1D case.
pub struct SinglePointVisitor {
  result: DocIdSetBuilder,
  comparator: ByteArrayComparatorEnum,
  num_dims: usize,
  bytes_per_dim: usize,
  point_bytes: Vec<u8>,
}

impl SinglePointVisitor {
  pub fn new(result: DocIdSetBuilder, num_dims: usize, bytes_per_dim: usize) -> Self {
    Self {
      result,
      comparator: ArrayUtil::get_unsigned_comparator(bytes_per_dim),
      num_dims,
      bytes_per_dim,
      point_bytes: vec![0u8; bytes_per_dim * num_dims],
    }
  }

  pub fn set_point(&mut self, point: &[u8]) {
    debug_assert!(point.len() == self.point_bytes.len());
    self.point_bytes.copy_from_slice(point);
  }
}

impl IntersectVisitor for SinglePointVisitor {
  fn grow(&mut self, count: usize) -> Result<()> {
    self.result.grow(count.try_convert()?);
    Ok(())
  }

  fn visit(&mut self, doc_id: i32) -> Result<()> {
    self.result.add_doc(doc_id);
    Ok(())
  }

  fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
    self.result.add_disi(iterator)?;
    Ok(())
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
    debug_assert!(packed_value.len() == self.point_bytes.len());
    if packed_value == self.point_bytes {
      self.visit(doc_id)?;
    }
    Ok(())
  }

  fn visit_iterator_with_packed_value(
    &mut self,
    iterator: &mut impl DocIdSetIterator,
    packed_value: &[u8],
  ) -> Result<()> {
    debug_assert!(packed_value.len() == self.point_bytes.len());
    if packed_value == self.point_bytes {
      self.result.add_disi(iterator)?;
    }
    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    let mut crosses = false;

    for dim in 0..self.num_dims {
      let offset = dim * self.bytes_per_dim;

      let cmp_min = self
        .comparator
        .compare(min_packed_value, offset, &self.point_bytes, offset);
      if cmp_min > 0 {
        return Ok(Relation::CellOutsideQuery);
      }

      let cmp_max = self
        .comparator
        .compare(max_packed_value, offset, &self.point_bytes, offset);
      if cmp_max < 0 {
        return Ok(Relation::CellOutsideQuery);
      }

      if cmp_min != 0 || cmp_max != 0 {
        crosses = true;
      }
    }

    if crosses {
      Ok(Relation::CellCrossesQuery)
    } else {
      Ok(Relation::CellInsideQuery)
    }
  }
}

fn packed_points_as_vec(sorted_packed_points: &PrefixCodedTermsArc) -> Result<Vec<Vec<u8>>> {
  let mut iterator = sorted_packed_points.iterator()?;
  let mut points = Vec::with_capacity(sorted_packed_points.size().try_convert()?);
  while let Some(point) = iterator.next()? {
    let point = point.as_ref();
    points.push(point.bytes[point.offset..point.offset + point.length].to_vec());
  }
  Ok(points)
}

pub trait PointInSetBase {
  /// Format a single packed point value as a human-readable string for debugging.
  fn to_string(&self, value: &[u8]) -> Result<String>;
}

/// Default raw-byte formatter for callers that do not have a typed formatter.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefaultPointInSetQuery;

impl PointInSetBase for DefaultPointInSetQuery {
  fn to_string(&self, value: &[u8]) -> Result<String> {
    Ok(format!("{:?}", value))
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PointInSetBaseEnum {
  Default(DefaultPointInSetQuery),
  BigInteger(BigIntegerPointInSetQuery),
  Binary(BinaryPointInSetQuery),
  Double(DoublePointInSetQuery),
  Float(FloatPointInSetQuery),
  HalfFloat(HalfFloatPointInSetQuery),
  InetAddress(InetAddressPointInSetQuery),
  Int(IntPointInSetQuery),
  Long(LongPointInSetQuery),
}

impl From<DefaultPointInSetQuery> for PointInSetBaseEnum {
  fn from(value: DefaultPointInSetQuery) -> Self {
    Self::Default(value)
  }
}

impl From<BigIntegerPointInSetQuery> for PointInSetBaseEnum {
  fn from(value: BigIntegerPointInSetQuery) -> Self {
    Self::BigInteger(value)
  }
}

impl From<BinaryPointInSetQuery> for PointInSetBaseEnum {
  fn from(value: BinaryPointInSetQuery) -> Self {
    Self::Binary(value)
  }
}

impl From<DoublePointInSetQuery> for PointInSetBaseEnum {
  fn from(value: DoublePointInSetQuery) -> Self {
    Self::Double(value)
  }
}

impl From<FloatPointInSetQuery> for PointInSetBaseEnum {
  fn from(value: FloatPointInSetQuery) -> Self {
    Self::Float(value)
  }
}

impl From<HalfFloatPointInSetQuery> for PointInSetBaseEnum {
  fn from(value: HalfFloatPointInSetQuery) -> Self {
    Self::HalfFloat(value)
  }
}

impl From<InetAddressPointInSetQuery> for PointInSetBaseEnum {
  fn from(value: InetAddressPointInSetQuery) -> Self {
    Self::InetAddress(value)
  }
}

impl From<IntPointInSetQuery> for PointInSetBaseEnum {
  fn from(value: IntPointInSetQuery) -> Self {
    Self::Int(value)
  }
}

impl From<LongPointInSetQuery> for PointInSetBaseEnum {
  fn from(value: LongPointInSetQuery) -> Self {
    Self::Long(value)
  }
}

impl PointInSetBase for PointInSetBaseEnum {
  fn to_string(&self, value: &[u8]) -> Result<String> {
    match self {
      PointInSetBaseEnum::Default(q) => q.to_string(value),
      PointInSetBaseEnum::BigInteger(q) => q.to_string(value),
      PointInSetBaseEnum::Binary(q) => q.to_string(value),
      PointInSetBaseEnum::Double(q) => q.to_string(value),
      PointInSetBaseEnum::Float(q) => q.to_string(value),
      PointInSetBaseEnum::HalfFloat(q) => q.to_string(value),
      PointInSetBaseEnum::InetAddress(q) => q.to_string(value),
      PointInSetBaseEnum::Int(q) => q.to_string(value),
      PointInSetBaseEnum::Long(q) => q.to_string(value),
    }
  }
}
