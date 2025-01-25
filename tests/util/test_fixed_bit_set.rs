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
use crate::common::{is_night_mode, my_random};
use crate::util::base_bit_set_test_case::{
    BaseBitSetTestCase, BaseBitSetTestCaseSupperImpl, RustUtilBitSet,
};
use crate::util::id_set_common::{clear_range, flip_bit, flip_bit_range, set_range};
use rand::rngs::StdRng;
use rand::Rng;
use rlucene::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use rlucene::util::bit_set::BitSet;
use rlucene::util::bit_set_iterator::BitSetIterator;
use rlucene::util::bits::Bits;
use rlucene::util::doc_base_bit_set_iterator::DocBaseBitSetIterator;

use crate::util::test_error::TestError;
use rlucene::util::error::lucene_error::LuceneError;
use rlucene::util::fixed_bit_set::FixedBitSet;
use rlucene::util::int_array_doc_id_set::IntArrayDocIdSetIterator;
use rlucene::util::sparse_fixed_bit_set::SparseFixedBitSet;
use std::hash::{DefaultHasher, Hash, Hasher};

struct TestFixedBitSet;

impl BaseBitSetTestCase for TestFixedBitSet {
    fn copy_of(
        &self,
        bs: &RustUtilBitSet,
        length: i32,
    ) -> (impl BitSet, Option<SparseFixedBitSet>) {
        let mut set = FixedBitSet::new(length);
        let mut doc = bs.next_set_bit(0);
        while doc != NO_MORE_DOCS {
            set.set(doc);
            if doc + 1 > length {
                doc = NO_MORE_DOCS;
            } else {
                doc = bs.next_set_bit(doc + 1);
            }
        }
        (set, None)
    }

    fn assert_equals<T: BitSet>(
        &self,
        set1: &RustUtilBitSet,
        set2: &T,
        max_doc: i32,
        _sfbs: &Option<SparseFixedBitSet>,
    ) {
        BaseBitSetTestCaseSupperImpl::assert_equals(self, set1, set2, max_doc, _sfbs);
    }

    fn test_prev_set_bit(&mut self, random: &mut StdRng) {
        check_prev_set_bit_array(random, vec![], 0);
        check_prev_set_bit_array(random, vec![0], 1);
        check_prev_set_bit_array(random, vec![0, 2], 3);
    }
}

impl BaseBitSetTestCaseSupperImpl for TestFixedBitSet {}

#[test]
fn test_cardinality() {
    let mut random = my_random("test_fixed_bit_set_cardinality".to_string());
    let mut fbs = TestFixedBitSet;
    fbs.test_cardinality(&mut random);
}
#[test]
fn test_prev_set_bit() {
    let mut random = my_random("test_fixed_bit_set_prev_set_bit".to_string());
    let mut fbs = TestFixedBitSet;
    fbs.test_prev_set_bit(&mut random);
}
#[test]
fn test_next_set_bit() {
    let mut random = my_random("test_fixed_bit_set_next_set_bit".to_string());
    let mut fbs = TestFixedBitSet;
    fbs.test_next_set_bit(&mut random);
}
#[test]
fn test_next_set_bit_in_range() {
    let mut random = my_random("test_fixed_bit_set_next_set_bit_in_range".to_string());
    let mut fbs = TestFixedBitSet;
    fbs.test_next_set_bit_in_range(&mut random);
}
#[test]
fn test_set() {
    let mut random = my_random("test_fixed_bit_set_set".to_string());
    let fbs = TestFixedBitSet;
    fbs.test_set(&mut random);
}
#[test]
fn test_get_and_set() {
    let mut random = my_random("test_fixed_bit_set_get_and_set".to_string());
    let fbs = TestFixedBitSet;
    fbs.test_get_and_set(&mut random);
}
#[test]
fn test_clear() {
    let mut random = my_random("test_fixed_bit_set_clear".to_string());
    let mut fbs = TestFixedBitSet;
    fbs.test_clear(&mut random);
}
#[test]
fn test_clear_range() {
    let mut random = my_random("test_fixed_bit_set_clear_range".to_string());
    let fbs = TestFixedBitSet;
    fbs.test_clear_range(&mut random);
}
#[test]
fn test_clear_all() {
    let mut random = my_random("test_fixed_bit_set_clear_all".to_string());
    let fbs = TestFixedBitSet;
    fbs.test_clear_all(&mut random);
}
#[test]
fn test_or_sparse() {
    let mut random = my_random("test_fixed_bit_set_or_sparse".to_string());
    let mut fbs = TestFixedBitSet;
    fbs.test_or_sparse(&mut random);
}
#[test]
fn test_or_dense() {
    let mut random = my_random("test_fixed_bit_set_or_dense".to_string());
    let mut fbs = TestFixedBitSet;
    fbs.test_or_dense(&mut random);
}
#[test]
fn test_or_random() {
    let mut random = my_random("test_fixed_bit_set_or_random".to_string());
    let mut fbs = TestFixedBitSet;
    fbs.test_or_random(&mut random);
}

