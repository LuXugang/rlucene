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
use crate::core::geo::circle::Circle;
use crate::core::geo::component2d::{
  Component2D, WithinRelation, contains_point, disjoint, point_in_triangle, within,
};
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::geo::rectangle::Rectangle;
use crate::core::geo::xy_circle::XYCircle;
use crate::core::geo::xy_rectangle::XYRectangle;
use crate::core::index::point_values::Relation;
use crate::core::util::SloppyMath;
use crate::core::util::error::lucene_error::Result;

/// 2D circle implementation containing spatial logic
pub struct Circle2D<T>
where
  T: DistanceCalculator,
{
  calculator: T,
}
impl<T> Circle2D<T>
where
  T: DistanceCalculator,
{
  pub(crate) fn new(calculator: T) -> Self {
    Self { calculator }
  }
}
impl<T> Component2D for Circle2D<T>
where
  T: DistanceCalculator,
{
  fn get_min_x(&self) -> f64 {
    self.calculator.get_min_x()
  }

  fn get_max_x(&self) -> f64 {
    self.calculator.get_max_x()
  }

  fn get_min_y(&self) -> f64 {
    self.calculator.get_min_y()
  }

  fn get_max_y(&self) -> f64 {
    self.calculator.get_max_y()
  }

  fn contains(&self, x: f64, y: f64) -> bool {
    self.calculator.contains(x, y)
  }

  fn relate(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Result<Relation> {
    if self.calculator.disjoint(min_x, max_x, min_y, max_y) {
      return Ok(Relation::CellOutsideQuery);
    }
    if self.calculator.within(min_x, max_x, min_y, max_y) {
      return Ok(Relation::CellCrossesQuery);
    }
    self.calculator.relate(min_x, max_x, min_y, max_y)
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
    if self.calculator.disjoint(min_x, max_x, min_y, max_y) {
      return false;
    }
    self.contains(a_x, a_y)
      || self.contains(b_x, b_y)
      || self.calculator.intersects_line(a_x, a_y, b_x, b_y)
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
    if self.calculator.disjoint(min_x, max_x, min_y, max_y) {
      return false;
    }
    self.contains(a_x, a_y)
      || self.contains(b_x, b_y)
      || self.contains(c_x, c_y)
      || point_in_triangle(
        min_x,
        max_x,
        min_y,
        max_y,
        self.calculator.get_x(),
        self.calculator.get_y(),
        a_x,
        a_y,
        b_x,
        b_y,
        c_x,
        c_y,
      )
      || self.calculator.intersects_line(a_x, a_y, b_x, b_y)
      || self.calculator.intersects_line(b_x, b_y, c_x, c_y)
      || self.calculator.intersects_line(c_x, c_y, a_x, a_y)
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
    if self.calculator.disjoint(min_x, max_x, min_y, max_y) {
      return false;
    }
    self.contains(a_x, a_y) && self.contains(b_x, b_y)
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
    if self.calculator.disjoint(min_x, max_x, min_y, max_y) {
      return false;
    }
    self.contains(a_x, a_y) && self.contains(b_x, b_y) && self.contains(c_x, c_y)
  }

  fn within_point(
    &self,
    x: f64,
    y: f64,
  ) -> crate::core::util::error::lucene_error::Result<WithinRelation> {
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
  ) -> crate::core::util::error::lucene_error::Result<WithinRelation> {
    if self.calculator.disjoint(min_x, max_x, min_y, max_y) {
      return Ok(WithinRelation::Disjoint);
    }
    if self.contains(a_x, a_y) || self.contains(b_x, b_y) {
      return Ok(WithinRelation::NotWithin);
    }
    if ab && self.calculator.intersects_line(a_x, a_y, b_x, b_y) {
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
  ) -> crate::core::util::error::lucene_error::Result<WithinRelation> {
    if self.calculator.disjoint(min_x, max_x, min_y, max_y) {
      return Ok(WithinRelation::Disjoint);
    }

    if self.contains(a_x, a_y) || self.contains(b_x, b_y) || self.contains(c_x, c_y) {
      return Ok(WithinRelation::NotWithin);
    }

    if ab && self.calculator.intersects_line(a_x, a_y, b_x, b_y) {
      return Ok(WithinRelation::NotWithin);
    }
    if bc && self.calculator.intersects_line(b_x, b_y, c_x, c_y) {
      return Ok(WithinRelation::NotWithin);
    }
    if ca && self.calculator.intersects_line(c_x, c_y, a_x, a_y) {
      return Ok(WithinRelation::NotWithin);
    }

    if point_in_triangle(
      min_x,
      max_x,
      min_y,
      max_y,
      self.calculator.get_x(),
      self.calculator.get_y(),
      a_x,
      a_y,
      b_x,
      b_y,
      c_x,
      c_y,
    ) {
      return Ok(WithinRelation::Candidate);
    }
    Ok(WithinRelation::Disjoint)
  }
}

pub trait DistanceCalculator {
  /// check if the point is within a distance
  fn contains(&self, x: f64, y: f64) -> bool;

  /// check if the line is within a distance
  fn intersects_line(&self, a_x: f64, a_y: f64, b_x: f64, b_y: f64) -> bool;

  /// Relates this calculator to the provided bounding box
  fn relate(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Result<Relation>;

  /// check if the bounding box is disjoint with this calculator bounding box
  fn disjoint(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> bool;

  /// check if the bounding box is contains this calculator bounding box
  fn within(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> bool;

  /// get min X of this calculator
  fn get_min_x(&self) -> f64;

  /// get max X of this calculator
  fn get_max_x(&self) -> f64;

  /// get min Y of this calculator
  fn get_min_y(&self) -> f64;

  /// get max Y of this calculator
  fn get_max_y(&self) -> f64;

  /// get center X
  fn get_x(&self) -> f64;

  /// get center Y
  fn get_y(&self) -> f64;
}
/// Builds a XYCircle2D from XYCircle. Distance calculations are performed using cartesian distance.
pub fn create_from_xy_circle(circle: &XYCircle) -> Result<Circle2D<CartesianDistance>> {
  let calculator = CartesianDistance::new(circle.get_x(), circle.get_y(), circle.get_radius())?;
  Ok(Circle2D::new(calculator))
}
/// Builds a Circle2D from Circle. Distance calculations are performed using haversin distance.
pub fn create_from_circle(circle: &Circle) -> Result<Circle2D<HaversinDistance>> {
  let calculator = HaversinDistance::new(circle.get_lon(), circle.get_lat(), circle.get_radius())?;
  Ok(Circle2D::new(calculator))
}
pub struct CartesianDistance {
  center_x: f64,
  center_y: f64,
  radius_squared: f64,
  rectangle: XYRectangle,
}
impl CartesianDistance {
  pub(crate) fn new(center_x: f32, center_y: f32, radius: f32) -> Result<Self> {
    let rectangle = XYRectangle::from_point_distance(center_x, center_y, radius)?;
    let radius_squared = radius as f64 * radius as f64;
    Ok(Self {
      center_x: center_x as f64,
      center_y: center_y as f64,
      radius_squared,
      rectangle,
    })
  }
}
impl DistanceCalculator for CartesianDistance {
  fn contains(&self, x: f64, y: f64) -> bool {
    if contains_point(
      x,
      y,
      self.rectangle.min_x as f64,
      self.rectangle.max_x as f64,
      self.rectangle.min_y as f64,
      self.rectangle.max_y as f64,
    ) {
      let diff_x = x - self.center_x;
      let diff_y = y - self.center_y;
      return diff_x * diff_x + diff_y * diff_y <= self.radius_squared;
    }
    false
  }

  fn intersects_line(&self, a_x: f64, a_y: f64, b_x: f64, b_y: f64) -> bool {
    intersects_line(self.center_x, self.center_y, a_x, a_y, b_x, b_y, self)
  }

  fn relate(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Result<Relation> {
    if contains_point(self.center_x, self.center_y, min_x, max_x, min_y, max_y) {
      if self.contains(min_x, min_y)
        && self.contains(max_x, min_y)
        && self.contains(max_x, max_y)
        && self.contains(min_x, max_y)
      {
        // we are fully enclosed, collect everything within this subtree
        return Ok(Relation::CellInsideQuery);
      }
    } else {
      let mut sum_of_squared_diffs = 0.0f64;
      if self.center_x < min_x {
        let diff = min_x - self.center_x;
        sum_of_squared_diffs += diff * diff;
      } else if self.center_x > max_x {
        let diff = max_x - self.center_x;
        sum_of_squared_diffs += diff * diff;
      }

      if self.center_y < min_y {
        let diff = min_y - self.center_y;
        sum_of_squared_diffs += diff * diff;
      } else if self.center_y > max_y {
        let diff = max_y - self.center_y;
        sum_of_squared_diffs += diff * diff;
      }

      if sum_of_squared_diffs > self.radius_squared {
        return Ok(Relation::CellOutsideQuery);
      }
    }
    Ok(Relation::CellCrossesQuery)
  }

  fn disjoint(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> bool {
    disjoint(
      self.rectangle.min_x as f64,
      self.rectangle.max_x as f64,
      self.rectangle.min_y as f64,
      self.rectangle.max_y as f64,
      min_x,
      max_x,
      min_y,
      max_y,
    )
  }

  fn within(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> bool {
    within(
      self.rectangle.min_x as f64,
      self.rectangle.max_x as f64,
      self.rectangle.min_y as f64,
      self.rectangle.max_y as f64,
      min_x,
      max_x,
      min_y,
      max_y,
    )
  }

  fn get_min_x(&self) -> f64 {
    self.rectangle.min_x as f64
  }

  fn get_max_x(&self) -> f64 {
    self.rectangle.max_x as f64
  }

  fn get_min_y(&self) -> f64 {
    self.rectangle.min_y as f64
  }

  fn get_max_y(&self) -> f64 {
    self.rectangle.max_y as f64
  }

  fn get_x(&self) -> f64 {
    self.center_x
  }

  fn get_y(&self) -> f64 {
    self.center_y
  }
}
fn intersects_line(
  center_x: f64,
  center_y: f64,
  a_x: f64,
  a_y: f64,
  b_x: f64,
  b_y: f64,
  calculator: &impl DistanceCalculator,
) -> bool {
  let vector_apx = center_x - a_x;
  let vector_apy = center_y - a_y;

  let vector_abx = b_x - a_x;
  let vector_aby = b_y - a_y;

  let magnitude_ab = vector_abx * vector_abx + vector_aby * vector_aby;
  let dot_product = vector_apx * vector_abx + vector_apy * vector_aby;

  let distance = dot_product / magnitude_ab;

  if !(0.0..=1.0).contains(&distance) {
    return false;
  }

  let p_x = a_x + vector_abx * distance;
  let p_y = a_y + vector_aby * distance;

  let min_x = a_x.min(b_x);
  let min_y = a_y.min(b_y);
  let max_x = a_x.max(b_x);
  let max_y = a_y.max(b_y);

  if p_x >= min_x && p_x <= max_x && p_y >= min_y && p_y <= max_y {
    return calculator.contains(p_x, p_y);
  }
  false
}
pub struct HaversinDistance {
  center_lat: f64,
  center_lon: f64,
  sort_key: f64,
  axis_lat: f64,
  rectangle: Rectangle,
  crosses_dateline: bool,
}
impl HaversinDistance {
  pub fn new(center_lon: f64, center_lat: f64, radius: f64) -> Result<Self> {
    let sort_key = GeoUtils::distance_query_sort_key(radius);
    let axis_lat = Rectangle::axis_lat(center_lat, radius);
    let rectangle = Rectangle::from_point_distance(center_lat, center_lon, radius)?;
    let crosses_dateline = rectangle.min_lon > rectangle.max_lon;
    Ok(Self {
      center_lat,
      center_lon,
      sort_key,
      axis_lat,
      rectangle,
      crosses_dateline,
    })
  }
}
impl DistanceCalculator for HaversinDistance {
  fn contains(&self, x: f64, y: f64) -> bool {
    if self.crosses_dateline {
      if contains_point(
        x,
        y,
        self.rectangle.min_lon,
        GeoUtils::MAX_LON_INCL,
        self.rectangle.min_lat,
        self.rectangle.max_lat,
      ) || contains_point(
        x,
        y,
        GeoUtils::MIN_LON_INCL,
        self.rectangle.max_lon,
        self.rectangle.min_lat,
        self.rectangle.max_lat,
      ) {
        return SloppyMath::haversin_sort_key(y, x, self.center_lat, self.center_lon)
          <= self.sort_key;
      }
    } else if contains_point(
      x,
      y,
      self.rectangle.min_lon,
      self.rectangle.max_lon,
      self.rectangle.min_lat,
      self.rectangle.max_lat,
    ) {
      return SloppyMath::haversin_sort_key(y, x, self.center_lat, self.center_lon)
        <= self.sort_key;
    }
    false
  }

  fn intersects_line(&self, a_x: f64, a_y: f64, b_x: f64, b_y: f64) -> bool {
    if intersects_line(self.center_lon, self.center_lat, a_x, a_y, b_x, b_y, self) {
      return true;
    }
    if self.crosses_dateline {
      let new_center_lon = if self.center_lon > 0.0 {
        self.center_lon - 360.0
      } else {
        self.center_lon + 360.0
      };
      return intersects_line(new_center_lon, self.center_lat, a_x, a_y, b_x, b_y, self);
    }
    false
  }

  fn relate(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Result<Relation> {
    GeoUtils::relate(
      min_y,
      max_y,
      min_x,
      max_x,
      self.center_lat,
      self.center_lon,
      self.sort_key,
      self.axis_lat,
    )
  }

  fn disjoint(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> bool {
    if self.crosses_dateline {
      disjoint(
        self.rectangle.min_lon,
        GeoUtils::MAX_LON_INCL,
        self.rectangle.min_lat,
        self.rectangle.max_lat,
        min_x,
        max_x,
        min_y,
        max_y,
      ) && disjoint(
        GeoUtils::MIN_LON_INCL,
        self.rectangle.max_lon,
        self.rectangle.min_lat,
        self.rectangle.max_lat,
        min_x,
        max_x,
        min_y,
        max_y,
      )
    } else {
      disjoint(
        self.rectangle.min_lon,
        self.rectangle.max_lon,
        self.rectangle.min_lat,
        self.rectangle.max_lat,
        min_x,
        max_x,
        min_y,
        max_y,
      )
    }
  }

  fn within(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> bool {
    if self.crosses_dateline {
      within(
        self.rectangle.min_lon,
        GeoUtils::MAX_LON_INCL,
        self.rectangle.min_lat,
        self.rectangle.max_lat,
        min_x,
        max_x,
        min_y,
        max_y,
      ) || within(
        GeoUtils::MIN_LON_INCL,
        self.rectangle.max_lon,
        self.rectangle.min_lat,
        self.rectangle.max_lat,
        min_x,
        max_x,
        min_y,
        max_y,
      )
    } else {
      within(
        self.rectangle.min_lon,
        self.rectangle.max_lon,
        self.rectangle.min_lat,
        self.rectangle.max_lat,
        min_x,
        max_x,
        min_y,
        max_y,
      )
    }
  }

  fn get_min_x(&self) -> f64 {
    if self.crosses_dateline {
      GeoUtils::MIN_LON_INCL
    } else {
      self.rectangle.min_lon
    }
  }

  fn get_max_x(&self) -> f64 {
    if self.crosses_dateline {
      GeoUtils::MAX_LON_INCL
    } else {
      self.rectangle.max_lon
    }
  }

  fn get_min_y(&self) -> f64 {
    self.rectangle.min_lat
  }

  fn get_max_y(&self) -> f64 {
    self.rectangle.max_lat
  }

  fn get_x(&self) -> f64 {
    self.center_lon
  }

  fn get_y(&self) -> f64 {
    self.center_lat
  }
}

#[cfg(test)]
mod tests {
  use crate::core::geo::circle::Circle;
  use crate::core::geo::circle2d::{CartesianDistance, Circle2D, HaversinDistance};
  use crate::core::geo::component2d::{Component2D, Component2DEnum2, WithinRelation};
  use crate::core::geo::lat_lon_geometry::LatLonGeometryType;
  use crate::core::geo::xy_circle::XYCircle;
  use crate::core::geo::xy_geometry::XYGeometryType;
  use crate::core::geo::{lat_lon_geometry, xy_geometry};
  use crate::core::index::point_values::Relation::{CellInsideQuery, CellOutsideQuery};
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::geo::geo_test_util::GeoTestUtil;
  use crate::test::core::geo::shape_test_util::ShapeTestUtil;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
  use rand::RngExt;

  #[allow(dead_code)] // for quick search
  struct TestCircle2D;
  #[test]
  fn test_triangle_disjoint() -> Result<()> {
    let mut random = random();
    let circle_2d: Component2DEnum2<
      LatLonGeometryType<Circle2D<HaversinDistance>>,
      XYGeometryType<Circle2D<CartesianDistance>>,
    > = if random.random_bool(0.5) {
      let circle = Circle::new(0.0, 0.0, 100.0)?;
      Component2DEnum2::A(lat_lon_geometry::create::<Circle>(&[circle])?)
    } else {
      let xy_circle = XYCircle::new(0f32, 0f32, 1f32)?;
      Component2DEnum2::B(xy_geometry::create::<XYCircle>(&[xy_circle])?)
    };
    let ax = 4f64;
    let ay = 4f64;
    let bx = 5f64;
    let by = 5f64;
    let cx = 5f64;
    let cy = 4f64;
    assert!(!circle_2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!circle_2d.intersects_line_values(ax, ay, bx, by));
    assert!(!circle_2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!circle_2d.contains_line_values(ax, ay, bx, by));
    assert_eq!(
      WithinRelation::Disjoint,
      circle_2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
    );
    Ok(())
  }
  #[test]
  fn test_triangle_intersects() -> Result<()> {
    let mut random = random();
    let circle_2d: Component2DEnum2<
      LatLonGeometryType<Circle2D<HaversinDistance>>,
      XYGeometryType<Circle2D<CartesianDistance>>,
    > = if random.random_bool(0.5) {
      let circle = Circle::new(0.0, 0.0, 1_000_000.0)?;
      Component2DEnum2::A(lat_lon_geometry::create::<Circle>(&[circle])?)
    } else {
      let xy_circle = XYCircle::new(0f32, 0f32, 10f32)?;
      Component2DEnum2::B(xy_geometry::create::<XYCircle>(&[xy_circle])?)
    };
    let ax = -20f64;
    let ay = 1f64;
    let bx = 20f64;
    let by = 1f64;
    let cx = 0f64;
    let cy = 90f64;
    assert!(circle_2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(circle_2d.intersects_line_values(ax, ay, bx, by));
    assert!(!circle_2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!circle_2d.contains_line_values(ax, ay, bx, by));
    assert_eq!(
      WithinRelation::NotWithin,
      circle_2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
    );
    Ok(())
  }

  #[test]
  fn test_triangle_date_line_intersects() -> Result<()> {
    let circle_2d = lat_lon_geometry::create::<Circle>(&[Circle::new(0.0, 179.0, 222400.0)?])?;
    let ax = -179f64;
    let ay = 1f64;
    let bx = -179f64;
    let by = -1f64;
    let cx = -178f64;
    let cy = 0f64;
    assert!(circle_2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(circle_2d.intersects_line_values(ax, ay, bx, by));
    assert!(!circle_2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!circle_2d.contains_line_values(ax, ay, bx, by));
    assert_eq!(
      WithinRelation::NotWithin,
      circle_2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
    );
    Ok(())
  }

  #[test]
  fn test_triangle_contains() -> Result<()> {
    let mut random = random();
    let circle_2d: Component2DEnum2<
      LatLonGeometryType<Circle2D<HaversinDistance>>,
      XYGeometryType<Circle2D<CartesianDistance>>,
    > = if random.random_bool(0.5) {
      let circle = Circle::new(0.0, 0.0, 1_000_000.0)?;
      Component2DEnum2::A(lat_lon_geometry::create::<Circle>(&[circle])?)
    } else {
      let xy_circle = XYCircle::new(0f32, 0f32, 1f32)?;
      Component2DEnum2::B(xy_geometry::create::<XYCircle>(&[xy_circle])?)
    };
    let ax = 0.25f64;
    let ay = 0.25f64;
    let bx = 0.5f64;
    let by = 0.5f64;
    let cx = 0.5f64;
    let cy = 0.25f64;
    assert!(circle_2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(circle_2d.intersects_line_values(ax, ay, bx, by));
    assert!(circle_2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(circle_2d.contains_line_values(ax, ay, bx, by));
    assert_eq!(
      WithinRelation::NotWithin,
      circle_2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
    );
    Ok(())
  }

  #[test]
  fn test_triangle_within() -> Result<()> {
    let mut random = random();
    let circle_2d: Component2DEnum2<
      LatLonGeometryType<Circle2D<HaversinDistance>>,
      XYGeometryType<Circle2D<CartesianDistance>>,
    > = if random.random_bool(0.5) {
      let circle = Circle::new(0.0, 0.0, 1000.0)?;
      Component2DEnum2::A(lat_lon_geometry::create::<Circle>(&[circle])?)
    } else {
      let xy_circle = XYCircle::new(0f32, 0f32, 1f32)?;
      Component2DEnum2::B(xy_geometry::create::<XYCircle>(&[xy_circle])?)
    };

    let ax = -20f64;
    let ay = -20f64;
    let bx = 20f64;
    let by = -20f64;
    let cx = 0f64;
    let cy = 20f64;
    assert!(circle_2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!circle_2d.intersects_line_values(bx, by, cx, cy));
    assert!(!circle_2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!circle_2d.contains_line_values(bx, by, cx, cy));
    assert_eq!(
      WithinRelation::Candidate,
      circle_2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
    );
    Ok(())
  }

  #[test]
  fn test_random_triangles() -> Result<()> {
    let mut random = random();
    let circle_2d: Component2DEnum2<
      LatLonGeometryType<Circle2D<HaversinDistance>>,
      XYGeometryType<Circle2D<CartesianDistance>>,
    > = if random.random_bool(0.5) {
      let circle = GeoTestUtil::next_circle(&mut random)?;
      Component2DEnum2::A(lat_lon_geometry::create::<Circle>(&[circle])?)
    } else {
      let circle = ShapeTestUtil::next_circle(&mut random)?;
      Component2DEnum2::B(xy_geometry::create::<XYCircle>(&[circle])?)
    };

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

      let r = circle_2d.relate(t_min_x, t_max_x, t_min_y, t_max_y)?;
      if r == CellOutsideQuery {
        assert!(!circle_2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
        assert!(!circle_2d.intersects_line_values(ax, ay, bx, by));
        assert!(!circle_2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
        assert!(!circle_2d.contains_line_values(ax, ay, bx, by));
        assert_eq!(
          WithinRelation::Disjoint,
          circle_2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
        );
      } else if r == CellInsideQuery {
        assert!(circle_2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
        assert!(circle_2d.intersects_line_values(ax, ay, bx, by));
        assert!(circle_2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
        assert!(circle_2d.contains_line_values(ax, ay, bx, by));
        assert_ne!(
          WithinRelation::Candidate,
          circle_2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
        );
      }
    }

    Ok(())
  }

  #[test]
  fn test_line_intersects() -> Result<()> {
    let mut random = random();
    let circle_2d: Component2DEnum2<
      LatLonGeometryType<Circle2D<HaversinDistance>>,
      XYGeometryType<Circle2D<CartesianDistance>>,
    > = if random.random_bool(0.5) {
      let circle = Circle::new(0.0, 0.0, 35000.0)?;
      Component2DEnum2::A(lat_lon_geometry::create::<Circle>(&[circle])?)
    } else {
      let xy_circle = XYCircle::new(0f32, 0f32, 0.3f32)?;
      Component2DEnum2::B(xy_geometry::create::<XYCircle>(&[xy_circle])?)
    };

    let ax = -0.25f64;
    let ay = 0.25f64;
    let bx = 0.25f64;
    let by = 0.25f64;
    let cx = 0.2f64;
    let cy = 0.25f64;
    assert!(circle_2d.intersects_line_values(ax, ay, bx, by));
    assert!(!circle_2d.intersects_line_values(bx, by, cx, cy));
    assert!(!circle_2d.intersects_line_values(cx, cy, bx, by));
    Ok(())
  }
}
