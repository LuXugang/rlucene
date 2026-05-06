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
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::util::TryIntoInt;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::bits::Bits;
use crate::core::util::doc_base_bit_set_iterator::DocBaseBitSetIterator;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::int_array_doc_id_set::IntArrayDocIdSetIterator;
use crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet;
use crate::test::core::util::base_bit_set_test_case::{
  BaseBitSetTestCase, BaseBitSetTestCaseSupperImpl, RustUtilBitSet,
};
use crate::test::core::util::id_set_common::{clear_range, flip_bit, flip_bit_range, set_range};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{is_night_mode, random};
use rand::prelude::StdRng;
use rand::{Rng, RngExt};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

pub struct TestFixedBitSet;
fn run_case<F>(f: F) -> crate::core::util::error::lucene_error::Result<()>
where
  F: FnOnce(&mut TestFixedBitSet, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let mut case = TestFixedBitSet;
  f(&mut case, &mut random)
}
impl BaseBitSetTestCaseSupperImpl for TestFixedBitSet {}
impl BaseBitSetTestCase for TestFixedBitSet {
  fn copy_of(
    &self,
    bs: &RustUtilBitSet,
    length: usize,
  ) -> (impl BitSet, Option<SparseFixedBitSet>) {
    let mut set = FixedBitSet::new(length);
    let mut doc = bs.next_set_bit(0);
    while doc != NO_MORE_DOCS as usize {
      set.set(doc);
      if doc + 1 > length {
        doc = NO_MORE_DOCS as usize;
      } else {
        doc = bs.next_set_bit(doc + 1);
      }
    }
    (set, None)
  }

  fn assert_equals(
    &self,
    set1: &RustUtilBitSet,
    set2: &impl BitSet,
    max_doc: usize,
    sfbs: Option<&SparseFixedBitSet>,
  ) {
    BaseBitSetTestCaseSupperImpl::assert_equals(self, set1, set2, max_doc, sfbs);
  }

  fn test_prev_set_bit<R>(&mut self, random: &mut R)
  where
    R: Rng + ?Sized,
  {
    check_prev_set_bit_array(random, vec![], 0);
    check_prev_set_bit_array(random, vec![0], 1);
    check_prev_set_bit_array(random, vec![0, 2], 3);
  }
}
mod base_doc_id_set_test_case_util {
  use super::*;
  #[test]
  fn test_cardinality() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_cardinality(random);
      Ok(())
    })
  }

  #[test]
  fn test_prev_set_bit() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_prev_set_bit(random);
      Ok(())
    })
  }

  #[test]
  fn test_next_set_bit() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_next_set_bit(random);
      Ok(())
    })
  }

  #[test]
  fn test_next_set_bit_in_range() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_next_set_bit_in_range(random);
      Ok(())
    })
  }

  #[test]
  fn test_set() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_set(random);
      Ok(())
    })
  }

  #[test]
  fn test_get_and_set() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_get_and_set(random);
      Ok(())
    })
  }

  #[test]
  fn test_clear() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_clear(random);
      Ok(())
    })
  }

  #[test]
  fn test_clear_range() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_clear_range(random);
      Ok(())
    })
  }

  #[test]
  fn test_clear_all() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_clear_all(random);
      Ok(())
    })
  }

  #[test]
  fn test_or_sparse() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_or_sparse(random);
      Ok(())
    })
  }

  #[test]
  fn test_or_dense() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_or_dense(random);
      Ok(())
    })
  }

  #[test]
  fn test_or_random() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_or_random(random);
      Ok(())
    })
  }
}

