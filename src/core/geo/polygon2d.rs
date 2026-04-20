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
      && self.tree.contains(x, y)
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
pub(crate) fn create_from_xy_polygon(xy_polygon: &XYPolygon) -> Result<Polygon2D> {
  let holes = if xy_polygon.get_holes().is_empty() {
    None
  } else {
    Some(HolesType::XYGeometry(xy_geometry::create(
      xy_polygon.get_holes(),
    )?))
  };
  Polygon2D::from_xy_polygon(xy_polygon, holes)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
  use crate::test::core::geo::geo_test_util::GeoTestUtil;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, random};
  use crate::test::core::util::test_util::TestUtil;
  use rand::RngExt;
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
  #[test]
  fn test_pac_man() -> Result<()> {
    let px = vec![
      0.0, 10.0, 10.0, 0.0, -8.0, -10.0, -8.0, 0.0, 10.0, 10.0, 0.0,
    ];
    let py = vec![0.0, 5.0, 9.0, 10.0, 9.0, 0.0, -9.0, -10.0, -9.0, -5.0, 0.0];

    let x_min = 2.0;
    let x_max = 11.0;
    let y_min = -1.0;
    let y_max = 1.0;

    let polygon = create_from_polygon(&Polygon::new(py, px, vec![])?)?;
    assert_eq!(
      Relation::CellCrossesQuery,
      polygon.relate(y_min, y_max, x_min, x_max)?
    );
    Ok(())
  }

  #[test]
  fn test_bounding_box() -> Result<()> {
    let mut random = random();
    for _ in 0..100 {
      let polygon = create_from_polygon(&GeoTestUtil::next_polygon(&mut random)?)?;

      for _ in 0..100 {
        let latitude = GeoTestUtil::next_latitude(&mut random);
        let longitude = GeoTestUtil::next_longitude(&mut random);
        if polygon.contains(longitude, latitude) {
          assert!(latitude >= polygon.get_min_y() && latitude <= polygon.get_max_y());
          assert!(longitude >= polygon.get_min_x() && longitude <= polygon.get_max_x());
        }
      }
    }
    Ok(())
  }
  #[test]
  fn test_bounding_box_edge_cases() -> Result<()> {
    let mut random = random();
    for _ in 0..100 {
      let polygon = GeoTestUtil::next_polygon(&mut random)?;
      let impl_ = create_from_polygon(&polygon)?;

      for _ in 0..100 {
        let point = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
        let latitude = point[0];
        let longitude = point[1];
        if impl_.contains(longitude, latitude) {
          assert!(latitude >= polygon.min_lat && latitude <= polygon.max_lat);
          assert!(longitude >= polygon.min_lon && longitude <= polygon.max_lon);
        }
      }
    }
    Ok(())
  }

  #[test]
  fn test_contains_random() -> Result<()> {
    let mut random = random();
    let iters = at_least(&mut random, 50);
    for _ in 0..iters {
      let polygon = GeoTestUtil::next_polygon(&mut random)?;
      let impl_ = create_from_polygon(&polygon)?;

      for _ in 0..100 {
        let rectangle = GeoTestUtil::next_box_near(&mut random, &polygon)?;
        if impl_.relate(
          rectangle.min_lat,
          rectangle.max_lat,
          rectangle.min_lon,
          rectangle.max_lon,
        )? == Relation::CellInsideQuery
        {
          for _ in 0..500 {
            let point = GeoTestUtil::next_point_near(&mut random, &rectangle)?;
            let latitude = point[0];
            let longitude = point[1];
            if latitude >= rectangle.min_lat
              && latitude <= rectangle.max_lat
              && longitude >= rectangle.min_lon
              && longitude <= rectangle.max_lon
            {
              assert!(impl_.contains(latitude, longitude));
            }
          }
          for _ in 0..100 {
            let point = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
            let latitude = point[0];
            let longitude = point[1];
            if latitude >= rectangle.min_lat
              && latitude <= rectangle.max_lat
              && longitude >= rectangle.min_lon
              && longitude <= rectangle.max_lon
            {
              assert!(impl_.contains(latitude, longitude));
            }
          }
        }
      }
    }
    Ok(())
  }
  #[test]
  fn test_contains_edge_cases() -> Result<()> {
    let mut random = random();
    for _ in 0..1000 {
      let polygon = GeoTestUtil::next_polygon(&mut random)?;
      let impl_ = create_from_polygon(&polygon)?;

      for _ in 0..10 {
        let rectangle = GeoTestUtil::next_box_near(&mut random, &polygon)?;
        if impl_.relate(
          rectangle.min_lon,
          rectangle.max_lon,
          rectangle.min_lat,
          rectangle.max_lat,
        )? == Relation::CellInsideQuery
        {
          for _ in 0..100 {
            let point = GeoTestUtil::next_point_near(&mut random, &rectangle)?;
            let latitude = point[0];
            let longitude = point[1];
            if latitude >= rectangle.min_lat
              && latitude <= rectangle.max_lat
              && longitude >= rectangle.min_lon
              && longitude <= rectangle.max_lon
            {
              assert!(impl_.contains(longitude, latitude));
            }
          }
          for _ in 0..20 {
            let point = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
            let latitude = point[0];
            let longitude = point[1];
            if latitude >= rectangle.min_lat
              && latitude <= rectangle.max_lat
              && longitude >= rectangle.min_lon
              && longitude <= rectangle.max_lon
            {
              assert!(impl_.contains(longitude, latitude));
            }
          }
        }
      }
    }
    Ok(())
  }

  #[test]
  fn test_intersect_random() -> Result<()> {
    let mut random = random();
    let iters = at_least(&mut random, 10);
    for _ in 0..iters {
      let polygon = GeoTestUtil::next_polygon(&mut random)?;
      let impl_ = create_from_polygon(&polygon)?;

      let inner_iters = at_least(&mut random, 10);
      for _ in 0..inner_iters {
        let rectangle = GeoTestUtil::next_box_near(&mut random, &polygon)?;
        if impl_.relate(
          rectangle.min_lon,
          rectangle.max_lon,
          rectangle.min_lat,
          rectangle.max_lat,
        )? == Relation::CellOutsideQuery
        {
          for _ in 0..1000 {
            let point = GeoTestUtil::next_point_near(&mut random, &rectangle)?;
            let latitude = point[0];
            let longitude = point[1];
            if latitude >= rectangle.min_lat
              && latitude <= rectangle.max_lat
              && longitude >= rectangle.min_lon
              && longitude <= rectangle.max_lon
            {
              assert!(!impl_.contains(longitude, latitude));
            }
          }
          for _ in 0..100 {
            let point = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
            let latitude = point[0];
            let longitude = point[1];
            if latitude >= rectangle.min_lat
              && latitude <= rectangle.max_lat
              && longitude >= rectangle.min_lon
              && longitude <= rectangle.max_lon
            {
              assert!(!impl_.contains(longitude, latitude));
            }
          }
        }
      }
    }
    Ok(())
  }
  #[test]
  fn test_intersect_edge_cases() -> Result<()> {
    let mut random = random();
    for _ in 0..100 {
      let polygon = GeoTestUtil::next_polygon(&mut random)?;
      let impl_ = create_from_polygon(&polygon)?;

      for _ in 0..10 {
        let rectangle = GeoTestUtil::next_box_near(&mut random, &polygon)?;
        if impl_.relate(
          rectangle.min_lon,
          rectangle.max_lon,
          rectangle.min_lat,
          rectangle.max_lat,
        )? == Relation::CellOutsideQuery
        {
          for _ in 0..100 {
            let point = GeoTestUtil::next_point_near(&mut random, &rectangle)?;
            let latitude = point[0];
            let longitude = point[1];
            if latitude >= rectangle.min_lat
              && latitude <= rectangle.max_lat
              && longitude >= rectangle.min_lon
              && longitude <= rectangle.max_lon
            {
              assert!(!impl_.contains(longitude, latitude));
            }
          }
          for _ in 0..50 {
            let point = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
            let latitude = point[0];
            let longitude = point[1];
            if latitude >= rectangle.min_lat
              && latitude <= rectangle.max_lat
              && longitude >= rectangle.min_lon
              && longitude <= rectangle.max_lon
            {
              assert!(!impl_.contains(longitude, latitude));
            }
          }
        }
      }
    }
    Ok(())
  }
  #[test]
  fn test_edge_insideness() -> Result<()> {
    let poly = create_from_polygon(&Polygon::new(
      vec![-2.0, -2.0, 2.0, 2.0, -2.0],
      vec![-2.0, 2.0, 2.0, -2.0, -2.0],
      vec![],
    )?)?;
    assert!(poly.contains(-2.0, -2.0));
    assert!(poly.contains(2.0, -2.0));
    assert!(poly.contains(-2.0, 2.0));
    assert!(poly.contains(2.0, 2.0));
    assert!(poly.contains(-1.0, -2.0));
    assert!(poly.contains(0.0, -2.0));
    assert!(poly.contains(1.0, -2.0));
    assert!(poly.contains(-1.0, 2.0));
    assert!(poly.contains(0.0, 2.0));
    assert!(poly.contains(1.0, 2.0));
    assert!(poly.contains(2.0, -1.0));
    assert!(poly.contains(2.0, 0.0));
    assert!(poly.contains(2.0, 1.0));
    assert!(poly.contains(-2.0, -1.0));
    assert!(poly.contains(-2.0, 0.0));
    assert!(poly.contains(-2.0, 1.0));
    Ok(())
  }

  #[test]
  fn test_intersects_same_edge() -> Result<()> {
    let poly = create_from_polygon(&Polygon::new(
      vec![-2.0, -2.0, 2.0, 2.0, -2.0],
      vec![-2.0, 2.0, 2.0, -2.0, -2.0],
      vec![],
    )?)?;
    assert!(poly.contains_triangle_values(-1.0, -1.0, 1.0, 1.0, -1.0, -1.0));
    assert!(poly.contains_triangle_values(-2.0, -2.0, 2.0, 2.0, -2.0, -2.0));
    assert!(poly.intersects_triangle_values(-1.0, -1.0, 1.0, 1.0, -1.0, -1.0));
    assert!(poly.intersects_triangle_values(-2.0, -2.0, 2.0, 2.0, -2.0, -2.0));

    assert!(!poly.contains_triangle_values(-4.0, -4.0, 4.0, 4.0, -4.0, -4.0));
    assert!(!poly.contains_triangle_values(-2.0, -2.0, 4.0, 4.0, 4.0, 4.0));
    assert!(poly.intersects_triangle_values(-4.0, -4.0, 4.0, 4.0, -4.0, -4.0));
    assert!(poly.intersects_triangle_values(-2.0, -2.0, 4.0, 4.0, 4.0, 4.0));

    assert!(!poly.contains_triangle_values(-1.0, -1.0, 3.0, 3.0, 1.0, 1.0));
    assert!(!poly.contains_triangle_values(-2.0, -2.0, 3.0, 3.0, 2.0, 2.0));
    assert!(poly.intersects_triangle_values(-1.0, -1.0, 3.0, 3.0, 1.0, 1.0));
    assert!(poly.intersects_triangle_values(-2.0, -2.0, 3.0, 3.0, 2.0, 2.0));

    assert!(!poly.contains_triangle_values(-4.0, -4.0, 7.0, 7.0, 4.0, 4.0));
    assert!(!poly.contains_triangle_values(-2.0, -2.0, 7.0, 7.0, 4.0, 4.0));
    assert!(poly.intersects_triangle_values(-4.0, -4.0, 7.0, 7.0, 4.0, 4.0));
    assert!(poly.intersects_triangle_values(-2.0, -2.0, 7.0, 7.0, 4.0, 4.0));
    Ok(())
  }

  #[test]
  fn test_contains_against_original() -> Result<()> {
    let mut random = random();
    let iters = at_least(&mut random, 100);
    for _ in 0..iters {
      let mut polygon = GeoTestUtil::next_polygon(&mut random)?;
      while !polygon.get_holes().is_empty() {
        polygon = GeoTestUtil::next_polygon(&mut random)?;
      }
      let impl_ = create_from_polygon(&polygon)?;

      for _ in 0..1000 {
        let point = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
        let latitude = point[0];
        let longitude = point[1];
        let expected = GeoTestUtil::contains_slowly(&polygon, longitude, latitude);
        assert_eq!(expected, impl_.contains(latitude, longitude));
      }
    }
    Ok(())
  }

  #[test]
  fn test_relate_triangle() -> Result<()> {
    let mut random = random();
    for _ in 0..100 {
      let polygon = GeoTestUtil::next_polygon(&mut random)?;
      let impl_ = create_from_polygon(&polygon)?;

      for _ in 0..100 {
        let a = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
        let b = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
        let c = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;

        if impl_.contains(a[1], a[0]) || impl_.contains(b[1], b[0]) || impl_.contains(c[1], c[0]) {
          assert!(impl_.intersects_triangle_values(a[1], a[0], b[1], b[0], c[1], c[0]));
        }
      }
    }
    Ok(())
  }

  #[test]
  fn test_relate_triangle_contains_polygon() -> Result<()> {
    let polygon = Polygon::new(
      vec![0.0, 0.0, 1.0, 1.0, 0.0],
      vec![0.0, 1.0, 1.0, 0.0, 0.0],
      vec![],
    )?;
    let impl_ = create_from_polygon(&polygon)?;
    assert!(impl_.intersects_triangle_values(-10.0, -1.0, 2.0, -1.0, 10.0, 10.0));
    Ok(())
  }

  #[test]
  fn test_relate_triangle_edge_cases() -> Result<()> {
    let mut random = random();
    for _ in 0..100 {
      let random_radius = random.random_range(1000..=100000);
      let num_vertices = random.random_range(100..=1000);
      let polygon =
        GeoTestUtil::create_regular_polygon(0.0, 0.0, random_radius as f64, num_vertices)?;
      let impl_ = create_from_polygon(&polygon)?;

      for j in 1..num_vertices {
        let a = [0.0, 0.0];
        let b = [polygon.get_poly_lat(j - 1), polygon.get_poly_lon(j - 1)];
        let c = if random.random_bool(0.5) {
          [polygon.get_poly_lat(j), polygon.get_poly_lon(j)]
        } else {
          [a[0], a[1]]
        };
        assert!(impl_.intersects_triangle_values(a[0], a[1], b[0], b[1], c[0], c[1]));
      }
    }
    Ok(())
  }

  #[test]
  fn test_line_crossing_polygon_points() -> Result<()> {
    let p = Polygon::new(
      vec![0.0, -1.0, 0.0, 1.0, 0.0],
      vec![-1.0, 0.0, 1.0, 0.0, -1.0],
      vec![],
    )?;
    let polygon2d = create_from_polygon(&p)?;
    let intersects = polygon2d.intersects_triangle_values(
      GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(-1.5)?),
      GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(0.0)?),
      GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(1.5)?),
      GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(0.0)?),
      GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(-1.5)?),
      GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(0.0)?),
    );
    assert!(intersects);
    Ok(())
  }

  #[test]
  fn test_random_line_crossing_polygon() -> Result<()> {
    let mut random = random();
    let p = GeoTestUtil::create_regular_polygon(
      0.0,
      0.0,
      1000_f64,
      TestUtil::next_usize(&mut random, 100, 10000),
    )?;
    let polygon2d = create_from_polygon(&p)?;
    for _ in 0..1000 {
      let longitude = GeoTestUtil::next_longitude(&mut random);
      let latitude = GeoTestUtil::next_latitude(&mut random);
      let intersects = polygon2d.intersects_triangle_values(
        GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(-longitude)?),
        GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(-latitude)?),
        GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(longitude)?),
        GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(latitude)?),
        GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(-longitude)?),
        GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(-latitude)?),
      );
      assert!(intersects);
    }
    Ok(())
  }
}
