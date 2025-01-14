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
use crate::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::util::accountable::Accountable;
use crate::util::bit_set::BitSet;
use crate::util::bits::Bits;

use crate::util::error::lucene_error::LuceneError;
use crate::util::fixed_bits::FixedBits;
use std::cmp::min;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

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
    bits: Vec<u64>,
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
    pub fn ensure_capacity(bits: &mut FixedBitSet, num_bits: i32) {
        if num_bits < bits.num_bits {
        } else {
            let num_words = Self::bits2words(num_bits);
            let arr_len = bits.bits.len() as i32;
            // TODO:should not add another 64bit so simply,see what Java lucene `ArrayUtil.grow`
            let grow = 1;
            bits.num_bits = num_bits + (64 * grow);
            bits.num_words = num_words + grow;
            // diff to Java Lucene
            for _i in 0..(bits.num_words - arr_len) {
                bits.bits.push(0);
            }
        }
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
        let num_common_words = min(a.num_words, b.num_words);
        for i in 0..num_common_words {
            tot += (a.bits[i as usize] & b.bits[i as usize]).count_ones();
        }
        tot as i64
    }

    //// Returns the popcount or cardinality of the union of the two sets. Neither set is modified.
    pub fn union_count(a: &FixedBitSet, b: &FixedBitSet) -> i64 {
        // Depends on the ghost bits being clear!
        let mut tot = 0;
        let num_common_words = min(a.num_words, b.num_words);
        for i in 0..num_common_words {
            tot += (a.bits[i as usize] | b.bits[i as usize]).count_ones();
        }
        for i in num_common_words..a.num_words {
            tot += a.bits[i as usize].count_ones();
        }
        for i in num_common_words..b.num_words {
            tot += b.bits[i as usize].count_ones();
        }
        tot as i64
    }

    /// Returns the popcount or cardinality of "a and not b" or "intersection(a not(b))". Neither set
    /// is modified.
    pub fn and_not_count(a: &FixedBitSet, b: &FixedBitSet) -> i64 {
        let mut tot = 0;
        let num_common_words = min(a.num_words, b.num_words);
        for i in 0..num_common_words {
            tot += (a.bits[i as usize] & !b.bits[i as usize]).count_ones();
        }
        for i in num_common_words..a.num_words {
            tot += a.bits[i as usize].count_ones();
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
        let bits: Vec<u64> = vec![0; size];
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
    pub fn with_capacity(stored_bits: Vec<u64>, num_bits: i32) -> Result<FixedBitSet, LuceneError> {
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

        let mask = u64::MAX << (fixed_bit_set.num_bits % 64);
        (fixed_bit_set.bits[(fixed_bit_set.num_words as usize) - 1] & mask) == 0
    }

    #[allow(unused)]
    fn get_bits(&self) -> &Vec<u64> {
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
        let word_num = index >> 6;
        let bit_mask = 1_u64 << (index % 64);
        let val = (self.bits[word_num as usize] & bit_mask) != 0;
        self.bits[word_num as usize] &= !bit_mask;
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

    fn or_impl(&mut self, other_offset_words: i32, other_arr: &[u64], other_num_words: i32) {
        debug_assert!(
            other_num_words + other_offset_words <= self.num_words,
            "num_words = {} other_num_words = {}",
            self.num_words,
            other_num_words
        );
        let pos = min(self.num_words - other_offset_words, other_num_words);
        for i in (0..pos).rev() {
            self.bits[(i + other_offset_words) as usize] |= other_arr[i as usize];
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
    fn xor_impl(&mut self, other_bits: &[u64], other_num_words: i32) {
        debug_assert!(
            other_num_words <= self.num_words,
            "num_words = {} other_num_words = {}",
            self.num_words,
            other_num_words
        );
        let pos = min(self.num_words, other_num_words);
        for i in (0..pos).rev() {
            self.bits[i as usize] ^= other_bits[i as usize];
        }
    }

    pub fn intersects(&self, other: &FixedBitSet) -> bool {
        // Depends on the ghost bits being clear!
        let pos = min(self.num_words, other.num_words);
        for i in (0..pos).rev() {
            if self.bits[i as usize] != other.bits[i as usize] {
                return true;
            }
        }
        false
    }

    /// this = this AND other
    pub fn and(&mut self, other: &FixedBitSet) {
        self.and_self(&other.bits, other.num_words);
    }

    pub fn and_self(&mut self, other_arr: &[u64], other_num_words: i32) {
        let pos = min(self.num_words, other_num_words);
        for i in (0..pos).rev() {
            self.bits[i as usize] &= other_arr[i as usize];
        }

        if self.num_words > other_num_words {
            for i in other_num_words..self.num_words {
                self.bits[i as usize] = 0;
            }
        }
    }

    pub fn and_not_iter(&mut self, mut iter: impl DocIdSetIterator) -> Result<(), LuceneError> {
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

    fn and_not_impl(&mut self, other_offset_words: i32, other_arr: &[u64], other_num_words: i32) {
        let pos = min(self.num_words - other_offset_words, other_num_words);
        for i in (0..pos).rev() {
            self.bits[(i + other_offset_words) as usize] &= !other_arr[i as usize];
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
        let start_word = start_index >> 6;
        let end_word = (end_index - 1) >> 6;

        let start_mask = u64::MAX << (start_index % 64);
        let end_mask = u64::MAX >> ((64 - (end_index % 64)) % 64);

        if start_word == end_word {
            self.bits[start_word as usize] ^= start_mask & end_mask;
            return;
        }

        self.bits[start_word as usize] ^= start_mask;

        for i in start_word + 1..end_word {
            self.bits[i as usize] = !self.bits[i as usize];
        }

        self.bits[end_word as usize] ^= end_mask;
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
        let bit_mask = 1_u64 << (index % 64);
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

        let start_word = start_index >> 6;
        let end_word = (end_index - 1) >> 6;

        let start_mask = u64::MAX << (start_index % 64);
        let end_mask = u64::MAX >> ((64 - (end_index % 64)) % 64);

        if start_word == end_word {
            self.bits[start_word as usize] |= start_mask & end_mask;
            return;
        }

        self.bits[start_word as usize] |= start_mask;
        for i in (start_word + 1)..end_word {
            self.bits[i as usize] = u64::MAX;
        }
        self.bits[end_word as usize] |= end_mask;
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
        let mut i = start >> 6;
        let mut word = self.bits[i as usize] >> (start % 64); //skip all the bits to the right of index

        if word != 0 {
            return start + word.trailing_zeros() as i32;
        }

        let limit = if upper_bound == self.num_bits {
            self.num_words
        } else {
            Self::bits2words(upper_bound)
        };
        i += 1;
        while i < limit {
            word = self.bits[i as usize];
            if word != 0 {
                return (i << 6) + word.trailing_zeros() as i32;
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
        let bit_mask = 1_u64 << (index % 64);
        (bit_mask & self.bits[i as usize]) != 0
    }

    fn length(&self) -> i32 {
        self.num_bits
    }
}

impl Accountable for FixedBitSet {
    fn ram_bytes_used(&self) -> u64 {
        todo!()
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
        let bit_mask = 1_u64 << (index % 64);
        self.bits[word_num as usize] |= bit_mask;
    }

    fn get_and_set(&mut self, index: i32) -> bool {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let word_num = index >> 6;
        let bit_mask = 1_u64 << (index % 64);
        let val = (self.bits[word_num as usize] & bit_mask) != 0;
        self.bits[word_num as usize] |= bit_mask;
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
        let bit_mask = 1_u64 << (index % 64);
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
        let start_word = start_index >> 6;
        let end_word = (end_index - 1) >> 6;

        let mut start_mask = u64::MAX << (start_index % 64);
        let mut end_mask = u64::MAX >> ((64 - (end_index % 64)) % 64);

        start_mask = !start_mask;
        end_mask = !end_mask;

        if start_word == end_word {
            self.bits[start_word as usize] &= start_mask | end_mask;
            return;
        }

        self.bits[start_word as usize] &= start_mask;
        for i in (start_word + 1)..end_word {
            self.bits[i as usize] = 0;
        }
        self.bits[end_word as usize] &= end_mask
    }

    /// Returns the number of set bits.
    ///
    /// # Note
    /// This visits every `u64` in the backing bits array, and the result is not internally cached.
    fn cardinality(&self) -> i32 {
        // Depends on the ghost bits being clear!
        let mut tot: i64 = 0;
        for i in 0..self.num_words {
            tot += self.bits[i as usize].count_ones() as i64;
        }

        tot as i32
    }

    fn approximate_cardinality(&self) -> i32 {
        // Naive sampling: compute the number of bits that are set on the first 16 longs every 1024
        // longs and scale the result by 1024/16.
        // This computes the pop count on ranges instead of single longs in order to take advantage of
        // vectorization.
        let range_length = 16;
        let interval = 1024;

        if self.num_words <= interval {
            return self.cardinality();
        }

        let mut pop_count: i64 = 0;
        let mut max_word = 0;
        while max_word + interval < self.num_words {
            for i in 0..range_length {
                pop_count += (self.bits[(max_word + i) as usize].count_ones()) as i64;
            }
            max_word += interval;
        }
        pop_count *= ((interval / range_length) * self.num_words / max_word) as i64;

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

    fn or<T: DocIdSetIterator>(&mut self, mut iter: T) -> Result<(), LuceneError> {
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
