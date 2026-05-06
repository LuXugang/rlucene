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
use rand::Rng;
use rand::prelude::StdRng;

use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::int_array_doc_id_set::IntArrayDocIdSet;
use crate::test::core::util::base_doc_id_set_test_case::{
  BaseDocIdSetTestCase, BaseDocIdSetTestCaseSupperImpl,
};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;

pub struct TestIntArrayDocIdSet;

impl BaseDocIdSetTestCase for TestIntArrayDocIdSet {
  type DocIdSet = IntArrayDocIdSet;

  fn copy_of(&self, bs: &bit_set::BitSet, _length: usize) -> Result<Self::DocIdSet> {
    let mut docs: Vec<i32> = vec![];
    let iter = bs.iter();
    for doc in iter {
      docs.push(doc as i32);
    }
    let l = docs.len() as i32;
    docs.push(NO_MORE_DOCS);
    IntArrayDocIdSet::new(docs, l)
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
    BaseDocIdSetTestCaseSupperImpl::assert_equals(self, random, num_bits, ds1, ds2)
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestIntArrayDocIdSet, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestIntArrayDocIdSet;
  f(&case, &mut random)
}

impl BaseDocIdSetTestCaseSupperImpl for TestIntArrayDocIdSet {}

mod base_doc_id_set_test_case {
  use super::*;

  #[test]
  fn test_bit_0() -> Result<()> {
    run_case(|case, random| case.test_bit_0(random))
  }

  #[test]
  fn test_bit_1() -> Result<()> {
    run_case(|case, random| case.test_bit_1(random))
  }

  #[test]
  fn test_bit_2() -> Result<()> {
    run_case(|case, random| case.test_bit_2(random))
  }

  #[test]
  fn test_against_bit_set() -> Result<()> {
    run_case(|case, random| case.test_against_bit_set(random))
  }

  #[test]
  fn test_ram_bytes_used() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_ram_bytes_used(random);
      Ok(())
    })
  }
}
