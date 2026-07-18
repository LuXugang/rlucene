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
use crate::core::geo::circle2d::{CartesianDistance, Circle2D, HaversinDistance};
use crate::core::geo::component2d::{Component2D, Component2DEnum2, WithinRelation};
use crate::core::geo::lat_lon_geometry::LatLonGeometryType;
use crate::core::geo::xy_circle::XYCircle;
use crate::core::geo::xy_geometry::XYGeometryType;
use crate::core::geo::{lat_lon_geometry, xy_geometry};
use crate::core::index::point_values::Relation::{CellInsideQuery, CellOutsideQuery};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::geo::ShapeTestUtil;
use crate::test_framework::core::geo::geo_test_util::GeoTestUtil;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::RngExt;
#[allow(dead_code)] // for quick search
struct TestCircle2D;

#[test]
fn test_triangle_disjoint() -> Result<()> {
  let mut rng = random();
  let circle_2d: Component2DEnum2<
    LatLonGeometryType<Circle2D<HaversinDistance>>,
    XYGeometryType<Circle2D<CartesianDistance>>,
  > = if rng.random_bool(0.5) {
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
  let mut rng = random();
  let circle_2d: Component2DEnum2<
    LatLonGeometryType<Circle2D<HaversinDistance>>,
    XYGeometryType<Circle2D<CartesianDistance>>,
  > = if rng.random_bool(0.5) {
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
  let mut rng = random();
  let circle_2d: Component2DEnum2<
    LatLonGeometryType<Circle2D<HaversinDistance>>,
    XYGeometryType<Circle2D<CartesianDistance>>,
  > = if rng.random_bool(0.5) {
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
  let mut rng = random();
  let circle_2d: Component2DEnum2<
    LatLonGeometryType<Circle2D<HaversinDistance>>,
    XYGeometryType<Circle2D<CartesianDistance>>,
  > = if rng.random_bool(0.5) {
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
  let mut rng = random();
  let circle_2d: Component2DEnum2<
    LatLonGeometryType<Circle2D<HaversinDistance>>,
    XYGeometryType<Circle2D<CartesianDistance>>,
  > = if rng.random_bool(0.5) {
    let circle = GeoTestUtil::next_circle(&mut rng)?;
    Component2DEnum2::A(lat_lon_geometry::create::<Circle>(&[circle])?)
  } else {
    let circle = ShapeTestUtil::next_circle(&mut rng)?;
    Component2DEnum2::B(xy_geometry::create::<XYCircle>(&[circle])?)
  };

  for _ in 0..100 {
    let ax = GeoTestUtil::next_longitude(&mut rng);
    let ay = GeoTestUtil::next_latitude(&mut rng);
    let bx = GeoTestUtil::next_longitude(&mut rng);
    let by = GeoTestUtil::next_latitude(&mut rng);
    let cx = GeoTestUtil::next_longitude(&mut rng);
    let cy = GeoTestUtil::next_latitude(&mut rng);

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
  let mut rng = random();
  let circle_2d: Component2DEnum2<
    LatLonGeometryType<Circle2D<HaversinDistance>>,
    XYGeometryType<Circle2D<CartesianDistance>>,
  > = if rng.random_bool(0.5) {
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
