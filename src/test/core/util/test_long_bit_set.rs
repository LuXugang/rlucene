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
// Migrated from src/core/util/long_bit_set.rs

use std::hash::{DefaultHasher, Hash, Hasher};

use bit_set::BitSet;
use rand::Rng;
use rand::RngExt;

use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::long_bit_set::LongBitSet;
use crate::test::core::util::id_set_common::{clear_range, flip_bit, flip_bit_range, set_range};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, at_least_usize, is_night_mode, random, random_multiplier,
};
use crate::test::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestLongBitSet;

fn do_get(a: &BitSet, b: &LongBitSet) {
  assert_eq!(a.count(), b.cardinality());
  let max = b.length();
  for i in 0..max {
    let abit = a.contains(i);
    let bbit = b.get(i);
    if abit != bbit {
      unreachable!("mismatch: BitSet[{}] = {}", i, abit);
    }
  }
}
fn do_next_set_bit(a: &BitSet, b: &LongBitSet) {
  let mut bb = None;

  let iter = a.iter();
  for index in iter {
    assert_eq!(index, b.next_set_bit(index).expect(""));
  }

  loop {
    let v = bb.map_or(0, |val| val + 1);
    if v >= b.length() {
      break;
    }
    bb = b.next_set_bit(v);
    if bb.is_none() {
      break;
    }
    assert!(a.contains(bb.expect("")));
  }
}

