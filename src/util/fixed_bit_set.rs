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
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::accountable::Accountable;
use crate::util::bit_set::BitSet;
use crate::util::bits::Bits;

use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bits::FixedBits;

use crate::search::doc_id_set_iterator::doc_id_set_iterator_static::NO_MORE_DOCS;
use std::hash::{Hash, Hasher};

// todo
#[allow(unused)]
const FIXED_BIT_SET_BASE_RAM_BYTES_USED: i64 = 0;

#[derive(Default)]
/// `BitSet` of fixed length (`num_bits`), backed by accessible (`get_bits`) `long[]`, accessed with
/// an `int` index, implementing [`Bits`] and [`DocIdSet`](crate::search::doc_id_set).
/// If you need to manage more than 2.1B bits, use [`LongBitSet`](crate::util::long_bit_set::LongBitSet).
///
/// # Note
/// This is an internal API.
pub struct FixedBitSet {
    // Array of longs holding the bits
    bits: Vec<i64>,
    // The number of bits in use
    num_bits: i32,
    // The exact number of longs needed to hold numBits (<= bits.length)
    num_words: i32,
}

impl Hash for FixedBitSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bits.hash(state);
        self.num_bits.hash(state);
        self.num_words.hash(state);
    }
}

impl PartialEq for FixedBitSet {
    fn eq(&self, other: &Self) -> bool {
        if self.num_bits == other.num_bits
            && self.num_words == other.num_words
            && self.bits == other.bits
        {
            return true;
        }
        false
    }
}

impl Clone for FixedBitSet {
    fn clone(&self) -> Self {
        let bits = self.bits.clone();
        Self::with_capacity(bits, self.num_bits).unwrap()
    }
}

/// If the given [`LongBitSet`](crate::util::long_bit_set::LongBitSet) is large enough to hold `num_bits + 1`,
/// returns the given bits, otherwise returns a new [`LongBitSet`](crate::util::long_bit_set::LongBitSet) that can hold the requested number of bits.
///
/// # Note
/// The returned bitset reuses the underlying `long[]` of the given `bits` if possible.
/// Also, calling `length()` on the returned bits may return a value greater than `num_bits`.
impl FixedBitSet {
    pub fn ensure_capacity(bits: &mut FixedBitSet, num_bits: i32) -> Result<()> {
        if num_bits < bits.num_bits {
        } else {
            let num_words = Self::bits2words(num_bits);
            let length = bits.bits.len();
            if num_words as usize >= length {
                ArrayUtil::grow_with_len(&mut bits.bits, num_words + 1)?;
            }
            debug_assert!(bits.bits.len() <= i32::MAX as usize);
            bits.num_bits = (bits.bits.len() as i32) << 6;
            bits.num_words = Self::bits2words(bits.num_bits);
        }
        Ok(())
    }

    /// returns the number of 64-bit words it would take to hold numBits
    pub fn bits2words(num_bits: i32) -> i32 {
        ((num_bits - 1) >> 6) + 1
    }

    /// Returns the popcount or cardinality of the intersection of the two sets. Neither set is
    /// modified.
    pub fn intersection_count(a: FixedBitSet, b: FixedBitSet) -> i64 {
        // Depends on the ghost bits being clear!
        let mut tot = 0;
        let num_common_words = std::cmp::min(a.num_words, b.num_words) as usize;
        for i in 0..num_common_words {
            tot += (a.bits[i] & b.bits[i]).count_ones();
        }
        tot as i64
    }

    //// Returns the popcount or cardinality of the union of the two sets. Neither set is modified.
    pub fn union_count(a: &FixedBitSet, b: &FixedBitSet) -> i64 {
        // Depends on the ghost bits being clear!
        let mut tot = 0;
        let num_common_words = std::cmp::min(a.num_words, b.num_words) as usize;
        for i in 0..num_common_words {
            tot += (a.bits[i] | b.bits[i]).count_ones();
        }
        for i in num_common_words..a.num_words as usize {
            tot += a.bits[i].count_ones();
        }
        for i in num_common_words..b.num_words as usize {
            tot += b.bits[i].count_ones();
        }
        tot as i64
    }

