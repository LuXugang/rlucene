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
use crate::core::document::shape_field::QueryRelation;
use crate::core::geo::component2d::{Component2D, WithinRelation};
use crate::core::geo::geo_encoding_utils::{Component2DPredicate, GeoEncodingUtils};
use crate::core::geo::geometry::Geometry;
use crate::core::geo::lat_lon_geometry;
use crate::core::geo::lat_lon_geometry::{
  LatLonGeometryEnum, LatLonGeometryEnumComponent2D, LatLonGeometryType,
};
use crate::core::index::doc_values::DocValues;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::two_phase_iterator::{TwoPhaseIterator, TwoPhaseIteratorEnum4};
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Finds all previously indexed geo points that comply the given [`QueryRelation`] with the
/// specified array of [`LatLonGeometryEnum`].
///
/// The field must be indexed using LatLonDocValuesField added per document.
#[derive(Clone)]
pub struct LatLonDocValuesQuery {
  id: Identity,
  field: String,
  geometries: Arc<Vec<LatLonGeometryEnum>>,
  query_relation: QueryRelation,
  component2d: Arc<LatLonGeometryType<<LatLonGeometryEnum as Geometry>::Component2D>>,
}

impl LatLonDocValuesQuery {
  pub(crate) fn new(
    field: String,
    query_relation: QueryRelation,
    geometries: Vec<LatLonGeometryEnum>,
  ) -> Result<Self> {
    if query_relation == QueryRelation::Within {
      for geometry in &geometries {
        if matches!(geometry, LatLonGeometryEnum::Line(_)) {
          return Err(LuceneError::illegal_argument(format!(
            "LatLonDocValuesPointQuery does not support {:?} queries with line geometries",
            QueryRelation::Within
          )));
        }
      }
    }
    if query_relation == QueryRelation::Contains {
      for geometry in &geometries {
        if !matches!(geometry, LatLonGeometryEnum::Point(_)) {
          return Err(LuceneError::illegal_argument(format!(
            "LatLonDocValuesPointQuery does not support {:?} queries with non-points geometries",
            QueryRelation::Contains
          )));
        }
      }
    }
    let component2d = Arc::new(lat_lon_geometry::create(geometries.as_slice())?);
    let geometries = Arc::new(geometries);
    Ok(Self {
      id: Identity::new(),
      field,
      geometries,
      query_relation,
      component2d,
    })
  }
}

impl Debug for LatLonDocValuesQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("LatLonDocValuesQuery")
      .field("field", &self.field)
      .field("query_relation", &self.query_relation)
      .finish()
  }
}

impl PartialEq for LatLonDocValuesQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field
      && self.query_relation == other.query_relation
      && self.geometries == other.geometries
  }
}

impl Eq for LatLonDocValuesQuery {}

impl Hash for LatLonDocValuesQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.field.hash(state);
    self.query_relation.hash(state);
    self.geometries.hash(state);
  }
}

impl HasIdentity for LatLonDocValuesQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for LatLonDocValuesQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    let mut sb = String::new();
    if self.field != field {
      sb.push_str(&self.field);
      sb.push(':');
    }
    sb.push_str(&format!("{:?}", self.query_relation));
    sb.push(':');
    sb.push_str("geometries(");
    for (i, geometry) in self.geometries.iter().enumerate() {
      if i > 0 {
        sb.push_str(", ");
      }
      sb.push_str(&geometry.to_string());
    }
    sb.push(')');
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
    Ok(Box::new(LatLonDocValuesQueryWeight::new(
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

#[allow(clippy::type_complexity)]
pub struct LatLonDocValuesQueryWeight {
  query: LatLonDocValuesQuery,
  base: ConstantScoreWeight,
  parent_query: Arc<Query>,
  score_mode: ScoreMode,
  boost: f32,
  component2d_predicate:
    Mutex<Option<Component2DPredicate<Arc<LatLonGeometryType<LatLonGeometryEnumComponent2D>>>>>,
}

impl LatLonDocValuesQueryWeight {
  fn new(query: LatLonDocValuesQuery, score_mode: ScoreMode, boost: f32) -> Result<Self> {
    let component2d_predicate = if query.query_relation == QueryRelation::Contains {
      None
    } else {
      Some(GeoEncodingUtils::create_component_predicate(
        query.component2d.clone(),
      )?)
    };
    let query_clone = query.clone();
    let parent_query = Arc::new(query.into());
    Ok(Self {
      query: query_clone,
      base: ConstantScoreWeight::new(boost),
      parent_query,
      score_mode,
      boost,
      component2d_predicate: Mutex::new(component2d_predicate),
    })
  }
}

impl<IRC> SegmentCacheable<IRC> for LatLonDocValuesQueryWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    let field = vec![self.query.field.clone()];
    DocValues::is_cacheable(ctx, field.as_ref())
  }
}

impl<IRC> Weight<IRC> for LatLonDocValuesQueryWeight
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
    let component2d_predicate = match self.query.query_relation {
      QueryRelation::Contains => None,
      _ => Some(GeoEncodingUtils::create_component_predicate(
        self.query.component2d.clone(),
      )?),
    };
    match context
      .reader()
      .get_sorted_numeric_doc_values(&self.query.field)?
    {
      Some(values) => {
        let iterator = match self.query.query_relation {
          QueryRelation::Intersects => {
            TwoPhaseIteratorEnum4::A(IntersectsTPI::new(values, component2d_predicate.unwrap()))
          },
          QueryRelation::Within => {
            TwoPhaseIteratorEnum4::B(WithinTPI::new(values, component2d_predicate.unwrap()))
          },
          QueryRelation::Disjoint => {
            TwoPhaseIteratorEnum4::C(DisjointTPI::new(values, component2d_predicate.unwrap()))
          },
          QueryRelation::Contains => {
            TwoPhaseIteratorEnum4::D(ContainsTPI::new(values, self.query.geometries.as_slice())?)
          },
        };
        let scorer = ConstantScoreScorer::from_tpi(self.boost, self.score_mode, iterator);
        Ok(Some(Box::new(DefaultScorerSupplier::new(scorer))))
      },
      None => Ok(None),
    }
  }
}

