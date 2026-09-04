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
use crate::core::geo::component_tree::{ComponentTree, create};
use crate::core::geo::component2d::{
  Component2D, Component2DEnum2, WithinRelation, contains_point, disjoint, point_in_triangle,
  within,
};
use crate::core::geo::geo_encoding_utils::{GeoEncodingUtils, MAX_LON_ENCODED, MIN_LON_ENCODED};
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::geo::rectangle::Rectangle;
use crate::core::geo::xy_rectangle::XYRectangle;
use crate::core::index::point_values::Relation;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::core_helper::CoreHelper;
use crate::core::util::error::lucene_error::Result;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::sync::LazyLock;

/// 2D rectangle implementation containing cartesian spatial logic.
#[derive(Debug)]
pub struct Rectangle2D {
  min_x: f64,
  max_x: f64,
  min_y: f64,
  max_y: f64,
}

impl Rectangle2D {
  pub(crate) fn new(min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Self {
    Self {
      min_x,
      max_x,
      min_y,
      max_y,
    }
  }
  fn edges_intersect(&self, a_x: f64, a_y: f64, b_x: f64, b_y: f64) -> bool {
    if CoreHelper::max_f64(a_x, b_x) < self.min_x
      || CoreHelper::min_f64(a_x, b_x) > self.max_x
      || CoreHelper::min_f64(a_y, b_y) > self.max_y
      || CoreHelper::max_f64(a_y, b_y) < self.min_y
    {
      return false;
    }

    GeoUtils::line_crosses_line_with_boundary(
      a_x, a_y, b_x, b_y, self.min_x, self.max_y, self.max_x, self.max_y,
    ) || GeoUtils::line_crosses_line_with_boundary(
      a_x, a_y, b_x, b_y, self.max_x, self.max_y, self.max_x, self.min_y,
    ) || GeoUtils::line_crosses_line_with_boundary(
      a_x, a_y, b_x, b_y, self.max_x, self.min_y, self.min_x, self.min_y,
    ) || GeoUtils::line_crosses_line_with_boundary(
      a_x, a_y, b_x, b_y, self.min_x, self.min_y, self.min_x, self.max_y,
    )
  }
}
impl Component2D for Rectangle2D {
  fn get_min_x(&self) -> f64 {
    self.min_x
  }

  fn get_max_x(&self) -> f64 {
    self.max_x
  }

  fn get_min_y(&self) -> f64 {
    self.min_y
  }

  fn get_max_y(&self) -> f64 {
    self.max_y
  }

  fn contains(&self, x: f64, y: f64) -> bool {
    contains_point(x, y, self.min_x, self.max_x, self.min_y, self.max_y)
  }

