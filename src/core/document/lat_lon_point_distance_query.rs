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
use crate::core::document::lat_lon_point::LatLonPoint;
use crate::core::geo::geo_encoding_utils::{DistancePredicate, GeoEncodingUtils};
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::geo::rectangle::Rectangle;
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::point_values::{IntersectVisitor, PointValues, Relation};
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
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::doc_id_set_builder::DocIdSetBuilder;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::numeric_utils::NumericUtils;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Distance query for [`LatLonPoint`].
#[derive(Debug, Clone)]
pub struct LatLonPointDistanceQuery {
  id: Identity,
  field: String,
  latitude: f64,
  longitude: f64,
  radius_meters: f64,
}

impl LatLonPointDistanceQuery {
  pub fn new(field: String, latitude: f64, longitude: f64, radius_meters: f64) -> Result<Self> {
    if !radius_meters.is_finite() || radius_meters < 0.0 {
      return Err(LuceneError::illegal_argument(format!(
        "radiusMeters: '{}' is invalid",
        radius_meters
      )));
    }
    GeoUtils::check_latitude(latitude)?;
    GeoUtils::check_longitude(longitude)?;
    Ok(Self {
      id: Identity::new(),
      field,
      latitude,
      longitude,
      radius_meters,
    })
  }
}

impl PartialEq for LatLonPointDistanceQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field
      && self.latitude.to_bits() == other.latitude.to_bits()
      && self.longitude.to_bits() == other.longitude.to_bits()
      && self.radius_meters.to_bits() == other.radius_meters.to_bits()
  }
}

impl Eq for LatLonPointDistanceQuery {}

impl Hash for LatLonPointDistanceQuery {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.field.hash(state);
    self.latitude.to_bits().hash(state);
    self.longitude.to_bits().hash(state);
    self.radius_meters.to_bits().hash(state);
  }
}

impl HasIdentity for LatLonPointDistanceQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for LatLonPointDistanceQuery {
  fn as_string(&self, field: &str) -> Result<String> {
    let mut sb = String::new();
    if self.field != field {
      sb.push_str(&self.field);
      sb.push(':');
    }
    sb.push_str(&self.latitude.to_string());
    sb.push(',');
    sb.push_str(&self.longitude.to_string());
    sb.push_str(" +/- ");
    sb.push_str(&self.radius_meters.to_string());
    sb.push_str(" meters");
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
    Ok(Box::new(LatLonPointDistanceWeight::new(
      self,
      *score_mode,
      boost,
    )?))
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

pub struct LatLonPointDistanceWeight {
  base: ConstantScoreWeight,
  parent_query: Arc<Query>,
  query: Arc<LatLonPointDistanceQuery>,
  score_mode: ScoreMode,
  min_lat: i32,
  max_lat: i32,
  min_lon: i32,
  max_lon: i32,
  min_lon2: i32,
  sort_key: f64,
  axis_lat: f64,
  distance_predicate: Arc<DistancePredicate>,
}

impl LatLonPointDistanceWeight {
  fn new(query: LatLonPointDistanceQuery, score_mode: ScoreMode, boost: f32) -> Result<Self> {
    let box_ =
      Rectangle::from_point_distance(query.latitude, query.longitude, query.radius_meters)?;
    let min_lat = GeoEncodingUtils::encode_latitude(box_.min_lat)?;
    let max_lat = GeoEncodingUtils::encode_latitude(box_.max_lat)?;
    let (min_lon, max_lon, min_lon2) = if box_.crosses_dateline() {
      (
        i32::MIN,
        GeoEncodingUtils::encode_longitude(box_.max_lon)?,
        GeoEncodingUtils::encode_longitude(box_.min_lon)?,
      )
    } else {
      (
        GeoEncodingUtils::encode_longitude(box_.min_lon)?,
        GeoEncodingUtils::encode_longitude(box_.max_lon)?,
        i32::MAX,
      )
    };
    let sort_key = GeoUtils::distance_query_sort_key(query.radius_meters);
    let axis_lat = Rectangle::axis_lat(query.latitude, query.radius_meters);
    let distance_predicate = GeoEncodingUtils::create_distance_predicate(
      query.latitude,
      query.longitude,
      query.radius_meters,
    )?;
    let query = Arc::new(query);
    Ok(Self {
      base: ConstantScoreWeight::new(boost),
      parent_query: Arc::new(query.as_ref().clone().into()),
      query,
      score_mode,
      min_lat,
      max_lat,
      min_lon,
      max_lon,
      min_lon2,
      sort_key,
      axis_lat,
      distance_predicate: Arc::new(distance_predicate),
    })
  }

  fn get_intersect_visitor(&self, result: DocIdSetBuilder) -> LatLonDistanceIntersectVisitor {
    LatLonDistanceIntersectVisitor::new(result, self.distance_context())
  }

  fn distance_context(&self) -> LatLonDistanceContext {
    LatLonDistanceContext {
      latitude: self.query.latitude,
      longitude: self.query.longitude,
      min_lat: self.min_lat,
      max_lat: self.max_lat,
      min_lon: self.min_lon,
      max_lon: self.max_lon,
      min_lon2: self.min_lon2,
      sort_key: self.sort_key,
      axis_lat: self.axis_lat,
      distance_predicate: self.distance_predicate.clone(),
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for LatLonPointDistanceWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(true)
  }
}

impl<IRC> Weight<IRC> for LatLonPointDistanceWeight
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
      Some(values) => values,
      // No docs in this segment had any points fields
      None => return Ok(None),
    };
    let field_infos = reader.get_field_infos()?;
    let Some(field_info) = field_infos.field_info_by_name(&self.query.field) else {
      // No docs in this segment indexed this field at all
      return Ok(None);
    };
    LatLonPoint::check_compatible(&field_info)?;
    let result = DocIdSetBuilder::from_point_values(reader.max_doc()?, &values, &self.query.field)?;
    let visitor = self.get_intersect_visitor(result);
    Ok(Some(Box::new(ScorerSupplierImpl::new(
      self.base.score(),
      self.score_mode,
      values,
      visitor,
    ))))
  }
}

