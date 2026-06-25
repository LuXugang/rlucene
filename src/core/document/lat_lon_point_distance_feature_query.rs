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
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::geo::rectangle::Rectangle;
use crate::core::index::doc_values::{DocValues, EmptyNumeric, SortedNumeric};
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::index_reader_context::{
  IRCLeafReader, IRCNDV, IRCSNDV, IndexReaderContext,
};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::numeric_doc_values::{NumericDocValues, NumericDocValuesEnum3};
use crate::core::index::point_values::{
  IntersectVisitor, PointValues, Relation, is_estimated_point_count_greater_than_or_equal_to,
};
use crate::core::index::sorted_numeric_doc_values::{
  SortedNumericDocValues, SortedNumericDocValuesEnum2,
};
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::{DocIdSetIteratorEnum2, EmptyDISI};
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{
  Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::Weight;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::doc_id_set_builder::{DocIdSetBuilder, DocIdSetBuilderIterator};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::numeric_utils::NumericUtils;
use crate::core::util::sloppy_math::SloppyMath;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct LatLonPointDistanceFeatureQuery {
  id: Identity,
  field: String,
  origin_lat: f64,
  origin_lon: f64,
  pivot_distance: f64,
}

impl LatLonPointDistanceFeatureQuery {
  pub(crate) fn new(
    field: String,
    origin_lat: f64,
    origin_lon: f64,
    pivot_distance: f64,
  ) -> Result<Self> {
    GeoUtils::check_latitude(origin_lat)?;
    GeoUtils::check_longitude(origin_lon)?;
    if pivot_distance <= 0.0 {
      return Err(LuceneError::illegal_argument(format!(
        "pivotDistance must be > 0, got {pivot_distance}"
      )));
    }
    Ok(Self {
      id: Identity::new(),
      field,
      origin_lat,
      origin_lon,
      pivot_distance,
    })
  }
}

impl PartialEq for LatLonPointDistanceFeatureQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field
      && self.origin_lat.to_bits() == other.origin_lat.to_bits()
      && self.origin_lon.to_bits() == other.origin_lon.to_bits()
      && self.pivot_distance.to_bits() == other.pivot_distance.to_bits()
  }
}

impl Eq for LatLonPointDistanceFeatureQuery {}

impl Hash for LatLonPointDistanceFeatureQuery {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.field.hash(state);
    self.origin_lat.to_bits().hash(state);
    self.origin_lon.to_bits().hash(state);
    self.pivot_distance.to_bits().hash(state);
  }
}

impl HasIdentity for LatLonPointDistanceFeatureQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for LatLonPointDistanceFeatureQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    Ok(format!(
      "LatLonPointDistanceFeatureQuery(field={field},originLat={},originLon={},pivotDistance={})",
      self.origin_lat, self.origin_lon, self.pivot_distance
    ))
  }

  fn create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    _score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(Box::new(LatLonPointDistanceFeatureWeight::new(self, boost)))
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
  }
}

pub struct LatLonPointDistanceFeatureWeight {
  parent_query: Arc<Query>,
  query_meta: QueryMeta,
  boost: f32,
}

impl LatLonPointDistanceFeatureWeight {
  fn new(query: LatLonPointDistanceFeatureQuery, boost: f32) -> Self {
    let query_meta = QueryMeta {
      field: query.field.clone(),
      origin_lat: query.origin_lat,
      origin_lon: query.origin_lon,
      pivot_distance: query.pivot_distance,
    };
    Self {
      parent_query: Arc::new(query.into()),
      query_meta,
      boost,
    }
  }
}
fn select_value_with_geo<IRC>(
  multi_doc_values: SortedNumeric<IRCLeafReader<IRC>>,
  origin_lat: f64,
  origin_lon: f64,
) -> Result<SelectValue<IRC>>
where
  IRC: IndexReaderContext,
{
  let r = match multi_doc_values {
    SortedNumericDocValuesEnum2::A(v) => {
      let v = NumericDocValuesImpl::new(v, origin_lat, origin_lon);
      NumericDocValuesEnum3::A(v)
    },
    SortedNumericDocValuesEnum2::B(v) => match v {
      SortedNumericDocValuesEnum2::A(mut v) => {
        let singleton = DocValues::unwrap_singleton_numeric(&mut v)?;
        NumericDocValuesEnum3::B(singleton)
      },
      SortedNumericDocValuesEnum2::B(mut v) => {
        let singleton = DocValues::unwrap_singleton_numeric(&mut v)?;
        NumericDocValuesEnum3::C(singleton)
      },
    },
  };
  Ok(r)
}
pub type SelectValue<IRC> =
  NumericDocValuesEnum3<NumericDocValuesImpl<IRCSNDV<IRC>>, IRCNDV<IRC>, EmptyNumeric>;

