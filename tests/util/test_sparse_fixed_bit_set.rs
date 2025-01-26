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
use crate::util::base_bit_set_test_case::{
    BaseBitSetTestCase, BaseBitSetTestCaseSupperImpl, RustUtilBitSet,
};
use crate::util::lucene_test_case::random;
use rand::Rng;
use rlucene::search::doc_id_set_iterator::NO_MORE_DOCS;
use rlucene::util::bit_set::BitSet;
use rlucene::util::bits::Bits;
use rlucene::util::sparse_fixed_bit_set::SparseFixedBitSet;

pub struct TestSparseFixedBitSet;

impl BaseBitSetTestCase for TestSparseFixedBitSet {
    fn copy_of(
        &self,
        bs: &RustUtilBitSet,
        length: i32,
    ) -> (impl BitSet, Option<SparseFixedBitSet>) {
        let mut set = SparseFixedBitSet::new(length).unwrap();
        let mut set1 = SparseFixedBitSet::new(length).unwrap();
        let mut doc = bs.next_set_bit(0);
        while doc != NO_MORE_DOCS {
            set.set(doc);
            set1.set(doc);
            if doc + 1 > length {
                doc = NO_MORE_DOCS;
            } else {
                doc = bs.next_set_bit(doc + 1);
            }
        }
        (set, Some(set1))
    }

    fn assert_equals<T: BitSet>(
        &self,
        set1: &RustUtilBitSet,
        set2: &T,
        max_doc: i32,
        sfbs: &Option<SparseFixedBitSet>,
    ) {
        // check invariants of the sparse set
        let mut non_zero_long_count = 0;
        let sparse_fixed_bit_set = sfbs.as_ref().unwrap();
        let length = sparse_fixed_bit_set.get_indices().len();
        for i in 0..length {
            let n = sparse_fixed_bit_set.get_indices()[i].count_ones();
            if n != 0 {
                non_zero_long_count += n;
                let mut j = n;
                while j < sparse_fixed_bit_set.get_bits()[i].as_ref().unwrap().len() as u32 {
                    let array = sparse_fixed_bit_set.get_bits()[i].as_ref().unwrap();
                    assert_eq!(array[j as usize], 0);
                    j += 1;
                }
            }
        }
        assert_eq!(
            non_zero_long_count,
            sfbs.as_ref().unwrap().get_non_zero_long_count() as u32
        );
        BaseBitSetTestCaseSupperImpl::assert_equals(self, set1, set2, max_doc, sfbs);
    }
}

impl BaseBitSetTestCaseSupperImpl for TestSparseFixedBitSet {}
#[test]
fn test_cardinality() {
    let mut random = random();
    let mut fbs = TestSparseFixedBitSet;
    fbs.test_cardinality(&mut random);
}
#[test]
fn test_prev_set_bit() {
    let mut random = random();
    let mut fbs = TestSparseFixedBitSet;
    fbs.test_prev_set_bit(&mut random);
}
#[test]
fn test_next_set_bit() {
    let mut random = random();
    let mut fbs = TestSparseFixedBitSet;
    fbs.test_next_set_bit(&mut random);
}
#[test]
fn test_next_set_bit_in_range() {
    let mut random = random();
    let mut fbs = TestSparseFixedBitSet;
    fbs.test_next_set_bit_in_range(&mut random);
}
#[test]
fn test_set() {
    let mut random = random();
    let fbs = TestSparseFixedBitSet;
    fbs.test_set(&mut random);
}
#[test]
fn test_get_and_set() {
    let mut random = random();
    let fbs = TestSparseFixedBitSet;
    fbs.test_get_and_set(&mut random);
}
#[test]
fn test_clear() {
    let mut random = random();
    let mut fbs = TestSparseFixedBitSet;
    fbs.test_clear(&mut random);
}
#[test]
fn test_clear_range() {
    let mut random = random();
    let fbs = TestSparseFixedBitSet;
    fbs.test_clear_range(&mut random);
}
#[test]
fn test_clear_all() {
    let mut random = random();
    let fbs = TestSparseFixedBitSet;
    fbs.test_clear_all(&mut random);
}
#[test]
fn test_or_sparse() {
    let mut random = random();
    let mut fbs = TestSparseFixedBitSet;
    fbs.test_or_sparse(&mut random);
}
#[test]
fn test_or_dense() {
    let mut random = random();
    let mut fbs = TestSparseFixedBitSet;
    fbs.test_or_dense(&mut random);
}
#[test]
fn test_or_random() {
    let mut random = random();
    let mut fbs = TestSparseFixedBitSet;
    fbs.test_or_random(&mut random);
}

#[test]
fn test_approximate_cardinality() {
    let mut random = random();
    let mut set = SparseFixedBitSet::new(100).unwrap();
    let first = random.gen_range(1000..10000);
    let interval = 200 + random.gen_range(100..1000);
    let mut i = first;
    while i < set.length() {
        set.set(i);
        i += interval;
    }
    let cardinality = set.cardinality();
    assert!((cardinality - set.approximate_cardinality()).abs() <= 20);
}
#[test]
fn test_approximate_cardinality_on_dense_set() {
    let mut random = random();
    let num_docs = random.gen_range(1..=10000);
    let mut set = SparseFixedBitSet::new(num_docs).unwrap();
    for i in 0..set.length() {
        set.set(i);
    }
    assert_eq!(num_docs, set.approximate_cardinality());
}
#[test]
#[allow(unused)]
fn test_ram_bytes_used() {
    // todo
}
