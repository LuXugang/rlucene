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
use crate::core::geo::xy_polygon::XYPolygon;
use crate::core::util::CoreHelper;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::geo::ShapeTestUtil;
use crate::test_framework::core::util::lucene_test_case::random;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
#[allow(dead_code)] // for quick search
struct TestXYPolygon;

#[test]
#[ignore = "Java-only: Rust coordinate vectors cannot be null"]
fn test_polygon_null_poly_lats() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust coordinate vectors cannot be null"]
fn test_polygon_null_poly_lons() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_polygon_line() {
  let err = XYPolygon::new(vec![18.0, 18.0, 18.0], vec![-66.0, -65.0, -66.0], vec![]);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("at least 4 polygon points required"));
  }
}

#[test]
fn test_polygon_bogus() {
  let err = XYPolygon::new(
    vec![18.0, 18.0, 19.0, 19.0],
    vec![-66.0, -65.0, -65.0, -66.0, -66.0],
    vec![],
  );
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("must be equal length"));
  }
}

#[test]
fn test_polygon_not_closed() {
  let err = XYPolygon::new(
    vec![18.0, 18.0, 19.0, 19.0, 19.0],
    vec![-66.0, -65.0, -65.0, -66.0, -67.0],
    vec![],
  );
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("it must close itself"));
  }
}

#[test]
fn test_polygon_nan() {
  let err = XYPolygon::new(
    vec![18.0, 18.0, 19.0, f32::NAN, 18.0],
    vec![-66.0, -65.0, -65.0, -66.0, -66.0],
    vec![],
  );
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value NaN"));
  }
}

#[test]
fn test_polygon_positive_infinite() {
  let err = XYPolygon::new(
    vec![18.0, 18.0, 19.0, 19.0, 18.0],
    vec![-66.0, f32::INFINITY, -65.0, -66.0, -66.0],
    vec![],
  );
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value inf"));
  }
}

#[test]
fn test_polygon_negative_infinite() {
  let err = XYPolygon::new(
    vec![18.0, 18.0, 19.0, 19.0, 18.0],
    vec![-66.0, -65.0, -65.0, f32::NEG_INFINITY, -66.0],
    vec![],
  );
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value -inf"));
  }
}

#[test]
fn test_equals_and_hash_code() -> Result<()> {
  let mut rng = random();
  let polygon = ShapeTestUtil::next_polygon(&mut rng)?;
  let copy = XYPolygon::new(
    polygon.get_poly_x().to_vec(),
    polygon.get_poly_y().to_vec(),
    polygon.get_holes().to_vec(),
  )?;
  assert_eq!(polygon, copy);

  let mut h1 = DefaultHasher::new();
  polygon.hash(&mut h1);
  let hash1 = h1.finish();

  let mut h2 = DefaultHasher::new();
  copy.hash(&mut h2);
  assert_eq!(hash1, h2.finish());

  let other_polygon = ShapeTestUtil::next_polygon(&mut rng)?;
  let same = CoreHelper::array_equals_f32(polygon.get_poly_x(), other_polygon.get_poly_x())
    && CoreHelper::array_equals_f32(polygon.get_poly_y(), other_polygon.get_poly_y())
    && polygon.get_holes() == other_polygon.get_holes();

  let mut h3 = DefaultHasher::new();
  other_polygon.hash(&mut h3);
  let hash3 = h3.finish();

  if !same {
    assert_ne!(polygon, other_polygon);
    assert_ne!(hash1, hash3);
  } else {
    assert_eq!(polygon, other_polygon);
    assert_eq!(hash1, hash3);
  }

  Ok(())
}