#[test]
fn test_approximate_cardinality() {
  // The approximate cardinality works in such a way that it should be
  // pretty accurate on a bitset whose bits are uniformly
  // distributed.
  let mut random = random();
  let mut set = FixedBitSet::new(random.random_range(100000..=200000));
  let first = random.random_range(0..=10);
  let interval = random.random_range(10..=20);
  let mut i = first;
  while i < set.length() {
    set.set(i);
    i += interval;
  }
  let cardinality = set.cardinality();
  assert!(cardinality.abs_diff(set.approximate_cardinality()) <= (cardinality / 20))
}
fn do_get(a: &bit_set::BitSet, b: &FixedBitSet) {
  assert_eq!(a.count(), b.cardinality());
  let max = b.length();
  for i in 0..max {
    assert_eq!(a.contains(i), b.get(i).unwrap());
  }
}

fn do_next_set_bit(a: &bit_set::BitSet, b: &FixedBitSet) {
  assert_eq!(a.count(), b.cardinality());
  let mut bb = 0;
  loop {
    bb = b.next_set_bit(bb);

    if bb == NO_MORE_DOCS as usize {
      assert!(!a.contains(bb));
      break;
    }
    assert!(a.contains(bb));
    bb += 1;
    if bb > b.length() - 1 {
      assert!(!a.contains(bb));
      break;
    }
  }

  let iter = a.iter();
  for index in iter {
    assert_eq!(index, b.next_set_bit(index));
  }
}

fn do_prev_set_bit(a: &bit_set::BitSet, b: &FixedBitSet) {
  assert_eq!(a.count(), b.cardinality());
  let mut bb = b.length().checked_sub(1);
  let mut count = 0;
  let mut iter: Vec<_> = a.iter().collect();
  iter.reverse();
  // check set a bit in BitSet should be in FixedBitSet
  for index in iter {
    bb = b.prev_set_bit(index);
    assert_eq!(*bb.as_ref().unwrap(), index);
  }
  if let Some(bb) = bb {
    // bb is the last match value, so prev_set_bit(bb - 1) should return None
    if bb > 0 {
      assert_eq!(b.prev_set_bit(bb - 1), None);
    }
  }

  bb = if b.length() < 1 {
    None
  } else {
    Option::from(b.length() - 1)
  };

  if bb.is_none() {
    assert_eq!(a.iter().count(), 0);
    return;
  }

  loop {
    bb = b.prev_set_bit(*bb.as_ref().unwrap());
    if bb.is_none() {
      break;
    }
    count += 1;
    assert!(a.contains(*bb.as_ref().unwrap()));
    if *bb.as_ref().unwrap() == 0 {
      break;
    }
    bb = bb.map(|x| x - 1);
  }
  assert_eq!(b.cardinality(), count);
}

fn do_iterate<R>(random: &mut R, a: &bit_set::BitSet, b: FixedBitSet) -> Result<FixedBitSet>
where
  R: Rng + ?Sized,
{
  assert_eq!(a.count(), b.cardinality());
  let mut iterator = BitSetIterator::new(b, 0)?;
  let iter = a.iter();
  for index in iter {
    let bb = if random.random_bool(0.5) {
      iterator.next_doc()?
    } else {
      iterator.advance(index as i32)?
    };
    assert_eq!(index, bb as usize);
  }
  assert_eq!(iterator.next_doc()?, NO_MORE_DOCS);
  Ok(iterator.bits)
}