#[derive(Clone)]
struct LatLonDistanceContext {
  latitude: f64,
  longitude: f64,
  min_lat: i32,
  max_lat: i32,
  min_lon: i32,
  max_lon: i32,
  min_lon2: i32,
  sort_key: f64,
  axis_lat: f64,
  distance_predicate: Arc<DistancePredicate>,
}

impl LatLonDistanceContext {
  fn matches(&self, packed_value: &[u8]) -> bool {
    let lat = NumericUtils::sortable_bytes_to_int(packed_value, 0);
    if lat > self.max_lat || lat < self.min_lat {
      return false;
    }
    let lon = NumericUtils::sortable_bytes_to_int(packed_value, LatLonPoint::BYTES);
    if (lon > self.max_lon || lon < self.min_lon) && lon < self.min_lon2 {
      return false;
    }
    self.distance_predicate.test(lat, lon)
  }
  // algorithm: we create a bounding box (two bounding boxes if we cross the dateline).
  // 1. check our bounding box(es) first. if the subtree is entirely outside of those, bail.
  // 2. check if the subtree is disjoint. it may cross the bounding box but not intersect with
  // circle
  // 3. see if the subtree is fully contained. if the subtree is enormous along the x axis,
  // wrapping half way around the world, etc: then this can't work, just go to step 4.
  // 4. recurse naively (subtrees crossing over circle edge)
  fn relate(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    let lat_lower_bound = NumericUtils::sortable_bytes_to_int(min_packed_value, 0);
    let lat_upper_bound = NumericUtils::sortable_bytes_to_int(max_packed_value, 0);
    if lat_lower_bound > self.max_lat || lat_upper_bound < self.min_lat {
      return Ok(Relation::CellOutsideQuery);
    }

    let lon_lower_bound = NumericUtils::sortable_bytes_to_int(min_packed_value, LatLonPoint::BYTES);
    let lon_upper_bound = NumericUtils::sortable_bytes_to_int(max_packed_value, LatLonPoint::BYTES);
    if (lon_lower_bound > self.max_lon || lon_upper_bound < self.min_lon)
      && lon_upper_bound < self.min_lon2
    {
      return Ok(Relation::CellOutsideQuery);
    }

    GeoUtils::relate(
      GeoEncodingUtils::decode_latitude(lat_lower_bound),
      GeoEncodingUtils::decode_latitude(lat_upper_bound),
      GeoEncodingUtils::decode_longitude(lon_lower_bound),
      GeoEncodingUtils::decode_longitude(lon_upper_bound),
      self.latitude,
      self.longitude,
      self.sort_key,
      self.axis_lat,
    )
  }
}

