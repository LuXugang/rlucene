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
// Migrated from src/core/util/doc_id_set_builder.rs

use crate::core::codecs::dummy::dummy_mutable_point_tree::DummyMutablePointTree;
use crate::core::index::BytesRef;
use crate::core::index::dummy::dummy_point_tree::DummyPointTree;
use crate::core::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::core::index::point_values::{PointTreeEnum, PointValues};
use crate::core::index::terms::Terms;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, RangeDISI};
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::bit_doc_id_set::BitDocIdSet;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::bits::Bits;
use crate::core::util::doc_id_set_builder::{DocIdSetBuilder, DocIdSetBuilderEnum};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::int_array_doc_id_set::IntArrayDocIdSet;
use crate::core::util::roaring_doc_id_set::Builder;
use crate::test_framework::core::util::lucene_test_case::{is_night_mode, random, rarely};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::RngExt;
use std::borrow::Cow;

#[allow(dead_code)] // for quick search
struct TestDocIdSetBuilder;
#[test]
fn test_empty() -> Result<()> {
  let mut random = random();
  let max_doc = random.random_range(1..1000);
  let doc_id_set: Option<IntArrayDocIdSet> = None;
  assert_equals(doc_id_set, Some(DocIdSetBuilder::new(max_doc).build()?))?;
  Ok(())
}
fn assert_equals<T1, T2>(mut d1: Option<T1>, mut d2: Option<T2>) -> Result<()>
where
  T1: DocIdSet,
  T2: DocIdSet,
{
  match (d1.as_mut(), d2.as_mut()) {
    (None, None) => {
      unreachable!("")
    },

    (None, Some(d2v)) => {
      assert_eq!(d2v.iterator()?.next_doc()?, NO_MORE_DOCS);
    },

    (Some(d1v), None) => {
      assert_eq!(d1v.iterator()?.next_doc()?, NO_MORE_DOCS);
    },

    (Some(d1v), Some(d2v)) => {
      let mut i1 = d1v.iterator()?;
      let mut i2 = d2v.iterator()?;

      let mut doc = i1.next_doc()?;
      while doc != NO_MORE_DOCS {
        assert_eq!(doc, i2.next_doc()?);
        doc = i1.next_doc()?;
      }
      assert_eq!(i2.next_doc()?, NO_MORE_DOCS);
    },
  }

  Ok(())
}

#[test]
fn test_sparse() -> Result<()> {
  let mut random = random();
  let max_doc = 1000000 + random.random_range(0..1000000);
  let mut builder = DocIdSetBuilder::new(max_doc);
  let num_iterators = 1 + random.random_range(0..10);
  let mut fixed_set_bit = FixedBitSet::new(max_doc as usize);
  for _i in 0..num_iterators {
    let base_inc = 200000 + random.random_range(0..10000);
    let mut b = Builder::new(max_doc as usize);
    let mut doc = random.random_range(0..100);
    while doc < max_doc {
      b.add(doc)?;
      fixed_set_bit.set(doc as usize);
      doc += base_inc + random.random_range(0..10000);
    }
    let roaring_doc_id_set = b.build();
    let mut iter = roaring_doc_id_set.iterator()?;
    builder.add_disi(&mut iter)?;
  }
  let result = builder.build()?;
  assert!(matches!(result, DocIdSetBuilderEnum::IntArray(_)));
  let bit_doc_id_set = BitDocIdSet::new(Some(fixed_set_bit))?;
  assert_equals(Some(bit_doc_id_set), Some(result))?;
  Ok(())
}
#[test]
fn test_dense() -> Result<()> {
  let mut random = random();
  let max_doc = 1000000 + random.random_range(0..1000000);
  let mut builder = DocIdSetBuilder::new(max_doc);
  let num_iterators = 1 + random.random_range(0..10);
  let mut fixed_set_bit = FixedBitSet::new(max_doc as usize);
  for _i in 0..num_iterators {
    let mut b = Builder::new(max_doc as usize);
    let mut doc = random.random_range(0..1000);
    while doc < max_doc {
      b.add(doc)?;
      fixed_set_bit.set(doc as usize);
      doc += 1 + random.random_range(0..100);
    }
    let roaring_doc_id_set = b.build();
    let mut iter = roaring_doc_id_set.iterator()?;
    builder.add_disi(&mut iter)?;
  }
  let result = builder.build()?;
  assert!(matches!(result, DocIdSetBuilderEnum::BitDoc(_)));
  let bit_doc_id_set = BitDocIdSet::new(Some(fixed_set_bit))?;
  assert_equals(Some(bit_doc_id_set), Some(result))?;
  Ok(())
}

