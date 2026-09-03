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
// Migrated from src/core/util/small_float.rs

use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::small_float::SmallFloat;
use crate::test_framework::core::util::lucene_test_case::{at_least, random};

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

use crate::test_framework::core::util::test_util::TestUtil;

#[test]
fn test_float_to_byte() {
  let mut random = random();
  assert_eq!(orig_float_to_byte_v13(5.8123817e-10f32), 0);
  assert_eq!(orig_float_to_byte(5.8123817e-10f32), 1);
  assert_eq!(SmallFloat::float_to_byte_3_15(5.8123817e-10f32), 1);

  // test some constants
  assert_eq!(SmallFloat::float_to_byte_3_15(0.0), 0);
  assert_eq!(SmallFloat::float_to_byte_3_15(BitUtil::F32_MIN_VALUE), 1); // underflow rounds up
  assert_eq!(SmallFloat::float_to_byte_3_15(f32::MAX), 255); // overflow rounds down
  assert_eq!(SmallFloat::float_to_byte_3_15(f32::INFINITY), 255);

  // all negatives map to 0
  assert_eq!(SmallFloat::float_to_byte_3_15(-BitUtil::F32_MIN_VALUE), 0);
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
#[ignore = "One-time test."]
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
