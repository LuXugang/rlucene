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
use crate::test_framework::core::util::lucene_test_case::{at_least, random};
use std::collections::HashSet;

use crate::core::index::index_reader::Identity;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::DocIdSetIteratorEnum5;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::HasIdentity;
use crate::core::util::accountable::Accountable;
use crate::core::util::bit_doc_id_set::BitDocIdSet;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::bits::Bits;
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::roaring_doc_id_set::{Builder as RoaringDocIdSetBuilder, RoaringDocIdSet};
use crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet;
use crate::test::core::util::id_set_common;
use crate::test::core::util::id_set_common::clear_range;
use rand::Rng;
use rand::RngExt;
use std::sync::Arc;

pub fn random_set<R>(random: &mut R, num_bits: usize, percent_set: f32) -> bit_set::BitSet
where
  R: Rng + ?Sized,
{
  random_set_impl(random, num_bits, (percent_set * num_bits as f32) as usize)
}

pub fn random_set_impl<R>(random: &mut R, num_bits: usize, num_bits_set: usize) -> bit_set::BitSet
where
  R: Rng + ?Sized,
{
  assert!(num_bits_set <= num_bits);
  let mut set = bit_set::BitSet::with_capacity(num_bits);
  if num_bits_set == num_bits {
    id_set_common::set_range(&mut set, 0, num_bits)
  } else {
    for _i in 0..num_bits_set {
      loop {
        let o = random.random_range(0..num_bits);
        if !set.contains(o) {
          set.insert(o);
          break;
        }
      }
    }
  }
  set
}
pub trait BaseBitSetTestCase {
  type TestBitSet: BitSet;

