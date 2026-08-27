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
use std::sync::Arc;

use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;
use rand::prelude::StdRng;
use rand::seq::SliceRandom;

use crate::core::search::doc_id_set::DocIdSet;
use crate::core::util::bit_doc_id_set::BitDocIdSet;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet;
use crate::test::core::util::base_doc_id_set_test_case::{
  BaseDocIdSetTestCase, BaseDocIdSetTestCaseSupperImpl,
};

#[allow(dead_code)] // for quick search
struct TestSparseFixedBitDocIdSet;

impl BaseDocIdSetTestCase for TestSparseFixedBitDocIdSet {
  type DocIdSet = BitDocIdSet<Arc<SparseFixedBitSet>>;

  fn copy_of<R>(
    &self,
    random: &mut R,
    bs: &bit_set::BitSet,
    length: usize,
  ) -> Result<Self::DocIdSet>
  where
    R: Rng + ?Sized,
  {
    let mut set = SparseFixedBitSet::new(length)?;
    // SparseFixedBitSet can be sensitive to the order of insertion so
    // randomize insertion a bit
    let mut buffer = Vec::new();
    for doc in bs.iter() {
      buffer.push(doc);
      if buffer.len() >= 100_000 {
        buffer.shuffle(random);
        for &i in &buffer {
          set.set(i)?;
        }
        buffer.clear();
      }
    }
    buffer.shuffle(random);
    for i in buffer {
      set.set(i)?;
    }
    let cost = set.approximate_cardinality() as i64;
    BitDocIdSet::with_cost(Some(Arc::new(set)), cost)
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
    let bits = ds2
      .bits()
      .ok_or_else(|| LuceneError::illegal_state("bits must not be None"))?;
    for i in 0..num_bits {
      assert_eq!(ds1.contains(i), bits.get(i)?);
    }
    assert_eq!(ds1.count(), bits.cardinality());
    BaseDocIdSetTestCaseSupperImpl::assert_equals(self, random, num_bits, ds1, ds2)
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestSparseFixedBitDocIdSet, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestSparseFixedBitDocIdSet;
  f(&case, &mut random)
}

impl BaseDocIdSetTestCaseSupperImpl for TestSparseFixedBitDocIdSet {}

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
