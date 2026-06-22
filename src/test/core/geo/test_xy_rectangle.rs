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
use crate::core::geo::component2d::Component2D;
use crate::core::geo::geometry::Geometry;
use crate::core::geo::xy_rectangle::XYRectangle;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::geo::shape_test_util::ShapeTestUtil;
use crate::test::core::util::lucene_test_case::{at_least, random};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
#[allow(dead_code)] // for quick search
struct TestXYRectangle;

#[test]
fn test_invalid_min_max_x() {
  let err = XYRectangle::new(5.0, 4.0, 3.0, 4.0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("5 > 4"));
}

#[test]
fn test_invalid_min_max_y() {
  let err = XYRectangle::new(4.0, 5.0, 5.0, 4.0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("5 > 4"));
}

#[test]
fn test_nan() {
  let err = XYRectangle::new(f32::NAN, 4.0, 3.0, 4.0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("invalid value NaN"));

  let err = XYRectangle::new(3.0, f32::NAN, 3.0, 4.0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("invalid value NaN"));

  let err = XYRectangle::new(3.0, 4.0, f32::NAN, 4.0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("invalid value NaN"));

  let err = XYRectangle::new(3.0, 4.0, 3.0, f32::NAN);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("invalid value NaN"));
}

#[test]
fn test_positive_inf() {
  let err = XYRectangle::new(3.0, f32::INFINITY, 3.0, 4.0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("invalid value inf"));

  let err = XYRectangle::new(3.0, 4.0, 3.0, f32::INFINITY);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("invalid value inf"));
}

#[test]
fn test_negative_inf() {
  let err = XYRectangle::new(f32::NEG_INFINITY, 4.0, 3.0, 4.0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("invalid value -inf"));

  let err = XYRectangle::new(3.0, 4.0, f32::NEG_INFINITY, 4.0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("invalid value -inf"));
}

#[test]
fn test_equals_and_hash_code() -> Result<()> {
  let mut rng = random();
  let rectangle = ShapeTestUtil::next_box(&mut rng)?;
  let copy = XYRectangle::new(
    rectangle.min_x,
    rectangle.max_x,
    rectangle.min_y,
    rectangle.max_y,
  )?;

  assert_eq!(rectangle, copy);
  use std::hash::{Hash, Hasher};
  let mut h1 = std::collections::hash_map::DefaultHasher::new();
  rectangle.hash(&mut h1);
  let mut h2 = std::collections::hash_map::DefaultHasher::new();
  copy.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());

  let other_rectangle = ShapeTestUtil::next_box(&mut rng)?;
  if rectangle.min_x.to_bits() != other_rectangle.min_x.to_bits()
    || rectangle.max_x.to_bits() != other_rectangle.max_x.to_bits()
    || rectangle.min_y.to_bits() != other_rectangle.min_y.to_bits()
    || rectangle.max_y.to_bits() != other_rectangle.max_y.to_bits()
  {
    assert_ne!(rectangle, other_rectangle);

    let mut h1 = std::collections::hash_map::DefaultHasher::new();
    rectangle.hash(&mut h1);
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    other_rectangle.hash(&mut h2);
    assert_ne!(h1.finish(), h2.finish());
  } else {
    assert_eq!(rectangle, other_rectangle);

    let mut h1 = std::collections::hash_map::DefaultHasher::new();
    rectangle.hash(&mut h1);
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    other_rectangle.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
  }

  Ok(())
}

#[test]
fn test_random_circle_to_bbox() -> Result<()> {
  let mut rng = random();
  let iters = at_least(&mut rng, 100);

  for _iter in 0..iters {
    let center_x = ShapeTestUtil::next_float(&mut rng);
    let center_y = ShapeTestUtil::next_float(&mut rng);

    let radius = if rng.random_bool(0.5) {
      rng.random::<f32>() * TestUtil::next_int(&mut rng, 1, 100000) as f32
    } else {
      ShapeTestUtil::next_float(&mut rng).abs()
    };

    let bbox = XYRectangle::from_point_distance(center_x, center_y, radius)?;
    let component_2d = bbox.to_component2d()?;

    let num_points_to_try = 1000;
    for _ in 0..num_points_to_try {
      let x = if rng.random_bool(0.5) {
        f32::MAX.min(center_x + radius + rng.random::<f64>() as f32) as f64
      } else {
        (-f32::MAX).max(center_x + radius - rng.random::<f64>() as f32) as f64
      };

      let y = if rng.random_bool(0.5) {
        f32::MAX.min(center_y + radius + rng.random::<f64>() as f32) as f64
      } else {
        (-f32::MAX).max(center_y + radius - rng.random::<f64>() as f32) as f64
      };

      let cartesian_says = component_2d.contains(x, y);
      let bbox_says = x >= bbox.min_x as f64
        && x <= bbox.max_x as f64
        && y >= bbox.min_y as f64
        && y <= bbox.max_y as f64;

      if cartesian_says && !bbox_says {
        unreachable!(
          "point was within the distance according to cartesian distance, but the bbox doesn't contain it; centerX={} centerY={} radius={} bbox: x={} to {} y={} to {} point: x={} y={}",
          center_x, center_y, radius, bbox.min_x, bbox.max_x, bbox.min_y, bbox.max_y, x, y
        );
      }
    }
  }

  Ok(())
}
