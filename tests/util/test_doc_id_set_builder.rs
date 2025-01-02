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
use crate::common::{is_night_mode, my_random, rarely};
use crate::util::test_error::TestError;
use rand::Rng;
use rlucene::search::doc_id_set::DocIdSet;
use rlucene::search::doc_id_set_iterator::{DocIdSetIterator, Range, NO_MORE_DOCS};
use rlucene::util::bit_doc_id_set::BitDocIdSet;
use rlucene::util::bit_set::BitSet;
use rlucene::util::bit_set_iterator::BitSetIterator;
use rlucene::util::bits::Bits;
use rlucene::util::doc_id_set_builder::{
    DocIdSetBuilder, DocIdSetBuilderEnum, DocIdSetBuilderIterator,
};
use rlucene::util::fixed_bit_set::FixedBitSet;
use rlucene::util::int_array_doc_id_set::IntArrayDocIdSet;
use rlucene::util::roaring_doc_id_set::RoaringDocIdSetBuilder;

#[allow(dead_code)] // for quick search
struct TestDocIdSetBuilder {}
#[test]
fn test_empty() -> Result<(), TestError> {
    let mut random = my_random("test_empty".to_string());
    let max_doc = random.gen_range(1..1000);
    let doc_id_set: Option<IntArrayDocIdSet> = None;
    assert_equals(
        doc_id_set,
        Some(DocIdSetBuilder::new(max_doc).build().unwrap()),
    )?;
    Ok(())
}

fn assert_equals<T1: DocIdSet, T2: DocIdSet>(
    mut d1: Option<T1>,
    mut d2: Option<T2>,
) -> Result<(), TestError> {
    if d1.is_none() {
        if d2.is_none() {
            assert_eq!(
                d2.as_mut()
                    .unwrap()
                    .iterator()
                    .as_mut()
                    .unwrap()
                    .next_doc()?,
                NO_MORE_DOCS
            );
        }
    } else if d2.is_none() {
        assert_eq!(
            d1.as_mut()
                .unwrap()
                .iterator()
                .as_mut()
                .unwrap()
                .next_doc()?,
            NO_MORE_DOCS
        );
    } else {
        let mut i1 = d1.as_mut().unwrap().iterator().unwrap();
        let mut i2 = d2.as_mut().unwrap().iterator().unwrap();
        let mut doc = i1.next_doc()?;
        while doc != NO_MORE_DOCS {
            assert_eq!(doc, i2.next_doc()?);
            doc = i1.next_doc()?;
        }
        assert_eq!(i2.next_doc()?, NO_MORE_DOCS);
    };
    Ok(())
}

#[test]
fn test_sparse()->Result<(),TestError> {
    let mut random = my_random("test_sparse".to_string());
    let max_doc = 1000000 + random.gen_range(0..1000000);
    let mut builder = DocIdSetBuilder::new(max_doc);
    let num_iterators = 1 + random.gen_range(0..10);
    let mut fixed_set_bit = FixedBitSet::new(max_doc);
    for _i in 0..num_iterators {
        let base_inc = 200000 + random.gen_range(0..10000);
        let mut b = RoaringDocIdSetBuilder::new(max_doc);
        let mut doc = random.gen_range(0..100);
        while doc < max_doc {
            let _ = b.add(doc);
            fixed_set_bit.set(doc);
            doc += base_inc + random.gen_range(0..10000);
        }
        let roaring_doc_id_set = b.build();
        let iter = roaring_doc_id_set.iterator().unwrap();
        builder.add_disi::<DocIdSetBuilderIterator>(iter)?;
    }
    let result = builder.build()?;
    let enum_type1 = "BitDocIdSet<FixedBitSet>";
    let enum_type2 = "IntArrayDocIdSet";
    let doc_id_set_type = match result {
        DocIdSetBuilderEnum::B(_) => enum_type1,
        DocIdSetBuilderEnum::I(_) => enum_type2,
    };
    assert_eq!(doc_id_set_type, enum_type2);
    let bit_doc_id_set = BitDocIdSet::new(Some(fixed_set_bit))?;
    assert_equals(Some(bit_doc_id_set), Some(result))?;
    Ok(())
}
#[test]
fn test_dense()->Result<(),TestError> {
    let mut random = my_random("test_dense".to_string());
    let max_doc = 1000000 + random.gen_range(0..1000000);
    let mut builder = DocIdSetBuilder::new(max_doc);
    let num_iterators = 1 + random.gen_range(0..10);
    let mut fixed_set_bit = FixedBitSet::new(max_doc);
    for _i in 0..num_iterators {
        let mut b = RoaringDocIdSetBuilder::new(max_doc);
        let mut doc = random.gen_range(0..1000);
        while doc < max_doc {
            let _ = b.add(doc);
            fixed_set_bit.set(doc);
            doc += 1 + random.gen_range(0..100);
        }
        let roaring_doc_id_set = b.build();
        let iter = roaring_doc_id_set.iterator().unwrap();
        builder.add_disi::<DocIdSetBuilderIterator>(iter)?;
    }
    let result = builder.build()?;
    let enum_type1 = "BitDocIdSet<FixedBitSet>";
    let enum_type2 = "IntArrayDocIdSet";
    let doc_id_set_type = match result {
        DocIdSetBuilderEnum::B(_) => enum_type1,
        DocIdSetBuilderEnum::I(_) => enum_type2,
    };
    assert_eq!(doc_id_set_type, enum_type1);
    let bit_doc_id_set = BitDocIdSet::new(Some(fixed_set_bit))?;
    assert_equals(Some(bit_doc_id_set), Some(result))?;
    Ok(())
}