  fn relate(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Result<Relation> {
    if disjoint(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return Ok(Relation::CellOutsideQuery);
    }

    if within(
      min_x, max_x, min_y, max_y, self.min_x, self.max_x, self.min_y, self.max_y,
    ) {
      return Ok(Relation::CellInsideQuery);
    }

    Ok(Relation::CellCrossesQuery)
  }

  fn intersects_line(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    a_x: f64,
    a_y: f64,
    b_x: f64,
    b_y: f64,
  ) -> bool {
    if disjoint(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return false;
    }

    self.contains(a_x, a_y) || self.contains(b_x, b_y) || self.edges_intersect(a_x, a_y, b_x, b_y)
  }

  fn intersects_triangle(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    a_x: f64,
    a_y: f64,
    b_x: f64,
    b_y: f64,
    c_x: f64,
    c_y: f64,
  ) -> bool {
    if disjoint(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return false;
    }

    self.contains(a_x, a_y)
      || self.contains(b_x, b_y)
      || self.contains(c_x, c_y)
      || point_in_triangle(
        min_x, max_x, min_y, max_y, self.min_x, self.min_y, a_x, a_y, b_x, b_y, c_x, c_y,
      )
      || self.edges_intersect(a_x, a_y, b_x, b_y)
      || self.edges_intersect(b_x, b_y, c_x, c_y)
      || self.edges_intersect(c_x, c_y, a_x, a_y)
  }

  fn contains_line(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    _a_x: f64,
    _a_y: f64,
    _b_x: f64,
    _b_y: f64,
  ) -> bool {
    within(
      min_x, max_x, min_y, max_y, self.min_x, self.max_x, self.min_y, self.max_y,
    )
  }

  fn contains_triangle(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    _a_x: f64,
    _a_y: f64,
    _b_x: f64,
    _b_y: f64,
    _c_x: f64,
    _c_y: f64,
  ) -> bool {
    within(
      min_x, max_x, min_y, max_y, self.min_x, self.max_x, self.min_y, self.max_y,
    )
  }

  fn within_point(&self, x: f64, y: f64) -> Result<WithinRelation> {
    Ok(if self.contains(x, y) {
      WithinRelation::NotWithin
    } else {
      WithinRelation::Disjoint
    })
  }

  fn within_line(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    a_x: f64,
    a_y: f64,
    ab: bool,
    b_x: f64,
    b_y: f64,
  ) -> Result<WithinRelation> {
    if disjoint(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return Ok(WithinRelation::Disjoint);
    }

    if self.contains(a_x, a_y) || self.contains(b_x, b_y) {
      return Ok(WithinRelation::NotWithin);
    }

    if ab && self.edges_intersect(a_x, a_y, b_x, b_y) {
      return Ok(WithinRelation::NotWithin);
    }

    Ok(WithinRelation::Disjoint)
  }

  fn within_triangle(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    a_x: f64,
    a_y: f64,
    ab: bool,
    b_x: f64,
    b_y: f64,
    bc: bool,
    c_x: f64,
    c_y: f64,
    ca: bool,
  ) -> Result<WithinRelation> {
    if disjoint(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return Ok(WithinRelation::Disjoint);
    }

    if self.contains(a_x, a_y) || self.contains(b_x, b_y) || self.contains(c_x, c_y) {
      return Ok(WithinRelation::NotWithin);
    }

    let mut relation = WithinRelation::Disjoint;

    if self.edges_intersect(a_x, a_y, b_x, b_y) {
      if ab {
        return Ok(WithinRelation::NotWithin);
      } else {
        relation = WithinRelation::Candidate;
      }
    }

    if self.edges_intersect(b_x, b_y, c_x, c_y) {
      if bc {
        return Ok(WithinRelation::NotWithin);
      } else {
        relation = WithinRelation::Candidate;
      }
    }

    if self.edges_intersect(c_x, c_y, a_x, a_y) {
      if ca {
        return Ok(WithinRelation::NotWithin);
      } else {
        relation = WithinRelation::Candidate;
      }
    }

    if relation == WithinRelation::Candidate {
      return Ok(WithinRelation::Candidate);
    }

    if point_in_triangle(
      min_x, max_x, min_y, max_y, self.min_x, self.min_y, a_x, a_y, b_x, b_y, c_x, c_y,
    ) {
      return Ok(WithinRelation::Candidate);
    }

    Ok(relation)
  }
}
impl Display for Rectangle2D {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Rectangle2D(x={} TO {} y={} TO {})",
      self.min_x, self.max_x, self.min_y, self.max_y
    )
  }
}
impl PartialEq for Rectangle2D {
  fn eq(&self, other: &Self) -> bool {
    CoreHelper::compare_f64(self.min_x, other.min_x).is_eq()
      && CoreHelper::compare_f64(self.max_x, other.max_x).is_eq()
      && CoreHelper::compare_f64(self.min_y, other.min_y).is_eq()
      && CoreHelper::compare_f64(self.max_y, other.max_y).is_eq()
  }
}

impl Eq for Rectangle2D {}

impl std::hash::Hash for Rectangle2D {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    (BitUtil::double_to_long_bits(self.min_x) as u64).hash(state);
    (BitUtil::double_to_long_bits(self.max_x) as u64).hash(state);
    (BitUtil::double_to_long_bits(self.min_y) as u64).hash(state);
    (BitUtil::double_to_long_bits(self.max_y) as u64).hash(state);
  }
}
static MIN_LON_INCL_QUANTIZE: LazyLock<f64> =
  LazyLock::new(|| GeoEncodingUtils::decode_longitude(*MIN_LON_ENCODED));

static MAX_LON_INCL_QUANTIZE: LazyLock<f64> =
  LazyLock::new(|| GeoEncodingUtils::decode_longitude(*MAX_LON_ENCODED));
pub(crate) fn create_from_xy_rectangle(rectangle: &XYRectangle) -> Rectangle2D {
  Rectangle2D::new(
    rectangle.min_x as f64,
    rectangle.max_x as f64,
    rectangle.min_y as f64,
    rectangle.max_y as f64,
  )
}

pub type Rectangle2DType = Component2DEnum2<ComponentTree<Rectangle2D>, Rectangle2D>;
pub(crate) fn create_from_rectangle(rectangle: &Rectangle) -> Result<Rectangle2DType> {
  let mut min_longitude = rectangle.min_lon;
  let mut crosses_dateline = rectangle.min_lon > rectangle.max_lon;
  if min_longitude == 180.0 && crosses_dateline {
    min_longitude = -180.0;
    crosses_dateline = false;
  }

  let q_min_lat =
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude_ceil(rectangle.min_lat)?);
  let q_max_lat =
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(rectangle.max_lat)?);
  let q_min_lon =
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude_ceil(min_longitude)?);
  let q_max_lon =
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(rectangle.max_lon)?);

  if crosses_dateline {
    let components = vec![
      Rectangle2D::new(*MIN_LON_INCL_QUANTIZE, q_max_lon, q_min_lat, q_max_lat),
      Rectangle2D::new(q_min_lon, *MAX_LON_INCL_QUANTIZE, q_min_lat, q_max_lat),
    ];
    Ok(Rectangle2DType::A(create(components)?))
  } else {
    Ok(Rectangle2DType::B(Rectangle2D::new(
      q_min_lon, q_max_lon, q_min_lat, q_max_lat,
    )))
  }
}
