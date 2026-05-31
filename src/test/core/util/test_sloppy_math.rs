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
use crate::core::util::error::lucene_error::Result;
use crate::core::util::sloppy_math::{SIN_COS_MAX_VALUE_FOR_INT_MODULO, SloppyMath, TO_METERS};
use crate::test::core::geo::geo_test_util::GeoTestUtil;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, random};
use rand::RngExt;
#[allow(dead_code)] // for quick search
struct TestSloppyMath;

const COS_DELTA: f64 = 1E-15;
const ASIN_DELTA: f64 = 1E-7;
const HAVERSIN_DELTA: f64 = 38E-2;
const REASONABLE_HAVERSIN_DELTA: f64 = 1E-5;

#[test]
fn test_cos() {
  assert!(SloppyMath::cos(f64::NAN).is_nan());
  assert!(SloppyMath::cos(f64::NEG_INFINITY).is_nan());
  assert!(SloppyMath::cos(f64::INFINITY).is_nan());
  assert!((1.0f64.cos() - SloppyMath::cos(1.0)).abs() <= COS_DELTA);
  assert!((0.0f64.cos() - SloppyMath::cos(0.0)).abs() <= COS_DELTA);
  assert!(
    ((std::f64::consts::PI / 2.0).cos() - SloppyMath::cos(std::f64::consts::PI / 2.0)).abs()
      <= COS_DELTA
  );
  assert!(
    ((-std::f64::consts::PI / 2.0).cos() - SloppyMath::cos(-std::f64::consts::PI / 2.0)).abs()
      <= COS_DELTA
  );
  assert!(
    ((std::f64::consts::PI / 4.0).cos() - SloppyMath::cos(std::f64::consts::PI / 4.0)).abs()
      <= COS_DELTA
  );
  assert!(
    ((-std::f64::consts::PI / 4.0).cos() - SloppyMath::cos(-std::f64::consts::PI / 4.0)).abs()
      <= COS_DELTA
  );
  assert!(
    (((std::f64::consts::PI * 2.0) / 3.0).cos()
      - SloppyMath::cos((std::f64::consts::PI * 2.0) / 3.0))
    .abs()
      <= COS_DELTA
  );
  assert!(
    ((-((std::f64::consts::PI * 2.0) / 3.0)).cos()
      - SloppyMath::cos(-((std::f64::consts::PI * 2.0) / 3.0)))
    .abs()
      <= COS_DELTA
  );
  assert!(
    ((std::f64::consts::PI / 6.0).cos() - SloppyMath::cos(std::f64::consts::PI / 6.0)).abs()
      <= COS_DELTA
  );
  assert!(
    ((-std::f64::consts::PI / 6.0).cos() - SloppyMath::cos(-std::f64::consts::PI / 6.0)).abs()
      <= COS_DELTA
  );

  let mut random = rand::rng();
  for _ in 0..10_000 {
    let mut d = random.random::<f64>() * SIN_COS_MAX_VALUE_FOR_INT_MODULO;
    if random.random::<bool>() {
      d = -d;
    }
    assert!((d.cos() - SloppyMath::cos(d)).abs() <= COS_DELTA, "d={d}");
  }
}

#[test]
#[allow(clippy::approx_constant)]
fn test_asin() {
  assert!(SloppyMath::asin(f64::NAN).is_nan());
  assert!(SloppyMath::asin(2.0).is_nan());
  assert!(SloppyMath::asin(-2.0).is_nan());
  assert!((-(std::f64::consts::PI / 2.0) - SloppyMath::asin(-1.0)).abs() <= ASIN_DELTA);
  assert!((-(std::f64::consts::PI / 3.0) - SloppyMath::asin(-0.8660254)).abs() <= ASIN_DELTA);
  assert!((-(std::f64::consts::PI / 4.0) - SloppyMath::asin(-0.7071068)).abs() <= ASIN_DELTA);
  assert!((-(std::f64::consts::PI / 6.0) - SloppyMath::asin(-0.5)).abs() <= ASIN_DELTA);
  assert!((0.0 - SloppyMath::asin(0.0)).abs() <= ASIN_DELTA);
  assert!(((std::f64::consts::PI / 6.0) - SloppyMath::asin(0.5)).abs() <= ASIN_DELTA);
  assert!(((std::f64::consts::PI / 4.0) - SloppyMath::asin(0.7071068)).abs() <= ASIN_DELTA);
  assert!(((std::f64::consts::PI / 3.0) - SloppyMath::asin(0.8660254)).abs() <= ASIN_DELTA);
  assert!(((std::f64::consts::PI / 2.0) - SloppyMath::asin(1.0)).abs() <= ASIN_DELTA);

  let mut random = rand::rng();
  for _ in 0..10_000 {
    let mut d = random.random::<f64>();
    if random.random::<bool>() {
      d = -d;
    }
    assert!(
      (d.asin() - SloppyMath::asin(d)).abs() <= ASIN_DELTA,
      "d={d}"
    );
    assert!(SloppyMath::asin(d) >= -std::f64::consts::FRAC_PI_2);
    assert!(SloppyMath::asin(d) <= std::f64::consts::FRAC_PI_2);
  }
}

