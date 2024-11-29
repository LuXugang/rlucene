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
use rlucene::search::doc_id_set::DocIdSet;
use rlucene::search::doc_id_set_iterator::NO_MORE_DOCS;
use rlucene::util::int_array_doc_id_set::IntArrayDocIdSet;

struct TestIntArrayDocIdSet;
impl BaseDocIdSetTestCase for TestIntArrayDocIdSet {
    fn copy_of(&self, bs: &bit_set::BitSet, _length: i32) -> impl DocIdSet {
        let mut docs: Vec<i32> = vec![];
        let iter = bs.iter();
        for doc in iter {
            docs.push(doc as i32);
        }
        let l = docs.len() as i32;
        docs.push(NO_MORE_DOCS);
        let result = IntArrayDocIdSet::new(docs, l);
        assert!(result.is_ok());
        result.unwrap()
    }

    fn assert_equals<T: DocIdSet>(
        &self,
        random: &mut StdRng,
        num_bits: i32,
        ds1: &bit_set::BitSet,
        ds2: T,
    ) {
        BaseDocIdSetTestCaseSupperImpl::assert_equals(self, random, num_bits, ds1, ds2);
    }
}
#[test]
fn test_bit_0() {
    let test_case = TestIntArrayDocIdSet;
    let mut random = my_random("test_int_array_doc_id_set".to_string());
    test_case.test_bit_0(&mut random);
}

#[test]
fn test_bit_1() {
    let test_case = TestIntArrayDocIdSet;
    let mut random = my_random("test_int_array_doc_id_set".to_string());
    test_case.test_bit_1(&mut random);
}
#[test]
fn test_bit_2() {
    let test_case = TestIntArrayDocIdSet;
    let mut random = my_random("test_int_array_doc_id_set".to_string());
    test_case.test_bit_2(&mut random);
}
#[test]
fn test_against_bit_set() {
    let test_case = TestIntArrayDocIdSet;
    let mut random = my_random("test_int_array_doc_id_set".to_string());
    test_case.test_against_bit_set(&mut random);
}
#[test]
fn test_ram_bytes_used() {
    let test_case = TestIntArrayDocIdSet;
    let mut random = my_random("test_int_array_doc_id_set".to_string());
    test_case.test_ram_bytes_used(&mut random);
}

impl BaseDocIdSetTestCaseSupperImpl for TestIntArrayDocIdSet {}
