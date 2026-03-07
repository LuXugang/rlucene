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
use once_cell::sync::Lazy;

use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;

pub struct SmallFloat;
impl SmallFloat {
    /// Converts a 32-bit float to an 8-bit float.
    ///
    /// Values less than zero are all mapped to zero.
    /// Values are truncated (rounded down) to the nearest 8-bit value.
    /// Values between zero and the smallest representable value are rounded up.
    ///
    /// # Arguments
    ///
    /// * `f` - The 32-bit float to be converted to an 8-bit float (u8)
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
    /// Converts an 8-bit float to a 32-bit float.
    pub fn byte_to_float(b: u8, num_mantissa_bits: i32, zero_exp: i32) -> f32 {
        // on Java1.5 & 1.6 JVMs, prebuilding a decoding array and doing a
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
            let v = (*NUM_FREE_VALUES as i64 + Self::int4_to_long(i - *NUM_FREE_VALUES))
                .try_convert()?;
            Ok(v)
        }
    }
}

static MAX_INT4: Lazy<i32> =
    Lazy::new(|| SmallFloat::long_to_int4(i32::MAX as i64).expect("should not fail"));
static NUM_FREE_VALUES: Lazy<i32> = Lazy::new(|| 255 - *MAX_INT4);

#[cfg(test)]
mod tests {
    use crate::core::util::error::lucene_error::Result;
    use crate::core::util::small_float::SmallFloat;

    #[allow(dead_code)] // for quick search
    struct TestSmallFloat;
    /// original lucene byte_to_float
    pub fn orig_byte_to_float(b: u8) -> f32 {
        if b == 0 {
            return 0.0;
        }
        let mantissa = (b & 7) as i32;
        let exponent = ((b >> 3) & 31) as i32;
        let bits = ((exponent + (63 - 15)) << 24) | ((mantissa) << 21);
        f32::from_bits(bits as u32)
    }

    /// original lucene float_to_byte (since lucene 1.3)
    pub fn orig_float_to_byte_v13(mut f: f32) -> u8 {
        if f < 0.0 {
            f = 0.0;
        }
        if f == 0.0 {
            return 0;
        }

        let bits = f.to_bits() as i32;
        let mut mantissa = (bits & 0x00ffffff) >> 21;
        let mut exponent = (((bits >> 24) & 0x7f) - 63) + 15;

        if exponent > 31 {
            exponent = 31;
            mantissa = 7;
        }

        if exponent < 0 {
            exponent = 0;
            mantissa = 1;
        }

        ((exponent << 3) | mantissa) as u8
    }

    /// original lucene float_to_byte with underflow bug fixed
    pub fn orig_float_to_byte(mut f: f32) -> u8 {
        if f < 0.0 {
            f = 0.0;
        }
        if f == 0.0 {
            return 0;
        }

        let bits = f.to_bits() as i32;
        let mut mantissa = (bits & 0x00ffffff) >> 21;
        let mut exponent = (((bits >> 24) & 0x7f) - 63) + 15;

        if exponent > 31 {
            exponent = 31;
            mantissa = 7;
        }

        if exponent < 0 || (exponent == 0 && mantissa == 0) {
            exponent = 0;
            mantissa = 1;
        }

        ((exponent << 3) | mantissa) as u8
    }
    #[test]
    fn test_byte_to_float() {
        for i in 0u8..=255 {
            let f1 = orig_byte_to_float(i);
            let f2 = SmallFloat::byte_to_float(i, 3, 15);
            let f3 = SmallFloat::byte_3_15_to_float(i);
            assert!(
                (f1 - f2).abs() <= 0.0,
                "f1 = {}, f2 = {} for i = {}",
                f1,
                f2,
                i
            );
            assert!(
                (f2 - f3).abs() <= 0.0,
                "f2 = {}, f3 = {} for i = {}",
                f2,
                f3,
                i
            );
        }
    }
    use rand::RngExt;

    use crate::test::util::lucene_test_case::lucene_test_case_util::{at_least, random};
    use crate::test::util::test_util::TestUtil;