#[test]
fn test_approximate_cardinality() {
    // The approximate cardinality works in such a way that it should be pretty accurate on a bitset
    // whose bits are uniformly distributed.
    let mut random = my_random("test_approximate_cardinality".to_string());
    let mut set = FixedBitSet::new(random.gen_range(100000..=200000));
    let first = random.gen_range(0..=10);
    let interval = random.gen_range(10..=20);
    let mut i = first;
    while i < set.length() {
        set.set(i);
        i += interval;
    }
    let cardinality = set.cardinality();
    assert!((cardinality - set.approximate_cardinality()).abs() <= (cardinality / 20))
}

fn do_get(a: &bit_set::BitSet, b: &FixedBitSet) {
    assert_eq!(a.len(), b.cardinality() as usize);
    let max = b.length();
    for i in 0..max {
        assert_eq!(a.contains(i as usize), b.get(i));
    }
}

fn do_next_set_bit(a: &bit_set::BitSet, b: &FixedBitSet) {
    assert_eq!(a.len(), b.cardinality() as usize);
    let mut bb = 0;
    loop {
        bb = b.next_set_bit(bb);

        if bb == NO_MORE_DOCS {
            assert!(!a.contains(bb as usize));
            break;
        }
        assert!(a.contains(bb as usize));
        bb += 1;
        if bb > b.length() - 1 {
            assert!(!a.contains(bb as usize));
            break;
        }
    }

    let iter = a.iter();
    for index in iter {
        assert_eq!(index, b.next_set_bit(index as i32) as usize);
    }
}

fn do_prev_set_bit(a: &bit_set::BitSet, b: &FixedBitSet) {
    assert_eq!(a.len(), b.cardinality() as usize);
    let mut bb = b.length() - 1;
    let mut count = 0;
    let mut iter: Vec<_> = a.iter().collect();
    iter.reverse();
    // check set a bit in BitSet should be in FixedBitSet
    for index in iter {
        bb = b.prev_set_bit(index as i32);
        assert_eq!(bb as usize, index);
    }
    if bb > 0 {
        // bb should be the last match value , so prev_set_bit(bb - 1) should return -1
        assert_eq!(b.prev_set_bit(bb - 1), -1);
    }

    bb = b.length() - 1;

    if bb == -1 {
        assert_eq!(a.iter().count(), 0);
        return;
    }

    loop {
        bb = b.prev_set_bit(bb);
        if bb == -1 {
            break;
        }
        count += 1;
        assert!(a.contains(bb as usize));
        if bb == 0 {
            break;
        }
        bb -= 1;
    }
    assert_eq!(b.cardinality(), count);
}

fn do_iterate(
    random: &mut StdRng,
    a: &bit_set::BitSet,
    b: &FixedBitSet,
    mode: i32,
) -> Result<(), TestError> {
    match mode {
        1 => do_iterate1(random, a, b),
        2 => do_iterate2(random, a, b),
        _ => Ok(()),
    }
}

fn do_iterate1(random: &mut StdRng, a: &bit_set::BitSet, b: &FixedBitSet) -> Result<(), TestError> {
    assert_eq!(a.len(), b.cardinality() as usize);
    let mut iterator = BitSetIterator::new(b, 0).unwrap();
    let iter = a.iter();
    for index in iter {
        let bb = if random.gen_bool(0.5) {
            iterator.next_doc()?
        } else {
            iterator.advance(index as i32)?
        };
        assert_eq!(index, bb as usize);
    }
    assert_eq!(iterator.next_doc()?, NO_MORE_DOCS);
    Ok(())
}

fn do_iterate2(random: &mut StdRng, a: &bit_set::BitSet, b: &FixedBitSet) -> Result<(), TestError> {
    assert_eq!(a.len(), b.cardinality() as usize);
    let mut iterator = BitSetIterator::new(b, 0).unwrap();
    let iter = a.iter();
    for index in iter {
        let bb = if random.gen_bool(0.5) {
            iterator.next_doc()?
        } else {
            iterator.advance(index as i32)?
        };
        assert_eq!(index, bb as usize);
    }
    assert_eq!(iterator.next_doc()?, NO_MORE_DOCS);
    Ok(())
}