    /// Returns the popcount or cardinality of "a and not b" or "intersection(a not(b))". Neither set
    /// is modified.
    pub fn and_not_count(a: &FixedBitSet, b: &FixedBitSet) -> i64 {
        let mut tot = 0;
        let num_common_words = std::cmp::min(a.num_words, b.num_words) as usize;
        for i in 0..num_common_words {
            tot += (a.bits[i] & !b.bits[i]).count_ones();
        }
        for i in num_common_words..a.num_words as usize {
            tot += a.bits[i].count_ones();
        }
        tot as i64
    }
    /// Creates a new `FixedBitSet`. The internally allocated `Vec<u64>` array will be exactly the size needed
    /// to accommodate the `num_bits` specified.
    ///
    /// # Arguments
    /// * `num_bits` - The number of bits needed.
    pub fn new(num_bits: i32) -> FixedBitSet {
        let size: usize = Self::bits2words(num_bits) as usize;
        let bits: Vec<i64> = vec![0; size];
        let exact_size = bits.len();
        debug_assert!(exact_size < i32::MAX as usize);
        FixedBitSet {
            bits,
            num_bits,
            num_words: exact_size as i32,
        }
    }
    /// Creates a new `FixedBitSet` using the provided `Vec<u64>` array as the backing store.
    /// The `stored_bits` array must be large enough to accommodate the `num_bits` specified,
    /// but may be larger. In that case, the 'extra' or 'ghost' bits must be clear (or they may provoke spurious side effects).
    ///
    /// # Arguments
    /// * `stored_bits` - The array to use as the backing store (`Vec<i64>`).
    /// * `num_bits` - The number of bits actually needed.
    pub fn with_capacity(stored_bits: Vec<i64>, num_bits: i32) -> Result<FixedBitSet> {
        let num_words = Self::bits2words(num_bits);
        if num_words as usize > stored_bits.len() {
            return Err(LuceneError::illegal_argument(format!(
                "The given long array is too small  to hold {} bits",
                num_words
            )));
        }
        let result = FixedBitSet {
            bits: stored_bits,
            num_bits,
            num_words,
        };
        debug_assert!(Self::verify_ghost_bits_clear(&result));
        Ok(result)
    }

    /// Checks if the bits past `num_bits` are clear. Some methods rely on this implicit assumption:
    /// search for "Depends on the ghost bits being clear!"
    ///
    /// # Returns
    /// `true` if the bits past `num_bits` are clear.
    fn verify_ghost_bits_clear(fixed_bit_set: &FixedBitSet) -> bool {
        for i in fixed_bit_set.num_words as usize..fixed_bit_set.bits.len() {
            if fixed_bit_set.bits[i] != 0 {
                return false;
            }
        }
        if (fixed_bit_set.num_bits & 0x3f) == 0 {
            return true;
        }

        let mask = -1 << (fixed_bit_set.num_bits % 64);
        (fixed_bit_set.bits[(fixed_bit_set.num_words as usize) - 1] & mask) == 0
    }

    pub fn get_bits(&self) -> &[i64] {
        &self.bits
    }

    #[allow(unused)]
    fn get_and_clear(&mut self, index: i32) -> bool {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let word_num = (index >> 6) as usize;
        let bit_mask = 1_i64 << (index % 64);
        let val = (self.bits[word_num] & bit_mask) != 0;
        self.bits[word_num] &= !bit_mask;
        val
    }

    /// this = this OR other
    pub fn or(&mut self, other: &FixedBitSet) {
        self.or_impl(0, &other.bits, other.num_words);
    }

    #[allow(unused)]
    fn or_offset(&mut self, other_offset_words: i32, other: &FixedBitSet) {
        self.or_impl(other_offset_words, &other.bits, other.num_words);
    }