fn do_prev_set_bit<R>(random: &mut R, a: &BitSet, b: &LongBitSet)
where
  R: Rng + ?Sized,
{
  assert_eq!(a.count(), b.cardinality());

  let mut aa = a.get_ref().len() as i64 + random.random_range(0..100);
  let mut bb = aa;

  loop {
    // simulate a.prevSetBit
    aa -= 1;
    while aa >= 0 && !a.contains(aa as usize) {
      aa -= 1;
    }

    if b.length() == 0 {
      bb = -1;
    } else if bb > (b.length() as i64 - 1) {
      bb = b.prev_set_bit(b.length() - 1).map_or(-1, |v| v as i64);
    } else if bb < 1 {
      bb = -1;
    } else {
      bb = if bb >= 1 {
        b.prev_set_bit((bb - 1) as usize).map_or(-1, |v| v as i64)
      } else {
        -1
      }
    }

    assert_eq!(aa, bb);
    if aa < 0 {
      break;
    }
  }
}
fn do_random_sets<R>(max_size: usize, iter: i32, _mode: i32, random: &mut R) -> Result<()>
where
  R: Rng + ?Sized,
{
  let mut a0: Option<BitSet> = None;
  let mut b0: Option<LongBitSet> = None;

  for _ in 0..iter {
    let sz = TestUtil::next_usize(random, 2, max_size);

    let mut a = BitSet::with_capacity(sz);
    let mut b = LongBitSet::new(sz)?;

    // test the various ways of setting bits
    if sz > 0 {
      let n_oper = random.random_range(0..sz);
      for _ in 0..n_oper {
        let mut idx = random.random_range(0..sz);
        a.insert(idx);
        b.set(idx);

        idx = random.random_range(0..sz);
        a.remove(idx);
        b.clear(idx);

        idx = random.random_range(0..sz);
        flip_bit_range(&mut a, idx, idx + 1);
        b.flip(idx, idx + 1);

        idx = random.random_range(0..sz);
        flip_bit(&mut a, idx);
        b.flip_one(idx);

        let val2 = b.get(idx);
        let val = b.get_and_set(idx);
        assert_eq!(val2, val);
        assert!(b.get(idx));
        if !val {
          b.clear(idx);
        }
        assert_eq!(b.get(idx), val);
      }
    }

    do_get(&a, &b);

    // Flip range
    let from_index = random.random_range(0..(sz / 2 + 1));
    let to_index = from_index + random.random_range(0..(sz - from_index + 1));
    let mut aa = a.clone();
    flip_bit_range(&mut aa, from_index, to_index);
    let mut bb = b.clone();
    bb.flip(from_index, to_index);

    // Clear range
    let from_index = random.random_range(0..(sz / 2 + 1));
    let to_index = from_index + random.random_range(0..(sz - from_index + 1));
    let mut aa = a.clone();
    clear_range(&mut aa, from_index, to_index);
    let mut bb = b.clone();
    bb.clear_range(from_index, to_index);

    do_next_set_bit(&aa, &bb);
    do_prev_set_bit(random, &aa, &bb);

    // Set range
    let from_index = random.random_range(0..(sz / 2 + 1));
    let to_index = from_index + random.random_range(0..(sz - from_index + 1));
    let mut aa = a.clone();
    set_range(&mut aa, from_index, to_index);
    let mut bb = b.clone();
    bb.set_range(from_index, to_index);

    do_next_set_bit(&aa, &bb);
    do_prev_set_bit(random, &aa, &bb);

    // bitwise ops
    if let (Some(a0), Some(b0)) = (&a0, &b0)
      && b0.length() <= b.length()
    {
      assert_eq!(a.count(), b.cardinality());

      let mut a_and = a.clone();
      a_and.intersect_with(a0);
      let mut a_or = a.clone();
      a_or.union_with(a0);
      let mut a_xor = a.clone();
      a_xor.symmetric_difference_with(a0);
      let mut a_andn = a.clone();
      a_andn.difference_with(a0);

      let mut b_and = b.clone();
      assert_eq!(b, b_and);
      b_and.and(b0);
      let mut b_or = b.clone();
      b_or.or(b0);
      let mut b_xor = b.clone();
      b_xor.xor(b0);
      let mut b_andn = b.clone();
      b_andn.and_not(b0);

      assert_eq!(a0.count(), b0.cardinality());
      assert_eq!(a_or.count(), b_or.cardinality());
      assert_eq!(a_and.count(), b_and.cardinality());
      assert_eq!(a_xor.count(), b_xor.cardinality());
      assert_eq!(a_andn.count(), b_andn.cardinality());
    }
    a0 = Some(a);
    b0 = Some(b);
  }
  Ok(())
}
// large enough to flush obvious bugs, small enough to run in <.5 sec as
// part of a larger testsuite.
#[test]
fn test_small() -> Result<()> {
  let mut random = random();
  let iters = if is_night_mode() {
    at_least(&mut random, 1000)
  } else {
    100
  };

  let size = at_least_usize(&mut random, 1200);
  do_random_sets(size, iters, 1, &mut random)?;
  do_random_sets(size, iters, 2, &mut random)?;
  Ok(())
}
#[test]
fn test_equals() -> Result<()> {
  let mut random = random();

  // This test can't handle num_bits == 0:
  let num_bits = random.random_range(1..2001);
  let mut b1 = LongBitSet::new(num_bits)?;
  let mut b2 = LongBitSet::new(num_bits)?;

  assert_eq!(b1, b2);
  assert_eq!(b2, b1);

  for _ in 0..10 * random_multiplier() {
    let idx = random.random_range(0..num_bits);
    if !b1.get(idx) {
      b1.set(idx);
      assert_ne!(b1, b2);
      assert_ne!(b2, b1);
      b2.set(idx);
      assert_eq!(b1, b2);
      assert_eq!(b2, b1);
    }
  }
  Ok(())
}
#[test]
fn test_hash_code_equals() -> Result<()> {
  let mut random = random();

  let num_bits = random.random_range(0..2000) + 1;
  let mut b1 = LongBitSet::new(num_bits)?;
  let mut b2 = LongBitSet::new(num_bits)?;
  for _i in 0..random.random_range(1000..5000) {
    let idx = random.random_range(0..num_bits);
    if !b1.get(idx) {
      b1.set(idx);
      assert!(!b1.eq(&b2));
      assert_ne!(calculate_hash(&b1), calculate_hash(&b2));
      b2.set(idx);
      assert!(b1.eq(&b2));
      assert_eq!(calculate_hash(&b1), calculate_hash(&b2));
    }
  }
  Ok(())
}
fn calculate_hash(a: &LongBitSet) -> u64 {
  let mut hasher = DefaultHasher::new();
  a.hash(&mut hasher);
  hasher.finish()
}
#[test]
fn test_too_large() {
  let result = LongBitSet::new(LongBitSet::MAX_NUM_BITS + 1);
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("num_bits must be 0")
  );
}
#[test]
fn test_negative_num_bits() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_small_bitsets() -> Result<()> {
  // Make sure size 0-10 bit sets are OK:
  for num_bits in 0..10 {
    let mut b1 = LongBitSet::new(num_bits)?;
    let b2 = LongBitSet::new(num_bits)?;
    assert!(b1.eq(&b2));
    assert_eq!(calculate_hash(&b1), calculate_hash(&b2));
    assert_eq!(0, b1.cardinality());
    if num_bits > 0 {
      b1.set_range(0, num_bits);
      assert_eq!(num_bits, b1.cardinality());
      b1.flip(0, num_bits);
      assert_eq!(0, b1.cardinality());
    }
  }
  Ok(())
}
fn make_long_bitset<R>(random: &mut R, a: &Vec<usize>, num_bits: usize) -> Result<LongBitSet>
where
  R: Rng + ?Sized,
{
  let mut bs: LongBitSet;
  if random.random_bool(0.5) {
    let bits_2_words = LongBitSet::bits2words(num_bits)?;
    let mut words: Vec<i64> = Vec::with_capacity(bits_2_words as usize);
    words.resize(num_bits, 0);
    bs = LongBitSet::from_bits(words, num_bits)?
  } else {
    bs = LongBitSet::new(num_bits)?
  }
  for e in a {
    bs.set(*e);
  }
  Ok(bs)
}