fn do_random_sets<R>(random: &mut R, iter: i32) -> Result<()>
where
  R: Rng + ?Sized,
{
  // let max_size = random.random_range(1200..=i32::MAX);
  let max_size = random.random_range(1200..=100000);
  let mut a0: bit_set::BitSet = Default::default();
  let mut b0: FixedBitSet = Default::default();
  let mut flag = 0;
  for _i in 0..iter {
    let sz = random.random_range(2..max_size);
    let mut a = bit_set::BitSet::with_capacity(sz);
    let mut b = FixedBitSet::new(sz);
    let n_oper = random.random_range(0..sz);
    for _j in 0..n_oper {
      let mut idx = random.random_range(0..sz);
      a.insert(idx);
      b.set(idx);

      idx = random.random_range(0..sz);
      a.remove(idx);
      b.clear_with_index(idx);

      idx = random.random_range(0..sz);
      flip_bit_range(&mut a, idx, idx + 1);
      b.flip_range(idx, idx + 1);

      idx = random.random_range(0..sz);
      flip_bit(&mut a, idx);
      b.flip(idx);

      let val2 = b.get(idx)?;
      let val = b.get_and_set(idx);
      assert_eq!(val2, val);
      assert!(b.get(idx)?);

      if !val {
        b.clear_with_index(idx);
      }
      assert_eq!(b.get(idx)?, val);
    }

    // test that the various ways of accessing the bits are equivalent
    do_get(&a, &b);

    // test ranges, including possible extension
    let mut from_index;
    let mut to_index;
    from_index = random.random_range(0..(sz / 2));
    to_index = from_index + random.random_range(0..(sz - from_index));
    let mut aa = a.clone();
    flip_bit_range(&mut aa, from_index, to_index);
    let mut bb = b.clone();
    bb.flip_range(from_index, to_index);

    do_iterate(random, &aa, bb)?; //  a problem here is from flip or doIterate

    from_index = random.random_range(0..(sz / 2));
    to_index = from_index + random.random_range(0..(sz - from_index));
    aa.clone_from(&a);
    clear_range(&mut aa, from_index, to_index);
    bb = b.clone();
    bb.clear_range(from_index, to_index);

    do_next_set_bit(&aa, &bb); // a problem here is from clear() or nextSetBit

    do_prev_set_bit(&aa, &bb);

    from_index = random.random_range(0..(sz / 2));
    to_index = from_index + random.random_range(0..(sz - from_index));
    aa.clone_from(&a);
    set_range(&mut aa, from_index, to_index);
    bb = b.clone();
    bb.set_with_range(from_index, to_index);

    do_next_set_bit(&aa, &bb); // a problem here is from set() or nextSetBit

    do_prev_set_bit(&aa, &bb);

    if flag == 1 && b0.length() <= b.length() {
      assert_eq!(a.count(), b.cardinality());

      let mut a_and = a.clone();
      a_and.intersect_with(&a0);
      let mut a_or = a.clone();
      a_or.union_with(&a0);
      let mut a_xor = a.clone();
      a_xor.symmetric_difference_with(&a0);
      let mut a_andn = a.clone();
      a_andn.difference_with(&a0);

      let mut b_and = b.clone();
      assert_eq!(b, b_and);
      b_and.and(&b0);
      let mut b_or = b.clone();
      b_or.or(&b0);
      let mut b_xor = b.clone();
      b_xor.xor(&b0);
      let mut b_andn = b.clone();
      b_andn.and_not_fixed_bit_set(&b0);

      assert_eq!(a0.count(), b0.cardinality());
      assert_eq!(a_or.count(), b_or.cardinality());

      assert_eq!(a_and.count(), b_and.cardinality());
      assert_eq!(a_or.count(), b_or.cardinality());
      assert_eq!(a_andn.count(), b_andn.cardinality());
      assert_eq!(a_xor.count(), b_xor.cardinality());

      do_iterate(random, &a_and, b_and)?;
      do_iterate(random, &a_xor, b_xor)?;
      do_iterate(random, &a_or, b_or)?;
      do_iterate(random, &a_andn, b_andn)?;

      a0 = a;
      b0 = b;
    } else {
      flag = 1;
      a0 = a;
      b0 = b;
    }
  }
  Ok(())
}
#[test]
fn test_small() -> Result<()> {
  let mut random = random();
  let iters = if is_night_mode() {
    random.random_range(1000..100000)
  } else {
    100
  };
  do_random_sets(&mut random, iters)?;
  Ok(())
}

#[test]
fn test_equals() {
  // This test can't handle numBits==0:
  let mut random = random();
  let num_bits = random.random_range(0..2000) + 1;
  let mut b1 = FixedBitSet::new(num_bits);
  let mut b2 = FixedBitSet::new(num_bits);
  assert!(b1.eq(&b2));
  assert!(b2.eq(&b1));
  for _i in 0..random.random_range(1000..5000) {
    let idx = random.random_range(0..num_bits);
    if !b1.get(idx).unwrap() {
      b1.set(idx);
      assert!(!b1.eq(&b2));
      assert!(!b2.eq(&b1));
      b2.set(idx);
      assert!(b1.eq(&b2));
      assert!(b2.eq(&b1));
    }
  }
}

