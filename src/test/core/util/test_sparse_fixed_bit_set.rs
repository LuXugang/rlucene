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
use rand::RngExt;
use rand::prelude::StdRng;

use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet;
use crate::test::core::util::base_bit_set_test_case::{
  BaseBitSetTestCase, BaseBitSetTestCaseSupperImpl, RustUtilBitSet,
};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;

pub struct TestSparseFixedBitSet;

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&mut TestSparseFixedBitSet, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let mut case = TestSparseFixedBitSet;
  f(&mut case, &mut random)
}

impl BaseBitSetTestCase for TestSparseFixedBitSet {
  fn copy_of(
    &self,
    bs: &RustUtilBitSet,
    length: usize,
  ) -> (impl BitSet, Option<SparseFixedBitSet>) {
    let mut set = SparseFixedBitSet::new(length).unwrap();
    let mut set1 = SparseFixedBitSet::new(length).unwrap();
    let mut doc = bs.next_set_bit(0);
    while doc != NO_MORE_DOCS as usize {
      set.set(doc);
      set1.set(doc);
      if doc + 1 > length {
        doc = NO_MORE_DOCS as usize;
      } else {
        doc = bs.next_set_bit(doc + 1);
      }
    }
    (set, Some(set1))
  }

  fn assert_equals(
    &self,
    set1: &RustUtilBitSet,
    set2: &impl BitSet,
    max_doc: usize,
    sfbs: Option<&SparseFixedBitSet>,
  ) {
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

mod base_doc_id_set_test_case_util {
  use super::*;

  #[test]
  fn test_cardinality() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_cardinality(random);
      Ok(())
    })
  }

  #[test]
  fn test_prev_set_bit() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_prev_set_bit(random);
      Ok(())
    })
  }

  #[test]
  fn test_next_set_bit() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_next_set_bit(random);
      Ok(())
    })
  }

  #[test]
  fn test_next_set_bit_in_range() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_next_set_bit_in_range(random);
      Ok(())
    })
  }

  #[test]
  fn test_set() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_set(random);
      Ok(())
    })
  }

  #[test]
  fn test_get_and_set() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_get_and_set(random);
      Ok(())
    })
  }

  #[test]
  fn test_clear() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_clear(random);
      Ok(())
    })
  }

  #[test]
  fn test_clear_range() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_clear_range(random);
      Ok(())
    })
  }

  #[test]
  fn test_clear_all() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_clear_all(random);
      Ok(())
    })
  }

  #[test]
  fn test_or_sparse() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_or_sparse(random);
      Ok(())
    })
  }

  #[test]
  fn test_or_dense() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_or_dense(random);
      Ok(())
    })
  }

  #[test]
  fn test_or_random() -> Result<()> {
    run_case(|case, random| {
      let _: () = case.test_or_random(random);
      Ok(())
    })
  }
}

#[test]
fn test_approximate_cardinality() -> Result<()> {
  let mut random = random();
  let mut set = SparseFixedBitSet::new(100)?;
  let first = random.random_range(1000..10000);
  let interval = 200 + random.random_range(100..1000);
  let mut i = first;
  while i < set.length() {
    set.set(i);
    i += interval;
  }
  let cardinality = set.cardinality();
  assert!(cardinality.abs_diff(set.approximate_cardinality()) <= 20);
  Ok(())
}

#[test]
fn test_approximate_cardinality_on_dense_set() -> Result<()> {
  let mut random = random();
  let num_docs = random.random_range(1..=10000);
  let mut set = SparseFixedBitSet::new(num_docs)?;
  for i in 0..set.length() {
    set.set(i);
  }
  assert_eq!(num_docs, set.approximate_cardinality());
  Ok(())
}

#[test]
fn test_ram_bytes_used() {
  // TODO: memory calculation not implement
}
