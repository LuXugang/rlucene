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
use std::sync::LazyLock;

pub struct SloppyMath;

const TO_METERS: f64 = 6_371_008.771_4;

const ONE_DIV_F2: f64 = 1.0 / 2.0;
const ONE_DIV_F3: f64 = 1.0 / 6.0;
const ONE_DIV_F4: f64 = 1.0 / 24.0;

const PIO2_HI: f64 = f64::from_bits(0x3FF921FB54400000);
const PIO2_LO: f64 = f64::from_bits(0x3DD0B4611A626331);
const TWOPI_HI: f64 = 4.0 * PIO2_HI;
const TWOPI_LO: f64 = 4.0 * PIO2_LO;

const SIN_COS_TABS_SIZE: usize = (1 << 11) + 1;
const SIN_COS_DELTA_HI: f64 = TWOPI_HI / ((SIN_COS_TABS_SIZE - 1) as f64);
const SIN_COS_DELTA_LO: f64 = TWOPI_LO / ((SIN_COS_TABS_SIZE - 1) as f64);
const SIN_COS_INDEXER: f64 = 1.0 / (SIN_COS_DELTA_HI + SIN_COS_DELTA_LO);

pub(crate) const SIN_COS_MAX_VALUE_FOR_INT_MODULO: f64 =
  (((i32::MAX >> 9) as f64) / SIN_COS_INDEXER) * 0.99;

const ASIN_MAX_VALUE_FOR_TABS: f64 = 0.956_304_755_963_035_4;
const ASIN_TABS_SIZE: usize = (1 << 13) + 1;
const ASIN_DELTA: f64 = ASIN_MAX_VALUE_FOR_TABS / ((ASIN_TABS_SIZE - 1) as f64);
const ASIN_INDEXER: f64 = 1.0 / ASIN_DELTA;

const ASIN_PIO2_HI: f64 = f64::from_bits(0x3FF921FB54442D18);
const ASIN_PIO2_LO: f64 = f64::from_bits(0x3C91A62633145C07);
const ASIN_PS0: f64 = f64::from_bits(0x3FC5555555555555);
const ASIN_PS1: f64 = f64::from_bits(0xBFD4D61203EB6F7D);
const ASIN_PS2: f64 = f64::from_bits(0x3FC9C1550E884455);
const ASIN_PS3: f64 = f64::from_bits(0xBFA48228B5688F3B);
const ASIN_PS4: f64 = f64::from_bits(0x3F49EFE07501B288);
const ASIN_PS5: f64 = f64::from_bits(0x3F023DE10DFDF709);
const ASIN_QS1: f64 = f64::from_bits(0xC0033A271C8A2D4B);
const ASIN_QS2: f64 = f64::from_bits(0x40002AE59C598AC8);
const ASIN_QS3: f64 = f64::from_bits(0xBFE6066C1B8D0159);
const ASIN_QS4: f64 = f64::from_bits(0x3FB3B8C5B12E9282);

static SIN_TAB: LazyLock<[f64; SIN_COS_TABS_SIZE]> = LazyLock::new(|| {
  let mut sin_tab = [0.0; SIN_COS_TABS_SIZE];
  let sin_cos_pi_index = (SIN_COS_TABS_SIZE - 1) / 2;
  let sin_cos_pi_mul_2_index = 2 * sin_cos_pi_index;

  for (i, value) in sin_tab.iter_mut().enumerate() {
    let angle = (i as f64) * SIN_COS_DELTA_HI + (i as f64) * SIN_COS_DELTA_LO;
    let mut sin_angle = angle.sin();
    if i == sin_cos_pi_index || i == sin_cos_pi_mul_2_index {
      sin_angle = 0.0;
    }
    *value = sin_angle;
  }

  sin_tab
});