    #[test]
    fn test_float_to_byte() {
        let mut random = random();
        assert_eq!(orig_float_to_byte_v13(5.8123817e-10f32), 0);
        assert_eq!(orig_float_to_byte(5.8123817e-10f32), 1);
        assert_eq!(SmallFloat::float_to_byte_3_15(5.8123817e-10f32), 1);

        // test some constants
        assert_eq!(SmallFloat::float_to_byte_3_15(0.0), 0);
        assert_eq!(SmallFloat::float_to_byte_3_15(f32::MIN_POSITIVE), 1); // underflow rounds up
        assert_eq!(SmallFloat::float_to_byte_3_15(f32::MAX), 255); // overflow rounds down
        assert_eq!(SmallFloat::float_to_byte_3_15(f32::INFINITY), 255);

        // all negatives map to 0
        assert_eq!(SmallFloat::float_to_byte_3_15(-f32::MIN_POSITIVE), 0);
        assert_eq!(SmallFloat::float_to_byte_3_15(-f32::MAX), 0);
        assert_eq!(SmallFloat::float_to_byte_3_15(f32::NEG_INFINITY), 0);

        // up iterations for more exhaustive test after changing something
        let num = at_least(&mut random, 100_000);

        for _ in 0..num {
            let bits: u32 = random.random();
            let f = f32::from_bits(bits);
            if f.is_nan() {
                continue;
            }

            let b1 = orig_float_to_byte(f);
            let b2 = SmallFloat::float_to_byte(f, 3, 15);
            let b3 = SmallFloat::float_to_byte_3_15(f);
            assert_eq!(b1, b2, "Mismatch: f = {}", f);
            assert_eq!(b2, b3, "Mismatch: f = {}", f);
        }
    }
    #[test]
    fn test_int4() -> Result<()> {
        for i in 0..=16 {
            // all values in 0-16 are encoded accurately
            let encoded = SmallFloat::long_to_int4(i)?;
            let decoded = SmallFloat::int4_to_long(encoded);
            assert_eq!(i, decoded, "round-trip failed at {}", i);
        }

        let max_encoded = SmallFloat::long_to_int4(i64::MAX)?;
        for i in 1..max_encoded {
            let v1 = SmallFloat::int4_to_long(i);
            let v0 = SmallFloat::int4_to_long(i - 1);
            assert!(v1 > v0, "non-monotonic at i = {}", i);
        }

        let mut random = random();
        let iters = at_least(&mut random, 1000);
        for _ in 0..iters {
            let end = TestUtil::next_int(&mut random, 5, 61);
            let l = TestUtil::next_long(&mut random, 0, 1i64 << end);
            let num_bits = 64 - l.leading_zeros();
            let expected = if num_bits > 4 {
                let mask = !0i64 << (num_bits - 4);
                l & mask
            } else {
                l
            };
            let round_trip = SmallFloat::int4_to_long(SmallFloat::long_to_int4(l)?);
            assert_eq!(
                expected, round_trip,
                "expected={}, got={}, input={}",
                expected, round_trip, l
            );
        }
        Ok(())
    }

    #[test]
    fn test_byte4() -> Result<()> {
        let mut random = random();
        let mut decoded = [0i32; 256];
        for (b, decoded_val) in decoded.iter_mut().enumerate() {
            *decoded_val = SmallFloat::byte4_to_int(b as u8)?;
            assert_eq!(b as u8, SmallFloat::int_to_byte4(*decoded_val)?);
        }
        for (i, window) in decoded.windows(2).enumerate() {
            assert!(window[1] > window[0], "failed at index {}", i + 1);
        }

        assert_eq!(255u8, SmallFloat::int_to_byte4(i32::MAX)?);
        let iters = at_least(&mut random, 1_000);
        for _ in 0..iters {
            let exp = TestUtil::next_usize(&mut random, 5, 30);
            let bound = 1usize << exp;
            let i = TestUtil::next_int(&mut random, 0, bound as i32);

            let idx = decoded
                .binary_search(&i)
                .unwrap_or_else(|ins| ins.saturating_sub(1));

            assert!(decoded[idx] <= i,);

            let b = SmallFloat::int_to_byte4(i)?;
            assert_eq!(idx as u8, b);
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_all_floats() -> Result<()> {
        let mut i = i32::MIN;
        loop {
            let f = f32::from_bits(i as u32);
            if !f.is_nan() {
                let b1 = orig_float_to_byte(f);
                let b2 = SmallFloat::float_to_byte_3_15(f);
                if b1 != b2 || (b2 == 0 && f > 0.0) {
                    unreachable!(
                        "Failed float_to_byte_3_15 for float = {:e}, source_bits = {:#x}, raw_bits = {:#x}",
                        f,
                        i,
                        f.to_bits()
                    );
                }
            }
            if i == i32::MAX {
                return Ok(());
            }
            i = i.wrapping_add(1);
        }
    }
}