#[test]
fn test_haversin() {
  assert!(SloppyMath::haversin_meters(1.0, 1.0, 1.0, f64::NAN).is_nan());
  assert!(SloppyMath::haversin_meters(1.0, 1.0, f64::NAN, 1.0).is_nan());
  assert!(SloppyMath::haversin_meters(1.0, f64::NAN, 1.0, 1.0).is_nan());
  assert!(SloppyMath::haversin_meters(f64::NAN, 1.0, 1.0, 1.0).is_nan());

  assert_eq!(0.0, SloppyMath::haversin_meters(0.0, 0.0, 0.0, 0.0));
  assert_eq!(0.0, SloppyMath::haversin_meters(0.0, -180.0, 0.0, -180.0));
  assert_eq!(0.0, SloppyMath::haversin_meters(0.0, -180.0, 0.0, 180.0));
  assert_eq!(0.0, SloppyMath::haversin_meters(0.0, 180.0, 0.0, 180.0));
  assert_eq!(0.0, SloppyMath::haversin_meters(90.0, 0.0, 90.0, 0.0));
  assert_eq!(0.0, SloppyMath::haversin_meters(90.0, -180.0, 90.0, -180.0));
  assert_eq!(0.0, SloppyMath::haversin_meters(90.0, -180.0, 90.0, 180.0));
  assert_eq!(0.0, SloppyMath::haversin_meters(90.0, 180.0, 90.0, 180.0));

  let half_circle = TO_METERS * std::f64::consts::PI;
  assert_eq!(
    half_circle,
    SloppyMath::haversin_meters(0.0, 0.0, 0.0, 180.0)
  );

  let mut random = rand::rng();
  let random_lat1 = 40.7143528 + (random.random_range(0..10) - 5) as f64 * 360.0;
  let random_lon1 = -74.0059731 + (random.random_range(0..10) - 5) as f64 * 360.0;
  let random_lat2 = 40.65 + (random.random_range(0..10) - 5) as f64 * 360.0;
  let random_lon2 = -73.95 + (random.random_range(0..10) - 5) as f64 * 360.0;
  assert!(
    (8_572.113_7 - SloppyMath::haversin_meters(random_lat1, random_lon1, random_lat2, random_lon2))
      .abs()
      <= 0.01
  );

  assert_eq!(
    0.0,
    SloppyMath::haversin_meters(40.7143528, -74.0059731, 40.7143528, -74.0059731)
  );
  assert!(
    (5_285.89 - SloppyMath::haversin_meters(40.7143528, -74.0059731, 40.759011, -73.9844722)).abs()
      <= 0.01
  );
  assert!(
    (462.10 - SloppyMath::haversin_meters(40.7143528, -74.0059731, 40.718266, -74.007819)).abs()
      <= 0.01
  );
  assert!(
    (1_054.98 - SloppyMath::haversin_meters(40.7143528, -74.0059731, 40.7051157, -74.0088305))
      .abs()
      <= 0.01
  );
  assert!(
    (1_258.12 - SloppyMath::haversin_meters(40.7143528, -74.0059731, 40.7247222, -74.0)).abs()
      <= 0.01
  );
  assert!(
    (2_028.52 - SloppyMath::haversin_meters(40.7143528, -74.0059731, 40.731033, -73.9962255)).abs()
      <= 0.01
  );
  assert!(
    (8_572.11 - SloppyMath::haversin_meters(40.7143528, -74.0059731, 40.65, -73.95)).abs() <= 0.01
  );
}

