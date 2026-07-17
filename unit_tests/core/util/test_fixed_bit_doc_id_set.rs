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
use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;
use rand::prelude::StdRng;

use crate::core::util::bit_doc_id_set::BitDocIdSet;
use crate::core::util::bit_set::BitSet;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::util_tests::base_doc_id_set_test_case::{
  BaseDocIdSetTestCase, BaseDocIdSetTestCaseSupperImpl,
};

pub struct TestFixedBitDocIdSet;

impl BaseDocIdSetTestCase for TestFixedBitDocIdSet {
  type DocIdSet = BitDocIdSet<FixedBitSet>;

  fn copy_of<R>(
    &self,
    _random: &mut R,
    bs: &bit_set::BitSet,
    length: usize,
  ) -> Result<Self::DocIdSet>
  where
    R: Rng + ?Sized,
  {
    let mut set = FixedBitSet::new(length);
    let iter = bs.iter();
    for doc in iter {
      set.set(doc);
    }
    BitDocIdSet::new(Some(set))
  }

  fn assert_equals<R>(
    &self,
    random: &mut R,
    num_bits: usize,
    ds1: &bit_set::BitSet,
    ds2: Self::DocIdSet,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    BaseDocIdSetTestCaseSupperImpl::assert_equals(self, random, num_bits, ds1, ds2)
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestFixedBitDocIdSet, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestFixedBitDocIdSet;
  f(&case, &mut random)
}

impl BaseDocIdSetTestCaseSupperImpl for TestFixedBitDocIdSet {}

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