#[test]
fn test_random() -> Result<(), TestError> {
    let mut random = my_random("test_random".to_string());
    let max_doc = if is_night_mode() {
        random.gen_range(1..10000000)
    } else {
        random.gen_range(0..100000)
    };
    let mut i = 1;
    while i < (max_doc / 2) {
        let num_docs = random.gen_range(1..=i);
        let mut docs = FixedBitSet::new(max_doc);
        let mut c = 0;
        while c < num_docs {
            let d = random.gen_range(0..max_doc);
            if !docs.get(d) {
                docs.set(d);
                c += 1
            }
        }
        let mut array = vec![0; num_docs as usize + random.gen_range(0..100)];
        let mut it = BitSetIterator::new(&docs, 0).unwrap();
        let mut j = 0;
        let mut doc = it.next_doc()?;
        while doc != NO_MORE_DOCS {
            array[j] = doc;
            j += 1;
            doc = it.next_doc()?;
        }
        assert_eq!(num_docs, j as i32);
        // add some duplicates
        while j < array.len() {
            array[j] = array[random.gen_range(0..num_docs as usize)];
            j += 1;
        }

        // shuffle
        for j in (1..array.len()).rev() {
            let k = random.gen_range(0..j);
            array.swap(j, k);
        }

        // add docs out of order
        let mut builder = DocIdSetBuilder::new(max_doc);
        for j in 0..array.len() {
            let l = random.gen_range(1..=array.len() - j);
            let mut k = 0;
            let mut budget = 0;
            while k < l {
                let rarely = rarely(&mut random);
                if budget == 0 || rarely {
                    budget = random.gen_range(1..=l - k + 5);
                    builder.grow(budget as i32);
                }
                builder.add_doc(array[j]);
                budget -= 1;
                k += 1;
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
fn test_misleading_disi_cost()->Result<(),TestError> {
    let mut random = my_random("test_misleading_disi_cost".to_string());
    let max_doc = random.gen_range(1000..=10000);
    let mut builder = DocIdSetBuilder::new(max_doc);
    let mut expected = FixedBitSet::new(max_doc);
    for _i in 0..100 {
        let mut docs = FixedBitSet::new(max_doc);
        let num_docs = random.gen_range(1..=max_doc / 1000);
        for _ in 0..num_docs {
            let doc = random.gen_range(0..max_doc);
            docs.set(doc);
        }
        expected.or(&docs);
        // We provide a cost of 0 here to make sure the builder can deal with wrong costs
        let bit_doc_id_set = BitSetIterator::new(&docs, 0)?;
        builder.add_disi::<DocIdSetBuilderIterator>(bit_doc_id_set)?;
    }
    let bit_doc_id_set = BitDocIdSet::new(Some(expected))?;
    assert_equals(Some(bit_doc_id_set), Some(builder.build()?))?;
    Ok(())
}

#[test]
fn test_leverage_stats() {
    // single-valued points
    let mut doc_count = 42;
    let mut value_count = 42;
    let mut builder = DocIdSetBuilder::new_with_count(100, doc_count, value_count);
    assert_eq!(1f64 - builder.get_num_values_per_doc(), 0f64);
    assert!(!builder.get_multi_valued());
    builder.grow(2);
    builder.add_doc(5);
    builder.add_doc(7);
    let mut set = builder.build().unwrap();
    let enum_type1 = "BitDocIdSet<FixedBitSet>";
    let enum_type2 = "IntArrayDocIdSet";
    let doc_id_set_type = match set {
        DocIdSetBuilderEnum::B(_) => enum_type1,
        DocIdSetBuilderEnum::I(_) => enum_type2,
    };
    assert_eq!(doc_id_set_type, enum_type1);
    assert_eq!(set.iterator().unwrap().cost(), 2);

    // multi-valued
    doc_count = 42;
    value_count = 63;
    builder = DocIdSetBuilder::new_with_count(100, doc_count, value_count);
    assert_eq!(builder.get_num_values_per_doc() - 1.5, 0.0);
    assert!(builder.get_multi_valued());
    builder.grow(2);
    builder.add_doc(5);
    builder.add_doc(7);
    set = builder.build().unwrap();
    let doc_id_set_type = match set {
        DocIdSetBuilderEnum::B(_) => enum_type1,
        DocIdSetBuilderEnum::I(_) => enum_type2,
    };
    assert_eq!(doc_id_set_type, enum_type1);
    assert_eq!(set.iterator().unwrap().cost(), 1);

    // incomplete stats
    doc_count = 42;
    value_count = -1;
    builder = DocIdSetBuilder::new_with_count(100, doc_count, value_count);
    assert_eq!(builder.get_num_values_per_doc() - 1.0, 0.0);
    assert!(builder.get_multi_valued());

    doc_count = -1;
    value_count = 82;
    builder = DocIdSetBuilder::new_with_count(100, doc_count, value_count);
    assert_eq!(builder.get_num_values_per_doc() - 1.0, 0.0);
    assert!(builder.get_multi_valued());
}

#[test]
fn test_cost_is_correct_after_bit_set_upgrade() ->Result<(),TestError>{
    let max_doc = 1000000;
    let mut builder = DocIdSetBuilder::new(max_doc);
    for i in 0..1000000 >> 6 {
        builder.add_disi::<Range>(Range::new(i, i + 1)?)?;
    }
    let set = builder.build()?;
    let enum_type1 = "BitDocIdSet<FixedBitSet>";
    let enum_type2 = "IntArrayDocIdSet";
    let doc_id_set_type = match set {
        DocIdSetBuilderEnum::B(_) => enum_type1,
        DocIdSetBuilderEnum::I(_) => enum_type2,
    };
    assert_eq!(doc_id_set_type, enum_type1);
    assert_eq!(set.iterator().unwrap().cost(), 1000000 >> 6);
    Ok(())
}