#[test]
fn test_haversin_sort_key() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 10000);

  for _ in 0..iters {
    let center_lat = GeoTestUtil::next_latitude(&mut random);
    let center_lon = GeoTestUtil::next_longitude(&mut random);

    let lat1 = GeoTestUtil::next_latitude(&mut random);
    let lon1 = GeoTestUtil::next_longitude(&mut random);

    let lat2 = GeoTestUtil::next_latitude(&mut random);
    let lon2 = GeoTestUtil::next_longitude(&mut random);

    let expected = f64::total_cmp(
      &SloppyMath::haversin_meters(center_lat, center_lon, lat1, lon1),
      &SloppyMath::haversin_meters(center_lat, center_lon, lat2, lon2),
    )
    .cmp(&std::cmp::Ordering::Equal) as i32;
    let expected = expected.signum();

    let actual = f64::total_cmp(
      &SloppyMath::haversin_sort_key(center_lat, center_lon, lat1, lon1),
      &SloppyMath::haversin_sort_key(center_lat, center_lon, lat2, lon2),
    )
    .cmp(&std::cmp::Ordering::Equal) as i32;
    let actual = actual.signum();

    assert_eq!(expected, actual);
    assert_eq!(
      SloppyMath::haversin_meters(center_lat, center_lon, lat1, lon1),
      SloppyMath::haversin_meters_from_sort_key(SloppyMath::haversin_sort_key(
        center_lat, center_lon, lat1, lon1
      )),
    );
    assert_eq!(
      SloppyMath::haversin_meters(center_lat, center_lon, lat2, lon2),
      SloppyMath::haversin_meters_from_sort_key(SloppyMath::haversin_sort_key(
        center_lat, center_lon, lat2, lon2
      )),
    );
  }

  Ok(())
}

#[test]
fn test_haversin_from_sort_key() {
  assert_eq!(0.0, SloppyMath::haversin_meters_from_sort_key(0.0));
}

#[test]
fn test_against_slow_version() -> Result<()> {
  let mut random = random();
  for _ in 0..100_000 {
    let lat1 = GeoTestUtil::next_latitude(&mut random);
    let lon1 = GeoTestUtil::next_longitude(&mut random);
    let lat2 = GeoTestUtil::next_latitude(&mut random);
    let lon2 = GeoTestUtil::next_longitude(&mut random);

    let expected = slow_haversin(lat1, lon1, lat2, lon2);
    let actual = SloppyMath::haversin_meters(lat1, lon1, lat2, lon2);
    assert!(
      (expected - actual).abs() <= HAVERSIN_DELTA,
      "expected={expected}, actual={actual}"
    );
  }
  Ok(())
}
#[test]
fn test_across_whole_world_steps() {
  for lat1 in (-90..=90).step_by(10) {
    for lon1 in (-180..=180).step_by(10) {
      for lat2 in (-90..=90).step_by(10) {
        for lon2 in (-180..=180).step_by(10) {
          let expected = slow_haversin(lat1 as f64, lon1 as f64, lat2 as f64, lon2 as f64);
          let actual =
            SloppyMath::haversin_meters(lat1 as f64, lon1 as f64, lat2 as f64, lon2 as f64);
          assert!(
            (expected - actual).abs() <= HAVERSIN_DELTA,
            "({lat1},{lon1}) -> ({lat2},{lon2})"
          );
        }
      }
    }
  }
}

#[test]
fn test_against_slow_version_reasonable() -> Result<()> {
  let mut random = random();
  for _ in 0..100_000 {
    let lat1 = GeoTestUtil::next_latitude(&mut random);
    let lon1 = GeoTestUtil::next_longitude(&mut random);
    let lat2 = GeoTestUtil::next_latitude(&mut random);
    let lon2 = GeoTestUtil::next_longitude(&mut random);

    let expected = SloppyMath::haversin_meters(lat1, lon1, lat2, lon2);
    if expected < 1_000_000.0 {
      let actual = slow_haversin(lat1, lon1, lat2, lon2);
      assert!(
        (expected - actual).abs() <= REASONABLE_HAVERSIN_DELTA,
        "expected={expected}, actual={actual}"
      );
    }
  }
  Ok(())
}
pub(crate) fn slow_haversin(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
  let h1 = (1.0 - (lat2.to_radians() - lat1.to_radians()).cos()) / 2.0;
  let h2 = (1.0 - (lon2.to_radians() - lon1.to_radians()).cos()) / 2.0;
  let h = h1 + lat1.to_radians().cos() * lat2.to_radians().cos() * h2;
  2.0 * TO_METERS * h.sqrt().min(1.0).asin()
}
