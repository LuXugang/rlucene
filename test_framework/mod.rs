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
use crate::core::util::bit_util::BitUtil;
use crate::core::util::core_helper::CoreHelper;

pub mod core;

pub(crate) fn f32_equals(expected: f32, actual: f32, delta: f32) -> bool {
  CoreHelper::compare_f32(expected, actual).is_eq() || (expected - actual).abs() <= delta
}

pub(crate) fn f64_equals(expected: f64, actual: f64, delta: f64) -> bool {
  CoreHelper::compare_f64(expected, actual).is_eq() || (expected - actual).abs() <= delta
}

pub(crate) fn array_equals_f32(expected: &[f32], actual: &[f32], delta: f32) -> bool {
  expected.len() == actual.len()
    && expected
      .iter()
      .zip(actual)
      .all(|(&expected, &actual)| f32_equals(expected, actual, delta))
}

pub(crate) fn array_equals_f64(expected: &[f64], actual: &[f64], delta: f64) -> bool {
  expected.len() == actual.len()
    && expected
      .iter()
      .zip(actual)
      .all(|(&expected, &actual)| f64_equals(expected, actual, delta))
}

pub(crate) fn ulp_f32(x: f32) -> f32 {
  let abs_bits = x.to_bits() & 0x7fff_ffff;
  let exponent = (abs_bits >> 23) as i32;

  if exponent == 0xff {
    return f32::from_bits(abs_bits);
  }
  if exponent == 0 {
    return BitUtil::F32_MIN_VALUE;
  }

  let ulp_exponent = exponent - 127 - 23;
  if ulp_exponent >= -126 {
    f32::from_bits(((ulp_exponent + 127) as u32) << 23)
  } else {
    f32::from_bits(1 << (ulp_exponent + 149))
  }
}
