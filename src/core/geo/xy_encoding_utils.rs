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
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::numeric_utils::NumericUtils;

pub struct XYEncodingUtils;

impl XYEncodingUtils {
  pub const MIN_VAL_INCL: f64 = -f32::MAX as f64;
  pub const MAX_VAL_INCL: f64 = f32::MAX as f64;

  /// validates value is a number and finite
  pub fn check_val(x: f32) -> Result<f32> {
    if !x.is_finite() {
      return Err(LuceneError::illegal_argument(format!(
        "invalid value {x}; must be between {} and {}",
        Self::MIN_VAL_INCL,
        Self::MAX_VAL_INCL
      )));
    }
    Ok(x)
  }

  /**
   * Quantizes double (64 bit) values into 32 bits
   *
   * @param x cartesian value
   * @return encoded value as a 32-bit `i32`
   * @throws IllegalArgumentException if value is out of bounds
   */
  pub fn encode(x: f32) -> Result<i32> {
    Ok(NumericUtils::float_to_sortable_int(Self::check_val(x)?))
  }

  /**
   * Turns quantized value from `encode` back into a double.
   *
   * @param encoded encoded value: 32-bit quantized value.
   * @return decoded value.
   */
  pub fn decode(encoded: i32) -> f32 {
    let result = NumericUtils::sortable_int_to_float(encoded);
    debug_assert!(result >= Self::MIN_VAL_INCL as f32 && result <= Self::MAX_VAL_INCL as f32);
    result
  }

  /**
   * Turns quantized value from byte array back into a double.
   *
   * @param src byte array containing 4 bytes to decode at `offset`
   * @param offset offset into `src` to decode from.
   * @return decoded value.
   */
  pub fn decode_bytes(src: &[u8], offset: usize) -> f32 {
    Self::decode(NumericUtils::sortable_bytes_to_int(src, offset))
  }

  /**
   * Convert an array of `f32` numbers to `f64` numbers.
   *
   * @param f The input floats
   * @return Corresponding double array.
   */
  pub fn float_array_to_double_array(f: &[f32]) -> Vec<f64> {
    f.iter().map(|&x| x as f64).collect()
  }
}