static COS_TAB: LazyLock<[f64; SIN_COS_TABS_SIZE]> = LazyLock::new(|| {
  let mut cos_tab = [0.0; SIN_COS_TABS_SIZE];
  let sin_cos_pi_index = (SIN_COS_TABS_SIZE - 1) / 2;
  let sin_cos_pi_mul_0_5_index = sin_cos_pi_index / 2;
  let sin_cos_pi_mul_1_5_index = 3 * sin_cos_pi_index / 2;

  for (i, value) in cos_tab.iter_mut().enumerate() {
    let angle = (i as f64) * SIN_COS_DELTA_HI + (i as f64) * SIN_COS_DELTA_LO;
    let mut cos_angle = angle.cos();
    if i == sin_cos_pi_mul_0_5_index || i == sin_cos_pi_mul_1_5_index {
      cos_angle = 0.0;
    }
    *value = cos_angle;
  }

  cos_tab
});

type AsinTables = (
  [f64; ASIN_TABS_SIZE],
  [f64; ASIN_TABS_SIZE],
  [f64; ASIN_TABS_SIZE],
  [f64; ASIN_TABS_SIZE],
  [f64; ASIN_TABS_SIZE],
);

static ASIN_TABS: LazyLock<AsinTables> = LazyLock::new(|| {
  let mut asin_tab = [0.0; ASIN_TABS_SIZE];
  let mut asin_der1_div_f1_tab = [0.0; ASIN_TABS_SIZE];
  let mut asin_der2_div_f2_tab = [0.0; ASIN_TABS_SIZE];
  let mut asin_der3_div_f3_tab = [0.0; ASIN_TABS_SIZE];
  let mut asin_der4_div_f4_tab = [0.0; ASIN_TABS_SIZE];

  for i in 0..ASIN_TABS_SIZE {
    let x = (i as f64) * ASIN_DELTA;
    asin_tab[i] = x.asin();

    let one_minus_x_sq_inv = 1.0 / (1.0 - x * x);
    let one_minus_x_sq_inv0_5 = one_minus_x_sq_inv.sqrt();
    let one_minus_x_sq_inv1_5 = one_minus_x_sq_inv0_5 * one_minus_x_sq_inv;
    let one_minus_x_sq_inv2_5 = one_minus_x_sq_inv1_5 * one_minus_x_sq_inv;
    let one_minus_x_sq_inv3_5 = one_minus_x_sq_inv2_5 * one_minus_x_sq_inv;

    asin_der1_div_f1_tab[i] = one_minus_x_sq_inv0_5;
    asin_der2_div_f2_tab[i] = (x * one_minus_x_sq_inv1_5) * ONE_DIV_F2;
    asin_der3_div_f3_tab[i] = ((1.0 + 2.0 * x * x) * one_minus_x_sq_inv2_5) * ONE_DIV_F3;
    asin_der4_div_f4_tab[i] =
      ((5.0 + 2.0 * x * (2.0 + x * (5.0 - 2.0 * x))) * one_minus_x_sq_inv3_5) * ONE_DIV_F4;
  }

  (
    asin_tab,
    asin_der1_div_f1_tab,
    asin_der2_div_f2_tab,
    asin_der3_div_f3_tab,
    asin_der4_div_f4_tab,
  )
});

impl SloppyMath {
  pub fn haversin_meters(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    Self::haversin_meters_from_sort_key(Self::haversin_sort_key(lat1, lon1, lat2, lon2))
  }

  pub fn haversin_meters_from_sort_key(sort_key: f64) -> f64 {
    let value = (sort_key * 0.5).sqrt();
    let value = if value.is_nan() {
      f64::NAN
    } else {
      value.min(1.0)
    };
    TO_METERS * 2.0 * Self::asin(value)
  }

  pub fn haversin_sort_key(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let x1 = lat1.to_radians();
    let x2 = lat2.to_radians();
    let h1 = 1.0 - Self::cos(x1 - x2);
    let h2 = 1.0 - Self::cos((lon1 - lon2).to_radians());
    let h = h1 + Self::cos(x1) * Self::cos(x2) * h2;
    f64::from_bits(h.to_bits() & 0xFFFFFFFFFFFFFFF8)
  }

