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
use crate::core::index::index_reader::Identity;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::HasIdentity;
use crate::core::util::accountable::Accountable;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;

const BASE_RAM_BYTES_USED: i64 = 0;
/// This [`DocIdSet`] encodes the negation of another
/// [`DocIdSet`]. It is cacheable and supports random-access
/// if the underlying set is cacheable and supports random-access.
///
/// # Note
/// This is an internal API.
pub struct NotDocIdSet<T>
where
  T: DocIdSet,
{
  max_doc: i32,
  set: T,
}

impl<T> NotDocIdSet<T>
where
  T: DocIdSet,
{
  pub fn new(max_doc: i32, set: T) -> Self {
    NotDocIdSet { max_doc, set }
  }
}

impl<T> Accountable for NotDocIdSet<T>
where
  T: DocIdSet,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    todo!()
  }
}

impl<T> DocIdSet for NotDocIdSet<T>
where
  T: DocIdSet,
{
  type DocIdSetIterator = NotDocDocIdSetIterator<T::DocIdSetIterator>;

  fn iterator(&self) -> Result<Self::DocIdSetIterator> {
    Ok(NotDocDocIdSetIterator::new(
      self.set.iterator()?,
      self.max_doc,
    ))
  }

  type Bits = NotDocIdBits<T::Bits>;

  fn bits(&self) -> Option<Self::Bits> {
    self.set.bits().map(NotDocIdBits::new)
  }
}

#[derive(Clone)]
pub struct NotDocIdBits<B>
where
  B: Bits,
{
  in_bit: B,
  id: Identity,
}

impl<B> NotDocIdBits<B>
where
  B: Bits,
{
  pub fn new(in_bits: B) -> NotDocIdBits<B> {
    NotDocIdBits {
      in_bit: in_bits,
      id: Identity::new(),
    }
  }
}

impl<B> HasIdentity for NotDocIdBits<B>
where
  B: Bits,
{
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<B> Bits for NotDocIdBits<B>
where
  B: Bits,
{
  fn get(&self, index: usize) -> Result<bool> {
    Ok(!self.in_bit.get(index)?)
  }

  fn length(&self) -> usize {
    self.in_bit.length()
  }
}

pub struct NotDocDocIdSetIterator<D: DocIdSetIterator> {
  in_iterator: D,
  doc: i32,
  next_skipped_doc: i32,
  max_doc: i32,
}

impl<D: DocIdSetIterator> NotDocDocIdSetIterator<D> {
  fn new(in_iterator: D, max_doc: i32) -> Self {
    NotDocDocIdSetIterator {
      in_iterator,
      doc: -1,
      next_skipped_doc: -1,
      max_doc,
    }
  }
}

impl<D: DocIdSetIterator> DocIdSetIterator for NotDocDocIdSetIterator<D> {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.advance(self.doc + 1)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.doc = target;
    if self.doc > self.next_skipped_doc {
      self.next_skipped_doc = self.in_iterator.advance(self.doc)?;
    }
    loop {
      if self.doc >= self.max_doc {
        self.doc = NO_MORE_DOCS;
        break;
      }
      debug_assert!(self.doc <= self.next_skipped_doc);
      if self.doc != self.next_skipped_doc {
        return Ok(self.doc);
      }
      self.doc += 1;
      self.next_skipped_doc = self.in_iterator.next_doc()?;
    }
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.max_doc as i64)
  }
}
#[cfg(test)]
mod tests {
  use std::cmp::Reverse;
  use std::collections::BinaryHeap;

  use rand::Rng;

  use crate::core::search::doc_id_set::{DocIdSet, EmptyDocIdSet};
  use crate::core::util::bit_doc_id_set::BitDocIdSet;
  use crate::core::util::bit_set::BitSet;
  use crate::core::util::bits::Bits;
  use crate::core::util::error::lucene_error::{LuceneError, Result};
  use crate::core::util::fixed_bit_set::FixedBitSet;
  use crate::core::util::not_doc_id_set::NotDocIdSet;
  use crate::test::core::util::base_doc_id_set_test_case::{
    BaseDocIdSetTestCase, BaseDocIdSetTestCaseSupperImpl,
  };
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;