#[test]
fn test_hash_code_equals() {
  let mut random = random();

  let num_bits = random.random_range(0..2000) + 1;
  let mut b1 = FixedBitSet::new(num_bits);
  let mut b2 = FixedBitSet::new(num_bits);
  for _i in 0..random.random_range(1000..5000) {
    let idx = random.random_range(0..num_bits);
    if !b1.get(idx).unwrap() {
      b1.set(idx);
      assert!(!b1.eq(&b2));
      assert_ne!(calculate_hash(&b1), calculate_hash(&b2));
      b2.set(idx);
      assert!(b1.eq(&b2));
      assert_eq!(calculate_hash(&b1), calculate_hash(&b2));
    }
  }
}

fn calculate_hash(a: &FixedBitSet) -> u64 {
  let mut hasher = DefaultHasher::new();
  a.hash(&mut hasher);
  hasher.finish()
}

#[test]
fn test_small_bitsets() {
  // Make sure size 0-10 bit sets are OK:
  for num_bits in 0..10 {
    let mut b1 = FixedBitSet::new(num_bits);
    let b2 = FixedBitSet::new(num_bits);
    assert!(b1.eq(&b2));
    assert_eq!(calculate_hash(&b1), calculate_hash(&b2));
    assert_eq!(0, b1.cardinality());
    if num_bits > 0 {
      b1.set_with_range(0, num_bits);
      assert_eq!(num_bits, b1.cardinality());
      b1.flip_range(0, num_bits);
      assert_eq!(0, b1.cardinality());
    }
  }
}
fn make_fixed_bitset<R>(random: &mut R, a: &[usize], num_bits: usize) -> Result<FixedBitSet>
where
  R: Rng + ?Sized,
{
  let mut bs: FixedBitSet;
  if random.random_bool(0.5) {
    let bits_2_words = FixedBitSet::bits2words(num_bits);
    let mut words: Vec<i64> = Vec::with_capacity(bits_2_words);
    words.resize(num_bits, 0);
    bs = FixedBitSet::with_capacity(words, num_bits)?
  } else {
    bs = FixedBitSet::new(num_bits)
  }
  for e in a {
    bs.set(*e);
  }
  Ok(bs)
}

fn make_bitset(a: &[usize]) -> bit_set::BitSet {
  let mut bs: bit_set::BitSet = bit_set::BitSet::with_capacity(a.len());
  for x in a {
    bs.insert(*x);
  }
  bs
}

fn check_prev_set_bit_array<R>(random: &mut R, a: Vec<usize>, num_bits: usize)
where
  R: Rng + ?Sized,
{
  let obs = make_fixed_bitset(random, &a, num_bits).unwrap();
  let bs = make_bitset(&a);
  do_prev_set_bit(&bs, &obs);
}

fn check_next_set_bit_array<R>(random: &mut R, a: Vec<usize>, num_bits: usize)
where
  R: Rng + ?Sized,
{
  let obs = make_fixed_bitset(random, &a, num_bits).unwrap();
  let bs = make_bitset(&a);
  do_next_set_bit(&bs, &obs);
}
#[test]
fn test_next_bitset() {
  let mut random = random();
  let capacity = random.random_range(0..1000);
  let mut set_bits = Vec::with_capacity(capacity);
  for _i in 0..capacity {
    set_bits.push(random.random_range(0..capacity));
  }
  let num_bits = set_bits.len() + random.random_range(0..10);
  check_next_set_bit_array(&mut random, set_bits, num_bits);
  check_next_set_bit_array(&mut random, vec![], num_bits);
}

