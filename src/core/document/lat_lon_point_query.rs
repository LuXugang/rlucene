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
use crate::core::document::shape_field::QueryRelation;
use crate::core::document::spatial_query::{SpatialQuery, SpatialQueryBase, SpatialVisitor};
use crate::core::geo::component2d::{Component2D, WithinRelation};
use crate::core::geo::geo_encoding_utils::{Component2DPredicate, GeoEncodingUtils};
use crate::core::geo::geometry::Geometry;
use crate::core::geo::lat_lon_geometry;
use crate::core::geo::lat_lon_geometry::{LatLonGeometryEnum, LatLonGeometryType};
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::point_values::Relation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::numeric_utils::NumericUtils;
use std::sync::Arc;
/// Finds all previously indexed geo points that comply the given [`QueryRelation`] with the
/// specified array of [`LatLonGeometry`].
///
/// The field must be indexed using one or more LatLonPoint added per document.
pub type LatLonPointQuery = SpatialQuery<LatLonGeometryEnum, LatLonPointSpatial>;

pub(crate) fn lat_lon_point_query(
  field: String,
  query_relation: QueryRelation,
  geometries: Vec<LatLonGeometryEnum>,
) -> Result<LatLonPointQuery> {
  let sub = LatLonPointSpatial::new(geometries.as_slice())?;
  SpatialQuery::new(
    field,
    query_relation,
    validate_geometry(query_relation, geometries)?,
    sub,
  )
}

impl QueryBase for LatLonPointQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    self.to_string(field)
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
    let query = Arc::new(self.clone().into());
    self.inner_create_weight(searcher, score_mode, boost, query)
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
    SpatialQuery::visit(self, visitor, query)
  }
}

#[derive(Clone)]
pub struct LatLonPointSpatial {
  query_component2d: Arc<LatLonGeometryType<<LatLonGeometryEnum as Geometry>::Component2D>>,
}
impl LatLonPointSpatial {
  pub fn new(geometries: &[LatLonGeometryEnum]) -> Result<Self> {
    let query_component2d = Arc::new(lat_lon_geometry::create(geometries)?);
    Ok(Self { query_component2d })
  }
}
impl SpatialQueryBase for LatLonPointSpatial {
  type SpatialVisitor =
    SpatialVisitorImpl<Arc<LatLonGeometryType<<LatLonGeometryEnum as Geometry>::Component2D>>>;

  fn get_spatial_visitor(&self) -> Result<Self::SpatialVisitor> {
    let component2d_predicate =
      GeoEncodingUtils::create_component_predicate(self.query_component2d.clone())?;
    // bounding box over all geometries, this can speed up tree intersection/cheaply improve
    // approximation for complex multi-geometries
    let min_lat = GeoEncodingUtils::encode_latitude(self.query_component2d.get_min_y())?;
    let max_lat = GeoEncodingUtils::encode_latitude(self.query_component2d.get_max_y())?;
    let min_lon = GeoEncodingUtils::encode_longitude(self.query_component2d.get_min_x())?;
    let max_lon = GeoEncodingUtils::encode_longitude(self.query_component2d.get_max_x())?;

    Ok(SpatialVisitorImpl::new(
      self.query_component2d.clone(),
      component2d_predicate,
      min_lat,
      max_lat,
      min_lon,
      max_lon,
    ))
  }
}
pub struct SpatialVisitorImpl<C>
where
  C: Component2D,
{
  query_component2d: C,
  component2d_predicate: Component2DPredicate<C>,
  min_lat: i32,
  max_lat: i32,
  min_lon: i32,
  max_lon: i32,
}
impl<C> SpatialVisitorImpl<C>
where
  C: Component2D,
{
  pub fn new(
    query_component2d: C,
    component2d_predicate: Component2DPredicate<C>,
    min_lat: i32,
    max_lat: i32,
    min_lon: i32,
    max_lon: i32,
  ) -> SpatialVisitorImpl<C> {
    SpatialVisitorImpl {
      query_component2d,
      component2d_predicate,
      min_lat,
      max_lat,
      min_lon,
      max_lon,
    }
  }
}
impl<C> SpatialVisitor for SpatialVisitorImpl<C>
where
  C: Component2D,
{
  fn relate(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    let lat_lower_bound = NumericUtils::sortable_bytes_to_int(min_packed_value, 0);
    let lat_upper_bound = NumericUtils::sortable_bytes_to_int(max_packed_value, 0);
    if lat_lower_bound > self.max_lat || lat_upper_bound < self.min_lat {
      return Ok(Relation::CellOutsideQuery);
    }

    let lon_lower_bound = NumericUtils::sortable_bytes_to_int(min_packed_value, LatLonPoint::BYTES);
    let lon_upper_bound = NumericUtils::sortable_bytes_to_int(max_packed_value, LatLonPoint::BYTES);
    if lon_lower_bound > self.max_lon || lon_upper_bound < self.min_lon {
      return Ok(Relation::CellOutsideQuery);
    }

    let cell_min_lat = GeoEncodingUtils::decode_latitude(lat_lower_bound);
    let cell_min_lon = GeoEncodingUtils::decode_longitude(lon_lower_bound);
    let cell_max_lat = GeoEncodingUtils::decode_latitude(lat_upper_bound);
    let cell_max_lon = GeoEncodingUtils::decode_longitude(lon_upper_bound);

    self
      .query_component2d
      .relate(cell_min_lon, cell_max_lon, cell_min_lat, cell_max_lat)
  }

  fn intersects(&self, packed_value: &[u8]) -> Result<bool> {
    Ok(self.component2d_predicate.test(
      NumericUtils::sortable_bytes_to_int(packed_value, 0),
      NumericUtils::sortable_bytes_to_int(packed_value, BitUtil::INT_BYTES),
    ))
  }

  fn within(&self, packed_value: &[u8]) -> Result<bool> {
    Ok(self.component2d_predicate.test(
      NumericUtils::sortable_bytes_to_int(packed_value, 0),
      NumericUtils::sortable_bytes_to_int(packed_value, BitUtil::INT_BYTES),
    ))
  }

  fn contains(&self, packed_value: &[u8]) -> Result<WithinRelation> {
    self.query_component2d.within_point(
      GeoEncodingUtils::decode_longitude(NumericUtils::sortable_bytes_to_int(
        packed_value,
        BitUtil::INT_BYTES,
      )),
      GeoEncodingUtils::decode_latitude(NumericUtils::sortable_bytes_to_int(packed_value, 0)),
    )
  }
}

fn validate_geometry(
  query_relation: QueryRelation,
  geometries: Vec<LatLonGeometryEnum>,
) -> Result<Vec<LatLonGeometryEnum>> {
  if query_relation == QueryRelation::Within {
    for geometry in &geometries {
      if matches!(geometry, LatLonGeometryEnum::Line(_)) {
        return Err(LuceneError::illegal_argument(format!(
          "LatLonPointQuery does not support {:?} queries with line geometries",
          QueryRelation::Within
        )));
      }
    }
  }

  if query_relation == QueryRelation::Contains {
    for geometry in &geometries {
      if !matches!(geometry, LatLonGeometryEnum::Point(_)) {
        return Err(LuceneError::illegal_argument(format!(
          "LatLonPointQuery does not support {:?} queries with non-points geometries",
          QueryRelation::Contains
        )));
      }
    }
  }
  Ok(geometries)
}

impl crate::core::util::accountable::Accountable for LatLonPointQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}
