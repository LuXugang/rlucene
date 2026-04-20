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
  Component2D, WithinRelation, contains_point, point_in_triangle,
};
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::geo::point::Point;
use crate::core::geo::xy_point::XYPoint;
use crate::core::index::point_values::Relation;
use crate::core::util::error::lucene_error::Result;

/// 2D point implementation containing geo spatial logic.
#[derive(Clone, Copy, Debug)]
pub struct Point2D {
  x: f64,
  y: f64,
}

impl Point2D {
  fn new(x: f64, y: f64) -> Self {
    Self { x, y }
  }
}
impl Component2D for Point2D {
  fn get_min_x(&self) -> f64 {
    self.x
  }

  fn get_max_x(&self) -> f64 {
    self.x
  }

  fn get_min_y(&self) -> f64 {
    self.y
  }

  fn get_max_y(&self) -> f64 {
    self.y
  }

  fn contains(&self, x: f64, y: f64) -> bool {
    x == self.x && y == self.y
  }

  fn relate(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Result<Relation> {
    if contains_point(self.x, self.y, min_x, max_x, min_y, max_y) {
      Ok(Relation::CellCrossesQuery)
    } else {
      Ok(Relation::CellOutsideQuery)
    }
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
    contains_point(self.x, self.y, min_x, max_x, min_y, max_y)
      && GeoUtils::orient(a_x, a_y, b_x, b_y, self.x, self.y) == 0
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
    point_in_triangle(
      min_x, max_x, min_y, max_y, self.x, self.y, a_x, a_y, b_x, b_y, c_x, c_y,
    )
  }

  fn contains_line(
    &self,
    _min_x: f64,
    _max_x: f64,
    _min_y: f64,
    _max_y: f64,
    _a_x: f64,
    _a_y: f64,
    _b_x: f64,
    _b_y: f64,
  ) -> bool {
    false
  }

  fn contains_triangle(
    &self,
    _min_x: f64,
    _max_x: f64,
    _min_y: f64,
    _max_y: f64,
    _a_x: f64,
    _a_y: f64,
    _b_x: f64,
    _b_y: f64,
    _c_x: f64,
    _c_y: f64,
  ) -> bool {
    false
  }

  fn within_point(&self, x: f64, y: f64) -> Result<WithinRelation> {
    if self.contains(x, y) {
      Ok(WithinRelation::Candidate)
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
    _ab: bool,
    b_x: f64,
    b_y: f64,
  ) -> Result<WithinRelation> {
    if self.intersects_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y) {
      Ok(WithinRelation::Candidate)
    } else {
      Ok(WithinRelation::Disjoint)
    }
  }

  fn within_triangle(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    a_x: f64,
    a_y: f64,
    _ab: bool,
    b_x: f64,
    b_y: f64,
    _bc: bool,
    c_x: f64,
    c_y: f64,
    _ca: bool,
  ) -> Result<WithinRelation> {
    if point_in_triangle(
      min_x, max_x, min_y, max_y, self.x, self.y, a_x, a_y, b_x, b_y, c_x, c_y,
    ) {
      Ok(WithinRelation::Candidate)
    } else {
      Ok(WithinRelation::Disjoint)
    }
  }
}
/// create a Point2D component tree from a LatLon point
pub fn create_from_point(point: &Point) -> Result<Point2D> {
  let q_lat = if point.get_lat() == GeoUtils::MAX_LAT_INCL {
    point.get_lat()
  } else {
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude_ceil(point.get_lat())?)
  };
  let q_lon = if point.get_lon() == GeoUtils::MAX_LON_INCL {
    point.get_lon()
  } else {
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude_ceil(point.get_lon())?)
  };
  Ok(Point2D::new(q_lon, q_lat))
}

