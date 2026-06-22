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
use crate::core::geo::xy_circle::XYCircle;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::geo::shape_test_util::ShapeTestUtil;
use crate::test::core::util::lucene_test_case::random;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
#[allow(dead_code)] // for quick search
struct TestXYCircle;

#[test]
fn test_nan() {
  let err = XYCircle::new(f32::NAN, 45.23, 35.5);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value NaN"));
  }

  let err = XYCircle::new(43.5, f32::NAN, 35.5);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value NaN"));
  }
}

#[test]
fn test_positive_inf() {
  let err = XYCircle::new(f32::INFINITY, 45.23, 35.5);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value inf"));
  }

  let err = XYCircle::new(43.5, f32::INFINITY, 35.5);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value inf"));
  }
}

#[test]
fn test_negative_inf() {
  let err = XYCircle::new(f32::NEG_INFINITY, 45.23, 35.5);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value -inf"));
  }

  let err = XYCircle::new(43.5, f32::NEG_INFINITY, 35.5);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value -inf"));
  }
}

#[test]
fn test_negative_radius() {
  let err = XYCircle::new(43.5, 45.23, -1000.0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(
      e.to_string()
        .contains("radius must be bigger than 0, got -1000")
    );
  }
}

#[test]
fn test_infinite_radius() {
  let err = XYCircle::new(43.5, 45.23, f32::INFINITY);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(
      e.to_string().contains("radius must be finite, got inf")
        || e
          .to_string()
          .contains("radius must be finite, got Infinity")
    );
  }
}

#[test]
fn test_equals_and_hash_code() -> Result<()> {
  let mut rng = random();
  let circle = ShapeTestUtil::next_circle(&mut rng)?;
  let copy = XYCircle::new(circle.get_x(), circle.get_y(), circle.get_radius())?;
  assert_eq!(circle, copy);

  let mut h1 = DefaultHasher::new();
  circle.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  copy.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());

  let other_circle = ShapeTestUtil::next_circle(&mut rng)?;
  let mut h3 = DefaultHasher::new();
  other_circle.hash(&mut h3);
  let hash3 = h3.finish();

  if circle.get_x() != other_circle.get_x()
    || circle.get_y() != other_circle.get_y()
    || circle.get_radius() != other_circle.get_radius()
  {
    assert_ne!(circle, other_circle);
    assert_ne!(h1.finish(), hash3);
  } else {
    assert_eq!(circle, other_circle);
    assert_eq!(h1.finish(), hash3);
  }

  Ok(())
}