  fn copy_of(
    &self,
    bs: &RustUtilBitSet,
    length: usize,
  ) -> Result<(Self::TestBitSet, Option<SparseFixedBitSet>)>;
  fn assert_equals(
    &self,
    set1: &RustUtilBitSet,
    set2: &impl BitSet,
    max_doc: usize,
    sfbs: Option<&SparseFixedBitSet>,
  );
  fn test_cardinality<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_bits = 1 + random.random_range(0..100000);
    for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
      let set1 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
      let (set2, _sfbs) = self.copy_of(&set1, num_bits)?;
      assert_eq!(set1.cardinality(), set2.cardinality());
    }
    Ok(())
  }
  fn test_prev_set_bit<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO: 1000 should be 100000
    let num_bits = 1 + random.random_range(0..1000);
    for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
      let set1 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
      let (set2, _sfbs) = self.copy_of(&set1, num_bits)?;
      for i in 0..num_bits {
        assert_eq!(set1.prev_set_bit(i), set2.prev_set_bit(i));
      }
    }
    Ok(())
  }
  fn test_next_set_bit<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO: 1000 should be 100000
    let num_bits = 1 + random.random_range(0..1000);
    for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
      let set1 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
      let (set2, _sfbs) = self.copy_of(&set1, num_bits)?;
      for i in 0..num_bits {
        assert_eq!(set1.next_set_bit(i), set2.next_set_bit(i));
      }
    }
    Ok(())
  }
  fn test_next_set_bit_in_range<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO: 1000 should be 100000
    let num_bits = 1 + random.random_range(0..1000);
    for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
      let set1 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
      let (set2, _sfbs) = self.copy_of(&set1, num_bits)?;
      for start in 0..num_bits {
        let end = if start + 1 == num_bits {
          num_bits
        } else {
          random.random_range(start + 1..num_bits)
        };
        assert_eq!(
          set1.next_set_bit_range(start, end),
          set2.next_set_bit_range(start, end),
          "start={}, end={}, num_bits = {}",
          start,
          end,
          num_bits
        );
      }
    }
    Ok(())
  }
  fn test_set<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_bits = 1 + random.random_range(0..100000) as usize;
    let set3 = RustUtilBitSet::new(random_set_impl(random, num_bits, 0), num_bits);
    let mut set1 = set3.clone();
    let (mut set2, sfbs) = self.copy_of(&set3, num_bits)?;
    let iters = 10000 + random.random_range(0..10000);
    for _i in 0..iters {
      let index = random.random_range(0..num_bits);
      set1.set(index)?;
      set2.set(index)?;
    }
    self.assert_equals(&set1, &set2, num_bits, sfbs.as_ref());
    Ok(())
  }
  fn test_get_and_set<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_bits = 1 + random.random_range(0..100000);
    let set3 = RustUtilBitSet::new(random_set_impl(random, num_bits, 0), num_bits);
    let mut set1 = set3.clone();
    let (mut set2, sfbs) = self.copy_of(&set3, num_bits)?;
    let iters = 10000 + random.random_range(0..10000);
    for _i in 0..iters {
      let index = random.random_range(0..num_bits);
      let v1 = set1.get_and_set(index)?;
      let v2 = set2.get_and_set(index)?;
      assert_eq!(v1, v2);
    }
    self.assert_equals(&set1, &set2, num_bits, sfbs.as_ref());
    Ok(())
  }
  fn test_clear<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_bits = 1 + random.random_range(0..100000);
    for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
      let set3 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
      let mut set1 = set3.clone();
      let (mut set2, _sfbs) = self.copy_of(&set3, num_bits)?;
      let iters = 1 + random.random_range(0..(num_bits * 2));
      for _i in 0..iters {
        let index = random.random_range(0..num_bits);
        set1.clear_with_index(index)?;
        set2.clear_with_index(index)?;
      }
    }
    Ok(())
  }

  fn test_clear_range<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_bits = 1 + random.random_range(0..100000);
    for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
      let set3 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
      let mut set1 = set3.clone();
      let (mut set2, sfbs) = self.copy_of(&set3, num_bits)?;
      let iters = at_least(random, 10);
      for _i in 0..iters {
        let from = random.random_range(0..num_bits);
        let to = random.random_range(0..(num_bits + 1));
        set1.clear_range(from, to)?;
        set2.clear_range(from, to)?;
        self.assert_equals(&set1, &set2, num_bits, sfbs.as_ref());
      }
    }
    Ok(())
  }
  fn test_clear_all<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_bits = 1 + random.random_range(0..100000);
    for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
      let set3 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
      let mut set1 = set3.clone();
      let (mut set2, sfbs) = self.copy_of(&set3, num_bits)?;
      let iters = at_least(random, 10);
      for _i in 0..iters {
        set1.clear()?;
        set2.clear()?;
        self.assert_equals(&set1, &set2, num_bits, sfbs.as_ref());
      }
    }
    Ok(())
  }

  fn test_or_sparse<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.test_or_impl(random, 0.001)
  }
  fn test_or_dense<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.test_or_impl(random, 0.5)
  }
  fn test_or_random<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let random_float: f32 = random.random();
    self.test_or_impl(random, random_float)
  }
  fn random_copy<R>(
    &self,
    random: &mut R,
    set: &RustUtilBitSet,
    num_bits: usize,
  ) -> Result<RandomCopyDocIdSet<Self::TestBitSet>>
  where
    R: Rng + ?Sized,
  {
    match random.random_range(0..5) {
      0 => Ok(RandomCopyDocIdSet::RustUtil(BitDocIdSet::with_cost(
        Some(Arc::new(set.clone())),
        set.cardinality() as i64,
      )?)),
      1 => {
        let (copy, _) = self.copy_of(set, num_bits)?;
        Ok(RandomCopyDocIdSet::Test(BitDocIdSet::with_cost(
          Some(Arc::new(copy)),
          set.cardinality() as i64,
        )?))
      },
      2 => {
        let mut builder = RoaringDocIdSetBuilder::new(num_bits);
        let mut doc = set.next_set_bit(0);
        while doc != NO_MORE_DOCS as usize {
          builder.add(doc as i32)?;
          doc = if doc + 1 >= num_bits {
            NO_MORE_DOCS as usize
          } else {
            set.next_set_bit(doc + 1)
          };
        }
        Ok(RandomCopyDocIdSet::Roaring(builder.build()))
      },
      3 => {
        let mut bit_set = FixedBitSet::new(num_bits);
        let mut iterator = BitSetIterator::new(set.clone(), 0)?;
        BitSet::or(&mut bit_set, &mut iterator)?;
        Ok(RandomCopyDocIdSet::Fixed(BitDocIdSet::new(Some(
          Arc::new(bit_set),
        ))?))
      },
      4 => {
        let mut bit_set = SparseFixedBitSet::new(num_bits)?;
        let mut iterator = BitSetIterator::new(set.clone(), 0)?;
        bit_set.or(&mut iterator)?;
        Ok(RandomCopyDocIdSet::Sparse(BitDocIdSet::new(Some(
          Arc::new(bit_set),
        ))?))
      },
      _ => unreachable!(),
    }
  }

  fn test_or_impl<R>(&self, random: &mut R, load: f32) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_bits = 1 + random.random_range(0..100000);
    let set1 = RustUtilBitSet::new(random_set(random, num_bits, 0.0), num_bits);
    let mut expected = set1.clone();
    let (mut actual, sfbs) = self.copy_of(&set1, num_bits)?;

    let iterations = at_least(random, 10);
    for _ in 0..iterations {
      let bit_set = RustUtilBitSet::new(random_set(random, num_bits, load), num_bits);
      let other_set = self.random_copy(random, &bit_set, num_bits)?;
      expected.or(&mut other_set.iterator()?)?;
      actual.or(&mut other_set.iterator()?)?;
      self.assert_equals(&expected, &actual, num_bits, sfbs.as_ref());
    }
    Ok(())
  }
}