    fn or_impl(&mut self, other_offset_words: i32, other_arr: &[i64], other_num_words: i32) {
        debug_assert!(
            other_num_words + other_offset_words <= self.num_words,
            "num_words = {} other_num_words = {}",
            self.num_words,
            other_num_words
        );
        let pos = std::cmp::min(self.num_words - other_offset_words, other_num_words) as usize;
        let offset = other_offset_words as usize;
        for i in (0..pos).rev() {
            self.bits[i + offset] |= other_arr[i];
        }
    }

    /// this = this XOR other
    pub fn xor(&mut self, other: &FixedBitSet) {
        self.xor_impl(&other.bits, other.num_words);
    }
    #[allow(unused)]
    pub fn xor_disi(&self, _iter: impl DocIdSetIterator) {
        // not used in Java Lucene, so we did not impl it
        todo!()
    }
    fn xor_impl(&mut self, other_bits: &[i64], other_num_words: i32) {
        debug_assert!(
            other_num_words <= self.num_words,
            "num_words = {} other_num_words = {}",
            self.num_words,
            other_num_words
        );
        let pos = std::cmp::min(self.num_words, other_num_words) as usize;
        for i in (0..pos).rev() {
            self.bits[i] ^= other_bits[i];
        }
    }

    pub fn intersects(&self, other: &FixedBitSet) -> bool {
        // Depends on the ghost bits being clear!
        let pos = std::cmp::min(self.num_words, other.num_words) as usize;
        for i in (0..pos).rev() {
            if self.bits[i] != other.bits[i] {
                return true;
            }
        }
        false
    }

    /// this = this AND other
    pub fn and(&mut self, other: &FixedBitSet) {
        self.and_self(&other.bits, other.num_words);
    }

    pub fn and_self(&mut self, other_arr: &[i64], other_num_words: i32) {
        let pos = std::cmp::min(self.num_words, other_num_words) as usize;
        for i in (0..pos).rev() {
            self.bits[i] &= other_arr[i];
        }

        if self.num_words > other_num_words {
            for i in other_num_words as usize..self.num_words as usize {
                self.bits[i] = 0;
            }
        }
    }

    pub fn and_not_iter(&mut self, mut iter: impl DocIdSetIterator) -> Result<()> {
        let mut doc = iter.next_doc()?;
        while doc != NO_MORE_DOCS {
            self.clear_with_index(doc);
            doc = iter.next_doc()?;
        }
        Ok(())
    }

    /// this = this AND NOT other
    pub fn and_not_fixed_bit_set(&mut self, other: &FixedBitSet) {
        self.and_not_impl(0, &other.bits, other.num_words)
    }

    #[allow(unused)]
    fn and_not_offset(&mut self, other_offset_words: i32, other: &FixedBitSet) {
        self.and_not_impl(other_offset_words, &other.bits, other.num_words);
    }

    fn and_not_impl(&mut self, other_offset_words: i32, other_arr: &[i64], other_num_words: i32) {
        let pos = std::cmp::min(self.num_words - other_offset_words, other_num_words) as usize;
        let offset = other_offset_words as usize;
        for i in (0..pos).rev() {
            self.bits[i + offset] &= !other_arr[i];
        }
    }

    /// Flips a range of bits.
    ///
    /// # Arguments
    /// * `start_index` - The lower index.
    /// * `end_index` - One-past the last bit to flip.
    pub fn flip_range(&mut self, start_index: i32, end_index: i32) {
        debug_assert!(start_index >= 0 && start_index < self.num_bits);
        debug_assert!(end_index >= 0 && end_index <= self.num_bits);
        if end_index <= start_index {
            return;
        }
        let start_word = (start_index >> 6) as usize;
        let end_word = ((end_index - 1) >> 6) as usize;

        let start_mask = -1_i64 << (start_index % 64);
        let end_mask = (!0u64) >> (-end_index as u64);
        if start_word == end_word {
            self.bits[start_word] ^= start_mask & end_mask as i64;
            return;
        }

        self.bits[start_word] ^= start_mask;

        for i in start_word + 1..end_word {
            self.bits[i] = !self.bits[i];
        }

        self.bits[end_word] ^= end_mask as i64;
    }

