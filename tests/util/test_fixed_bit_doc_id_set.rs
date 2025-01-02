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
use rand::prelude::StdRng;
use rlucene::search::doc_id_set::DocIdSet;
use rlucene::util::bit_doc_id_set::BitDocIdSet;
use rlucene::util::bit_set::BitSet;
use rlucene::util::fixed_bit_set::FixedBitSet;
use crate::util::test_error::TestError;

impl BaseDocIdSetTestCase for TestFixedBitDocIdSet {
    fn copy_of(&self, bs: &bit_set::BitSet, length: i32) -> impl DocIdSet {
        let mut set = FixedBitSet::new(length);
        let iter = bs.iter();
        for doc in iter {
            set.set(doc as i32);
        }
        let result = BitDocIdSet::new(Some(set));
        assert!(result.is_ok());
        result.unwrap()
    }

    fn assert_equals<T: DocIdSet>(
        &self,
        random: &mut StdRng,
        num_bits: i32,
        ds1: &bit_set::BitSet,
        ds2: T,
    )->Result<(),TestError> {
        BaseDocIdSetTestCaseSupperImpl::assert_equals(self, random, num_bits, ds1, ds2)
    }
}

pub struct TestFixedBitDocIdSet;

#[test]
fn test_bit_0() ->Result<(),TestError> {
    let test_case = TestFixedBitDocIdSet;
    let mut random = my_random("test_fixed_bit_doc_id_set".to_string());
    test_case.test_bit_0(&mut random)
}
#[test]
fn test_bit_1() ->Result<(),TestError> {
    let test_case = TestFixedBitDocIdSet;
    let mut random = my_random("test_fixed_bit_doc_id_set".to_string());
    test_case.test_bit_1(&mut random)
}
#[test]
fn test_bit_2() ->Result<(),TestError> {
    let test_case = TestFixedBitDocIdSet;
    let mut random = my_random("test_fixed_bit_doc_id_set".to_string());
    test_case.test_bit_2(&mut random)
}
#[test]
fn test_against_bit_set()  ->Result<(),TestError>{
    let test_case = TestFixedBitDocIdSet;
    let mut random = my_random("test_fixed_bit_doc_id_set".to_string());
    test_case.test_against_bit_set(&mut random)
}
#[test]
fn test_ram_bytes_used() {
    let test_case = TestFixedBitDocIdSet;
    let mut random = my_random("test_fixed_bit_doc_id_set".to_string());
    test_case.test_ram_bytes_used(&mut random);
}

impl BaseDocIdSetTestCaseSupperImpl for TestFixedBitDocIdSet {}
