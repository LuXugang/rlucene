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
use crate::core::document::document::Document;
use crate::core::index::index_reader::IndexReader;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;
use crate::sandbox::document::half_float_point::HalfFloatPoint;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_searcher_with_reader, random,
};
use crate::test::support::core::util::test_util::TestUtil;
use rand::RngExt;

#[allow(dead_code)] // for quick search
struct TestHalfFloatPoint;

fn test_half_float(sbits: &str, value: f32) {
  let bits = u16::from_str_radix(sbits, 2).expect("valid bits") as i16;
  let converted = HalfFloatPoint::short_bits_to_half_float(bits);
  if value.is_nan() {
    assert!(converted.is_nan());
  } else {
    assert_eq!(value.to_bits(), converted.to_bits());
  }
  let bits2 = HalfFloatPoint::half_float_to_short_bits(converted);
  assert_eq!(bits, bits2);
}

#[test]
fn test_half_float_conversion() {
  assert_eq!(0, HalfFloatPoint::half_float_to_short_bits(0f32));
  assert_eq!(
    (1u16 << 15) as i16,
    HalfFloatPoint::half_float_to_short_bits(-0f32)
  );
  assert_eq!(
    0,
    HalfFloatPoint::half_float_to_short_bits(f32::MIN_POSITIVE)
  );

  test_half_float("0011110000000000", 1.0);
  test_half_float("0011110000000001", 1.000_976_6);
  test_half_float("1100000000000000", -2.0);
  test_half_float("0111101111111111", 65504.0);
  test_half_float("0000010000000000", 2f32.powi(-14));
  test_half_float("0000001111111111", 2f32.powi(-14) - 2f32.powi(-24));
  test_half_float("0000000000000001", 2f32.powi(-24));
  test_half_float("0000000000000000", 0.0);
  test_half_float("1000000000000000", -0.0);
  test_half_float("0111110000000000", f32::INFINITY);
  test_half_float("1111110000000000", f32::NEG_INFINITY);
  test_half_float("0111111000000000", f32::NAN);
  test_half_float("0011010101010101", 0.333_251_95);
}

#[test]
fn test_round_shift() {
  assert_eq!(0, HalfFloatPoint::round_shift(0, 2));
  assert_eq!(0, HalfFloatPoint::round_shift(1, 2));
  assert_eq!(0, HalfFloatPoint::round_shift(2, 2));
  assert_eq!(1, HalfFloatPoint::round_shift(3, 2));
  assert_eq!(1, HalfFloatPoint::round_shift(4, 2));
  assert_eq!(1, HalfFloatPoint::round_shift(5, 2));
  assert_eq!(2, HalfFloatPoint::round_shift(6, 2));
  assert_eq!(2, HalfFloatPoint::round_shift(7, 2));
  assert_eq!(2, HalfFloatPoint::round_shift(8, 2));
  assert_eq!(2, HalfFloatPoint::round_shift(9, 2));
  assert_eq!(2, HalfFloatPoint::round_shift(10, 2));
  assert_eq!(3, HalfFloatPoint::round_shift(11, 2));
  assert_eq!(3, HalfFloatPoint::round_shift(12, 2));
  assert_eq!(3, HalfFloatPoint::round_shift(13, 2));
  assert_eq!(4, HalfFloatPoint::round_shift(14, 2));
  assert_eq!(4, HalfFloatPoint::round_shift(15, 2));
  assert_eq!(4, HalfFloatPoint::round_shift(16, 2));
}