    /// Flip the bit at the provided index.
    pub fn flip(&mut self, index: i32) {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let word_num = index >> 6;
        let bit_mask = 1_i64 << (index % 64);
        self.bits[word_num as usize] ^= bit_mask;
    }

    /// Sets a range of bits.
    ///
    /// # Arguments
    /// * `start_index` - The lower index.
    /// * `end_index` - One-past the last bit to set.
    pub fn set_with_range(&mut self, start_index: i32, end_index: i32) {
        debug_assert!(
            start_index >= 0 && start_index < self.num_bits,
            "start_index = {}, num_bits = {}",
            start_index,
            end_index
        );
        debug_assert!(
            end_index >= 0 && end_index <= self.num_bits,
            "end_index = {}, num_bits = {}",
            end_index,
            start_index
        );
        if end_index <= start_index {
            return;
        }

        let start_word = (start_index >> 6) as usize;
        let end_word = ((end_index - 1) >> 6) as usize;

        let start_mask = !0u64 << (start_index % 64);
        let end_mask = (!0u64) >> (-end_index as u64);

        if start_word == end_word {
            self.bits[start_word] |= start_mask as i64 & end_mask as i64;
            return;
        }

        self.bits[start_word] |= start_mask as i64;
        for i in (start_word + 1)..end_word {
            self.bits[i] = -1_i64;
        }
        self.bits[end_word] |= end_mask as i64;
    }
    fn next_set_bit_impl(&self, start: i32, upper_bound: i32) -> i32 {
        // Depends on the ghost bits being clear!
        debug_assert!(
            start >= 0 && start < self.num_bits,
            "index = {}, num_bits = {}",
            start,
            self.num_bits
        );
        debug_assert!(
            start < upper_bound,
            "index = {}, upper_bound= {}",
            start,
            upper_bound
        );
        debug_assert!(
            upper_bound <= self.num_bits,
            "upper_bound = {}, num_bits = {}",
            upper_bound,
            self.num_bits
        );
        let mut i = (start >> 6) as usize;
        let mut word = self.bits[i] >> (start % 64); //skip all the bits to the right of index

        if word != 0 {
            return start + word.trailing_zeros() as i32;
        }

        let limit = if upper_bound == self.num_bits {
            self.num_words as usize
        } else {
            Self::bits2words(upper_bound) as usize
        };
        i += 1;
        while i < limit {
            word = self.bits[i];
            if word != 0 {
                return ((i << 6) + word.trailing_zeros() as usize) as i32;
            }
            i += 1;
        }
        NO_MORE_DOCS
    }

    pub fn copy_of() -> FixedBitSet {
        todo!()
    }

    /// Converts this instance to a read-only [`Bits`].
    /// This is useful in cases where this [`FixedBitSet`]
    /// is returned as a [`Bits`] instance, to ensure that consumers cannot
    /// get write access by casting to a [`FixedBitSet`].
    ///
    /// # Note
    /// Changes to this [`FixedBitSet`] will be reflected
    /// on the returned [`Bits`].
    pub fn as_read_only_bits(&self) -> FixedBits {
        FixedBits::new(&self.bits, self.num_bits)
    }
}

impl Bits for FixedBitSet {
    fn get(&self, index: i32) -> bool {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let i = index >> 6;
        // signed shift will keep a negative index and force an
        // array-index-out-of-bounds-exception, removing the need for an explicit check.
        let bit_mask = 1_i64 << (index % 64);
        (bit_mask & self.bits[i as usize]) != 0
    }

    fn length(&self) -> i32 {
        self.num_bits
    }
}

impl Accountable for FixedBitSet {
    fn ram_bytes_used(&self) -> Result<i64> {
        // TODO
        Ok(0)
    }
}

