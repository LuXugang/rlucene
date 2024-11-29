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
use crate::common::my_random;
use crate::util::id_set_common;
use crate::util::id_set_common::clear_range;
use rand::rngs::StdRng;
use rand::Rng;
use rlucene::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use rlucene::util::accountable::Accountable;
use rlucene::util::bit_set::BitSet;
use rlucene::util::bits::Bits;
use rlucene::util::error::runtime_error::RuntimeError;
use rlucene::util::sparse_fixed_bit_set::SparseFixedBitSet;
use std::collections::HashSet;

pub fn random_set(random: &mut StdRng, num_bits: i32, percent_set: f32) -> bit_set::BitSet {
    random_set_impl(random, num_bits, (percent_set * num_bits as f32) as i32)
}

pub fn random_set_impl(random: &mut StdRng, num_bits: i32, num_bits_set: i32) -> bit_set::BitSet {
    assert!(num_bits_set <= num_bits);
    let mut set = bit_set::BitSet::with_capacity(num_bits as usize);
    if num_bits_set == num_bits {
        id_set_common::set_range(&mut set, 0, num_bits as usize)
    } else {
        for _i in 0..num_bits_set {
            loop {
                let o = random.gen_range(0..num_bits);
                if !set.contains(o as usize) {
                    set.insert(o as usize);
                    break;
                }
            }
        }
    }
    set
}
pub trait BaseBitSetTestCase {
    fn copy_of(&self, bs: &RustUtilBitSet, length: i32)
        -> (impl BitSet, Option<SparseFixedBitSet>);
    fn assert_equals<T: BitSet>(
        &self,
        set1: &RustUtilBitSet,
        set2: &T,
        max_doc: i32,
        sfbs: &Option<SparseFixedBitSet>,
    );
    fn test_cardinality(&mut self, random: &mut StdRng) {
        let num_bits = 1 + random.gen_range(0..100000);
        for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
            let set1 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
            let (set2, _sfbs) = self.copy_of(&set1, num_bits);
            assert_eq!(set1.cardinality(), set2.cardinality());
        }
    }
    fn test_prev_set_bit(&mut self, random: &mut StdRng) {
        // TODO: 1000 should be 100000
        let num_bits = 1 + random.gen_range(0..1000);
        for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
            let set1 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
            let (set2, _sfbs) = self.copy_of(&set1, num_bits);
            for i in 0..num_bits {
                assert_eq!(set1.prev_set_bit(i), set2.prev_set_bit(i));
            }
        }
    }
    fn test_next_set_bit(&mut self, random: &mut StdRng) {
        // TODO: 1000 should be 100000
        let num_bits = 1 + random.gen_range(0..1000);
        for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
            let set1 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
            let (set2, _sfbs) = self.copy_of(&set1, num_bits);
            for i in 0..num_bits {
                assert_eq!(set1.next_set_bit(i), set2.next_set_bit(i));
            }
        }
    }
    fn test_next_set_bit_in_range(&mut self, random: &mut StdRng) {
        // TODO: 1000 should be 100000
        let num_bits = 1 + random.gen_range(0..1000);
        for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
            let set1 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
            let (set2, _sfbs) = self.copy_of(&set1, num_bits);
            for start in 0..num_bits {
                let end = if start + 1 == num_bits {
                    num_bits
                } else {
                    random.gen_range(start + 1..num_bits)
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
    }
    fn test_set(&self, random: &mut StdRng) {
        let num_bits = 1 + random.gen_range(0..100000);
        let set3 = RustUtilBitSet::new(random_set_impl(random, num_bits, 0), num_bits);
        let mut set1 = set3.clone();
        let (mut set2, sfbs) = self.copy_of(&set3, num_bits);
        let iters = 10000 + random.gen_range(0..10000);
        for _i in 0..iters {
            let index = random.gen_range(0..num_bits);
            set1.set(index);
            set2.set(index);
        }
        self.assert_equals(&set1, &set2, num_bits, &sfbs);
    }
    fn test_get_and_set(&self, random: &mut StdRng) {
        let num_bits = 1 + random.gen_range(0..100000);
        let set3 = RustUtilBitSet::new(random_set_impl(random, num_bits, 0), num_bits);
        let mut set1 = set3.clone();
        let (mut set2, sfbs) = self.copy_of(&set3, num_bits);
        let iters = 10000 + random.gen_range(0..10000);
        for _i in 0..iters {
            let index = random.gen_range(0..num_bits);
            let v1 = set1.get_and_set(index);
            let v2 = set2.get_and_set(index);
            assert_eq!(v1, v2);
        }
        self.assert_equals(&set1, &set2, num_bits, &sfbs);
    }
    fn test_clear(&mut self, random: &mut StdRng) {
        let num_bits = 1 + random.gen_range(0..100000);
        for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
            let set3 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
            let mut set1 = set3.clone();
            let (mut set2, _sfbs) = self.copy_of(&set3, num_bits);
            let iters = 1 + random.gen_range(0..(num_bits * 2));
            for _i in 0..iters {
                let index = random.gen_range(0..num_bits);
                set1.clear_with_index(index);
                set2.clear_with_index(index);
            }
        }
    }

    fn test_clear_range(&self, random: &mut StdRng) {
        let num_bits = 1 + random.gen_range(0..1000);
        for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
            let set3 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
            let mut set1 = set3.clone();
            let (mut set2, sfbs) = self.copy_of(&set3, num_bits);
            let iters = random.gen_range(10..1000);
            for _i in 0..iters {
                let from = random.gen_range(0..num_bits);
                let to = random.gen_range(0..(num_bits + 1));
                set1.clear_range(from, to);
                set2.clear_range(from, to);
                self.assert_equals(&set1, &set2, num_bits, &sfbs);
            }
        }
    }
    fn test_clear_all(&self, random: &mut StdRng) {
        let num_bits = 1 + random.gen_range(0..100000);
        for percent_set in [0f32, 0.01, 0.1, 0.5, 0.9, 0.99, 1f32] {
            let set3 = RustUtilBitSet::new(random_set(random, num_bits, percent_set), num_bits);
            let mut set1 = set3.clone();
            let (mut set2, sfbs) = self.copy_of(&set3, num_bits);
            let iters = random.gen_range(10..1000);
            for _i in 0..iters {
                set1.clear();
                set2.clear();
                self.assert_equals(&set1, &set2, num_bits, &sfbs);
            }
        }
    }

    fn test_or_sparse(&mut self, random: &mut StdRng) {
        self.test_or_impl(random, 0.001)
    }
    fn test_or_dense(&mut self, random: &mut StdRng) {
        self.test_or_impl(random, 0.5)
    }
    fn test_or_random(&mut self, random: &mut StdRng) {
        let random_float: f32 = random.gen();
        self.test_or_impl(random, random_float)
    }
    fn test_or_impl(&self, _random: &mut StdRng, _load: f32) {
        // let num_bits = 1 + random.gen_range(0..100000);
        // let set1 = RustUtilBitSet::new(random_set(random, num_bits, 0f32), num_bits);
        // let (mut set2, sfbs) = self.copy_of(&set1, num_bits);
        //
        // let iteration = random.gen_range(10..1000);
        // for iter in 0..iteration {
        //     let bitset = RustUtilBitSet::new(random_set(random, num_bits, 0f32), num_bits);
        //     // let other_set = random_copy(random,bitset,num_bits);
        //     todo!()
        // }
    }
}