fn do_random_sets(random: &mut StdRng, iter: i32, mode: i32) -> Result<(), TestError> {
    // let max_size = random.gen_range(1200..=i32::MAX);
    let max_size = random.gen_range(1200..=100000);
    let mut a0: bit_set::BitSet = Default::default();
    let mut b0: FixedBitSet = Default::default();
    let mut flag = 0;
    for _i in 0..iter {
        let sz = random.gen_range(2..max_size);
        let mut a = bit_set::BitSet::with_capacity(sz as usize);
        let mut b = FixedBitSet::new(sz);
        let n_oper = random.gen_range(0..sz);
        for _j in 0..n_oper {
            let mut idx = random.gen_range(0..sz);
            a.insert(idx as usize);
            b.set(idx);

            idx = random.gen_range(0..sz);
            a.remove(idx as usize);
            b.clear_with_index(idx);

            idx = random.gen_range(0..sz);
            flip_bit_range(&mut a, idx as usize, (idx + 1) as usize);
            b.flip_range(idx, idx + 1);

            idx = random.gen_range(0..sz);
            flip_bit(&mut a, idx as usize);
            b.flip(idx);

            let val2 = b.get(idx);
            let val = b.get_and_set(idx);
            assert_eq!(val2, val);
            assert!(b.get(idx));

            if !val {
                b.clear_with_index(idx);
            }
            assert_eq!(b.get(idx), val);
        }

        // test that the various ways of accessing the bits are equivalent
        do_get(&a, &b);

        // test ranges, including possible extension
        let mut from_index: i32;
        let mut to_index: i32;
        from_index = random.gen_range(0..(sz / 2));
        to_index = from_index + random.gen_range(0..(sz - from_index));
        let mut aa = a.clone();
        flip_bit_range(&mut aa, from_index as usize, to_index as usize);
        let mut bb = b.clone();
        bb.flip_range(from_index, to_index);

        do_iterate(random, &aa, &bb, mode)?; //  a problem here is from flip or doIterate

        from_index = random.gen_range(0..(sz / 2));
        to_index = from_index + random.gen_range(0..(sz - from_index));
        aa.clone_from(&a);
        clear_range(&mut aa, from_index as usize, to_index as usize);
        bb = b.clone();
        bb.clear_range(from_index, to_index);

        do_next_set_bit(&aa, &bb); // a problem here is from clear() or nextSetBit

        do_prev_set_bit(&aa, &bb);

        from_index = random.gen_range(0..(sz / 2));
        to_index = from_index + random.gen_range(0..(sz - from_index));
        aa.clone_from(&a);
        set_range(&mut aa, from_index as usize, to_index as usize);
        bb = b.clone();
        bb.set_with_range(from_index, to_index);

        do_next_set_bit(&aa, &bb); // a problem here is from set() or nextSetBit

        do_prev_set_bit(&aa, &bb);

        if flag == 1 && b0.length() <= b.length() {
            assert_eq!(a.len(), b.cardinality() as usize);

            let mut a_and = a.clone();
            a_and.intersect_with(&a0);
            let mut a_or = a.clone();
            a_or.union_with(&a0);
            let mut a_xor = a.clone();
            a_xor.symmetric_difference_with(&a0);
            let mut a_andn = a.clone();
            a_andn.difference_with(&a0);

            let mut b_and = b.clone();
            assert!(b == b_and);
            b_and.and(&b0);
            let mut b_or = b.clone();
            b_or.or(&b0);
            let mut b_xor = b.clone();
            b_xor.xor(&b0);
            let mut b_andn = b.clone();
            b_andn.and_not_fixed_bit_set(&b0);

            assert_eq!(a0.len(), b0.cardinality() as usize);
            assert_eq!(a_or.len(), b_or.cardinality() as usize);

            assert_eq!(a_and.len(), b_and.cardinality() as usize);
            assert_eq!(a_or.len(), b_or.cardinality() as usize);
            assert_eq!(a_andn.len(), b_andn.cardinality() as usize);
            assert_eq!(a_xor.len(), b_xor.cardinality() as usize);

            do_iterate(random, &a_and, &b_and, mode)?;
            do_iterate(random, &a_xor, &b_xor, mode)?;
            do_iterate(random, &a_or, &b_or, mode)?;
            do_iterate(random, &a_andn, &b_andn, mode)?;

            a0 = a;
            b0 = b;
        } else {
            flag = 1;
            a0 = a;
            b0 = b;
        }
    }
    Ok(())
}

