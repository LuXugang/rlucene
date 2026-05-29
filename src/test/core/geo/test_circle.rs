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
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::geo::geo_test_util::GeoTestUtil;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
#[allow(dead_code)] // for quick search
struct TestCircle;
#[test]
fn test_invalid_lat() {
  let err = Circle::new(134.14, 45.23, 1000.0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  let err = err.unwrap_err();
  assert!(
    err
      .to_string()
      .contains("invalid latitude 134.14; must be between -90 and 90")
  );
}

#[test]
fn test_invalid_lon() {
  let err = Circle::new(43.5, 180.5, 1000.0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  let err = err.unwrap_err();
  assert!(
    err
      .to_string()
      .contains("invalid longitude 180.5; must be between -180 and 180")
  );
}

#[test]
fn test_negative_radius() {
  let err = Circle::new(43.5, 45.23, -1000.0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  let err = err.unwrap_err();
  assert!(err.to_string().contains("radiusMeters: '-1000' is invalid"));
}

#[test]
fn test_infinite_radius() {
  let err = Circle::new(43.5, 45.23, f64::INFINITY);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  let err = err.unwrap_err();
  assert!(err.to_string().contains("radiusMeters: 'inf' is invalid"));
}

#[test]
fn test_equals_and_hash_code() -> Result<()> {
  let mut rng = random();
  let circle = GeoTestUtil::next_circle(&mut rng)?;
  let copy = Circle::new(circle.get_lat(), circle.get_lon(), circle.get_radius())?;
  assert_eq!(circle, copy);

  let mut h1 = DefaultHasher::new();
  circle.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  copy.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());

  let other_circle = GeoTestUtil::next_circle(&mut rng)?;
  if circle.get_lon().to_bits() != other_circle.get_lon().to_bits()
    || circle.get_lat().to_bits() != other_circle.get_lat().to_bits()
    || circle.get_radius().to_bits() != other_circle.get_radius().to_bits()
  {
    assert_ne!(circle, other_circle);

    let mut h1 = DefaultHasher::new();
    circle.hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    other_circle.hash(&mut h2);
    assert_ne!(h1.finish(), h2.finish());
  } else {
    assert_eq!(circle, other_circle);

    let mut h1 = DefaultHasher::new();
    circle.hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    other_circle.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
  }

  Ok(())
}
