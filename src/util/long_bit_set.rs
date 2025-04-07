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
use crate::util::accountable::Accountable;
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::hash::Hash;
/// BitSet of fixed length (`numBits`), backed by accessible ([`get_bits`](LongBitSet::get_bits)) `&[i64]`,
/// accessed with a `long` index. Use it only if you intend to store more than 2.1B bits,
/// otherwise you should use [`FixedBitSet`](crate::util::fixed_bit_set::FixedBitSet).
pub struct LongBitSet {
    bits: Vec<i64>, // Array of longs holding the bits
    num_bits: i64,  // The number of bits in use
    num_words: i32, // The exact number of longs needed to hold numBits (<= bits.length)
}
impl LongBitSet {
    pub const MAX_NUM_BITS: i64 = 64 * ArrayUtil::MAX_ARRAY_LENGTH as i64;
    /// If the given [`LongBitSet`] is large enough to hold `num_bits + 1`, returns the given bitset,
    /// otherwise returns a new [`LongBitSet`] which can hold the requested number of bits.
    ///
    /// **NOTE:** the returned bitset reuses the underlying `long[]` of the given `bits` if possible.
    /// Also, calling [`length()`](LongBitSet::length) on the returned bitset may return a value greater than `num_bits`.
    pub fn ensure_capacity(bits: LongBitSet, num_bits: i64) -> Result<LongBitSet> {
        if num_bits < bits.num_bits {
            return Ok(bits);
        }

        // Depends on the ghost bits being clear!
        // (Otherwise, they may become visible in the new instance)
        let num_words = LongBitSet::bits2words(num_bits)?;
        let mut arr = bits.bits;

        if num_words as usize >= arr.len() {
            ArrayUtil::grow_with_len(&mut arr, num_words + 1)?;
        }

        let len = arr.len();
        LongBitSet::from_bits(arr, (len << 6) as i64)
    }
    /// Returns the number of 64-bit words needed to hold `num_bits`.
    pub fn bits2words(num_bits: i64) -> Result<i32> {
        if num_bits > Self::MAX_NUM_BITS {
            return Err(LuceneError::illegal_argument(format!(
                "num_bits must be in range 0..={} but got {}",
                Self::MAX_NUM_BITS,
                num_bits
            )));
        }
        Ok((((num_bits - 1) >> 6) + 1) as i32)
    }
    /// Creates a new [`LongBitSet`]. The internally allocated `[i64]` will be exactly the size
    /// needed to accommodate the `numBits` specified.
    ///
    /// # Arguments
    /// * `num_bits` - the number of bits needed
    pub fn new(num_bits: i64) -> Result<Self> {
        let num_words = Self::bits2words(num_bits)?;
        let bits = vec![0i64; num_words as usize];
        Ok(Self {
            bits,
            num_bits,
            num_words,
        })
    }
    /// Creates a new [`LongBitSet`] using the provided `long[]` array as backing store.  
    /// The `stored_bits` array must be large enough to accommodate the `num_bits` specified,  
    /// but may be larger. In that case the 'extra' or 'ghost' bits must be clear  
    /// (or they may provoke spurious side-effects).
    ///
    /// # Arguments
    /// * `stored_bits` - the array to use as backing store  
    /// * `num_bits` - the number of bits actually needed
    pub fn from_bits(stored_bits: Vec<i64>, num_bits: i64) -> Result<Self> {
        let num_words = Self::bits2words(num_bits)?;
        if num_words as usize > stored_bits.len() {
            return Err(LuceneError::illegal_argument(format!(
                "The given long array is too small to hold {} bits",
                num_bits
            )));
        }

        let bitset = Self {
            bits: stored_bits,
            num_bits,
            num_words,
        };
        debug_assert!(bitset.verify_ghost_bits_clear());
        Ok(bitset)
    }
    /**
     * Checks if the bits past numBits are clear. Some methods rely on this implicit assumption:
     * search for "Depends on the ghost bits being clear!"
     *
     * return true if the bits past numBits are clear.
     */
    fn verify_ghost_bits_clear(&self) -> bool {
        for i in self.num_words as usize..self.bits.len() {
            if self.bits[i] != 0 {
                return false;
            }
        }

        if (self.num_bits & 0x3f) == 0 {
            return true;
        }

        let mask = !0i64 << self.num_bits;
        (self.bits[self.num_words as usize - 1] & mask) == 0
    }
    /// Returns the number of bits stored in this bitset.
    pub fn length(&self) -> i64 {
        self.num_bits
    }