impl BitSet for FixedBitSet {
    fn set(&mut self, index: i32) {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let word_num = index >> 6;
        let bit_mask = 1_i64 << (index % 64);
        self.bits[word_num as usize] |= bit_mask;
    }

    fn get_and_set(&mut self, index: i32) -> bool {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let word_num = (index >> 6) as usize;
        let bit_mask = 1_i64 << (index % 64);
        let val = (self.bits[word_num] & bit_mask) != 0;
        self.bits[word_num] |= bit_mask;
        val
    }

    fn clear_with_index(&mut self, index: i32) {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let word_num = index >> 6;
        let bit_mask = 1_i64 << (index % 64);
        self.bits[word_num as usize] &= !bit_mask;
    }

    fn clear_range(&mut self, start_index: i32, end_index: i32) {
        debug_assert!(
            start_index >= 0 && start_index < self.num_bits,
            "start_index = {}, num_bits = {}",
            start_index,
            self.num_bits
        );
        debug_assert!(
            end_index >= 0 && end_index <= self.num_bits,
            "end_index = {}, num_bits = {}",
            end_index,
            self.num_bits
        );
        if end_index <= start_index {
            return;
        }
        let start_word = (start_index >> 6) as usize;
        let end_word = ((end_index - 1) >> 6) as usize;

        let mut start_mask = u64::MAX << (start_index % 64);
        let mut end_mask = u64::MAX >> ((64 - (end_index % 64)) % 64);

        start_mask = !start_mask;
        end_mask = !end_mask;
        if start_word == end_word {
            self.bits[start_word] &= start_mask as i64 | end_mask as i64;
            return;
        }

        self.bits[start_word] &= start_mask as i64;
        for i in (start_word + 1)..end_word {
            self.bits[i] = 0;
        }
        self.bits[end_word] &= end_mask as i64
    }

    /// Returns the number of set bits.
    ///
    /// # Note
    /// This visits every `u64` in the backing bits array, and the result is not internally cached.
    fn cardinality(&self) -> i32 {
        // Depends on the ghost bits being clear!
        let mut tot: i64 = 0;
        for i in 0..self.num_words as usize {
            tot += self.bits[i].count_ones() as i64;
        }

        tot as i32
    }

    fn approximate_cardinality(&self) -> i32 {
        // Naive sampling: compute the number of bits that are set on the first 16 longs every 1024
        // longs and scale the result by 1024/16.
        // This computes the pop count on ranges instead of single longs in order to take advantage of
        // vectorization.
        let range_length = 16;
        let interval: usize = 1024;

        if self.num_words as usize <= interval {
            return self.cardinality();
        }

        let mut pop_count: i64 = 0;
        let mut max_word = 0;
        let num = self.num_words as usize;
        while max_word + interval < num {
            for i in 0..range_length {
                pop_count += (self.bits[max_word + i].count_ones()) as i64;
            }
            max_word += interval;
        }
        pop_count *= ((interval / range_length) * self.num_words as usize / max_word) as i64;

        pop_count as i32
    }

    fn prev_set_bit(&self, index: i32) -> i32 {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let mut i = index >> 6;
        let sub_index = index & 0x3f; //  index within the word

        let mut word = self.bits[i as usize] << (63 - (sub_index % 64));

        if word != 0 {
            return (i << 6) + sub_index - word.leading_zeros() as i32;
        }

        i -= 1;

        while i >= 0 {
            word = self.bits[i as usize];
            if word != 0 {
                return (i << 6) + 63 - word.leading_zeros() as i32;
            }
            i -= 1;
        }
        -1
    }

    fn next_set_bit(&self, index: i32) -> i32 {
        self.next_set_bit_range(index, self.num_bits)
    }

    /// Returns the next set a bit in the specified range, but treats `upper_bound` as a best-effort hint
    /// rather than a hard requirement. Note that this may return a result that is greater than or equal
    /// to `upper_bound` in some cases, so callers must add their own check if `upper_bound` is a hard requirement.
    fn next_set_bit_range(&self, start: i32, upper_bound: i32) -> i32 {
        let res = self.next_set_bit_impl(start, upper_bound);
        if res < upper_bound {
            res
        } else {
            NO_MORE_DOCS
        }
    }