pub enum RandomCopyDocIdSet<T>
where
  T: BitSet,
{
  RustUtil(BitDocIdSet<Arc<RustUtilBitSet>>),
  Test(BitDocIdSet<Arc<T>>),
  Roaring(RoaringDocIdSet),
  Fixed(BitDocIdSet<Arc<FixedBitSet>>),
  Sparse(BitDocIdSet<Arc<SparseFixedBitSet>>),
}

impl<T> Accountable for RandomCopyDocIdSet<T>
where
  T: BitSet,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    match self {
      Self::RustUtil(set) => set.ram_bytes_used(),
      Self::Test(set) => set.ram_bytes_used(),
      Self::Roaring(set) => set.ram_bytes_used(),
      Self::Fixed(set) => set.ram_bytes_used(),
      Self::Sparse(set) => set.ram_bytes_used(),
    }
  }
}

impl<T> DocIdSet for RandomCopyDocIdSet<T>
where
  T: BitSet,
{
  type DocIdSetIterator = DocIdSetIteratorEnum5<
    BitSetIterator<Arc<RustUtilBitSet>>,
    BitSetIterator<Arc<T>>,
    crate::core::util::roaring_doc_id_set::Iterator,
    BitSetIterator<Arc<FixedBitSet>>,
    BitSetIterator<Arc<SparseFixedBitSet>>,
  >;

  fn iterator(&self) -> Result<Self::DocIdSetIterator> {
    match self {
      Self::RustUtil(set) => Ok(DocIdSetIteratorEnum5::A(set.iterator()?)),
      Self::Test(set) => Ok(DocIdSetIteratorEnum5::B(set.iterator()?)),
      Self::Roaring(set) => Ok(DocIdSetIteratorEnum5::C(set.iterator()?)),
      Self::Fixed(set) => Ok(DocIdSetIteratorEnum5::D(set.iterator()?)),
      Self::Sparse(set) => Ok(DocIdSetIteratorEnum5::E(set.iterator()?)),
    }
  }

  type Bits = DummyBits;

  fn bits(&self) -> Option<Self::Bits> {
    None
  }
}

pub trait BaseBitSetTestCaseSupperImpl {
  fn assert_equals<T>(
    &self,
    set1: &RustUtilBitSet,
    set2: &T,
    max_doc: usize,
    _sfbs: Option<&SparseFixedBitSet>,
  ) where
    T: BitSet,
  {
    for i in 0..max_doc {
      assert_eq!(
        set1.get(i).unwrap(),
        set2.get(i).unwrap(),
        "Different at: {}",
        i
      );
    }
  }
}