    /// Expert.
    pub fn get_bits(&self) -> &[i64] {
        &self.bits
    }

    /// Returns number of set bits.
    ///
    /// NOTE: this visits every long in the backing bits array,
    /// and the result is not internally cached!
    ///
    /// This relies on ghost bits being clear.
    pub fn cardinality(&self) -> i64 {
        // Depends on the ghost bits being clear!
        self.bits[..self.num_words as usize]
            .iter()
            .map(|v| v.count_ones() as i64)
            .sum()
    }

    pub fn get(&self, index: i64) -> bool {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let i = (index >> 6) as usize;
        // signed shift will keep a negative index and force an
        // array-index-out-of-bounds-exception, removing the need for an explicit check.
        let bitmask = 1i64 << index;
        (self.bits[i] & bitmask) != 0
    }

    pub fn set(&mut self, index: i64) {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let word_num = (index >> 6) as usize;
        let bitmask = 1i64 << index;
        self.bits[word_num] |= bitmask;
    }

    /// Returns the previous value of the bit at `index`, and sets it.
    pub fn get_and_set(&mut self, index: i64) -> bool {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let word_num = (index >> 6) as usize;
        let bitmask = 1i64 << index;
        let val = (self.bits[word_num] & bitmask) != 0;
        self.bits[word_num] |= bitmask;
        val
    }
    pub fn clear(&mut self, index: i64) {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let word_num = (index >> 6) as usize;
        let bitmask = 1i64 << index;
        self.bits[word_num] &= !bitmask;
    }

    /// Returns the previous value of the bit at `index`, and clears it.
    #[allow(unused)]
    pub fn get_and_clear(&mut self, index: i64) -> bool {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let word_num = (index >> 6) as usize;
        let bitmask = 1i64 << index;
        let val = (self.bits[word_num] & bitmask) != 0;
        self.bits[word_num] &= !bitmask;
        val
    }

    /// Returns the index of the first set bit starting at the given `index`.
    /// Returns -1 if no such bit is found.
    ///
    /// Depends on ghost bits being clear!
    pub fn next_set_bit(&self, index: i64) -> i64 {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );

        let mut i = (index >> 6) as usize;
        let sub_index = index & 63;
        let mut word = self.bits[i] >> sub_index;

        if word != 0 {
            return index + word.trailing_zeros() as i64;
        }

