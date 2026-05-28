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

pub(crate) const TO_METERS: f64 = 6_371_008.771_4;

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