#[test]
fn test_small() -> Result<(), TestError> {
    let mut random = my_random("test_small".to_string());
    let iters = if is_night_mode() {
        random.gen_range(1000..100000)
    } else {
        100
    };
    do_random_sets(&mut random, iters, 1)?;
    do_random_sets(&mut random, iters, 2)?;
    Ok(())
}

#[test]
fn test_equals() {
    // This test can't handle numBits==0:
    let mut random = my_random("test_equals".to_string());
    let num_bits = random.gen_range(0..2000) + 1;
    let mut b1 = FixedBitSet::new(num_bits);
    let mut b2 = FixedBitSet::new(num_bits);
    assert!(b1.eq(&b2));
    assert!(b2.eq(&b1));
    for _i in 0..random.gen_range(1000..5000) {
        let idx = random.gen_range(0..num_bits);
        if !b1.get(idx) {
            b1.set(idx);
            assert!(!b1.eq(&b2));
            assert!(!b2.eq(&b1));
            b2.set(idx);
            assert!(b1.eq(&b2));
            assert!(b2.eq(&b1));
        }
    }
}

#[test]
fn test_hash_code_equals() {
    let mut random = my_random("test_hash_code_equals".to_string());

    let num_bits = random.gen_range(0..2000) + 1;
    let mut b1 = FixedBitSet::new(num_bits);
    let mut b2 = FixedBitSet::new(num_bits);
    for _i in 0..random.gen_range(1000..5000) {
        let idx = random.gen_range(0..num_bits);
        if !b1.get(idx) {
            b1.set(idx);
            assert!(!b1.eq(&b2));
            assert_ne!(calculate_hash(&b1), calculate_hash(&b2));
            b2.set(idx);
            assert!(b1.eq(&b2));
            assert_eq!(calculate_hash(&b1), calculate_hash(&b2));
        }
    }
}

