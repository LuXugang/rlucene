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
use crate::search::doc_id_set::DocIdSet;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::test::util::base_bit_set_test_case::random_set;
use crate::test::util::lucene_test_case::is_night_mode;

use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::test::util::test_util::TestUtil;
use crate::util::bits::Bits;
use crate::util::error::lucene_error::Result;
use rand::prelude::StdRng;
use rand::Rng;

pub trait BaseDocIdSetTestCase {
    fn copy_of(&self, bs: &bit_set::BitSet, length: i32) -> impl DocIdSet;
    /// Test length=0.
    fn test_bit_0(&self, random: &mut StdRng) -> Result<()> {
        let bs = bit_set::BitSet::with_capacity(1);
        let copy = self.copy_of(&bs, 1);
        self.assert_equals(random, 1, &bs, copy)
    }
    /// Test length=1.
    fn test_bit_1(&self, random: &mut StdRng) -> Result<()> {
        let mut bs = bit_set::BitSet::with_capacity(1);
        if random.random_bool(0.5) {
            bs.insert(0);
        }
        let copy = self.copy_of(&bs, 1);
        self.assert_equals(random, 1, &bs, copy)
    }
    /// Test length=2.
    fn test_bit_2(&self, random: &mut StdRng) -> Result<()> {
        let mut bs = bit_set::BitSet::with_capacity(2);
        if random.random_bool(0.5) {
            bs.insert(0);
        }
        if random.random_bool(0.5) {
            bs.insert(1);
        }
        let copy = self.copy_of(&bs, 2);
        self.assert_equals(random, 2, &bs, copy)
    }
    /// Compare the content of the set against a {@link BitSet}.
    fn test_against_bit_set(&self, random: &mut StdRng) -> Result<()> {
        let num_bits = random.random_range(100..1 << 20);
        let random_float: f32 = random.random();
        for percent_set in [0f32, 0.0001f32, random_float, 0.9f32, 1f32] {
            let set = random_set(random, num_bits, percent_set);
            let copy = self.copy_of(&set, num_bits);
            self.assert_equals(random, num_bits, &set, copy)?;
        }
        // test one doc
        let mut set = bit_set::BitSet::with_capacity(num_bits as usize);
        set.insert(0); // 0 first
        let mut copy = self.copy_of(&set, num_bits);
        self.assert_equals(random, num_bits, &set, copy)?;
        set.remove(0);
        set.insert(random.random_range(0..num_bits as usize));
        copy = self.copy_of(&set, num_bits);
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

            set = bit_set::BitSet::with_capacity(num_bits as usize);
            let mut d = random.random_range(0..=10);
            while d < num_bits {
                set.insert(d as usize);
                d += inc;
            }
            copy = self.copy_of(&set, num_bits);
            self.assert_equals(random, num_bits, &set, copy)?;
            inc += TestUtil::next_int(random, 1, 100);
        }
        Ok(())
    }
    //TODO
    /// Test ram usage estimation.
    fn test_ram_bytes_used(&self, _random: &mut StdRng) {}
    fn assert_equals(
        &self,
        random: &mut StdRng,
        num_bits: i32,
        ds1: &bit_set::BitSet,
        ds2: impl DocIdSet,
    ) -> Result<()>;
}
// todo
#[allow(unused)]
fn ram_bytes_used(_set: impl DocIdSet, _length: i32) -> i64 {
    0
}
pub trait BaseDocIdSetTestCaseSupperImpl {
    fn assert_equals(
        &self,
        random: &mut StdRng,
        num_bits: i32,
        ds1: &bit_set::BitSet,
        ds2: impl DocIdSet,
    ) -> Result<()> {
        // nextDoc
        let mut it2 = ds2.iterator();
        if it2.is_none() {
            assert!(ds1.is_empty())
        } else {
            assert_eq!(-1, it2.unwrap().doc_id());
            let mut disi = ds2.iterator().unwrap();
            let iter = ds1.iter();
            for doc in iter {
                assert_eq!(doc, disi.next_doc()? as usize);
                assert_eq!(doc, disi.doc_id() as usize);
            }
            assert_eq!(disi.next_doc()?, NO_MORE_DOCS);
            assert_eq!(disi.doc_id(), NO_MORE_DOCS);
        }

        // nextDoc / advance
        it2 = ds2.iterator();
        if it2.is_none() {
            assert!(ds1.is_empty())
        } else {
            let mut disi = it2.unwrap();
            let iter = ds1.iter();
            let mut docs = vec![];
            iter.for_each(|doc| docs.push(doc));
            let mut index = 0;
            let mut _doc = 0;
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
                        _doc = target
                    } else {
                        _doc = NO_MORE_DOCS as usize;
                        break;
                    }
                    assert_eq!(_doc as i32, disi.advance(target as i32)?);
                    assert_eq!(_doc as i32, disi.doc_id());
                }
            }
        }
        // bits)
        let bitss = ds2.bits();
        let mut doc = -1;
        let mut previes_doc = -1;
        if bitss.is_some() {
            let bits = bitss.unwrap();
            let mut disi = ds2.iterator().unwrap();
            while doc != NO_MORE_DOCS {
                let mut _i = 0;
                doc = disi.next_doc()?;
                let max = if doc == NO_MORE_DOCS {
                    bits.length()
                } else {
                    doc
                };
                _i = previes_doc + 1;
                while _i < max {
                    assert!(!bits.get(_i));
                    _i += 1;
                }
                if doc == NO_MORE_DOCS {
                    break;
                }
                previes_doc = doc;
                assert!(bits.get(doc));
            }
        }
        Ok(())
    }
}
