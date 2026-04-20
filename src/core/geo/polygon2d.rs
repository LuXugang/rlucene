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
use crate::core::geo::edge_tree::{EdgeTree, create_tree};
use crate::core::geo::lat_lon_geometry::LatLonGeometryType;
use crate::core::geo::polygon::Polygon;
use crate::core::geo::xy_encoding_utils::XYEncodingUtils;
use crate::core::geo::xy_geometry::XYGeometryType;
use crate::core::geo::xy_polygon::XYPolygon;
use crate::core::geo::{lat_lon_geometry, xy_geometry};
use crate::core::index::point_values::Relation;
use crate::core::util::error::lucene_error::Result;
use crate::either_component2d_named;

/// 2D polygon implementation represented as a balanced interval tree of edges.
///
/// Loosely based on the algorithm described in
/// <http://www-ma2.upc.es/geoc/Schirra-pointPolygon.pdf>.
pub struct Polygon2D {
  /// minimum Y of this geometry's bounding box area
  min_y: f64,

  /// maximum Y of this geometry's bounding box area
  max_y: f64,

  /// minimum X of this geometry's bounding box area
  min_x: f64,

  /// maximum X of this geometry's bounding box area
  max_x: f64,

  /// tree of holes, or null
  pub(crate) holes: Option<Box<HolesType>>,

  /// Edges of the polygon represented as a 2-d interval tree.
  tree: EdgeTree,
}
either_component2d_named!(pub HolesEnum{ LatLonGeometry: A, XYGeometry: B});
pub type HolesType = HolesEnum<LatLonGeometryType<Polygon2D>, XYGeometryType<Polygon2D>>;

impl Polygon2D {
  fn new(
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    x: Vec<f64>,
    y: Vec<f64>,
    holes: Option<HolesType>,
  ) -> Result<Self> {
    let holes = holes.map(Box::new);
    Ok(Self {
      min_y,
      max_y,
      min_x,
      max_x,
      holes,
      tree: create_tree(&x, &y)?,
    })
  }

  fn from_xy_polygon(polygon: &XYPolygon, holes: Option<HolesType>) -> Result<Self> {
    Self::new(
      polygon.min_x as f64,
      polygon.max_x as f64,
      polygon.min_y as f64,
      polygon.max_y as f64,
      XYEncodingUtils::float_array_to_double_array(polygon.get_poly_x()),
      XYEncodingUtils::float_array_to_double_array(polygon.get_poly_y()),
      holes,
    )
  }

  fn from_polygon(polygon: &Polygon, holes: Option<HolesType>) -> Result<Self> {
    Self::new(
      polygon.min_lon,
      polygon.max_lon,
      polygon.min_lat,
      polygon.max_lat,
      polygon.get_poly_lons().to_vec(),
      polygon.get_poly_lats().to_vec(),
      holes,
    )
  }
  fn number_of_corners(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> i32 {
    let mut contains_count = 0;
    if self.contains(min_x, min_y) {
      contains_count += 1;
    }
    if self.contains(max_x, min_y) {
      contains_count += 1;
    }
    if contains_count == 1 {
      return contains_count;
    }
    if self.contains(max_x, max_y) {
      contains_count += 1;
    }
    if contains_count == 2 {
      return contains_count;
    }
    if self.contains(min_x, max_y) {
      contains_count += 1;
    }
    contains_count
  }
}
impl Component2D for Polygon2D {
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
    if contains_point(x, y, self.min_x, self.max_x, self.min_y, self.max_y)
      && self.tree.contains_pn_poly(x, y) == EdgeTree::TRUE
    {
      return self
        .holes
        .as_ref()
        .is_none_or(|holes| !holes.contains(x, y));
    }
    false
  }

  fn relate(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Result<Relation> {
    if disjoint(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return Ok(Relation::CellOutsideQuery);
    }
    if within(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return Ok(Relation::CellCrossesQuery);
    }

    if let Some(holes) = &self.holes {
      let hole_relation = holes.relate(min_x, max_x, min_y, max_y)?;
      if hole_relation == Relation::CellCrossesQuery {
        return Ok(Relation::CellCrossesQuery);
      } else if hole_relation == Relation::CellInsideQuery {
        return Ok(Relation::CellOutsideQuery);
      }
    }
    // check each corner: if < 4 && > 0 are present, its cheaper than crossesSlowly
    let num_corners = self.number_of_corners(min_x, max_x, min_y, max_y);
    if num_corners == 4 {
      if self.tree.crosses_box(min_x, max_x, min_y, max_y, true) {
        return Ok(Relation::CellCrossesQuery);
      }
      return Ok(Relation::CellInsideQuery);
    } else if num_corners == 0 {
      if contains_point(self.tree.x1, self.tree.y1, min_x, max_x, min_y, max_y) {
        return Ok(Relation::CellCrossesQuery);
      }
      if self.tree.crosses_box(min_x, max_x, min_y, max_y, true) {
        return Ok(Relation::CellCrossesQuery);
      }
      return Ok(Relation::CellOutsideQuery);
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
    if self.contains(a_x, a_y)
      || self.contains(b_x, b_y)
      || self
        .tree
        .crosses_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, true)
    {
      return self
        .holes
        .as_ref()
        .is_none_or(|holes| !holes.contains_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y));
    }
    false
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
    if self.contains(a_x, a_y)
      || self.contains(b_x, b_y)
      || self.contains(c_x, c_y)
      || point_in_triangle(
        min_x,
        max_x,
        min_y,
        max_y,
        self.tree.x1,
        self.tree.y1,
        a_x,
        a_y,
        b_x,
        b_y,
        c_x,
        c_y,
      )
      || self.tree.crosses_triangle(
        min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y, true,
      )
    {
      return self.holes.as_ref().is_none_or(|holes| {
        !holes.contains_triangle(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y)
      });
    }
    false
  }