fn make_bitset(a: &Vec<usize>) -> BitSet {
  let mut bs = BitSet::with_capacity(a.len());
  for x in a {
    bs.insert(*x);
  }
  bs
}

fn check_prev_set_bit_array<R>(random: &mut R, a: Vec<usize>, num_bits: usize) -> Result<()>
where
  R: Rng + ?Sized,
{
  let obs = make_long_bitset(random, &a, num_bits)?;
  let bs = make_bitset(&a);
  do_prev_set_bit(random, &bs, &obs);
  Ok(())
}
#[test]
fn test_prev_set_bit() -> Result<()> {
  let mut random = random();

  check_prev_set_bit_array(&mut random, vec![], 0)?;
  check_prev_set_bit_array(&mut random, vec![0], 1)?;
  check_prev_set_bit_array(&mut random, vec![0, 2], 3)?;

  Ok(())
}

fn check_next_set_bit_array<R>(random: &mut R, a: Vec<usize>, num_bits: usize) -> Result<()>
where
  R: Rng + ?Sized,
{
  let obs = make_long_bitset(random, &a, num_bits)?;
  let bs = make_bitset(&a);
  do_next_set_bit(&bs, &obs);
  Ok(())
}
#[test]
fn test_next_bit_set() -> Result<()> {
  let mut random = random();
  let len = random.random_range(0..1000);
  let mut set_bits = Vec::with_capacity(len);
  for _ in 0..len {
    set_bits.push(random.random_range(0..len));
  }
  let mut num_bits = len + random.random_range(0..10);
  check_next_set_bit_array(&mut random, set_bits.clone(), num_bits)?;
  num_bits = len + random.random_range(0..10);
  check_next_set_bit_array(&mut random, vec![], num_bits)?;

  Ok(())
}
#[test]
fn test_ensure_capacity() -> Result<()> {
  let mut bits = LongBitSet::new(5)?;
  bits.set(1);
  bits.set(4);
  LongBitSet::ensure_capacity(&mut bits, 8)?;
  let mut new_bits = bits.clone();
  assert!(bits.get(1));
  assert!(bits.get(4));
  bits.clear(1);
  assert!(!bits.get(1));
  assert!(new_bits.get(1));

  new_bits.set(1);
  let length = bits.length();
  LongBitSet::ensure_capacity(&mut new_bits, length - 2)?;
  assert!(new_bits.get(1));

  new_bits.set(1);
  LongBitSet::ensure_capacity(&mut new_bits, 72)?;
  assert!(new_bits.get(1));
  assert!(new_bits.get(4));
  new_bits.clear(1);
  // we grew the long[], so it's not shared
  assert!(!bits.get(1));
  assert!(!new_bits.get(1));
  Ok(())
}
#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_huge_capacity() -> Result<()> {
  let more_than_max_int = i32::MAX as usize + 5;

  let mut bits = LongBitSet::new(42)?;
  assert_eq!(bits.length(), 42);

  LongBitSet::ensure_capacity(&mut bits, more_than_max_int)?;
  assert!(bits.length() >= more_than_max_int);

  Ok(())
}
#[test]
fn test_bits2words() -> Result<()> {
  assert_eq!(LongBitSet::bits2words(0)?, 0);
  assert_eq!(LongBitSet::bits2words(1)?, 1);
  assert_eq!(LongBitSet::bits2words(64)?, 1);
  assert_eq!(LongBitSet::bits2words(65)?, 2);
  assert_eq!(LongBitSet::bits2words(128)?, 2);
  assert_eq!(LongBitSet::bits2words(129)?, 3);

  let v1 = LongBitSet::bits2words(i32::MAX as usize + 1)?;
  assert_eq!(v1, 1 << (31 - 6));

  let v2 = LongBitSet::bits2words(i32::MAX as usize + 2)?;
  assert_eq!(v2, (1 << (31 - 6)) + 1);

  let v3 = LongBitSet::bits2words(1 << 32)?;
  assert_eq!(v3, 1 << (32 - 6));

  let v4 = LongBitSet::bits2words((1 << 32) + 1)?;
  assert_eq!(v4, (1 << (32 - 6)) + 1);

  // Ensure MAX_NUM_BITS doesn't throw
  let v5 = LongBitSet::bits2words(LongBitSet::MAX_NUM_BITS)?;
  assert!(v5 > 0);

  Ok(())
}
