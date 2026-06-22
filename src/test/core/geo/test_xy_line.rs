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
use crate::core::geo::xy_line::XYLine;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::geo::shape_test_util::ShapeTestUtil;
use crate::test::core::util::lucene_test_case::random;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
#[allow(dead_code)] // for quick search
struct TestXYLine;

#[test]
fn test_line_null_xs() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_polygon_null_ys() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_line_enough_points() {
  let err = XYLine::new(vec![18.0], vec![-66.0]);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("at least 2 line points required"));
  }
}

#[test]
fn test_lines_bogus() {
  let err = XYLine::new(
    vec![18.0, 18.0, 19.0, 19.0],
    vec![-66.0, -65.0, -65.0, -66.0, -66.0],
  );
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("must be equal length"));
  }
}

#[test]
fn test_line_nan() {
  let err = XYLine::new(
    vec![18.0, 18.0, 19.0, f32::NAN, 18.0],
    vec![-66.0, -65.0, -65.0, -66.0, -66.0],
  );
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(e.to_string().contains("invalid value NaN"));
  }
}

#[test]
fn test_line_positive_infinite() {
  let err = XYLine::new(
    vec![18.0, 18.0, 19.0, 19.0, 18.0],
    vec![-66.0, f32::INFINITY, -65.0, -66.0, -66.0],
  );
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(
      e.to_string().contains("invalid value inf") || e.to_string().contains("invalid value Inf")
    );
  }
}

#[test]
fn test_line_negative_infinite() {
  let err = XYLine::new(
    vec![18.0, 18.0, 19.0, 19.0, 18.0],
    vec![-66.0, -65.0, -65.0, f32::NEG_INFINITY, -66.0],
  );
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(
      e.to_string().contains("invalid value -inf") || e.to_string().contains("invalid value -Inf")
    );
  }
}

#[test]
fn test_equals_and_hash_code() -> Result<()> {
  let mut rng = random();
  let line = ShapeTestUtil::next_line(&mut rng)?;
  let copy = XYLine::new(line.get_xs().to_vec(), line.get_ys().to_vec())?;
  assert_eq!(line, copy);

  let mut h1 = DefaultHasher::new();
  line.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  copy.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());

  let other_line = ShapeTestUtil::next_line(&mut rng)?;
  if line.get_xs() != other_line.get_xs() || line.get_ys() != other_line.get_ys() {
    assert_ne!(line, other_line);

    let mut h3 = DefaultHasher::new();
    other_line.hash(&mut h3);
    assert_ne!(h1.finish(), h3.finish());
  } else {
    assert_eq!(line, other_line);

    let mut h3 = DefaultHasher::new();
    other_line.hash(&mut h3);
    assert_eq!(h1.finish(), h3.finish());
  }

  Ok(())
}