#[test]
fn test_rounding() -> Result<()> {
  let mut values = Vec::new();
  for i in i16::MIN as i32..=i16::MAX as i32 {
    let v = HalfFloatPoint::sortable_short_to_half_float(i as i16);
    if v.is_finite() {
      values.push(v);
    }
  }

  let mut random = random();
  let iters = at_least(&mut random, 1000000);
  for _ in 0..iters {
    let f = if random.random() {
      f32::from_bits(random.random())
    } else {
      (2.0 * random.random::<f32>() - 1.0)
        * 2f64.powi(TestUtil::next_int(&mut random, -16, 16)) as f32
    };
    let rounded =
      HalfFloatPoint::short_bits_to_half_float(HalfFloatPoint::half_float_to_short_bits(f));
    if !f.is_finite() {
      let f_bits = if f.is_nan() {
        f32::NAN.to_bits()
      } else {
        f.to_bits()
      };
      let rounded_bits = if rounded.is_nan() {
        f32::NAN.to_bits()
      } else {
        rounded.to_bits()
      };
      assert_eq!(f_bits, rounded_bits);
    } else if !rounded.is_finite() {
      assert!(!rounded.is_nan());
      assert!(f.abs() >= 65520.0);
    } else {
      let index = values.binary_search_by(|probe| probe.total_cmp(&f));
      let closest = match index {
        Ok(index) => values[index],
        Err(index) => {
          let mut closest = f32::INFINITY;
          if index < values.len() {
            closest = values[index];
          }
          if index >= 1
            && (f - values[index - 1] < closest - f
              || (f - values[index - 1] == closest - f
                && values[index - 1].to_bits().trailing_zeros()
                  > closest.to_bits().trailing_zeros()))
          {
            closest = values[index - 1];
          }
          closest
        },
      };
      assert_eq!(closest.to_bits(), rounded.to_bits());
    }
  }
  Ok(())
}

#[test]
fn test_sortable_bits() {
  let mut low = i16::MIN as i32;
  let mut high = i16::MAX as i32;
  while HalfFloatPoint::sortable_short_to_half_float(low as i16).is_nan() {
    low += 1;
  }
  while HalfFloatPoint::sortable_short_to_half_float(low as i16) == f32::NEG_INFINITY {
    low += 1;
  }
  while HalfFloatPoint::sortable_short_to_half_float(high as i16).is_nan() {
    high -= 1;
  }
  while HalfFloatPoint::sortable_short_to_half_float(high as i16) == f32::INFINITY {
    high -= 1;
  }
  for i in low..=high + 1 {
    let previous = HalfFloatPoint::sortable_short_to_half_float((i - 1) as i16);
    let current = HalfFloatPoint::sortable_short_to_half_float(i as i16);
    assert_eq!(
      i as i16,
      HalfFloatPoint::half_float_to_sortable_short(current)
    );
    assert!(previous.total_cmp(&current).is_lt());
  }
}

#[test]
fn test_sortable_bytes() {
  for i in i16::MIN as i32 + 1..=i16::MAX as i32 {
    let mut previous = vec![0u8; HalfFloatPoint::BYTES];
    HalfFloatPoint::short_to_sortable_bytes((i - 1) as i16, &mut previous, 0);
    let mut current = vec![0u8; HalfFloatPoint::BYTES];
    HalfFloatPoint::short_to_sortable_bytes(i as i16, &mut current, 0);
    assert!(previous < current);
    assert_eq!(
      i as i16,
      HalfFloatPoint::sortable_bytes_to_short(&current, 0)
    );
  }
}