/// create a Point2D component tree from a XY point
pub fn create_from_xy_point(xy_point: &XYPoint) -> Point2D {
  Point2D::new(xy_point.get_x() as f64, xy_point.get_y() as f64)
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::core::geo::geo_test_util::GeoTestUtil;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
  use rand::RngExt;
  #[allow(dead_code)] // for quick search
  struct TestPoint2D;
  #[test]
  fn test_triangle_disjoint() -> Result<()> {
    let mut random = random();
    let point2d = create_from_point(&Point::new(0.0, 0.0)?)?;
    let ax = 4.0;
    let ay = 4.0;
    let bx = 5.0;
    let by = 5.0;
    let cx = 5.0;
    let cy = 4.0;
    assert!(!point2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!point2d.intersects_line_values(ax, ay, bx, by));
    assert!(!point2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!point2d.contains_line_values(ax, ay, bx, by));
    assert_eq!(
      WithinRelation::Disjoint,
      point2d.within_triangle_values(
        ax,
        ay,
        random.random_bool(0.5),
        bx,
        by,
        random.random_bool(0.5),
        cx,
        cy,
        random.random_bool(0.5),
      )?
    );
    Ok(())
  }
  #[test]
  fn test_triangle_intersects() -> Result<()> {
    let mut random = random();
    let point2d = create_from_point(&Point::new(0.0, 0.0)?)?;
    let ax = 0.0;
    let ay = 0.0;
    let bx = 1.0;
    let by = 0.0;
    let cx = 0.0;
    let cy = 1.0;
    assert!(point2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(point2d.intersects_line_values(ax, ay, bx, by));
    assert!(!point2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!point2d.contains_line_values(ax, ay, bx, by));
    assert_eq!(
      WithinRelation::Candidate,
      point2d.within_triangle_values(
        ax,
        ay,
        random.random_bool(0.5),
        bx,
        by,
        random.random_bool(0.5),
        cx,
        cy,
        random.random_bool(0.5),
      )?
    );
    Ok(())
  }

  #[test]
  fn test_triangle_contains() -> Result<()> {
    let mut random = random();
    let point2d = create_from_point(&Point::new(0.0, 0.0)?)?;
    let ax = 0.0;
    let ay = 0.0;
    assert!(point2d.contains(ax, ay));
    assert_eq!(
      WithinRelation::Candidate,
      point2d.within_triangle_values(
        ax,
        ay,
        random.random_bool(0.5),
        ax,
        ay,
        random.random_bool(0.5),
        ax,
        ay,
        random.random_bool(0.5),
      )?
    );
    Ok(())
  }

  #[test]
  fn test_random_triangles() -> Result<()> {
    let mut random = random();
    let point2d = create_from_point(&Point::new(
      GeoTestUtil::next_latitude(&mut random),
      GeoTestUtil::next_longitude(&mut random),
    )?)?;

    for _ in 0..100 {
      let ax = GeoTestUtil::next_longitude(&mut random);
      let ay = GeoTestUtil::next_latitude(&mut random);
      let bx = GeoTestUtil::next_longitude(&mut random);
      let by = GeoTestUtil::next_latitude(&mut random);
      let cx = GeoTestUtil::next_longitude(&mut random);
      let cy = GeoTestUtil::next_latitude(&mut random);

      let t_min_x = ax.min(bx).min(cx);
      let t_max_x = ax.max(bx).max(cx);
      let t_min_y = ay.min(by).min(cy);
      let t_max_y = ay.max(by).max(cy);

      let r = point2d.relate(t_min_x, t_max_x, t_min_y, t_max_y)?;
      if r == Relation::CellOutsideQuery {
        assert!(!point2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
        assert!(!point2d.intersects_line_values(ax, ay, bx, by));
        assert!(!point2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
        assert!(!point2d.contains_line_values(ax, ay, bx, by));
        assert_eq!(
          WithinRelation::Disjoint,
          point2d.within_triangle_values(
            ax,
            ay,
            random.random_bool(0.5),
            bx,
            by,
            random.random_bool(0.5),
            cx,
            cy,
            random.random_bool(0.5),
          )?
        );
      }
    }

    Ok(())
  }
}
