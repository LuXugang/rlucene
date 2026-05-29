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
use crate::core::geo::component2d::{Component2D, WithinRelation};
use crate::core::geo::point::Point;
use crate::core::geo::point2d::create_from_point;
use crate::core::index::point_values::Relation;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::geo::geo_test_util::GeoTestUtil;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use rand::RngExt;
#[allow(dead_code)] // for quick search
struct TestPoint2D;

#[test]
fn test_triangle_disjoint() -> Result<()> {
  let mut rng = random();
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
      rng.random_bool(0.5),
      bx,
      by,
      rng.random_bool(0.5),
      cx,
      cy,
      rng.random_bool(0.5),
    )?
  );
  Ok(())
}

#[test]
fn test_triangle_intersects() -> Result<()> {
  let mut rng = random();
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
      rng.random_bool(0.5),
      bx,
      by,
      rng.random_bool(0.5),
      cx,
      cy,
      rng.random_bool(0.5),
    )?
  );
  Ok(())
}

#[test]
fn test_triangle_contains() -> Result<()> {
  let mut rng = random();
  let point2d = create_from_point(&Point::new(0.0, 0.0)?)?;
  let ax = 0.0;
  let ay = 0.0;
  assert!(point2d.contains(ax, ay));
  assert_eq!(
    WithinRelation::Candidate,
    point2d.within_triangle_values(
      ax,
      ay,
      rng.random_bool(0.5),
      ax,
      ay,
      rng.random_bool(0.5),
      ax,
      ay,
      rng.random_bool(0.5),
    )?
  );
  Ok(())
}

#[test]
fn test_random_triangles() -> Result<()> {
  let mut rng = random();
  let point2d = create_from_point(&Point::new(
    GeoTestUtil::next_latitude(&mut rng),
    GeoTestUtil::next_longitude(&mut rng),
  )?)?;

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
          rng.random_bool(0.5),
          bx,
          by,
          rng.random_bool(0.5),
          cx,
          cy,
          rng.random_bool(0.5),
        )?
      );
    }
  }

  Ok(())
}