/** Add a single value and search for it */
#[test]
fn test_basics() -> Result<()> {
  let mut random = random();
  let mut dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  // add a doc with an single dimension
  let mut document = Document::new();
  document.add(HalfFloatPoint::new("field", [1.25f32])?);
  writer.add_document(&mut random, document)?;

  // search and verify we found our doc
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;
  assert_eq!(
    1,
    searcher.count(HalfFloatPoint::new_exact_query("field", 1.25f32)?)?
  );
  assert_eq!(
    0,
    searcher.count(HalfFloatPoint::new_exact_query("field", 1f32)?)?
  );
  assert_eq!(
    0,
    searcher.count(HalfFloatPoint::new_exact_query("field", 2f32)?)?
  );
  assert_eq!(
    1,
    searcher.count(HalfFloatPoint::new_range_query("field", 1f32, 2f32)?)?
  );
  assert_eq!(
    0,
    searcher.count(HalfFloatPoint::new_range_query("field", 0f32, 1f32)?)?
  );
  assert_eq!(
    0,
    searcher.count(HalfFloatPoint::new_range_query("field", 1.5f32, 2f32)?)?
  );
  assert_eq!(
    1,
    searcher.count(HalfFloatPoint::new_set_query("field", [1.25f32])?)?
  );
  assert_eq!(
    1,
    searcher.count(HalfFloatPoint::new_set_query("field", [1f32, 1.25f32])?)?
  );
  assert_eq!(
    0,
    searcher.count(HalfFloatPoint::new_set_query("field", [1f32])?)?
  );
  assert_eq!(
    0,
    searcher.count(HalfFloatPoint::new_set_query("field", Vec::<f32>::new())?)?
  );

  searcher.get_index_reader().close()?;
  writer.close(&mut random)?;
  dir.close()?;
  Ok(())
}

/** Add a single multi-dimensional value and search for it */
#[test]
fn test_basics_multi_dims() -> Result<()> {
  let mut random = random();
  let mut dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  // add a doc with two dimensions
  let mut document = Document::new();
  document.add(HalfFloatPoint::new("field", [1.25f32, -2f32])?);
  writer.add_document(&mut random, document)?;

  // search and verify we found our doc
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;
  assert_eq!(
    1,
    searcher.count(HalfFloatPoint::new_range_query_n(
      "field",
      [0f32, -5f32],
      [1.25f32, -1f32]
    )?)?
  );
  assert_eq!(
    0,
    searcher.count(HalfFloatPoint::new_range_query_n(
      "field",
      [0f32, 0f32],
      [2f32, 2f32]
    )?)?
  );
  assert_eq!(
    0,
    searcher.count(HalfFloatPoint::new_range_query_n(
      "field",
      [-10f32, -10f32],
      [1f32, 2f32]
    )?)?
  );

  searcher.get_index_reader().close()?;
  writer.close(&mut random)?;
  dir.close()?;
  Ok(())
}

#[test]
fn test_next_up() {
  assert!(HalfFloatPoint::next_up(f32::NAN).is_nan());
  assert_eq!(f32::INFINITY, HalfFloatPoint::next_up(f32::INFINITY));
  assert_eq!(-65504.0, HalfFloatPoint::next_up(f32::NEG_INFINITY));
  assert_eq!(
    HalfFloatPoint::short_bits_to_half_float(0),
    HalfFloatPoint::next_up(-0f32)
  );
  assert_eq!(
    HalfFloatPoint::short_bits_to_half_float(1),
    HalfFloatPoint::next_up(0f32)
  );
  assert_eq!(
    HalfFloatPoint::next_up(0f32),
    HalfFloatPoint::next_up(f32::MIN_POSITIVE)
  );
  assert_eq!(
    (-0f32).to_bits(),
    HalfFloatPoint::next_up(-f32::MIN_POSITIVE).to_bits()
  );
  assert_eq!(0f32.to_bits(), HalfFloatPoint::next_up(-0f32).to_bits());
}

#[test]
fn test_next_down() {
  assert!(HalfFloatPoint::next_down(f32::NAN).is_nan());
  assert_eq!(
    f32::NEG_INFINITY,
    HalfFloatPoint::next_down(f32::NEG_INFINITY)
  );
  assert_eq!(65504.0, HalfFloatPoint::next_down(f32::INFINITY));
  assert_eq!((-0f32).to_bits(), HalfFloatPoint::next_down(0f32).to_bits());
  assert_eq!(
    0f32.to_bits(),
    HalfFloatPoint::next_down(f32::MIN_POSITIVE).to_bits()
  );
  assert_eq!(
    HalfFloatPoint::next_down(-0f32),
    HalfFloatPoint::next_down(-f32::MIN_POSITIVE)
  );
  assert_eq!((-0f32).to_bits(), HalfFloatPoint::next_down(0f32).to_bits());
}