    fn or<T: DocIdSetIterator>(&mut self, mut iter: T) -> Result<()> {
        //TODO: this is a naive implementation, we can optimize it from Java Lucene
        Self::check_unpositioned(&iter)?;
        let mut doc = iter.next_doc()?;
        while doc != NO_MORE_DOCS {
            self.set(doc);
            doc = iter.next_doc()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::test::util::base_bit_set_test_case::{
        BaseBitSetTestCase, BaseBitSetTestCaseSupperImpl, RustUtilBitSet,
    };
    use crate::util::bit_set::BitSet;
    use crate::util::bit_set_iterator::BitSetIterator;
    use crate::util::bits::Bits;
    use crate::util::doc_base_bit_set_iterator::DocBaseBitSetIterator;
    use rand::rngs::StdRng;
    use rand::Rng;

    use crate::test::util::id_set_common::{clear_range, flip_bit, flip_bit_range, set_range};
    use crate::test::util::lucene_test_case::{is_night_mode, random};

    use crate::search::doc_id_set_iterator::doc_id_set_iterator_static::NO_MORE_DOCS;
    use crate::util::error::lucene_error::Result;
    use crate::util::fixed_bit_set::FixedBitSet;
    use crate::util::int_array_doc_id_set::IntArrayDocIdSetIterator;
    use crate::util::sparse_fixed_bit_set::SparseFixedBitSet;
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
        let mut random = random();
        let mut fbs = TestFixedBitSet;
        fbs.test_cardinality(&mut random);
    }
    #[test]
    fn test_prev_set_bit() {
        let mut random = random();
        let mut fbs = TestFixedBitSet;
        fbs.test_prev_set_bit(&mut random);
    }
    #[test]
    fn test_next_set_bit() {
        let mut random = random();
        let mut fbs = TestFixedBitSet;
        fbs.test_next_set_bit(&mut random);
    }
    #[test]
    fn test_next_set_bit_in_range() {
        let mut random = random();
        let mut fbs = TestFixedBitSet;
        fbs.test_next_set_bit_in_range(&mut random);
    }
    #[test]
    fn test_set() {
        let mut random = random();
        let fbs = TestFixedBitSet;
        fbs.test_set(&mut random);
    }
    #[test]
    fn test_get_and_set() {
        let mut random = random();
        let fbs = TestFixedBitSet;
        fbs.test_get_and_set(&mut random);
    }
    #[test]
    fn test_clear() {
        let mut random = random();
        let mut fbs = TestFixedBitSet;
        fbs.test_clear(&mut random);
    }
    #[test]
    fn test_clear_range() {
        let mut random = random();
        let fbs = TestFixedBitSet;
        fbs.test_clear_range(&mut random);
    }
    #[test]
    fn test_clear_all() {
        let mut random = random();
        let fbs = TestFixedBitSet;
        fbs.test_clear_all(&mut random);
    }
    #[test]
    fn test_or_sparse() {
        let mut random = random();
        let mut fbs = TestFixedBitSet;
        fbs.test_or_sparse(&mut random);
    }
    #[test]
    fn test_or_dense() {
        let mut random = random();
        let mut fbs = TestFixedBitSet;
        fbs.test_or_dense(&mut random);
    }
    #[test]
    fn test_or_random() {
        let mut random = random();
        let mut fbs = TestFixedBitSet;
        fbs.test_or_random(&mut random);
    }

    #[test]
    fn test_approximate_cardinality() {
        // The approximate cardinality works in such a way that it should be pretty accurate on a bitset
        // whose bits are uniformly distributed.
        let mut random = random();
        let mut set = FixedBitSet::new(random.random_range(100000..=200000));
        let first = random.random_range(0..=10);
        let interval = random.random_range(10..=20);
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
    ) -> Result<()> {
        match mode {
            1 => do_iterate1(random, a, b),
            2 => do_iterate2(random, a, b),
            _ => Ok(()),
        }
    }

    fn do_iterate1(random: &mut StdRng, a: &bit_set::BitSet, b: &FixedBitSet) -> Result<()> {
        assert_eq!(a.len(), b.cardinality() as usize);
        let mut iterator = BitSetIterator::new(b, 0)?;
        let iter = a.iter();
        for index in iter {
            let bb = if random.random_bool(0.5) {
                iterator.next_doc()?
            } else {
                iterator.advance(index as i32)?
            };
            assert_eq!(index, bb as usize);
        }
        assert_eq!(iterator.next_doc()?, NO_MORE_DOCS);
        Ok(())
    }

    fn do_iterate2(random: &mut StdRng, a: &bit_set::BitSet, b: &FixedBitSet) -> Result<()> {
        assert_eq!(a.len(), b.cardinality() as usize);
        let mut iterator = BitSetIterator::new(b, 0)?;
        let iter = a.iter();
        for index in iter {
            let bb = if random.random_bool(0.5) {
                iterator.next_doc()?
            } else {
                iterator.advance(index as i32)?
            };
            assert_eq!(index, bb as usize);
        }
        assert_eq!(iterator.next_doc()?, NO_MORE_DOCS);
        Ok(())
    }

    fn do_random_sets(random: &mut StdRng, iter: i32, mode: i32) -> Result<()> {
        // let max_size = random.random_range(1200..=i32::MAX);
        let max_size = random.random_range(1200..=100000);
        let mut a0: bit_set::BitSet = Default::default();
        let mut b0: FixedBitSet = Default::default();
        let mut flag = 0;
        for _i in 0..iter {
            let sz = random.random_range(2..max_size);
            let mut a = bit_set::BitSet::with_capacity(sz as usize);
            let mut b = FixedBitSet::new(sz);
            let n_oper = random.random_range(0..sz);
            for _j in 0..n_oper {
                let mut idx = random.random_range(0..sz);
                a.insert(idx as usize);
                b.set(idx);

                idx = random.random_range(0..sz);
                a.remove(idx as usize);
                b.clear_with_index(idx);

                idx = random.random_range(0..sz);
                flip_bit_range(&mut a, idx as usize, (idx + 1) as usize);
                b.flip_range(idx, idx + 1);

                idx = random.random_range(0..sz);
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
            from_index = random.random_range(0..(sz / 2));
            to_index = from_index + random.random_range(0..(sz - from_index));
            let mut aa = a.clone();
            flip_bit_range(&mut aa, from_index as usize, to_index as usize);
            let mut bb = b.clone();
            bb.flip_range(from_index, to_index);

            do_iterate(random, &aa, &bb, mode)?; //  a problem here is from flip or doIterate

            from_index = random.random_range(0..(sz / 2));
            to_index = from_index + random.random_range(0..(sz - from_index));
            aa.clone_from(&a);
            clear_range(&mut aa, from_index as usize, to_index as usize);
            bb = b.clone();
            bb.clear_range(from_index, to_index);

            do_next_set_bit(&aa, &bb); // a problem here is from clear() or nextSetBit

            do_prev_set_bit(&aa, &bb);

            from_index = random.random_range(0..(sz / 2));
            to_index = from_index + random.random_range(0..(sz - from_index));
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
    fn test_small() -> Result<()> {
        let mut random = random();
        let iters = if is_night_mode() {
            random.random_range(1000..100000)
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
        let mut random = random();
        let num_bits = random.random_range(0..2000) + 1;
        let mut b1 = FixedBitSet::new(num_bits);
        let mut b2 = FixedBitSet::new(num_bits);
        assert!(b1.eq(&b2));
        assert!(b2.eq(&b1));
        for _i in 0..random.random_range(1000..5000) {
            let idx = random.random_range(0..num_bits);
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
        let mut random = random();

        let num_bits = random.random_range(0..2000) + 1;
        let mut b1 = FixedBitSet::new(num_bits);
        let mut b2 = FixedBitSet::new(num_bits);
        for _i in 0..random.random_range(1000..5000) {
            let idx = random.random_range(0..num_bits);
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

    fn make_fixed_bitset(random: &mut StdRng, a: &Vec<i32>, num_bits: i32) -> Result<FixedBitSet> {
        let mut bs: FixedBitSet;
        if random.random_bool(0.5) {
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
        let mut random = random();
        let capacity = random.random_range(0..1000);
        let mut set_bits: Vec<i32> = Vec::with_capacity(capacity as usize);
        for _i in 0..capacity {
            set_bits.push(random.random_range(0..capacity));
        }
        let num_bits = set_bits.len() + random.random_range(0..10);
        check_next_set_bit_array(&mut random, set_bits, num_bits as i32);
        check_next_set_bit_array(&mut random, vec![], num_bits as i32);
    }

    #[test]
    fn test_ensure_capacity() -> Result<()> {
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
            rv.push(random.random_range(min..=max));
        }
        rv
    }

    #[test]
    fn test_intersection_count() {
        let mut random = random();

        let num_bits1 = random.random_range(1000..=2000);
        let num_bits2 = random.random_range(1000..=2000);

        let count1 = random.random_range(0..=num_bits1 - 1);
        let count2 = random.random_range(0..=num_bits2 - 1);

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
    fn test_and_not() -> Result<()> {
        let mut random = random();

        let num_bits2 = random.random_range(1000..=2000);
        let num_bits1 = random.random_range(1000..=num_bits2);

        let count1 = random.random_range(0..=num_bits1 - 1);
        let count2 = random.random_range(0..=num_bits2 - 1);

        let min = random.random_range(0..=(num_bits1 - 1));
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
    fn test_union_count() -> Result<()> {
        let mut random = random();
        let num_bits1 = random.random_range(1000..=2000);
        let num_bits2 = random.random_range(1000..=2000);

        let count1 = random.random_range(0..=num_bits1 - 1);
        let count2 = random.random_range(0..=num_bits2 - 1);

        let bits1 = make_int_array(&mut random, count1, 0, num_bits1 - 1);
        let bits2 = make_int_array(&mut random, count2, 0, num_bits2 - 1);

        let fixed_bit_set1 = make_fixed_bitset(&mut random, &bits1, num_bits1)?;
        let fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2)?;

        let union_count = FixedBitSet::union_count(&fixed_bit_set1, &fixed_bit_set2);

        let mut bit_set1 = make_bitset(&bits1);
        let bit_set2 = make_bitset(&bits2);
        bit_set1.union_with(&bit_set2);

        assert_eq!(bit_set1.len(), union_count as usize);
        Ok(())
    }

    #[test]
    fn test_and_not_count() -> Result<()> {
        let mut random = random();

        let num_bits1 = random.random_range(1000..=2000);
        let num_bits2 = random.random_range(1000..=2000);

        let count1 = random.random_range(0..=num_bits1 - 1);
        let count2 = random.random_range(0..=num_bits2 - 1);

        let bits1 = make_int_array(&mut random, count1, 0, num_bits1 - 1);
        let bits2 = make_int_array(&mut random, count2, 0, num_bits2 - 1);

        let fixed_bit_set1 = make_fixed_bitset(&mut random, &bits1, num_bits1)?;
        let fixed_bit_set2 = make_fixed_bitset(&mut random, &bits2, num_bits2)?;

        let and_not_count = FixedBitSet::and_not_count(&fixed_bit_set1, &fixed_bit_set2);

        let mut bit_set1 = make_bitset(&bits1);
        let bit_set2 = make_bitset(&bits2);

        bit_set1.difference_with(&bit_set2);

        assert_eq!(bit_set1.len(), and_not_count as usize);
        Ok(())
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
}