struct ScorerSupplierImpl<PV>
where
  PV: PointValues,
{
  score: f32,
  score_mode: ScoreMode,
  values: PV,
  visitor: LatLonDistanceIntersectVisitor,
  cost: i64,
}

impl<PV> ScorerSupplierImpl<PV>
where
  PV: PointValues,
{
  fn new(
    score: f32,
    score_mode: ScoreMode,
    values: PV,
    visitor: LatLonDistanceIntersectVisitor,
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
  for ScorerSupplierImpl<<IRCLeafReader<IRC> as LeafReader>::PointValues>
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
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::Scorer> {
    let reader = context.reader();
    let size = self.values.size()? as i32;
    if self.values.get_doc_count()? == reader.max_doc()?
      && self.values.get_doc_count()? == size
      && self.cost(context, searcher)? > reader.max_doc()? as i64 / 2
    {
      let max_doc = reader.max_doc()?;
      let mut result = FixedBitSet::new(max_doc as usize);
      result.set_with_range(0, max_doc as usize);
      let cost = {
        let mut visitor = LatLonDistanceInverseIntersectVisitor::new(
          &mut result,
          max_doc as i64,
          self.visitor.ctx.clone(),
        );
        self.values.intersect(&mut visitor)?;
        visitor.cost
      };
      let iterator = BitSetIterator::new(result, cost)?;
      return Ok(Box::new(ConstantScoreScorer::from_disi(
        self.score,
        self.score_mode,
        iterator,
      )));
    }
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

struct LatLonDistanceIntersectVisitor {
  result: DocIdSetBuilder,
  ctx: LatLonDistanceContext,
}

impl LatLonDistanceIntersectVisitor {
  fn new(result: DocIdSetBuilder, ctx: LatLonDistanceContext) -> Self {
    Self { result, ctx }
  }
}

impl IntersectVisitor for LatLonDistanceIntersectVisitor {
  fn grow(&mut self, count: usize) -> Result<()> {
    self.result.grow(count as i32);
    Ok(())
  }

  fn visit(&mut self, doc_id: i32) -> Result<()> {
    self.result.add_doc(doc_id);
    Ok(())
  }

  fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
    self.result.add_disi(iterator)
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
    if self.ctx.matches(packed_value) {
      self.visit(doc_id)?;
    }
    Ok(())
  }

  fn visit_iterator_with_packed_value(
    &mut self,
    iterator: &mut impl DocIdSetIterator,
    packed_value: &[u8],
  ) -> Result<()> {
    if self.ctx.matches(packed_value) {
      self.result.add_disi(iterator)?;
    }
    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    self.ctx.relate(min_packed_value, max_packed_value)
  }
}

struct LatLonDistanceInverseIntersectVisitor<'a> {
  result: &'a mut FixedBitSet,
  cost: i64,
  ctx: LatLonDistanceContext,
}

impl<'a> LatLonDistanceInverseIntersectVisitor<'a> {
  fn new(result: &'a mut FixedBitSet, cost: i64, ctx: LatLonDistanceContext) -> Self {
    Self { result, cost, ctx }
  }
}

impl IntersectVisitor for LatLonDistanceInverseIntersectVisitor<'_> {
  fn visit(&mut self, doc_id: i32) -> Result<()> {
    self.result.clear_with_index(doc_id as usize);
    self.cost -= 1;
    Ok(())
  }

  fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
    self.result.and_not_iter(iterator)?;
    self.cost = (self.cost - iterator.cost()?).max(0);
    Ok(())
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
    if !self.ctx.matches(packed_value) {
      self.visit(doc_id)?;
    }
    Ok(())
  }

  fn visit_iterator_with_packed_value(
    &mut self,
    iterator: &mut impl DocIdSetIterator,
    packed_value: &[u8],
  ) -> Result<()> {
    if !self.ctx.matches(packed_value) {
      self.visit_with_iterator(iterator)?;
    }
    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    Ok(match self.ctx.relate(min_packed_value, max_packed_value)? {
      Relation::CellInsideQuery => Relation::CellOutsideQuery,
      Relation::CellOutsideQuery => Relation::CellInsideQuery,
      Relation::CellCrossesQuery => Relation::CellCrossesQuery,
    })
  }
}