  struct TestNotDocIdSet;
  impl BaseDocIdSetTestCase for TestNotDocIdSet {
    type DocIdSet = NotDocIdSet<BitDocIdSet<FixedBitSet>>;

    fn copy_of(&self, bs: &bit_set::BitSet, length: usize) -> Result<Self::DocIdSet> {
      let mut set = FixedBitSet::new(length);
      for i in 0..length {
        if !bs.contains(i) {
          set.set(i);
        }
      }
      let bit_doc_id_set = BitDocIdSet::new(Some(set))?;
      Ok(NotDocIdSet::new(length as i32, bit_doc_id_set))
    }

    fn assert_equals<R>(
      &self,
      random: &mut R,
      num_bits: usize,
      ds1: &bit_set::BitSet,
      ds2: impl DocIdSet,
    ) -> Result<()>
    where
      R: Rng + ?Sized,
    {
      let bits = ds2.bits().ok_or_else(|| LuceneError::illegal_state(""))?;
      assert_eq!(num_bits, bits.length());
      for i in 0..num_bits {
        assert_eq!(ds1.contains(i), bits.get(i)?);
      }
      BaseDocIdSetTestCaseSupperImpl::assert_equals(self, random, num_bits, ds1, ds2)
    }
  }

  #[test]
  fn test_bit_0() -> Result<()> {
    let test_case = TestNotDocIdSet;
    let mut random = random();
    test_case.test_bit_0(&mut random)
  }
  #[test]
  fn test_bit_1() -> Result<()> {
    let test_case = TestNotDocIdSet;
    let mut random = random();
    test_case.test_bit_1(&mut random)
  }
  #[test]
  fn test_bit_2() -> Result<()> {
    let test_case = TestNotDocIdSet;
    let mut random = random();
    test_case.test_bit_2(&mut random)
  }
  #[test]
  fn test_against_bit_set() -> Result<()> {
    let test_case = TestNotDocIdSet;
    let mut random = random();
    test_case.test_against_bit_set(&mut random)
  }
  #[test]
  fn test_ram_bytes_used() {
    let test_case = TestNotDocIdSet;
    let mut random = random();
    test_case.test_ram_bytes_used(&mut random);
  }

  impl BaseDocIdSetTestCaseSupperImpl for TestNotDocIdSet {}
  #[test]
  fn test_bits() -> Result<()> {
    assert!(NotDocIdSet::new(3, EmptyDocIdSet).bits().is_none());
    assert!(
      NotDocIdSet::new(3, BitDocIdSet::new(Some(FixedBitSet::new(3)))?)
        .bits()
        .is_some()
    );
    Ok(())
  }
  struct Buffer {
    array: Vec<i32>,
  }
  #[test]
  fn main() {
    let buffers = vec![
      Buffer {
        array: vec![3, 1, 4],
      },
      Buffer {
        array: vec![5, 9, 2],
      },
      Buffer {
        array: vec![6, 8, 7],
      },
    ];
    let mut heap = BinaryHeap::new();
    let mut iterators: Vec<_> = buffers
      .into_iter()
      .map(|buffer| buffer.array.into_iter())
      .collect();

    for (i, it) in iterators.iter_mut().enumerate() {
      if let Some(value) = it.next() {
        heap.push(Reverse((value, i)));
      }
    }

    let mut merged_array = Vec::new();

    while let Some(Reverse((value, i))) = heap.pop() {
      merged_array.push(value);
      if let Some(next_value) = iterators[i].next() {
        heap.push(Reverse((next_value, i)));
      }
    }

    println!("{:?}", merged_array);
  }
}
