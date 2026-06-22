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
use crate::test::core::util::lucene_test_case::random;
use rand::Rng;
use rand::prelude::StdRng;

use crate::core::search::doc_id_set::DocIdSet;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::roaring_doc_id_set::Builder;
use crate::core::util::roaring_doc_id_set::RoaringDocIdSet;
use crate::test::core::util::base_doc_id_set_test_case::{
  BaseDocIdSetTestCase, BaseDocIdSetTestCaseSupperImpl,
};

pub struct TestRoaringDocIdSet;

impl BaseDocIdSetTestCase for TestRoaringDocIdSet {
  type DocIdSet = RoaringDocIdSet;

  fn copy_of(&self, bs: &bit_set::BitSet, length: usize) -> Result<Self::DocIdSet> {
    let mut builder = Builder::new(length);
    let iter = bs.iter();
    for doc in iter {
      builder.add(doc as i32)?;
    }
    Ok(builder.build())
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
  F: FnOnce(&TestRoaringDocIdSet, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestRoaringDocIdSet;
  f(&case, &mut random)
}

impl BaseDocIdSetTestCaseSupperImpl for TestRoaringDocIdSet {}

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