#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let max_doc = if is_night_mode() {
    TestUtil::next_int(&mut random, 1, 10000000)
  } else {
    TestUtil::next_int(&mut random, 1, 100000)
  };
  let mut i = 1;
  while i < (max_doc / 2) {
    let num_docs = TestUtil::next_int(&mut random, 1, i);
    let mut docs = FixedBitSet::new(max_doc as usize);
    let mut c = 0;
    while c < num_docs {
      let d = random.random_range(0..max_doc);
      if !docs.get(d as usize)? {
        docs.set(d as usize);
        c += 1
      }
    }
    let mut array = vec![0; num_docs as usize + random.random_range(0..100)];
    let (mut j, v) = {
      let mut it = BitSetIterator::new(docs, 0)?;
      let mut j = 0;
      let mut doc = it.next_doc()?;
      while doc != NO_MORE_DOCS {
        array[j] = doc;
        j += 1;
        doc = it.next_doc()?;
      }
      (j, it.get_bit_set().clone())
    };

    let docs = v;
    assert_eq!(num_docs, j as i32);
    // add some duplicates
    while j < array.len() {
      array[j] = array[random.random_range(0..num_docs as usize)];
      j += 1;
    }

    // shuffle
    for j in (1..array.len()).rev() {
      let k = random.random_range(0..j);
      array.swap(j, k);
    }

    // add docs out of order
    let mut builder = DocIdSetBuilder::new(max_doc);
    let mut j = 0;
    while j < array.len() {
      let l = TestUtil::next_int(&mut random, 1, (array.len() - j) as i32);
      let mut k = 0;
      let mut budget = 0;
      while k < l {
        let rarely = rarely(&mut random);
        if budget == 0 || rarely {
          budget = TestUtil::next_int(&mut random, 1, l - k + 5);
          builder.grow(budget);
        }
        builder.add_doc(array[j]);
        budget -= 1;
        k += 1;
        j += 1;
      }
    }
    i <<= 1;
    let expected = BitDocIdSet::new(Some(docs))?;
    let actual = builder.build()?;
    assert_equals(Some(expected), Some(actual))?;
  }
  Ok(())
}
#[test]
fn test_misleading_disi_cost() -> Result<()> {
  let mut random = random();
  let max_doc = TestUtil::next_int(&mut random, 1000, 10000);
  let mut builder = DocIdSetBuilder::new(max_doc);
  let mut expected = FixedBitSet::new(max_doc as usize);
  for _i in 0..100 {
    let mut docs = FixedBitSet::new(max_doc as usize);
    let num_docs = random.random_range(1..=max_doc / 1000);
    for _ in 0..num_docs {
      let doc = random.random_range(0..max_doc);
      docs.set(doc as usize);
    }
    expected.or(&docs);
    // We provide a cost of 0 here to make sure the builder can deal
    // with wrong costs
    let mut bit_doc_id_set = BitSetIterator::new(docs, 0)?;
    builder.add_disi(&mut bit_doc_id_set)?;
  }
  let bit_doc_id_set = BitDocIdSet::new(Some(expected))?;
  assert_equals(Some(bit_doc_id_set), Some(builder.build()?))?;
  Ok(())
}
#[test]
fn test_empty_points() -> Result<()> {
  let values = DummyPointValues::new(0, 0);
  let builder = DocIdSetBuilder::from_point_values(1, &values, "foo")?;
  assert_eq!(1.0_f64, builder.get_num_values_per_doc());
  Ok(())
}