pub trait BaseBitSetTestCaseSupperImpl {
    fn assert_equals<T: BitSet>(
        &self,
        set1: &RustUtilBitSet,
        set2: &T,
        max_doc: i32,
        _sfbs: &Option<SparseFixedBitSet>,
    ) {
        for i in 0..max_doc {
            assert_eq!(set1.get(i), set2.get(i), "Different at: {}", i);
        }
    }
}

//TODO
#[allow(dead_code)]
fn random_copy(_random: &mut StdRng, _set: impl BitSet, _num_bits: i32) {
    todo!()
}
pub struct RustUtilBitSet {
    bitset: bit_set::BitSet,
    num_bits: i32,
    index_hash_set: HashSet<i32>,
}

impl RustUtilBitSet {
    pub(crate) fn new(bitset: bit_set::BitSet, num_bits: i32) -> Self {
        let iter = bitset.iter();
        let mut index_hash_set = HashSet::new();
        for index in iter {
            index_hash_set.insert(index as i32);
        }
        RustUtilBitSet {
            bitset,
            num_bits,
            index_hash_set,
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

impl Bits for RustUtilBitSet {
    fn get(&self, index: i32) -> bool {
        self.bitset.contains(index as usize)
    }

    fn length(&self) -> i32 {
        self.num_bits
    }
}

impl Accountable for RustUtilBitSet {
    fn ram_bytes_used(&self) -> i64 {
        -1
    }
}

impl BitSet for RustUtilBitSet {
    fn clear(&mut self) {
        self.bitset.clear();
    }

    fn set(&mut self, i: i32) {
        self.bitset.insert(i as usize);
    }

    fn get_and_set(&mut self, index: i32) -> bool {
        let v = self.get(index);
        self.set(index);
        v
    }

    fn clear_with_index(&mut self, i: i32) {
        self.bitset.remove(i as usize);
    }

    fn clear_range(&mut self, start_index: i32, end_index: i32) {
        if start_index >= end_index {
            return;
        }
        clear_range(&mut self.bitset, start_index as usize, end_index as usize);
    }

    fn cardinality(&self) -> i32 {
        self.bitset.len() as i32
    }

    fn approximate_cardinality(&self) -> i32 {
        self.bitset.len() as i32
    }

    fn prev_set_bit(&self, mut index: i32) -> i32 {
        while index >= 0 {
            if self.bitset.contains((index) as usize) {
                return index;
            }
            index -= 1
        }
        -1
    }

    fn next_set_bit_range(&self, start: i32, upper_bound: i32) -> i32 {
        // TODO:: this implement too slow
        for index in start..upper_bound {
            if self.index_hash_set.contains(&index) {
                return index;
            }
        }
        NO_MORE_DOCS
    }

    fn or<T: DocIdSetIterator>(&mut self, _iter: T) -> Result<(), RuntimeError> {
        todo!()
    }
}
#[test]
fn bit_set_util_equal_and_clone() {
    let mut random = my_random("bit_set_util_equal_and_clone".to_string());
    let num_bits = 10;
    let mut bit1 = bit_set::BitSet::new();
    let mut bit2 = bit_set::BitSet::new();
    let iter = random.gen_range(0..100000);
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