pub struct RustUtilBitSet {
  bitset: bit_set::BitSet,
  num_bits: usize,
  index_hash_set: HashSet<usize>,
  id: Identity,
}

impl RustUtilBitSet {
  pub(crate) fn new(bitset: bit_set::BitSet, num_bits: usize) -> Self {
    let iter = bitset.iter();
    let mut index_hash_set = HashSet::new();
    for index in iter {
      index_hash_set.insert(index);
    }
    RustUtilBitSet {
      bitset,
      num_bits,
      index_hash_set,
      id: Identity::new(),
    }
  }
}

impl Clone for RustUtilBitSet {
  fn clone(&self) -> Self {
    let bitset = self.bitset.clone();
    let num_bits = self.num_bits;
    let index_hash_set = self.index_hash_set.clone();
    RustUtilBitSet {
      bitset,
      num_bits,
      index_hash_set,
      id: Identity::new(),
    }
  }
}

impl PartialEq for RustUtilBitSet {
  fn eq(&self, other: &Self) -> bool {
    if self.bitset == other.bitset && self.num_bits == other.num_bits {
      return true;
    }
    false
  }
}

impl HasIdentity for RustUtilBitSet {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl Bits for RustUtilBitSet {
  fn get(&self, index: usize) -> Result<bool> {
    Ok(self.bitset.contains(index))
  }

  fn length(&self) -> usize {
    self.num_bits
  }
}

impl Accountable for RustUtilBitSet {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(0)
  }
}

impl BitSet for RustUtilBitSet {
  fn clear(&mut self) -> Result<()> {
    self.bitset.make_empty();
    self.index_hash_set.clear();
    Ok(())
  }

  fn set(&mut self, i: usize) -> Result<()> {
    self.bitset.insert(i);
    self.index_hash_set.insert(i);
    Ok(())
  }

  fn get_and_set(&mut self, i: usize) -> Result<bool> {
    let v = self.get(i)?;
    self.set(i)?;
    Ok(v)
  }

  fn clear_with_index(&mut self, i: usize) -> Result<()> {
    self.bitset.remove(i);
    self.index_hash_set.remove(&i);
    Ok(())
  }

  fn clear_range(&mut self, start_index: usize, end_index: usize) -> Result<()> {
    if start_index >= end_index {
      return Ok(());
    }
    clear_range(&mut self.bitset, start_index, end_index);
    self
      .index_hash_set
      .retain(|index| *index < start_index || *index >= end_index);
    Ok(())
  }

  fn cardinality(&self) -> usize {
    self.bitset.count()
  }

  fn approximate_cardinality(&self) -> usize {
    self.bitset.count()
  }

  fn prev_set_bit(&self, index: usize) -> Option<usize> {
    let mut index = index as i32;
    while index >= 0 {
      if self.bitset.contains((index) as usize) {
        return Option::from(index as usize);
      }
      index -= 1
    }
    None
  }

  fn next_set_bit_range(&self, start: usize, end: usize) -> usize {
    // TODO:: this implement too slow
    for index in start..end {
      if self.index_hash_set.contains(&index) {
        return index;
      }
    }
    NO_MORE_DOCS as usize
  }

  fn or<T>(&mut self, iter: &mut T) -> Result<()>
  where
    T: DocIdSetIterator,
  {
    self.default_or(iter)
  }
}

impl crate::core::util::bit_set::BitSetExtensions for RustUtilBitSet {}
#[test]
fn bit_set_util_equal_and_clone() {
  let mut random = random();
  let num_bits = 10;
  let mut bit1 = bit_set::BitSet::new();
  let mut bit2 = bit_set::BitSet::new();
  let iter = random.random_range(0..100000);
  for i in 0..iter {
    bit1.insert(i);
    bit2.insert(i);
  }
  let bit_set_util1 = RustUtilBitSet::new(bit1, num_bits);
  let bit_set_util2 = RustUtilBitSet::new(bit2, num_bits);
  assert!(bit_set_util1.eq(&bit_set_util2));

  let bit_set_util3 = bit_set_util2.clone();
  assert!(bit_set_util3.eq(&bit_set_util2));
}