#[test]
fn test_leverage_stats() -> Result<()> {
  // single-valued points
  let values = DummyPointValues::new(42, 42);
  let mut builder = DocIdSetBuilder::from_point_values(100, &values, "foo")?;
  assert_eq!(1.0_f64, builder.get_num_values_per_doc());
  assert!(!builder.get_multi_valued());

  {
    builder.grow(2);
    builder.add_doc(5);
    builder.add_doc(7);
  }

  let set = builder.build()?;
  assert!(matches!(set, DocIdSetBuilderEnum::BitDoc(_)));

  let it = set.iterator()?;
  assert_eq!(2, it.cost()?);

  // multi-valued points
  let values = DummyPointValues::new(42, 63);
  let mut builder = DocIdSetBuilder::from_point_values(100, &values, "foo")?;
  assert_eq!(1.5_f64, builder.get_num_values_per_doc());
  assert!(builder.get_multi_valued());

  builder.grow(2);
  builder.add_doc(5);
  builder.add_doc(7);

  let set = builder.build()?;
  assert!(matches!(set, DocIdSetBuilderEnum::BitDoc(_)));

  let it = set.iterator()?;
  // it thinks the same doc was added twice
  assert_eq!(1, it.cost()?);

  let values = DummyPointValues::new(42, -1);
  let builder = DocIdSetBuilder::from_point_values(100, &values, "foo");
  assert!(builder.is_err());

  // incomplete stats: doc_count unknown
  let values = DummyPointValues::new(-1, 84);
  let builder = DocIdSetBuilder::from_point_values(100, &values, "foo")?;
  assert_eq!(1.0_f64, builder.get_num_values_per_doc());
  assert!(builder.get_multi_valued());

  // single-valued terms
  let terms = DummyTerms::new(42, 42);
  let mut builder = DocIdSetBuilder::from_terms(100, &terms)?;
  assert_eq!(1.0_f64, builder.get_num_values_per_doc());
  assert!(!builder.get_multi_valued());

  builder.grow(2);
  builder.add_doc(5);
  builder.add_doc(7);

  let set = builder.build()?;
  assert!(matches!(set, DocIdSetBuilderEnum::BitDoc(_)));

  let it = set.iterator()?;
  assert_eq!(2, it.cost()?);

  // multi-valued terms
  let terms = DummyTerms::new(42, 63);
  let mut builder = DocIdSetBuilder::from_terms(100, &terms)?;
  assert_eq!(1.5_f64, builder.get_num_values_per_doc());
  assert!(builder.get_multi_valued());

  builder.grow(2);
  builder.add_doc(5);
  builder.add_doc(7);

  let set = builder.build()?;
  assert!(matches!(set, DocIdSetBuilderEnum::BitDoc(_)));

  let it = set.iterator()?;
  // it thinks the same doc was added twice
  assert_eq!(1, it.cost()?);

  // incomplete stats: num_values unknown
  let terms = DummyTerms::new(42, -1);
  let builder = DocIdSetBuilder::from_terms(100, &terms)?;
  assert_eq!(1.0_f64, builder.get_num_values_per_doc());
  assert!(builder.get_multi_valued());

  // incomplete stats: doc_count unknown
  let terms = DummyTerms::new(-1, 84);
  let builder = DocIdSetBuilder::from_terms(100, &terms)?;
  assert_eq!(1.0_f64, builder.get_num_values_per_doc());
  assert!(builder.get_multi_valued());

  Ok(())
}

#[test]
fn test_cost_is_correct_after_bit_set_upgrade() -> Result<()> {
  let max_doc = 1000000;
  let mut builder = DocIdSetBuilder::new(max_doc);
  for i in 0..1000000 >> 6 {
    builder.add_disi(&mut RangeDISI::new(i, i + 1)?)?;
  }
  let set = builder.build()?;

  assert!(matches!(set, DocIdSetBuilderEnum::BitDoc(_)));
  assert_eq!(set.iterator()?.cost()?, 1000000 >> 6);
  Ok(())
}

struct DummyPointValues {
  doc_count: i32,
  num_points: i32,
}
impl DummyPointValues {
  fn new(doc_count: i32, num_points: i32) -> Self {
    Self {
      doc_count,
      num_points,
    }
  }
}

impl Clone for DummyPointValues {
  fn clone(&self) -> Self {
    unreachable!()
  }
}

impl PointValues for DummyPointValues {
  fn get_min_packed_value(&self) -> Result<Option<Cow<'_, [u8]>>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_max_packed_value(&self) -> Result<Option<Cow<'_, [u8]>>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_num_dimensions(&self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_num_index_dimensions(&self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_bytes_per_dimension(&self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn size(&self) -> Result<usize> {
    Ok(self.num_points as usize)
  }

  fn get_doc_count(&self) -> Result<i32> {
    Ok(self.doc_count)
  }

  type PointTree = DummyPointTree;
  type MutablePointTree = DummyMutablePointTree;

  fn get_point_tree(&self) -> Result<PointTreeEnum<Self::MutablePointTree, Self::PointTree>> {
    unreachable!()
  }
}

struct DummyTerms {
  doc_count: i32,
  num_values: i32,
}
impl DummyTerms {
  fn new(doc_count: i32, num_values: i32) -> Self {
    Self {
      doc_count,
      num_values,
    }
  }
}
impl Terms for DummyTerms {
  type TermsEnum = DummyTermsEnum;

  fn iterator(&self) -> Result<Self::TermsEnum> {
    Err(LuceneError::unsupported_operation(""))
  }

  type IntersectIter = DummyTermsEnum;

  fn intersect(
    &self,
    _compiled: &CompiledAutomaton,
    _start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Self::IntersectIter> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn size(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_sum_total_term_freq(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_sum_doc_freq(&self) -> Result<i64> {
    Ok(self.num_values as i64)
  }

  fn get_doc_count(&self) -> Result<i32> {
    Ok(self.doc_count)
  }

  fn has_freqs(&self) -> bool {
    unreachable!()
  }

  fn has_offsets(&self) -> bool {
    unreachable!()
  }

  fn has_positions(&self) -> bool {
    unreachable!()
  }

  fn has_payloads(&self) -> bool {
    unreachable!()
  }
}
