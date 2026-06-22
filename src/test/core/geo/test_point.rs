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
use crate::core::geo::point::Point;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::geo::geo_test_util::GeoTestUtil;
use crate::test::core::util::lucene_test_case::random;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
#[allow(dead_code)] // for quick search
struct TestPoint;

#[test]
fn test_invalid_lat() {
  let err = Point::new(134.14, 45.23);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(
      e.to_string()
        .contains("invalid latitude 134.14; must be between -90 and 90")
    );
  }
}

#[test]
fn test_invalid_lon() {
  let err = Point::new(43.5, 180.5);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(e) = err {
    assert!(
      e.to_string()
        .contains("invalid longitude 180.5; must be between -180 and 180")
    );
  }
}

#[test]
fn test_equals_and_hash_code() -> Result<()> {
  let mut rng = random();
  let point = GeoTestUtil::next_point(&mut rng)?;
  let copy = Point::new(point.get_lat(), point.get_lon())?;

  assert_eq!(point, copy);

  let mut h1 = DefaultHasher::new();
  point.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  copy.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());

  let other_point = GeoTestUtil::next_point(&mut rng)?;
  if point.get_lat() != other_point.get_lat() || point.get_lon() != other_point.get_lon() {
    assert_ne!(point, other_point);
  } else {
    assert_eq!(point, other_point);

    let mut h3 = DefaultHasher::new();
    other_point.hash(&mut h3);
    assert_eq!(h1.finish(), h3.finish());
  }

  Ok(())
}
