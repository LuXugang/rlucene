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
use crate::util::base_doc_id_set_test_case::{
    BaseDocIdSetTestCase, BaseDocIdSetTestCaseSupperImpl,
};
use rand::rngs::StdRng;
use rlucene::search::doc_id_set::{DocIdSet, EmptyDocIdSet};
use rlucene::util::bit_doc_id_set::BitDocIdSet;
use rlucene::util::bit_set::BitSet;
use rlucene::util::bits::Bits;
use rlucene::util::fixed_bit_set::FixedBitSet;
use rlucene::util::not_doc_id_set::NotDocIdSet;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use crate::util::test_error::TestError;

struct TestNotDocIdSet;
impl BaseDocIdSetTestCase for TestNotDocIdSet {
    fn copy_of(&self, bs: &bit_set::BitSet, length: i32) -> impl DocIdSet {
        let mut set = FixedBitSet::new(length);
        for i in 0..length {
            if !bs.contains(i as usize) {
                set.set(i);
            }
        }
        let bit_doc_id_set = BitDocIdSet::new(Some(set)).unwrap();
        NotDocIdSet::new(length, bit_doc_id_set)
    }

    fn assert_equals<T: DocIdSet>(
        &self,
        random: &mut StdRng,
        num_bits: i32,
        ds1: &bit_set::BitSet,
        ds2: T,
    )->Result<(),TestError> {
        let bits2_wrap = ds2.bits();
        assert!(bits2_wrap.is_some());
        let bits = bits2_wrap.unwrap();
        assert_eq!(num_bits, bits.length());
        for i in 0..num_bits {
            assert_eq!(ds1.contains(i as usize), bits.get(i));
        }
        BaseDocIdSetTestCaseSupperImpl::assert_equals(self, random, num_bits, ds1, ds2)
    }
}

#[test]
fn test_bit_0() ->Result<(),TestError> {
    let test_case = TestNotDocIdSet;
    let mut random = my_random("test_not_doc_id_set".to_string());
    test_case.test_bit_0(&mut random)
}
#[test]
fn test_bit_1() ->Result<(),TestError> {
    let test_case = TestNotDocIdSet;
    let mut random = my_random("test_not_doc_id_set".to_string());
    test_case.test_bit_1(&mut random)
}
#[test]
fn test_bit_2() ->Result<(),TestError> {
    let test_case = TestNotDocIdSet;
    let mut random = my_random("test_not_doc_id_set".to_string());
    test_case.test_bit_2(&mut random)
}
#[test]
fn test_against_bit_set() ->Result<(),TestError> {
    let test_case = TestNotDocIdSet;
    let mut random = my_random("test_not_doc_id_set".to_string());
    test_case.test_against_bit_set(&mut random)
}
#[test]
fn test_ram_bytes_used() {
    let test_case = TestNotDocIdSet;
    let mut random = my_random("test_not_doc_id_set".to_string());
    test_case.test_ram_bytes_used(&mut random);
}

impl BaseDocIdSetTestCaseSupperImpl for TestNotDocIdSet {}
#[test]
fn test_bits() {
    assert!(NotDocIdSet::new(3, EmptyDocIdSet).bits().is_none());
    assert!(
        NotDocIdSet::new(3, BitDocIdSet::new(Some(FixedBitSet::new(3))).unwrap())
            .bits()
            .is_some()
    );
}
struct Buffer {
    array: Vec<i32>,
}
#[test]
fn main() {
    // 假设有一个 Vec<Buffer>
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

    // 初始化最小堆
    for (i, it) in iterators.iter_mut().enumerate() {
        if let Some(value) = it.next() {
            heap.push(Reverse((value, i))); // 用 Reverse 实现小顶堆
        }
    }

    let mut merged_array = Vec::new();

    // 多路归并
    while let Some(Reverse((value, i))) = heap.pop() {
        merged_array.push(value);
        if let Some(next_value) = iterators[i].next() {
            heap.push(Reverse((next_value, i)));
        }
    }

    // 输出排序结果
    println!("{:?}", merged_array);
}