  fn contains_line(
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
    if self.contains(a_x, a_y)
      && self.contains(b_x, b_y)
      && !self
        .tree
        .crosses_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, false)
    {
      return self.holes.as_ref().is_none_or(|holes| {
        !holes.intersects_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y)
      });
    }
    false
  }

  fn contains_triangle(
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
    if self.contains(a_x, a_y)
      && self.contains(b_x, b_y)
      && self.contains(c_x, c_y)
      && !self.tree.crosses_triangle(
        min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y, false,
      )
    {
      return self.holes.as_ref().is_none_or(|holes| {
        !holes.intersects_triangle(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y)
      });
    }
    false
  }

  fn within_point(&self, x: f64, y: f64) -> Result<WithinRelation> {
    if self.contains(x, y) {
      Ok(WithinRelation::NotWithin)
    } else {
      Ok(WithinRelation::Disjoint)
    }
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
    if ab
      && self
        .tree
        .crosses_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, true)
    {
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

    if self
      .tree
      .crosses_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, true)
    {
      if ab {
        return Ok(WithinRelation::NotWithin);
      } else {
        relation = WithinRelation::Candidate;
      }
    }

    if self
      .tree
      .crosses_line(min_x, max_x, min_y, max_y, b_x, b_y, c_x, c_y, true)
    {
      if bc {
        return Ok(WithinRelation::NotWithin);
      } else {
        relation = WithinRelation::Candidate;
      }
    }

    if self
      .tree
      .crosses_line(min_x, max_x, min_y, max_y, c_x, c_y, a_x, a_y, true)
    {
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
      min_x,
      max_x,
      min_y,
      max_y,
      self.tree.x1,
      self.tree.y1,
      a_x,
      a_y,
      b_x,
      b_y,
      c_x,
      c_y,
    ) {
      return Ok(WithinRelation::Candidate);
    }
    Ok(relation)
  }
}
/// Builds a Polygon2D from LatLon polygon
pub(crate) fn create_from_polygon(polygon: &Polygon) -> Result<Polygon2D> {
  let holes = if polygon.get_holes().is_empty() {
    None
  } else {
    Some(HolesType::LatLonGeometry(lat_lon_geometry::create(
      polygon.get_holes(),
    )?))
  };
  Polygon2D::from_polygon(polygon, holes)
}

/// Builds a Polygon2D from XY polygon
pub(crate) fn create_from_xy_polygon(polygon: &XYPolygon) -> Result<Polygon2D> {
  let holes = if polygon.get_holes().is_empty() {
    None
  } else {
    Some(HolesType::XYGeometry(xy_geometry::create(
      polygon.get_holes(),
    )?))
  };
  Polygon2D::from_xy_polygon(polygon, holes)
}

#[cfg(test)]
mod tests {
  use super::*;
  #[cfg(test)] // for quick search
  struct TestPolygon2D;
  #[test]
  fn test_multi_polygon() -> Result<()> {
    let hole = Polygon::new(
      vec![-10.0, -10.0, 10.0, 10.0, -10.0],
      vec![-10.0, 10.0, 10.0, -10.0, -10.0],
      vec![],
    )?;
    let outer = Polygon::new(
      vec![-50.0, -50.0, 50.0, 50.0, -50.0],
      vec![-50.0, 50.0, 50.0, -50.0, -50.0],
      vec![hole],
    )?;
    let island = Polygon::new(
      vec![-5.0, -5.0, 5.0, 5.0, -5.0],
      vec![-5.0, 5.0, 5.0, -5.0, -5.0],
      vec![],
    )?;
    let polygon = lat_lon_geometry::create::<Polygon>(&[outer, island])?;

    assert!(polygon.contains(-2.0, 2.0));
    assert!(!polygon.contains(-6.0, 6.0));
    assert!(polygon.contains(-25.0, 25.0));
    assert!(!polygon.contains(-51.0, 51.0));

    assert_eq!(
      Relation::CellInsideQuery,
      polygon.relate(-2.0, 2.0, -2.0, 2.0)?
    );
    assert_eq!(
      Relation::CellOutsideQuery,
      polygon.relate(6.0, 7.0, 6.0, 7.0)?
    );
    assert_eq!(
      Relation::CellInsideQuery,
      polygon.relate(24.0, 25.0, 24.0, 25.0)?
    );
    assert_eq!(
      Relation::CellOutsideQuery,
      polygon.relate(51.0, 52.0, 51.0, 52.0)?
    );
    assert_eq!(
      Relation::CellCrossesQuery,
      polygon.relate(-60.0, 60.0, -60.0, 60.0)?
    );
    assert_eq!(
      Relation::CellCrossesQuery,
      polygon.relate(49.0, 51.0, 49.0, 51.0)?
    );
    assert_eq!(
      Relation::CellCrossesQuery,
      polygon.relate(9.0, 11.0, 9.0, 11.0)?
    );
    assert_eq!(
      Relation::CellCrossesQuery,
      polygon.relate(5.0, 6.0, 5.0, 6.0)?
    );

    Ok(())
  }
}