impl<IRC> SegmentCacheable<IRC> for LatLonPointDistanceFeatureWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(false)
  }
}

impl<IRC> Weight<IRC> for LatLonPointDistanceFeatureWeight
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
    let reader = context.reader();
    let mut multi_doc_values = DocValues::get_sorted_numeric(reader, &self.query_meta.field)?;
    if !multi_doc_values.advance_exact(doc)? {
      return Ok(Explanation::no_match_no_details(format!(
        "Document {doc} doesn't have a value for field {}",
        self.query_meta.field
      )));
    }
    let encoded = select_value(
      &mut multi_doc_values,
      self.query_meta.origin_lat,
      self.query_meta.origin_lon,
    )?;
    let latitude_bits = (encoded >> 32) as i32;
    let longitude_bits = encoded as i32;
    let lat = GeoEncodingUtils::decode_latitude(latitude_bits);
    let lon = GeoEncodingUtils::decode_longitude(longitude_bits);
    let distance = get_distance_from_encoded(
      encoded,
      self.query_meta.origin_lat,
      self.query_meta.origin_lon,
    );
    let score = (self.boost as f64
      * (self.query_meta.pivot_distance / (self.query_meta.pivot_distance + distance)))
      as f32;
    Ok(Explanation::match_(
      score,
      "Distance score, computed as weight * pivotDistance / (pivotDistance + abs(distance)) from:",
      vec![
        Explanation::match_no_details(self.boost, "weight"),
        Explanation::match_no_details(self.query_meta.pivot_distance, "pivotDistance"),
        Explanation::match_no_details(self.query_meta.origin_lat, "originLat"),
        Explanation::match_no_details(self.query_meta.origin_lon, "originLon"),
        Explanation::match_no_details(lat, "current lat"),
        Explanation::match_no_details(lon, "current lon"),
        Explanation::match_no_details(distance, "distance"),
      ],
    ))
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
    if reader.get_point_values(&self.query_meta.field)?.is_none() {
      return Ok(None);
    }
    let multi_doc_values = DocValues::get_sorted_numeric(reader, &self.query_meta.field)?;
    let max_doc = reader.max_doc()?;
    let doc_values = select_value_with_geo::<IRC>(
      multi_doc_values,
      self.query_meta.origin_lat,
      self.query_meta.origin_lon,
    )?;
    let v = ScorerSupplierImpl::new(
      self.query_meta.clone(),
      max_doc,
      self.boost,
      Some(doc_values),
    )?;
    Ok(Some(Box::new(v)))
  }
}
#[derive(Clone)]
pub struct QueryMeta {
  field: String,
  origin_lat: f64,
  origin_lon: f64,
  pivot_distance: f64,
}
pub struct ScorerSupplierImpl<IRC>
where
  IRC: IndexReaderContext,
{
  query_meta: QueryMeta,
  max_doc: i32,
  boost: f32,
  doc_values: Option<SelectValue<IRC>>,
  cost: i64,
}
impl<IRC> ScorerSupplierImpl<IRC>
where
  IRC: IndexReaderContext,
{
  fn new(
    query_meta: QueryMeta,
    max_doc: i32,
    boost: f32,
    doc_values: Option<SelectValue<IRC>>,
  ) -> Result<Self> {
    let cost = doc_values
      .as_ref()
      .ok_or_else(|| {
        LuceneError::illegal_state("docValues must be present to compute cost".to_string())
      })?
      .cost()?;
    Ok(Self {
      query_meta,
      max_doc,
      boost,
      doc_values,
      cost,
    })
  }
}
impl<IRC> ScorerSupplier<IRC> for ScorerSupplierImpl<IRC>
where
  IRC: IndexReaderContext,
{
  type Scorer = QueryWeightSsScorer;
  type BulkScorer = QueryWeightSsBulkScorer;

  fn get(
    &mut self,
    lead_cost: i64,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::Scorer> {
    let reader = context.reader();
    let point_values = reader
      .get_point_values(&self.query_meta.field)?
      .ok_or_else(|| {
        LuceneError::illegal_state(format!(
          "Field {} does not have point values",
          self.query_meta.field
        ))
      })?;
    let doc_values = match self.doc_values.take() {
      Some(v) => v,
      None => return Err(LuceneError::illegal_state("get only call once")),
    };
    let v = DistanceScorer::new(
      self.max_doc,
      lead_cost,
      self.boost,
      point_values,
      doc_values,
      self.query_meta.clone(),
    );
    Ok(Box::new(v))
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
    Ok(self.cost)
  }
}

pub struct DistanceScorer<PV, ND> {
  max_doc: i32,
  disi: DocIdSetIteratorImpl<ND>,
  lead_cost: i64,
  boost: f32,
  point_values: PV,
  max_distance: f64,
  set_min_competitive_score_counter: i32,
  query_meta: QueryMeta,
}

impl<PV, ND> DistanceScorer<PV, ND>
where
  ND: NumericDocValues,
  PV: PointValues,
{
  fn new(
    max_doc: i32,
    lead_cost: i64,
    boost: f32,
    point_values: PV,
    doc_values: ND,
    query_meta: QueryMeta,
  ) -> Self {
    let disi = DocIdSetIteratorImpl::new(doc_values);
    Self {
      max_doc,
      disi,
      lead_cost,
      boost,
      point_values,
      max_distance: GeoUtils::EARTH_MEAN_RADIUS_METERS * std::f64::consts::PI,
      set_min_competitive_score_counter: 0,
      query_meta,
    }
  }
  fn score_with_distance(&self, distance: f64) -> f32 {
    (self.boost as f64
      * (self.query_meta.pivot_distance / (self.query_meta.pivot_distance + distance))) as f32
  }
  /// Inverting the score computation is very hard due to all potential rounding errors,
  /// so we binary search the maximum distance. The limit is set to 1 meter.
  fn compute_max_distance(&self, min_score: f32, previous_max_distance: f64) -> f64 {
    debug_assert!(self.score_with_distance(0.0) >= min_score);

    if self.score_with_distance(previous_max_distance) >= min_score {
      // minScore did not decrease enough to require an update to the max distance
      return previous_max_distance;
    }

    debug_assert!(self.score_with_distance(previous_max_distance) < min_score);

    let mut min = 0.0f64;
    let mut max = previous_max_distance;

    while max - min > 1.0 {
      let mid = (min + max) / 2.0;
      let score = self.score_with_distance(mid);
      if score >= min_score {
        min = mid;
      } else {
        max = mid;
      }
    }

    debug_assert!(self.score_with_distance(min) >= min_score);
    debug_assert!(min == f64::MAX || self.score_with_distance(min + 1.0) < min_score);

    min
  }
}

impl<PV, ND> Scorable for DistanceScorer<PV, ND>
where
  ND: NumericDocValues + 'static,
  PV: PointValues,
{
  fn score(&mut self) -> Result<f32> {
    let doc_id = self.doc_id()?;
    if !self.disi.doc_values.advance_exact(doc_id)? {
      return Ok(0.0);
    }
    let long_value = self.disi.doc_values.long_value()?;
    Ok(self.score_with_distance(get_distance_from_encoded(
      long_value,
      self.query_meta.origin_lat,
      self.query_meta.origin_lon,
    )))
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    if min_score > self.boost {
      self.disi.set_empty();
      return Ok(());
    }

    self.set_min_competitive_score_counter += 1;
    // We sample the calls to this method as it is expensive to recalculate the iterator.
    if self.set_min_competitive_score_counter > 256
      && (self.set_min_competitive_score_counter & 0x1f) != 0x1f
    {
      return Ok(());
    }

    let previous_max_distance = self.max_distance;
    self.max_distance = self.compute_max_distance(min_score, self.max_distance);
    if self.max_distance == previous_max_distance {
      return Ok(());
    }

    let box_ = Rectangle::from_point_distance(
      self.query_meta.origin_lat,
      self.query_meta.origin_lon,
      self.max_distance,
    )?;
    let min_lat = GeoEncodingUtils::encode_latitude(box_.min_lat)?;
    let max_lat = GeoEncodingUtils::encode_latitude(box_.max_lat)?;
    let min_lon = GeoEncodingUtils::encode_longitude(box_.min_lon)?;
    let max_lon = GeoEncodingUtils::encode_longitude(box_.max_lon)?;
    let cross_dateline = box_.crosses_dateline();

    let result = DocIdSetBuilder::new(self.max_doc);
    let doc = self.doc_id()?;
    let mut visitor = DistanceScorerIntersectVisitor::new(
      result,
      doc,
      min_lat,
      max_lat,
      min_lon,
      max_lon,
      cross_dateline,
    );

    let current_query_cost = self.lead_cost.min(self.disi.cost()?);
    let threshold = ((current_query_cost as u64) >> 3) as i64;
    if is_estimated_point_count_greater_than_or_equal_to(
      &visitor,
      &mut self.point_values.get_point_tree()?,
      threshold,
    )? {
      return Ok(());
    }

    self.point_values.intersect(&mut visitor)?;
    let it = visitor.result.build()?.iterator()?;
    self.disi.set_builder_iterator(it);
    Ok(())
  }
}

impl<PV, ND> FixedScore for DistanceScorer<PV, ND>
where
  ND: NumericDocValues,
  PV: PointValues,
{
}

impl<PV, ND> Scorer for DistanceScorer<PV, ND>
where
  ND: NumericDocValues + 'static,
  PV: PointValues,
{
  fn doc_id(&mut self) -> Result<i32> {
    Ok(self.disi.doc)
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.disi)
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.disi)
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let DistanceScorer { disi, .. } = *self;
    Box::new(disi)
  }

  fn get_max_score(&mut self, _upto: i32) -> Result<f32> {
    Ok(self.boost)
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

struct DistanceScorerIntersectVisitor {
  result: DocIdSetBuilder,
  doc: i32,
  min_lat: i32,
  max_lat: i32,
  min_lon: i32,
  max_lon: i32,
  cross_dateline: bool,
}

impl DistanceScorerIntersectVisitor {
  fn new(
    result: DocIdSetBuilder,
    doc: i32,
    min_lat: i32,
    max_lat: i32,
    min_lon: i32,
    max_lon: i32,
    cross_dateline: bool,
  ) -> Self {
    Self {
      result,
      doc,
      min_lat,
      max_lat,
      min_lon,
      max_lon,
      cross_dateline,
    }
  }
}

impl IntersectVisitor for DistanceScorerIntersectVisitor {
  fn grow(&mut self, count: usize) -> Result<()> {
    self.result.grow(count as i32);
    Ok(())
  }

  fn visit(&mut self, doc_id: i32) -> Result<()> {
    if doc_id > self.doc {
      self.result.add_doc(doc_id);
    }
    Ok(())
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
    if doc_id <= self.doc {
      return Ok(());
    }

    let lat = NumericUtils::sortable_bytes_to_int(packed_value, 0);
    if lat > self.max_lat || lat < self.min_lat {
      return Ok(());
    }

    let lon = NumericUtils::sortable_bytes_to_int(packed_value, LatLonPoint::BYTES);
    if self.cross_dateline {
      if lon < self.min_lon && lon > self.max_lon {
        return Ok(());
      }
    } else if lon > self.max_lon || lon < self.min_lon {
      return Ok(());
    }

    self.result.add_doc(doc_id);
    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    let lat_lower_bound = NumericUtils::sortable_bytes_to_int(min_packed_value, 0);
    let lat_upper_bound = NumericUtils::sortable_bytes_to_int(max_packed_value, 0);
    if lat_lower_bound > self.max_lat || lat_upper_bound < self.min_lat {
      return Ok(Relation::CellOutsideQuery);
    }

    let mut crosses = lat_lower_bound < self.min_lat || lat_upper_bound > self.max_lat;
    let lon_lower_bound = NumericUtils::sortable_bytes_to_int(min_packed_value, LatLonPoint::BYTES);
    let lon_upper_bound = NumericUtils::sortable_bytes_to_int(max_packed_value, LatLonPoint::BYTES);
    if self.cross_dateline {
      if lon_lower_bound > self.max_lon && lon_upper_bound < self.min_lon {
        return Ok(Relation::CellOutsideQuery);
      }
      crosses |= lon_lower_bound < self.max_lon || lon_upper_bound > self.min_lon;
    } else {
      if lon_lower_bound > self.max_lon || lon_upper_bound < self.min_lon {
        return Ok(Relation::CellOutsideQuery);
      }
      crosses |= lon_lower_bound < self.min_lon || lon_upper_bound > self.max_lon;
    }

    if crosses {
      Ok(Relation::CellCrossesQuery)
    } else {
      Ok(Relation::CellInsideQuery)
    }
  }
}

pub struct DocIdSetIteratorImpl<ND> {
  it: Option<DocIdSetIteratorEnum2<EmptyDISI, DocIdSetBuilderIterator>>,
  doc_values: ND,
  doc: i32,
}
impl<ND> DocIdSetIteratorImpl<ND>
where
  ND: NumericDocValues,
{
  fn new(it: ND) -> Self {
    Self {
      it: None,
      doc_values: it,
      doc: -1,
    }
  }

  fn set_empty(&mut self) {
    self.it = Some(DocIdSetIteratorEnum2::A(EmptyDISI::new()));
  }

  fn set_builder_iterator(&mut self, it: DocIdSetBuilderIterator) {
    self.it = Some(DocIdSetIteratorEnum2::B(it));
  }
}
impl<ND> DocIdSetIterator for DocIdSetIteratorImpl<ND>
where
  ND: NumericDocValues,
{
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.doc = match self.it {
      Some(ref mut v) => v.next_doc()?,
      None => self.doc_values.next_doc()?,
    };
    Ok(self.doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.doc = match self.it {
      Some(ref mut v) => v.advance(target)?,
      None => self.doc_values.advance(target)?,
    };
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    match self.it {
      Some(ref v) => v.cost(),
      None => self.doc_values.cost(),
    }
  }
}

pub struct NumericDocValuesImpl<T> {
  multi_doc_values: T,
  value: i64,
  origin_lat: f64,
  origin_lon: f64,
}
impl<T> NumericDocValuesImpl<T>
where
  T: SortedNumericDocValues,
{
  fn new(multi_doc_values: T, origin_lat: f64, origin_lon: f64) -> Self {
    Self {
      multi_doc_values,
      value: 0,
      origin_lat,
      origin_lon,
    }
  }
}

impl<T> DocValuesIterator for NumericDocValuesImpl<T>
where
  T: SortedNumericDocValues,
{
  fn advance_exact(&mut self, target: i32) -> Result<bool> {
    if self.multi_doc_values.advance_exact(target)? {
      self.value = select_value(&mut self.multi_doc_values, self.origin_lat, self.origin_lon)?;
      Ok(true)
    } else {
      Ok(false)
    }
  }
}

impl<T> DocIdSetIterator for NumericDocValuesImpl<T>
where
  T: SortedNumericDocValues,
{
  fn doc_id(&self) -> i32 {
    self.multi_doc_values.doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.multi_doc_values.next_doc()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.multi_doc_values.advance(target)
  }

  fn cost(&self) -> Result<i64> {
    self.multi_doc_values.cost()
  }
}

impl<T> NumericDocValues for NumericDocValuesImpl<T>
where
  T: SortedNumericDocValues,
{
  fn long_value(&mut self) -> Result<i64> {
    Ok(self.value)
  }
}
fn select_value<SN>(multi_doc_values: &mut SN, origin_lat: f64, origin_lon: f64) -> Result<i64>
where
  SN: SortedNumericDocValues,
{
  let count = multi_doc_values.doc_value_count()?;
  let mut value = multi_doc_values.next_value()?;
  if count == 1 {
    return Ok(value);
  }
  let mut distance = get_distance_key_from_encoded(value, origin_lat, origin_lon);
  for _ in 1..count {
    let next_value = multi_doc_values.next_value()?;
    let next_distance = get_distance_key_from_encoded(next_value, origin_lat, origin_lon);
    if next_distance < distance {
      distance = next_distance;
      value = next_value;
    }
  }
  Ok(value)
}
fn get_distance_from_encoded(encoded: i64, origin_lat: f64, origin_lon: f64) -> f64 {
  SloppyMath::haversin_meters_from_sort_key(get_distance_key_from_encoded(
    encoded, origin_lat, origin_lon,
  ))
}

fn get_distance_key_from_encoded(encoded: i64, origin_lat: f64, origin_lon: f64) -> f64 {
  let latitude_bits = (encoded >> 32) as i32;
  let longitude_bits = encoded as i32;
  let lat = GeoEncodingUtils::decode_latitude(latitude_bits);
  let lon = GeoEncodingUtils::decode_longitude(longitude_bits);
  SloppyMath::haversin_sort_key(origin_lat, origin_lon, lat, lon)
}

impl crate::core::util::accountable::Accountable for LatLonPointDistanceFeatureQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}
