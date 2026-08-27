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

use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;

pub struct SmallFloat;
impl SmallFloat {
  /// Converts an `f32` to an 8-bit floating-point representation.
  ///
  /// Values less than zero are all mapped to zero.
  /// Values are truncated (rounded down) to the nearest 8-bit value.
  /// Values between zero and the smallest representable value are rounded up.
  ///
  /// # Arguments
  ///
  /// * `f` - The `f32` value to convert to an 8-bit representation (`u8`).
  /// * `num_mantissa_bits` - The number of mantissa bits to use in the byte,
  ///   with the remainder to be used in the exponent
  /// * `zero_exp` - The zero-point in the range of exponent values
  ///
  /// # Returns
  ///
  /// The 8-bit float representation
  pub fn float_to_byte(f: f32, num_mantissa_bits: i32, zero_exp: i32) -> u8 {
    // Adjustment from a float zero exponent to our zero exponent,
    // shifted over to our exponent position.
    let fzero = (63 - zero_exp) << num_mantissa_bits;
    let bits = f.to_bits() as i32;
    let smallfloat = bits >> (24 - num_mantissa_bits);
    if smallfloat <= fzero {
      if bits <= 0 {
        0 // negative numbers and zero both map to 0 byte
      } else {
        1 // underflow is mapped to smallest non-zero number.
      }
    } else if smallfloat >= fzero + 0x100 {
      255 // overflow maps to largest number
    } else {
      (smallfloat - fzero) as u8
    }
  }
  /// Converts an 8-bit floating-point representation to `f32`.
  pub fn byte_to_float(b: u8, num_mantissa_bits: i32, zero_exp: i32) -> f32 {
    // Prebuilding a decoding array and using a
    // lookup is only a little bit faster (anywhere from 0% to 7%)
    if b == 0 {
      return 0.0f32;
    }
    let mut bits = (b as i32) << (24 - num_mantissa_bits);
    bits += (63 - zero_exp) << 24;
    f32::from_bits(bits as u32)
  }

  /// float_to_byte(f, mantissa_bits=3, zero_exponent=15)
  /// smallest non-zero value = 5.820766E-10
  /// largest value = 7.5161928E9
  /// epsilon = 0.125
  pub fn float_to_byte_3_15(f: f32) -> u8 {
    let bits = f.to_bits() as i32;
    let smallfloat = bits >> (24 - 3);
    if smallfloat <= ((63 - 15) << 3) {
      return if bits <= 0 { 0 } else { 1 };
    }
    if smallfloat >= ((63 - 15) << 3) + 0x100 {
      return 255;
    }
    (smallfloat - ((63 - 15) << 3)) as u8
  }
  /// byte_to_float(b, mantissa_bits=3, zero_exponent=15)
  pub fn byte_3_15_to_float(b: u8) -> f32 {
    if b == 0 {
      return 0.0;
    }
    let mut bits = (b as i32) << (24 - 3);
    bits += (63 - 15) << 24;
    f32::from_bits(bits as u32)
  }

  /// Float-like encoding for positive longs that preserves ordering and 4
  /// significant bits.
  pub fn long_to_int4(i: i64) -> Result<i32> {
    if i < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "Only supports positive values, got {i}"
      )));
    }
    let num_bits = 64 - i.leading_zeros();
    if num_bits < 4 {
      let v = i.try_convert()?;
      Ok(v)
    } else {
      // normal value
      let shift = num_bits as i32 - 4;
      let mut encoded = ((i as u64 >> shift) as i64).try_convert()?;
      // only keep the 5 most significant bits
      encoded &= 0x07;
      // encode the shift, adding 1 because 0 is reserved for subnormal
      // values
      encoded |= (shift + 1) << 3;
      Ok(encoded)
    }
  }

  /// Decode values encoded with `long_to_int4`.
  pub fn int4_to_long(i: i32) -> i64 {
    let bits = (i & 0x07) as i64;
    let shift = (i as u32 >> 3) as i32 - 1;
    if shift == -1 {
      // subnormal value
      bits
    } else {
      // normal value
      (bits | 0x08) << shift
    }
  }

  /// Encode an integer to a byte using long_to_int4.
  /// Values less than NUM_FREE_VALUES are encoded directly.
  pub fn int_to_byte4(i: i32) -> Result<u8> {
    if i < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "Only supports positive values, got {i}"
      )));
    }
    if i < *NUM_FREE_VALUES {
      Ok(i as u8)
    } else {
      Ok((*NUM_FREE_VALUES + Self::long_to_int4((i - *NUM_FREE_VALUES) as i64)?) as u8)
    }
  }

  /// Decode values that have been encoded with `int_to_byte4`.
  pub fn byte4_to_int(b: u8) -> Result<i32> {
    let i = b as i32;
    if i < *NUM_FREE_VALUES {
      Ok(i)
    } else {
      let v = (*NUM_FREE_VALUES as i64 + Self::int4_to_long(i - *NUM_FREE_VALUES)).try_convert()?;
      Ok(v)
    }
  }
}

static MAX_INT4: LazyLock<i32> = LazyLock::new(|| {
  expect_invariant!(
    SmallFloat::long_to_int4(i32::MAX as i64),
    "i32::MAX is supported by the built-in small-float encoding"
  )
});
static NUM_FREE_VALUES: LazyLock<i32> = LazyLock::new(|| 255 - *MAX_INT4);