  pub fn cos(mut a: f64) -> f64 {
    if !a.is_finite() {
      return a.cos();
    }
    if a < 0.0 {
      a = -a;
    }
    if a > SIN_COS_MAX_VALUE_FOR_INT_MODULO {
      return a.cos();
    }

    let mut index = (a * SIN_COS_INDEXER + 0.5) as usize;
    let delta = (a - (index as f64) * SIN_COS_DELTA_HI) - (index as f64) * SIN_COS_DELTA_LO;
    index &= SIN_COS_TABS_SIZE - 2;

    let index_cos = COS_TAB[index];
    let index_sin = SIN_TAB[index];

    index_cos
      + delta
        * (-index_sin
          + delta
            * (-index_cos * ONE_DIV_F2
              + delta * (index_sin * ONE_DIV_F3 + delta * index_cos * ONE_DIV_F4)))
  }

  pub fn asin(mut a: f64) -> f64 {
    let negate_result = if a < 0.0 {
      a = -a;
      true
    } else {
      false
    };

    if a <= ASIN_MAX_VALUE_FOR_TABS {
      let index = (a * ASIN_INDEXER + 0.5) as usize;
      let delta = a - (index as f64) * ASIN_DELTA;
      let (
        asin_tab,
        asin_der1_div_f1_tab,
        asin_der2_div_f2_tab,
        asin_der3_div_f3_tab,
        asin_der4_div_f4_tab,
      ) = &*ASIN_TABS;
      let result = asin_tab[index]
        + delta
          * (asin_der1_div_f1_tab[index]
            + delta
              * (asin_der2_div_f2_tab[index]
                + delta * (asin_der3_div_f3_tab[index] + delta * asin_der4_div_f4_tab[index])));
      if negate_result { -result } else { result }
    } else if a < 1.0 {
      let t = (1.0 - a) * 0.5;
      let p = t
        * (ASIN_PS0
          + t * (ASIN_PS1 + t * (ASIN_PS2 + t * (ASIN_PS3 + t * (ASIN_PS4 + t * ASIN_PS5)))));
      let q = 1.0 + t * (ASIN_QS1 + t * (ASIN_QS2 + t * (ASIN_QS3 + t * ASIN_QS4)));
      let s = t.sqrt();
      let z = s + s * (p / q);
      let result = ASIN_PIO2_HI - ((z + z) - ASIN_PIO2_LO);
      if negate_result { -result } else { result }
    } else if a == 1.0 {
      if negate_result {
        -std::f64::consts::FRAC_PI_2
      } else {
        std::f64::consts::FRAC_PI_2
      }
    } else {
      f64::NAN
    }
  }
}
fn slow_haversin(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
  let h1 = (1.0 - (lat2.to_radians() - lat1.to_radians()).cos()) / 2.0;
  let h2 = (1.0 - (lon2.to_radians() - lon1.to_radians()).cos()) / 2.0;
  let h = h1 + lat1.to_radians().cos() * lat2.to_radians().cos() * h2;
  2.0 * TO_METERS * h.sqrt().min(1.0).asin()
}
#[cfg(test)]
mod tests {
  use super::{SIN_COS_MAX_VALUE_FOR_INT_MODULO, SloppyMath, TO_METERS, slow_haversin};
  use crate::core::util::error::lucene_error::Result;
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
      (8_572.113_7
        - SloppyMath::haversin_meters(random_lat1, random_lon1, random_lat2, random_lon2))
      .abs()
        <= 0.01
    );

    assert_eq!(
      0.0,
      SloppyMath::haversin_meters(40.7143528, -74.0059731, 40.7143528, -74.0059731)
    );
    assert!(
      (5_285.89 - SloppyMath::haversin_meters(40.7143528, -74.0059731, 40.759011, -73.9844722))
        .abs()
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
      (2_028.52 - SloppyMath::haversin_meters(40.7143528, -74.0059731, 40.731033, -73.9962255))
        .abs()
        <= 0.01
    );
    assert!(
      (8_572.11 - SloppyMath::haversin_meters(40.7143528, -74.0059731, 40.65, -73.95)).abs()
        <= 0.01
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
}