fn calculate_hash(a: &FixedBitSet) -> u64 {
    let mut hasher = DefaultHasher::new();
    a.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn test_small_bitsets() {
    // Make sure size 0-10 bit sets are OK:
    for num_bits in 0..10 {
        let mut b1 = FixedBitSet::new(num_bits);
        let b2 = FixedBitSet::new(num_bits);
        assert!(b1.eq(&b2));
        assert_eq!(calculate_hash(&b1), calculate_hash(&b2));
        assert_eq!(0, b1.cardinality());
        if num_bits > 0 {
            b1.set_with_range(0, num_bits);
            assert_eq!(num_bits, b1.cardinality());
            b1.flip_range(0, num_bits);
            assert_eq!(0, b1.cardinality());
        }
    }
}

fn make_fixed_bitset(
    random: &mut StdRng,
    a: &Vec<i32>,
    num_bits: i32,
) -> Result<FixedBitSet, LuceneError> {
    let mut bs: FixedBitSet;
    if random.gen_bool(0.5) {
        let bits_2_words = FixedBitSet::bits2words(num_bits);
        let mut words: Vec<i64> = Vec::with_capacity(bits_2_words as usize);
        words.resize(num_bits as usize, 0);
        bs = FixedBitSet::with_capacity(words, num_bits)?
    } else {
        bs = FixedBitSet::new(num_bits)
    }
    for e in a {
        bs.set(*e);
    }
    Ok(bs)
}

fn make_bitset(a: &Vec<i32>) -> bit_set::BitSet {
    let mut bs: bit_set::BitSet = bit_set::BitSet::with_capacity(a.len());
    for x in a {
        bs.insert(*x as usize);
    }
    bs
}

fn check_prev_set_bit_array(random: &mut StdRng, a: Vec<i32>, num_bits: i32) {
    let obs = make_fixed_bitset(random, &a, num_bits).unwrap();
    let bs = make_bitset(&a);
    do_prev_set_bit(&bs, &obs);
}

fn check_next_set_bit_array(random: &mut StdRng, a: Vec<i32>, num_bits: i32) {
    let obs = make_fixed_bitset(random, &a, num_bits).unwrap();
    let bs = make_bitset(&a);
    do_next_set_bit(&bs, &obs);
}

#[test]
fn test_next_bitset() {
    let mut random = my_random("test_next_bitset".to_string());
    let capacity = random.gen_range(0..1000);
    let mut set_bits: Vec<i32> = Vec::with_capacity(capacity as usize);
    for _i in 0..capacity {
        set_bits.push(random.gen_range(0..capacity));
    }
    let num_bits = set_bits.len() + random.gen_range(0..10);
    check_next_set_bit_array(&mut random, set_bits, num_bits as i32);
    check_next_set_bit_array(&mut random, vec![], num_bits as i32);
}

#[test]
fn test_ensure_capacity() -> Result<(), LuceneError> {
    let mut bits = FixedBitSet::new(5);
    bits.set(1);
    bits.set(4);

    let mut bits_clone = bits.clone();
    FixedBitSet::ensure_capacity(&mut bits, 8)?;
    assert!(bits.get(1));
    assert!(bits.get(4));
    bits.clear_with_index(1);
    assert!(bits_clone.get(1));
    assert!(!bits.get(1));

    bits.set(1);
    let length = bits.length();
    let bits_clone_1 = bits.clone();
    FixedBitSet::ensure_capacity(&mut bits, length - 2)?;
    assert_eq!(bits_clone_1.length(), bits.length());
    assert!(bits.get(1));

    bits_clone.set(1);
    let bits_clone_2 = bits_clone.clone();
    FixedBitSet::ensure_capacity(&mut bits_clone, 72)?;
    assert!(bits_clone.length() > bits_clone_2.length());
    assert!(bits_clone.get(1));
    assert!(bits_clone.get(4));
    bits_clone.clear_with_index(1);
    // we grew the long[], so it's not shared
    assert!(bits_clone_2.get(1));
    assert!(!bits_clone.get(1));
    Ok(())
}

#[test]
fn test_bits2words() {
    assert_eq!(0, FixedBitSet::bits2words(0));
    assert_eq!(1, FixedBitSet::bits2words(1));

    assert_eq!(1, FixedBitSet::bits2words(64));
    assert_eq!(2, FixedBitSet::bits2words(65));

    assert_eq!(2, FixedBitSet::bits2words(128));
    assert_eq!(3, FixedBitSet::bits2words(129));

    assert_eq!(1024, FixedBitSet::bits2words(65536));
    assert_eq!(1025, FixedBitSet::bits2words(65537));

    assert_eq!(1 << (31 - 6), FixedBitSet::bits2words(i32::MAX));
}

fn make_int_array(random: &mut StdRng, count: i32, min: i32, max: i32) -> Vec<i32> {
    let mut rv = vec![0; count as usize];
    for _i in 0..count {
        rv.push(random.gen_range(min..=max));
    }
    rv
}

#[test]
fn test_intersection_count() {
    let mut random = my_random("test_intersection_count".to_string());

    let num_bits1 = random.gen_range(1000..=2000);
    let num_bits2 = random.gen_range(1000..=2000);

    let count1 = random.gen_range(0..=num_bits1 - 1);
    let count2 = random.gen_range(0..=num_bits2 - 1);

    let bits1 = make_int_array(&mut random, count1, 0, num_bits1 - 1);
    let bits2 = make_int_array(&mut random, count2, 0, num_bits2 - 1);

    let fixed_bit_set1 = make_fixed_bitset(&mut random, &bits1, num_bits1);
    let fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2);
    // If ghost bits are present, these may fail too, but that's not what we want to demonstrate
    // here
    // assertTrue(fixedBitSet1.cardinality() <= bits1.length);
    // assertTrue(fixedBitSet2.cardinality() <= bits2.length);
    let intersection_count =
        FixedBitSet::intersection_count(fixed_bit_set1.unwrap(), fixed_bit_set2.unwrap());

    let mut bit_set1 = make_bitset(&bits1);
    let bit_set2 = make_bitset(&bits2);
    // If ghost bits are present, these may fail too, but that's not what we want to demonstrate
    // here
    // assertEquals(bitSet1.cardinality(), fixedBitSet1.cardinality());
    // assertEquals(bitSet2.cardinality(), fixedBitSet2.cardinality());

    bit_set1.intersect_with(&bit_set2);
    assert_eq!(bit_set1.len(), intersection_count as usize);
}

