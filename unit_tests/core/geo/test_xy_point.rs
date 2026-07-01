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
use crate::core::geo::xy_point::XYPoint;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::support::core::geo::ShapeTestUtil;
use crate::test::support::core::util::lucene_test_case::random;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
#[allow(dead_code)] // for quick search
struct TestXYPoint;

#[test]
fn test_nan() {
  let err = XYPoint::new(f32::NAN, 45.23);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value NaN"));
  }

  let err = XYPoint::new(43.5, f32::NAN);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value NaN"));
  }
}

#[test]
fn test_positive_inf() {
  let err = XYPoint::new(f32::INFINITY, 45.23);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value inf"));
  }

  let err = XYPoint::new(43.5, f32::INFINITY);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value inf"));
  }
}

#[test]
fn test_negative_inf() {
  let err = XYPoint::new(f32::NEG_INFINITY, 45.23);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value -inf"));
  }

  let err = XYPoint::new(43.5, f32::NEG_INFINITY);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value -inf"));
  }
}

#[test]
fn test_equals_and_hash_code() -> Result<()> {
  let mut rng = random();
  let point = XYPoint::new(
    ShapeTestUtil::next_float(&mut rng),
    ShapeTestUtil::next_float(&mut rng),
  )?;
  let copy = XYPoint::new(point.get_x(), point.get_y())?;
  assert_eq!(point, copy);

  let mut h1 = DefaultHasher::new();
  point.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  copy.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());

  let other_point = XYPoint::new(
    ShapeTestUtil::next_float(&mut rng),
    ShapeTestUtil::next_float(&mut rng),
  )?;
  if point.get_x() != other_point.get_x() || point.get_y() != other_point.get_y() {
    assert_ne!(point, other_point);
  } else {
    assert_eq!(point, other_point);

    let mut h3 = DefaultHasher::new();
    other_point.hash(&mut h3);
    assert_eq!(h1.finish(), h3.finish());
  }

  Ok(())
}
