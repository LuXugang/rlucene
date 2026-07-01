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
use crate::core::geo::rectangle2d::create_from_xy_rectangle;
use crate::core::geo::xy_rectangle::XYRectangle;
use crate::core::index::point_values::Relation::{CellInsideQuery, CellOutsideQuery};
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::geo::ShapeTestUtil;
use crate::test::support::core::util::lucene_test_case::random;
use rand::RngExt;
#[allow(dead_code)] // for quick search
struct TestRectangle2D;

#[test]
fn test_triangle_disjoint() -> Result<()> {
  let rectangle = XYRectangle::new(0f32, 1f32, 0f32, 1f32)?;
  let rectangle_2d = create_from_xy_rectangle(&rectangle);
  let ax = 4f64;
  let ay = 4f64;
  let bx = 5f64;
  let by = 5f64;
  let cx = 5f64;
  let cy = 4f64;
  assert!(!rectangle_2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
  assert!(!rectangle_2d.intersects_line_values(ax, ay, bx, by));
  assert!(!rectangle_2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
  assert!(!rectangle_2d.contains_line_values(ax, ay, bx, by));
  let mut rng = random();
  assert_eq!(
    WithinRelation::Disjoint,
    rectangle_2d.within_triangle_values(
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
  let rectangle = XYRectangle::new(0f32, 1f32, 0f32, 1f32)?;
  let rectangle_2d = create_from_xy_rectangle(&rectangle);
  let ax = 0.5f64;
  let ay = 0.5f64;
  let bx = 2f64;
  let by = 2f64;
  let cx = 0.5f64;
  let cy = 2f64;
  assert!(rectangle_2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
  assert!(rectangle_2d.intersects_line_values(ax, ay, bx, by));
  assert!(!rectangle_2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
  assert!(!rectangle_2d.contains_line_values(ax, ay, bx, by));
  assert_eq!(
    WithinRelation::NotWithin,
    rectangle_2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
  );
  Ok(())
}

#[test]
fn test_triangle_contains() -> Result<()> {
  let rectangle = XYRectangle::new(0f32, 1f32, 0f32, 1f32)?;
  let rectangle_2d = create_from_xy_rectangle(&rectangle);
  let ax = 0.25f64;
  let ay = 0.25f64;
  let bx = 0.5f64;
  let by = 0.5f64;
  let cx = 0.5f64;
  let cy = 0.25f64;
  assert!(rectangle_2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
  assert!(rectangle_2d.intersects_line_values(ax, ay, bx, by));
  assert!(rectangle_2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
  assert!(rectangle_2d.contains_line_values(ax, ay, bx, by));
  assert_eq!(
    WithinRelation::NotWithin,
    rectangle_2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
  );
  Ok(())
}

#[test]
fn test_random_triangles() -> Result<()> {
  let mut rng = random();
  let rectangle = ShapeTestUtil::next_box(&mut rng)?;
  let rectangle_2d = create_from_xy_rectangle(&rectangle);
  for _ in 0..100 {
    let ax = ShapeTestUtil::next_float(&mut rng) as f64;
    let ay = ShapeTestUtil::next_float(&mut rng) as f64;
    let bx = ShapeTestUtil::next_float(&mut rng) as f64;
    let by = ShapeTestUtil::next_float(&mut rng) as f64;
    let cx = ShapeTestUtil::next_float(&mut rng) as f64;
    let cy = ShapeTestUtil::next_float(&mut rng) as f64;

    let t_min_x = ax.min(bx).min(cx);
    let t_max_x = ax.max(bx).max(cx);
    let t_min_y = ay.min(by).min(cy);
    let t_max_y = ay.max(by).max(cy);

    let r = rectangle_2d.relate(t_min_x, t_max_x, t_min_y, t_max_y)?;
    if r == CellOutsideQuery {
      assert!(!rectangle_2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
      assert!(!rectangle_2d.intersects_line_values(ax, ay, bx, by));
      assert!(!rectangle_2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
      assert!(!rectangle_2d.contains_line_values(ax, ay, bx, by));
      assert_eq!(
        WithinRelation::Disjoint,
        rectangle_2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
      );
    } else if r == CellInsideQuery {
      assert!(rectangle_2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
      assert!(rectangle_2d.intersects_line_values(ax, ay, bx, by));
      assert!(rectangle_2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
      assert!(rectangle_2d.contains_line_values(ax, ay, bx, by));
    }
  }
  Ok(())
}

#[test]
fn test_equals_and_hash_code() -> Result<()> {
  let mut rng = random();
  let xy_rectangle = ShapeTestUtil::next_box(&mut rng)?;
  let rectangle_2d = create_from_xy_rectangle(&xy_rectangle);

  let copy = create_from_xy_rectangle(&xy_rectangle);
  assert_eq!(rectangle_2d, copy);

  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let mut h1 = DefaultHasher::new();
  rectangle_2d.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  copy.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());

  let other_xy_rectangle = ShapeTestUtil::next_box(&mut rng)?;
  let other_rectangle_2d = create_from_xy_rectangle(&other_xy_rectangle);

  if rectangle_2d.get_min_x().to_bits() != other_rectangle_2d.get_min_x().to_bits()
    || rectangle_2d.get_max_x().to_bits() != other_rectangle_2d.get_max_x().to_bits()
    || rectangle_2d.get_min_y().to_bits() != other_rectangle_2d.get_min_y().to_bits()
    || rectangle_2d.get_max_y().to_bits() != other_rectangle_2d.get_max_y().to_bits()
  {
    assert_ne!(rectangle_2d, other_rectangle_2d);

    let mut h1 = DefaultHasher::new();
    rectangle_2d.hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    other_rectangle_2d.hash(&mut h2);
    assert_ne!(h1.finish(), h2.finish());
  } else {
    assert_eq!(rectangle_2d, other_rectangle_2d);

    let mut h1 = DefaultHasher::new();
    rectangle_2d.hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    other_rectangle_2d.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
  }

  Ok(())
}