#[test]
fn test_and_not() -> Result<(), TestError> {
    let mut random = my_random("test_and_not".to_string());

    let num_bits2 = random.gen_range(1000..=2000);
    let num_bits1 = random.gen_range(1000..=num_bits2);

    let count1 = random.gen_range(0..=num_bits1 - 1);
    let count2 = random.gen_range(0..=num_bits2 - 1);

    let min = random.gen_range(0..=(num_bits1 - 1));
    let off_set_word1 = min >> 6;
    let offset1 = off_set_word1 >> 6;
    let bits1 = make_int_array(&mut random, count1, min, num_bits1 - 1);
    let bits2 = make_int_array(&mut random, count2, 0, num_bits2 - 1);

    let bitset1 = make_bitset(&bits1);
    let mut bitset2 = make_bitset(&bits2);
    bitset2.difference_with(&bitset1);

    {
        // test BitSetIterator
        let mut fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2)?;
        let fixed_bit = make_fixed_bitset(&mut random, &bits1, num_bits1)?;
        let disi = BitSetIterator::new(&fixed_bit, count1 as i64)?;
        fixed_bit_set2.and_not_iter(disi)?;
        do_get(&bitset2, &fixed_bit_set2);
    }
    {
        // test DocBaseBitSetIterator
        let mut fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2)?;
        let offset_bits: Vec<i32> = bits1.iter().map(|&i| i - offset1).collect();
        let fixed_bit = make_fixed_bitset(&mut random, &offset_bits, num_bits1 - offset1)?;
        let disi = DocBaseBitSetIterator::new(fixed_bit, count1 as i64, offset1)?;
        fixed_bit_set2.and_not_iter(disi)?;
        do_get(&bitset2, &fixed_bit_set2);
    }
    {
        // test other
        let mut fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2)?;
        let mut sorted = bits1.clone();
        sorted.push(0);
        sorted[bits1.len()] = NO_MORE_DOCS;
        let disi = IntArrayDocIdSetIterator::new(&sorted, count1);
        fixed_bit_set2.and_not_iter(disi)?;
        do_get(&bitset2, &fixed_bit_set2);
    }
    Ok(())
}

// Demonstrates that the presence of ghost bits in the last used word can cause spurious failures
#[test]
fn test_union_count() {
    let mut random = my_random("test_union_count".to_string());
    let num_bits1 = random.gen_range(1000..=2000);
    let num_bits2 = random.gen_range(1000..=2000);

    let count1 = random.gen_range(0..=num_bits1 - 1);
    let count2 = random.gen_range(0..=num_bits2 - 1);

    let bits1 = make_int_array(&mut random, count1, 0, num_bits1 - 1);
    let bits2 = make_int_array(&mut random, count2, 0, num_bits2 - 1);

    let fixed_bit_set1 = make_fixed_bitset(&mut random, &bits1, num_bits1).unwrap();
    let fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2).unwrap();

    let union_count = FixedBitSet::union_count(&fixed_bit_set1, &fixed_bit_set2);

    let mut bit_set1 = make_bitset(&bits1);
    let bit_set2 = make_bitset(&bits2);
    bit_set1.union_with(&bit_set2);

    assert_eq!(bit_set1.len(), union_count as usize);
}

#[test]
fn test_and_not_count() {
    let mut random = my_random("test_and_not_count".to_string());

    let num_bits1 = random.gen_range(1000..=2000);
    let num_bits2 = random.gen_range(1000..=2000);

    let count1 = random.gen_range(0..=num_bits1 - 1);
    let count2 = random.gen_range(0..=num_bits2 - 1);

    let bits1 = make_int_array(&mut random, count1, 0, num_bits1 - 1);
    let bits2 = make_int_array(&mut random, count2, 0, num_bits2 - 1);

    let fixed_bit_set1 = make_fixed_bitset(&mut random, &bits1, num_bits1).unwrap();
    let fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2).unwrap();

    let and_not_count = FixedBitSet::and_not_count(&fixed_bit_set1, &fixed_bit_set2);

    let mut bit_set1 = make_bitset(&bits1);
    let bit_set2 = make_bitset(&bits2);

    bit_set1.difference_with(&bit_set2);

    assert_eq!(bit_set1.len(), and_not_count as usize);
}

#[test]
// todo
fn test_copy_of() {}

#[test]
fn test_as_bits() {
    let mut set = FixedBitSet::new(10);
    set.set(3);
    set.set(4);
    set.set(9);
    let bits = set.as_read_only_bits();
    assert_eq!(set.length(), bits.length());
    for i in 0..set.length() {
        assert_eq!(set.get(i), bits.get(i));
    }
    // The data in bits is a reference to set, so it is not necessary to
    // verify whether changes in set are reflected in bits.
    // Further changes are reflected
    // set.set(5);
    // assertTrue(bits.get(5));
}