pub struct IntersectsTPI<S, C>
where
  S: SortedNumericDocValues,
  C: Component2D,
{
  values: S,
  component2d_predicate: Component2DPredicate<C>,
}
impl<S, C> IntersectsTPI<S, C>
where
  S: SortedNumericDocValues,
  C: Component2D,
{
  fn new(values: S, component2d_predicate: Component2DPredicate<C>) -> Self {
    Self {
      values,
      component2d_predicate,
    }
  }
}
impl<S, C> TwoPhaseIterator for IntersectsTPI<S, C>
where
  S: SortedNumericDocValues,
  C: Component2D,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.values)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.values)
  }

  fn matches(&mut self) -> Result<bool> {
    let count = self.values.doc_value_count()?;
    for _ in 0..count {
      let value = self.values.next_value()? as u64;
      let lat = (value >> 32) as i32;
      let lon = value as u32 as i32;
      if self.component2d_predicate.test(lat, lon) {
        return Ok(true);
      }
    }
    Ok(false)
  }

  fn match_cost(&self) -> f32 {
    1000f32
  }
}
pub struct WithinTPI<S, C>
where
  S: SortedNumericDocValues,
  C: Component2D,
{
  values: S,
  component2d_predicate: Component2DPredicate<C>,
}
impl<S, C> WithinTPI<S, C>
where
  S: SortedNumericDocValues,
  C: Component2D,
{
  fn new(values: S, component2d_predicate: Component2DPredicate<C>) -> Self {
    Self {
      values,
      component2d_predicate,
    }
  }
}
impl<S, C> TwoPhaseIterator for WithinTPI<S, C>
where
  S: SortedNumericDocValues,
  C: Component2D,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.values)
  }
  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.values)
  }
  fn matches(&mut self) -> Result<bool> {
    let count = self.values.doc_value_count()?;
    for _ in 0..count {
      let value = self.values.next_value()? as u64;
      let lat = (value >> 32) as i32;
      let lon = value as u32 as i32;
      if !self.component2d_predicate.test(lat, lon) {
        return Ok(false);
      }
    }
    Ok(true)
  }

  fn match_cost(&self) -> f32 {
    1000f32
  }
}
pub struct DisjointTPI<S, C>
where
  S: SortedNumericDocValues,
  C: Component2D,
{
  values: S,
  component2d_predicate: Component2DPredicate<C>,
}
impl<S, C> DisjointTPI<S, C>
where
  S: SortedNumericDocValues,
  C: Component2D,
{
  fn new(values: S, component2d_predicate: Component2DPredicate<C>) -> Self {
    Self {
      values,
      component2d_predicate,
    }
  }
}
impl<S, C> TwoPhaseIterator for DisjointTPI<S, C>
where
  S: SortedNumericDocValues,
  C: Component2D,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.values)
  }
  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.values)
  }

  fn matches(&mut self) -> Result<bool> {
    let count = self.values.doc_value_count()?;
    for _ in 0..count {
      let value = self.values.next_value()? as u64;
      let lat = (value >> 32) as i32;
      let lon = value as u32 as i32;
      if self.component2d_predicate.test(lat, lon) {
        return Ok(false);
      }
    }
    Ok(true)
  }

  fn match_cost(&self) -> f32 {
    1000f32
  }
}
pub struct ContainsTPI<S>
where
  S: SortedNumericDocValues,
{
  values: S,
  component2ds: Vec<LatLonGeometryType<<LatLonGeometryEnum as Geometry>::Component2D>>,
}
impl<S> ContainsTPI<S>
where
  S: SortedNumericDocValues,
{
  fn new(values: S, geometries: &[LatLonGeometryEnum]) -> Result<Self> {
    let mut component2ds = Vec::with_capacity(geometries.len());
    for geometry in geometries {
      component2ds.push(lat_lon_geometry::create(std::slice::from_ref(geometry))?);
    }
    Ok(Self {
      values,
      component2ds,
    })
  }
}
impl<S> TwoPhaseIterator for ContainsTPI<S>
where
  S: SortedNumericDocValues,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.values)
  }
  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.values)
  }

  fn matches(&mut self) -> Result<bool> {
    let mut answer = WithinRelation::Disjoint;
    let count = self.values.doc_value_count()?;
    for _ in 0..count {
      let value = self.values.next_value()? as u64;
      let lat = (value >> 32) as i32;
      let lon = value as u32 as i32;
      let lat = GeoEncodingUtils::decode_latitude(lat);
      let lon = GeoEncodingUtils::decode_longitude(lon);
      for component2d in self.component2ds.as_slice() {
        let relation = component2d.within_point(lon, lat)?;
        if relation == WithinRelation::NotWithin {
          return Ok(false);
        } else if relation != WithinRelation::Disjoint {
          answer = relation;
        }
      }
    }
    Ok(answer == WithinRelation::Candidate)
  }

  fn match_cost(&self) -> f32 {
    1000f32
  }
}

impl crate::core::util::accountable::Accountable for LatLonDocValuesQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}
