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
use crate::core::geo::component2d::{
  Component2D, WithinRelation, contains_point, disjoint, point_in_triangle, within,
};
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::geo::xy_rectangle::XYRectangle;
use crate::core::index::point_values::Relation;
use crate::core::util::error::lucene_error::Result;
use std::fmt;
use std::fmt::{Display, Formatter};

/// 2D rectangle implementation containing cartesian spatial logic.
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
    if a_x.max(b_x) < self.min_x
      || a_x.min(b_x) > self.max_x
      || a_y.min(b_y) > self.max_y
      || a_y.max(b_y) < self.min_y
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
  pub(crate) fn create(rectangle: &XYRectangle) -> Rectangle2D {
    Rectangle2D::new(
      rectangle.max_x as f64,
      rectangle.max_x as f64,
      rectangle.min_y as f64,
      rectangle.max_y as f64,
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

  fn relate(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Relation {
    if disjoint(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return Relation::CellOutsideQuery;
    }

    if within(
      min_x, max_x, min_y, max_y, self.min_x, self.max_x, self.min_y, self.max_y,
    ) {
      return Relation::CellInsideQuery;
    }

    Relation::CellCrossesQuery
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