#[test]
fn test_ensure_capacity() -> Result<()> {
  let mut bits = FixedBitSet::new(5);
  bits.set(1);
  bits.set(4);
  bits.ensure_capacity(8);
  let mut new_bits = bits.clone();
  assert!(bits.get(1)?);
  assert!(bits.get(4)?);
  bits.clear_with_index(1);
  assert!(!bits.get(1)?);
  assert!(new_bits.get(1)?);

  new_bits.set(1);
  let length = bits.length();
  new_bits.ensure_capacity(length - 2);
  assert!(new_bits.get(1)?);

  new_bits.set(1);
  new_bits.ensure_capacity(72);
  assert!(new_bits.get(1)?);
  assert!(new_bits.get(4)?);
  new_bits.clear_with_index(1);
  // we grew the long[], so it's not shared
  assert!(!bits.get(1)?);
  assert!(!new_bits.get(1)?);
  Ok(())
}

#[test]
fn test_bits2words() {
  assert_eq!(0, FixedBitSet::bits2words(0));
  assert_eq!(1, FixedBitSet::bits2words(1));

  assert_eq!(1, FixedBitSet::bits2words(64));
  assert_eq!(2, FixedBitSet::bits2words(65));

  assert_eq!(2, FixedBitSet::bits2words(128));
  assert_eq!(3, FixedBitSet::bits2words(129));

  assert_eq!(1024, FixedBitSet::bits2words(65536));
  assert_eq!(1025, FixedBitSet::bits2words(65537));

  assert_eq!(1 << (31 - 6), FixedBitSet::bits2words(i32::MAX as usize));
}

fn make_int_array<R>(random: &mut R, count: usize, min: usize, max: usize) -> Vec<usize>
where
  R: Rng + ?Sized,
{
  let mut rv = vec![0; count];
  for _i in 0..count {
    rv.push(random.random_range(min..=max));
  }
  rv
}

#[test]
fn test_intersection_count() {
  let mut random = random();

  let num_bits1 = random.random_range(1000..=2000);
  let num_bits2 = random.random_range(1000..=2000);

  let count1 = random.random_range(0..=num_bits1 - 1);
  let count2 = random.random_range(0..=num_bits2 - 1);

  let bits1 = make_int_array(&mut random, count1, 0, num_bits1 - 1);
  let bits2 = make_int_array(&mut random, count2, 0, num_bits2 - 1);

  let fixed_bit_set1 = make_fixed_bitset(&mut random, &bits1, num_bits1);
  let fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2);
  // If ghost bits are present, these may fail too, but that's not what we
  // want to demonstrate here
  // assertTrue(fixedBitSet1.cardinality() <= bits1.length);
  // assertTrue(fixedBitSet2.cardinality() <= bits2.length);
  let intersection_count =
    FixedBitSet::intersection_count(fixed_bit_set1.unwrap(), fixed_bit_set2.unwrap());

  let mut bit_set1 = make_bitset(&bits1);
  let bit_set2 = make_bitset(&bits2);
  // If ghost bits are present, these may fail too, but that's not what we
  // want to demonstrate here
  // assertEquals(bitSet1.cardinality(), fixedBitSet1.cardinality());
  // assertEquals(bitSet2.cardinality(), fixedBitSet2.cardinality());

  bit_set1.intersect_with(&bit_set2);
  assert_eq!(bit_set1.count(), intersection_count as usize);
}