        i += 1;
        while i < self.num_words as usize {
            word = self.bits[i];
            if word != 0 {
                return (i as i64) << 6 | word.trailing_zeros() as i64;
            }
            i += 1;
        }
        -1
    }
    /// Returns the index of the last set bit before or on the given `index`.
    /// Returns -1 if there are no more set bits.
    pub fn prev_set_bit(&self, index: i64) -> i64 {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );

        let mut i = (index >> 6) as i32;
        let sub_index = (index & 0x3f) as i32;
        let mut word = self.bits[i as usize] << (63 - sub_index);

        if word != 0 {
            return (i << 6) as i64 + sub_index as i64 - word.leading_zeros() as i64;
        }

        i -= 1;
        while i >= 0 {
            word = self.bits[i as usize];
            if word != 0 {
                return (i << 6) as i64 + 63 - word.leading_zeros() as i64;
            }
            i -= 1;
        }

        -1
    }

    /// Performs bitwise OR: this = this OR other
    pub fn or(&mut self, other: &LongBitSet) {
        debug_assert!(
            other.num_words <= self.num_words,
            "num_words = {}, other.num_words = {}",
            self.num_words,
            other.num_words
        );

        let pos = std::cmp::min(self.num_words, other.num_words);
        for i in 0..pos as usize {
            self.bits[i] |= other.bits[i];
        }
    }
    /// Performs bitwise XOR: this = this XOR other
    pub fn xor(&mut self, other: &LongBitSet) {
        debug_assert!(
            other.num_words <= self.num_words,
            "num_words = {}, other.num_words = {}",
            self.num_words,
            other.num_words
        );

        let pos = std::cmp::min(self.num_words, other.num_words);
        for i in 0..pos as usize {
            self.bits[i] ^= other.bits[i];
        }
    }

    /// Returns true if the sets have any elements in common.
    ///
    /// Depends on the ghost bits being clear!
    pub fn intersects(&self, other: &LongBitSet) -> bool {
        let pos = std::cmp::min(self.num_words, other.num_words);
        for i in 0..pos as usize {
            if (self.bits[i] & other.bits[i]) != 0 {
                return true;
            }
        }
        false
    }

    /// Performs bitwise AND: this = this AND other
    pub fn and(&mut self, other: &LongBitSet) {
        let pos = std::cmp::min(self.num_words, other.num_words);
        for i in 0..pos as usize {
            self.bits[i] &= other.bits[i];
        }
        if self.num_words > other.num_words {
            for i in other.num_words as usize..self.num_words as usize {
                self.bits[i] = 0;
            }
        }
    }

    /// Performs bitwise AND NOT: this = this AND NOT other
    pub fn and_not(&mut self, other: &LongBitSet) {
        let pos = std::cmp::min(self.num_words, other.num_words);
        for i in 0..pos as usize {
            self.bits[i] &= !other.bits[i];
        }
    }
    /// Scans the backing store to check if all bits are clear.
    ///
    /// The method is deliberately not called "is_empty" to emphasize it is not low cost.
    ///
    /// This depends on the ghost bits being clear!
    #[allow(unused)]
    pub fn scan_is_empty(&self) -> bool {
        for i in 0..self.num_words as usize {
            if self.bits[i] != 0 {
                return false;
            }
        }
        true
    }

    /// Flips a range of bits in [start_index, end_index)
    ///
    /// - `start_index`: lower bound (inclusive)
    /// - `end_index`: upper bound (exclusive)
    pub fn flip(&mut self, start_index: i64, end_index: i64) {
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

        // Example: 11111111_11100000
        let start_mask = !0i64 << start_index;

        // Note: 64 - (end_index & 0x3f) == -end_index in 6-bit context
        let end_mask = (!0i64 as u64 >> -end_index) as i64;

        if start_word == end_word {
            self.bits[start_word] ^= start_mask & end_mask;
            return;
        }

        self.bits[start_word] ^= start_mask;

        for i in (start_word + 1)..end_word {
            self.bits[i] = !self.bits[i];
        }

        self.bits[end_word] ^= end_mask;
    }
    /// Flip the bit at the provided index.
    pub fn flip_one(&mut self, index: i64) {
        debug_assert!(
            index >= 0 && index < self.num_bits,
            "index = {}, num_bits = {}",
            index,
            self.num_bits
        );
        let word_num = (index >> 6) as usize;
        let bitmask = 1i64 << index; // mod 64 is implicit
        self.bits[word_num] ^= bitmask;
    }

    /// Sets a range of bits in [start_index, end_index)
    ///
    /// - `start_index`: lower index (inclusive)
    /// - `end_index`: one-past the last bit to set
    pub fn set_range(&mut self, start_index: i64, end_index: i64) {
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

        let start_mask = !0i64 << start_index;
        let end_mask = (!0i64 as u64 >> -end_index) as i64;

        if start_word == end_word {
            self.bits[start_word] |= start_mask & end_mask;
            return;
        }

        self.bits[start_word] |= start_mask;
        for i in (start_word + 1)..end_word {
            self.bits[i] = -1;
        }
        self.bits[end_word] |= end_mask;
    }
    /// Clears a range of bits in [start_index, end_index)
    ///
    /// - `start_index`: lower index (inclusive)
    /// - `end_index`: one-past the last bit to clear
    pub fn clear_range(&mut self, start_index: i64, end_index: i64) {
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

        let mut start_mask = !0i64 << start_index;
        let mut end_mask = ((!0i64 as u64) >> -end_index) as i64;

        // Invert masks since we are clearing
        start_mask = !start_mask;
        end_mask = !end_mask;

        if start_word == end_word {
            self.bits[start_word] &= start_mask | end_mask;
            return;
        }

        self.bits[start_word] &= start_mask;
        for i in (start_word + 1)..end_word {
            self.bits[i] = 0;
        }
        self.bits[end_word] &= end_mask;
    }
}
impl Accountable for LongBitSet {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}
impl Clone for LongBitSet {
    fn clone(&self) -> Self {
        LongBitSet::from_bits(self.bits.clone(), self.num_bits).unwrap()
    }
}
impl PartialEq for LongBitSet {
    fn eq(&self, other: &Self) -> bool {
        if self.num_bits != other.num_bits {
            return false;
        }
        self.bits == other.bits
    }
}
impl Eq for LongBitSet {}
impl Hash for LongBitSet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bits.hash(state);
        self.num_bits.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use crate::test::util::id_set_common::{clear_range, flip_bit, flip_bit_range, set_range};
    use crate::test::util::lucene_test_case::{at_least, is_night_mode, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;
    use crate::util::long_bit_set::LongBitSet;
    use bit_set::BitSet;
    use rand::rngs::StdRng;
    use rand::Rng;

    #[allow(dead_code)] // for quick search
    struct TestLongBitSet;

    fn do_get(a: &BitSet, b: &LongBitSet) {
        assert_eq!(a.len(), b.cardinality() as usize);
        let max = b.length();
        for i in 0..max {
            let abit = a.contains(i as usize);
            let bbit = b.get(i);
            if abit != bbit {
                unreachable!("mismatch: BitSet[{}] = {}", i, abit);
            }
        }
    }
    fn do_next_set_bit(a: &BitSet, b: &LongBitSet) {
        assert_eq!(a.len(), b.cardinality() as usize);
        let mut aa: i64 = -1;
        let mut bb: i64 = -1;
        loop {
            aa += 1;
            while aa < a.len() as i64 && !a.contains(aa as usize) {
                aa += 1;
            }
            if aa >= a.len() as i64 {
                aa = -1;
            }

            // b.next_set_bit
            if bb < b.length() - 1 {
                bb = b.next_set_bit(bb + 1);
            } else {
                bb = -1;
            }

            assert_eq!(aa, bb);
            if aa < 0 {
                break;
            }
        }
    }

    fn do_prev_set_bit(random: &mut StdRng, a: &BitSet, b: &LongBitSet) {
        assert_eq!(a.len(), b.cardinality() as usize);

        let mut aa = a.get_ref().len() as i64 + random.random_range(0..100);
        let mut bb = aa;

        loop {
            // simulate a.prevSetBit
            aa -= 1;
            while aa >= 0 && !a.contains(aa as usize) {
                aa -= 1;
            }

            if b.length() == 0 {
                bb = -1;
            } else if bb > (b.length() - 1) {
                bb = b.prev_set_bit(b.length() - 1);
            } else if bb < 1 {
                bb = -1;
            } else {
                bb = if bb >= 1 { b.prev_set_bit(bb - 1) } else { -1 }
            }

            assert_eq!(aa, bb);
            if aa < 0 {
                break;
            }
        }
    }
    fn do_random_sets(max_size: i32, iter: i32, _mode: i32, random: &mut StdRng) -> Result<()> {
        let mut a0: Option<BitSet> = None;
        let mut b0: Option<LongBitSet> = None;

        for _ in 0..iter {
            let sz = TestUtil::next_int(random, 2, max_size as i32) as usize;

            let mut a = BitSet::with_capacity(sz);
            let mut b = LongBitSet::new(sz as i64)?;

            // test the various ways of setting bits
            if sz > 0 {
                let n_oper = random.random_range(0..sz);
                for _ in 0..n_oper {
                    let mut idx = random.random_range(0..sz);
                    a.insert(idx);
                    b.set(idx as i64);

                    idx = random.random_range(0..sz);
                    a.remove(idx);
                    b.clear(idx as i64);

                    idx = random.random_range(0..sz);
                    flip_bit_range(&mut a, idx, idx + 1);
                    b.flip(idx as i64, (idx + 1) as i64);

                    idx = random.random_range(0..sz);
                    flip_bit(&mut a, idx);
                    b.flip_one(idx as i64);

                    let val2 = b.get(idx as i64);
                    let val = b.get_and_set(idx as i64);
                    assert_eq!(val2, val);
                    assert!(b.get(idx as i64));
                    if !val {
                        b.clear(idx as i64);
                    }
                    assert_eq!(b.get(idx as i64), val);
                }
            }

            do_get(&a, &b);

            // Flip range
            let from_index = random.random_range(0..(sz / 2 + 1));
            let to_index = from_index + random.random_range(0..(sz - from_index + 1));
            let aa = a.clone();
            flip_bit_range(&mut a, from_index, to_index);
            let mut bb = b.clone();
            bb.flip(from_index as i64, to_index as i64);

            // Clear range
            let from_index = random.random_range(0..(sz / 2 + 1));
            let to_index = from_index + random.random_range(0..(sz - from_index + 1));
            let mut aa = a.clone();
            clear_range(&mut aa, from_index as usize, to_index as usize);
            let mut bb = b.clone();
            bb.clear_range(from_index as i64, to_index as i64);

            do_next_set_bit(&aa, &bb);
            do_prev_set_bit(random, &aa, &bb);

            // Set range
            let from_index = random.random_range(0..(sz / 2 + 1));
            let to_index = from_index + random.random_range(0..(sz - from_index + 1));
            let mut aa = a.clone();
            set_range(&mut aa, from_index as usize, to_index as usize);
            let mut bb = b.clone();
            bb.set_range(from_index as i64, to_index as i64);

            do_next_set_bit(&aa, &bb);
            do_prev_set_bit(random, &aa, &bb);

            // bitwise ops
            if let (Some(ref a0), Some(ref b0)) = (&a0, &b0) {
                if b0.length() <= b.length() {
                    assert_eq!(a.len(), b.cardinality() as usize);

                    let mut a_and = a.clone();
                    a_and.intersect_with(a0);
                    let mut a_or = a.clone();
                    a_or.union_with(a0);
                    let mut a_xor = a.clone();
                    a_xor.symmetric_difference_with(a0);
                    let mut a_andn = a.clone();
                    a_andn.difference_with(a0);

                    let mut b_and = b.clone();
                    assert!(b == b_and);
                    b_and.and(&b0);
                    let mut b_or = b.clone();
                    b_or.or(&b0);
                    let mut b_xor = b.clone();
                    b_xor.xor(&b0);
                    let mut b_andn = b.clone();
                    b_andn.and_not(&b0);

                    assert_eq!(a0.len(), b0.cardinality() as usize);
                    assert_eq!(a_or.len(), b_or.cardinality() as usize);
                    assert_eq!(a_and.len(), b_and.cardinality() as usize);
                    assert_eq!(a_xor.len(), b_xor.cardinality() as usize);
                    assert_eq!(a_andn.len(), b_andn.cardinality() as usize);
                }
            }
            a0 = Some(a);
            b0 = Some(b);
        }
        Ok(())
    }
    #[test]
    fn test_small() -> Result<()> {
        let mut random = random();
        let iters = if is_night_mode() {
            at_least(&mut random, 1000)
        } else {
            100
        };

        let size = at_least(&mut random, 1200);
        do_random_sets(size, iters, 1, &mut random)?;
        do_random_sets(size, iters, 2, &mut random)?;
        Ok(())
    }
}
