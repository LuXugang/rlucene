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
use crate::test_framework::core::util::lucene_test_case::is_night_mode;
use rand::Rng;
use rand::RngExt;

use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::TryIntoInt;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::base_bit_set_test_case::random_set;
use crate::test_framework::core::util::test_util::TestUtil;

pub trait BaseDocIdSetTestCase {
  type DocIdSet: DocIdSet;
  fn copy_of<R>(
    &self,
    random: &mut R,
    bs: &bit_set::BitSet,
    length: usize,
  ) -> Result<Self::DocIdSet>
  where
    R: Rng + ?Sized;
  /// Test length=0.
  fn test_bit_0<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let bs = bit_set::BitSet::with_capacity(1);
    let copy = self.copy_of(random, &bs, 1)?;
    self.assert_equals(random, 1, &bs, copy)
  }
  /// Test length=1.
  fn test_bit_1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut bs = bit_set::BitSet::with_capacity(1);
    if random.random_bool(0.5) {
      bs.insert(0);
    }
    let copy = self.copy_of(random, &bs, 1)?;
    self.assert_equals(random, 1, &bs, copy)
  }
  /// Test length=2.
  fn test_bit_2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut bs = bit_set::BitSet::with_capacity(2);
    if random.random_bool(0.5) {
      bs.insert(0);
    }
    if random.random_bool(0.5) {
      bs.insert(1);
    }
    let copy = self.copy_of(random, &bs, 2)?;
    self.assert_equals(random, 2, &bs, copy)
  }
  /// Compares the contents of the set against a [`BitSet`].
  fn test_against_bit_set<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_bits = random.random_range(100..1 << 20) as usize;
    let random_float: f32 = random.random();
    for percent_set in [0f32, 0.0001f32, random_float, 0.9f32, 1f32] {
      let set = random_set(random, num_bits, percent_set);
      let copy = self.copy_of(random, &set, num_bits)?;
      self.assert_equals(random, num_bits, &set, copy)?;
    }
    // test one doc
    let mut set = bit_set::BitSet::with_capacity(num_bits);
    set.insert(0); // 0 first
    let mut copy = self.copy_of(random, &set, num_bits)?;
    self.assert_equals(random, num_bits, &set, copy)?;
    set.remove(0);
    set.insert(random.random_range(0..num_bits));
    copy = self.copy_of(random, &set, num_bits)?;
    self.assert_equals(random, num_bits, &set, copy)?;
    // rest regular increments
    let max_iterations = if is_night_mode() { i32::MAX } else { 10 };
    let mut iterations = 0;
    let mut inc = 2;
    while inc < 1000 {
      if iterations >= max_iterations {
        break;
      }
      iterations += 1;

      set = bit_set::BitSet::with_capacity(num_bits);
      let mut d = random.random_range(0..=10);
      while d < num_bits {
        set.insert(d);
        d += inc;
      }
      copy = self.copy_of(random, &set, num_bits)?;
      self.assert_equals(random, num_bits, &set, copy)?;
      inc += TestUtil::next_usize(random, 1, 100);
    }
    Ok(())
  }
  //TODO
  /// Test ram usage estimation.
  fn test_ram_bytes_used<R>(&self, _random: &mut R)
  where
    R: Rng + ?Sized,
  {
  }
  fn assert_equals<R>(
    &self,
    random: &mut R,
    num_bits: usize,
    ds1: &bit_set::BitSet,
    ds2: Self::DocIdSet,
  ) -> Result<()>
  where
    R: Rng + ?Sized;
}
pub trait BaseDocIdSetTestCaseSupperImpl {
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
    // nextDoc
    let it2 = ds2.iterator()?;
    assert_eq!(-1, it2.doc_id());
    let mut disi = ds2.iterator()?;
    let iter = ds1.iter();
    for doc in iter {
      assert_eq!(doc, disi.next_doc()? as usize);
      assert_eq!(doc, disi.doc_id() as usize);
    }
    assert_eq!(disi.next_doc()?, NO_MORE_DOCS);
    assert_eq!(disi.doc_id(), NO_MORE_DOCS);
    // nextDoc / advance
    let mut disi = ds2.iterator()?;
    let iter = ds1.iter();
    let mut docs = vec![];
    iter.for_each(|doc| docs.push(doc));
    let mut index = 0;
    let mut doc;
    while index < docs.len() {
      if random.random_bool(0.5) {
        assert_eq!(docs[index], disi.next_doc()? as usize);
        assert_eq!(docs[index], disi.doc_id() as usize);
        index += 1;
      } else {
        let skip_length = if random.random_bool(0.5) {
          64
        } else {
          std::cmp::max(num_bits / 8, 1)
        };
        let target = docs[index] + 1 + random.random_range(0..=skip_length) as usize;
        if let Some(i) = docs.iter().position(|&doc| doc == target) {
          index = i + 1;
          doc = target
        } else {
          break;
        }
        assert_eq!(doc as i32, disi.advance(target as i32)?);
        assert_eq!(doc as i32, disi.doc_id());
      }
    }
    if docs.is_empty() {
      assert_eq!(disi.next_doc()?, NO_MORE_DOCS);
    }
    // bits
    let bitss = ds2.bits();
    let mut doc = -1;
    let mut previes_doc: Option<usize> = None;
    if let Some(bits) = bitss {
      let mut disi = ds2.iterator()?;
      while doc != NO_MORE_DOCS {
        doc = disi.next_doc()?;
        let max = if doc == NO_MORE_DOCS {
          bits.length()
        } else {
          doc.try_convert()?
        };

        let mut i = match previes_doc {
          Some(v) => v + 1,
          None => 0,
        };
        while i < max {
          assert!(!bits.get(i)?);
          i += 1;
        }

        if doc == NO_MORE_DOCS {
          break;
        }
        previes_doc = Some(doc as usize);
        assert!(bits.get(doc as usize)?);
      }
    }
    Ok(())
  }
}