#[test]
fn test_and_not() -> Result<()> {
  let mut random = random();

  let num_bits2 = random.random_range(1000..=2000);
  let num_bits1 = random.random_range(1000..=num_bits2);

  let count1 = random.random_range(0..=num_bits1 - 1);
  let count2 = random.random_range(0..=num_bits2 - 1);

  let min = random.random_range(0..=(num_bits1 - 1));
  let off_set_word1 = min >> 6;
  let offset1 = off_set_word1 >> 6;
  let bits1 = make_int_array(&mut random, count1, min, num_bits1 - 1);
  let bits2 = make_int_array(&mut random, count2, 0, num_bits2 - 1);

  let bitset1 = make_bitset(&bits1);
  let mut bitset2 = make_bitset(&bits2);
  bitset2.difference_with(&bitset1);

  {
    // test BitSetIterator
    let mut fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2)?;
    let fixed_bit = make_fixed_bitset(&mut random, &bits1, num_bits1)?;
    let mut disi = BitSetIterator::new(fixed_bit, count1 as i64)?;
    fixed_bit_set2.and_not_iter(&mut disi)?;
    do_get(&bitset2, &fixed_bit_set2);
  }
  {
    // test DocBaseBitSetIterator
    let mut fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2)?;
    let offset_bits: Vec<usize> = bits1.iter().map(|&i| i - offset1).collect();
    let fixed_bit = make_fixed_bitset(&mut random, &offset_bits, num_bits1 - offset1)?;
    let mut disi = DocBaseBitSetIterator::new(fixed_bit, count1 as i64, offset1)?;
    fixed_bit_set2.and_not_iter(&mut disi)?;
    do_get(&bitset2, &fixed_bit_set2);
  }
  {
    // test other
    let mut fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2)?;
    let mut sorted: Vec<i32> = bits1
      .iter()
      .map(|&x| {
        debug_assert!(x <= i32::MAX as usize);
        x as i32
      })
      .collect();
    sorted.push(0);
    sorted[bits1.len()] = NO_MORE_DOCS;
    let mut disi = IntArrayDocIdSetIterator::new(Rc::new(sorted), count1.try_convert()?);
    fixed_bit_set2.and_not_iter(&mut disi)?;
    do_get(&bitset2, &fixed_bit_set2);
  }
  Ok(())
}

// Demonstrates that the presence of ghost bits in the last used word can
// cause spurious failures
#[test]
fn test_union_count() -> Result<()> {
  let mut random = random();
  let num_bits1 = random.random_range(1000..=2000);
  let num_bits2 = random.random_range(1000..=2000);

  let count1 = random.random_range(0..=num_bits1 - 1);
  let count2 = random.random_range(0..=num_bits2 - 1);

  let bits1 = make_int_array(&mut random, count1, 0, num_bits1 - 1);
  let bits2 = make_int_array(&mut random, count2, 0, num_bits2 - 1);

  let fixed_bit_set1 = make_fixed_bitset(&mut random, &bits1, num_bits1)?;
  let fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2)?;

  let union_count = FixedBitSet::union_count(&fixed_bit_set1, &fixed_bit_set2);

  let mut bit_set1 = make_bitset(&bits1);
  let bit_set2 = make_bitset(&bits2);
  bit_set1.union_with(&bit_set2);

  assert_eq!(bit_set1.count(), union_count as usize);
  Ok(())
}

#[test]
fn test_and_not_count() -> Result<()> {
  let mut random = random();

  let num_bits1 = random.random_range(1000..=2000);
  let num_bits2 = random.random_range(1000..=2000);

  let count1 = random.random_range(0..=num_bits1 - 1);
  let count2 = random.random_range(0..=num_bits2 - 1);

  let bits1 = make_int_array(&mut random, count1, 0, num_bits1 - 1);
  let bits2 = make_int_array(&mut random, count2, 0, num_bits2 - 1);

  let fixed_bit_set1 = make_fixed_bitset(&mut random, &bits1, num_bits1)?;
  let fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2)?;

  let and_not_count = FixedBitSet::and_not_count(&fixed_bit_set1, &fixed_bit_set2);

  let mut bit_set1 = make_bitset(&bits1);
  let bit_set2 = make_bitset(&bits2);

  bit_set1.difference_with(&bit_set2);

  assert_eq!(bit_set1.count(), and_not_count as usize);
  Ok(())
}

#[test]
fn test_copy_of() {
  // this test is not required in Rust Lucene
}

#[test]
fn test_as_bits() {
  // this test is not required in Rust Lucene
}
