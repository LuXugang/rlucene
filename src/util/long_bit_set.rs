/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::hash::Hash;

use crate::util::accountable::Accountable;
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
/// BitSet of fixed length (`numBits`), backed by accessible
/// ([`get_bits`](LongBitSet::get_bits)) `&[i64]`, accessed with a `long` index.
/// Use it only if you intend to store more than 2.1B bits, otherwise you should
/// use [`FixedBitSet`](crate::util::fixed_bit_set::FixedBitSet).
#[derive(Debug)]
pub struct LongBitSet {
    bits: Vec<i64>, // Array of longs holding the bits
    num_bits: i64,  // The number of bits in use
    num_words: i32, /* The exact number of longs needed to hold numBits (<=
                     * bits.length)  */
}
impl LongBitSet {
    pub const MAX_NUM_BITS: i64 = 64 * ArrayUtil::MAX_ARRAY_LENGTH as i64;
    /// If the given [`LongBitSet`] is large enough to hold `num_bits + 1`,
    /// returns the given bitset, otherwise returns a new [`LongBitSet`]
    /// which can hold the requested number of bits.
    ///
    /// **NOTE:** the returned bitset reuses the underlying `long[]` of the
    /// given `bits` if possible. Also, calling
    /// [`length()`](LongBitSet::length) on the returned bitset may return a
    /// value greater than `num_bits`.
    pub fn ensure_capacity(bits: &mut LongBitSet, num_bits: i64) -> Result<()> {
        if num_bits < bits.num_bits {
        } else {
            let num_words = Self::bits2words(num_bits)?;
            let length = bits.bits.len();
            if num_words as usize >= length {
                ArrayUtil::grow_with_len(&mut bits.bits, (num_words + 1) as usize);
            }
            debug_assert!(bits.bits.len() <= i32::MAX as usize);
            bits.num_bits = (bits.bits.len() as i64) << 6;
            bits.num_words = Self::bits2words(bits.num_bits)?;
        }
        Ok(())
    }
    /// Returns the number of 64-bit words needed to hold `num_bits`.
    pub fn bits2words(num_bits: i64) -> Result<i32> {
        if !(0..=Self::MAX_NUM_BITS).contains(&num_bits) {
            return Err(LuceneError::illegal_argument(format!(
                "num_bits must be 0..{}; got {}",
                Self::MAX_NUM_BITS,
                num_bits
            )));
        }
        Ok((((num_bits - 1) >> 6) + 1) as i32)
    }
    /// Creates a new [`LongBitSet`]. The internally allocated `[i64]` will be
    /// exactly the size needed to accommodate the `numBits` specified.
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
    /// Creates a new [`LongBitSet`] using the provided `long[]` array as
    /// backing store. The `stored_bits` array must be large enough to
    /// accommodate the `num_bits` specified, but may be larger. In that
    /// case the 'extra' or 'ghost' bits must be clear (or they may provoke
    /// spurious side-effects).
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
     * Checks if the bits past numBits are clear. Some methods rely on this
     * implicit assumption: search for "Depends on the ghost bits being
     * clear!"
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
        // array-index-out-of-bounds-exception, removing the need for an
        // explicit check.
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
    /// The method is deliberately not called "is_empty" to emphasize it is not
    /// low cost.
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
    use std::hash::{DefaultHasher, Hash, Hasher};

    use bit_set::BitSet;
    use rand::Rng;

    use crate::test::util::id_set_common::{clear_range, flip_bit, flip_bit_range, set_range};
    use crate::test::util::lucene_test_case::{at_least, is_night_mode, random, random_multiplier};
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::{LuceneError, Result};
    use crate::util::long_bit_set::LongBitSet;

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
        let mut bb = -1;

        let iter = a.iter();
        for index in iter {
            assert_eq!(index, b.next_set_bit(index as i64) as usize);
        }

        loop {
            if bb >= b.length() - 1 {
                break;
            }
            bb = b.next_set_bit(bb + 1);
            if bb == -1 {
                break;
            }
            assert!(a.contains(bb as usize));
        }
    }

    fn do_prev_set_bit<R: Rng + ?Sized>(random: &mut R, a: &BitSet, b: &LongBitSet) {
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
    fn do_random_sets<R: Rng + ?Sized>(
        max_size: i32,
        iter: i32,
        _mode: i32,
        random: &mut R,
    ) -> Result<()> {
        let mut a0: Option<BitSet> = None;
        let mut b0: Option<LongBitSet> = None;

        for _ in 0..iter {
            let sz = TestUtil::next_int(random, 2, max_size) as usize;

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
            let mut aa = a.clone();
            flip_bit_range(&mut aa, from_index, to_index);
            let mut bb = b.clone();
            bb.flip(from_index as i64, to_index as i64);

            // Clear range
            let from_index = random.random_range(0..(sz / 2 + 1));
            let to_index = from_index + random.random_range(0..(sz - from_index + 1));
            let mut aa = a.clone();
            clear_range(&mut aa, from_index, to_index);
            let mut bb = b.clone();
            bb.clear_range(from_index as i64, to_index as i64);

            do_next_set_bit(&aa, &bb);
            do_prev_set_bit(random, &aa, &bb);

            // Set range
            let from_index = random.random_range(0..(sz / 2 + 1));
            let to_index = from_index + random.random_range(0..(sz - from_index + 1));
            let mut aa = a.clone();
            set_range(&mut aa, from_index, to_index);
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
                    b_and.and(b0);
                    let mut b_or = b.clone();
                    b_or.or(b0);
                    let mut b_xor = b.clone();
                    b_xor.xor(b0);
                    let mut b_andn = b.clone();
                    b_andn.and_not(b0);

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
    // large enough to flush obvious bugs, small enough to run in <.5 sec as
    // part of a larger testsuite.
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
    #[test]
    fn test_equals() -> Result<()> {
        let mut random = random();

        // This test can't handle num_bits == 0:
        let num_bits = random.random_range(1..2001);
        let mut b1 = LongBitSet::new(num_bits)?;
        let mut b2 = LongBitSet::new(num_bits)?;

        assert_eq!(b1, b2);
        assert_eq!(b2, b1);

        for _ in 0..10 * random_multiplier() {
            let idx = random.random_range(0..num_bits);
            if !b1.get(idx) {
                b1.set(idx);
                assert_ne!(b1, b2);
                assert_ne!(b2, b1);
                b2.set(idx);
                assert_eq!(b1, b2);
                assert_eq!(b2, b1);
            }
        }
        Ok(())
    }
    #[test]
    fn test_hash_code_equals() -> Result<()> {
        let mut random = random();

        let num_bits = random.random_range(0..2000) + 1;
        let mut b1 = LongBitSet::new(num_bits)?;
        let mut b2 = LongBitSet::new(num_bits)?;
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
        Ok(())
    }
    fn calculate_hash(a: &LongBitSet) -> u64 {
        let mut hasher = DefaultHasher::new();
        a.hash(&mut hasher);
        hasher.finish()
    }
    #[test]
    fn test_too_large() {
        let result = LongBitSet::new(LongBitSet::MAX_NUM_BITS + 1);
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("num_bits must be 0"));
    }
    #[test]
    fn test_negative_num_bits() {
        let result = LongBitSet::new(-17);
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("num_bits must be 0"));
    }
    #[test]
    fn test_small_bitsets() -> Result<()> {
        // Make sure size 0-10 bit sets are OK:
        for num_bits in 0..10 {
            let mut b1 = LongBitSet::new(num_bits)?;
            let b2 = LongBitSet::new(num_bits)?;
            assert!(b1.eq(&b2));
            assert_eq!(calculate_hash(&b1), calculate_hash(&b2));
            assert_eq!(0, b1.cardinality());
            if num_bits > 0 {
                b1.set_range(0, num_bits);
                assert_eq!(num_bits, b1.cardinality());
                b1.flip(0, num_bits);
                assert_eq!(0, b1.cardinality());
            }
        }
        Ok(())
    }
    fn make_long_bitset<R: Rng + ?Sized>(
        random: &mut R,
        a: &Vec<i32>,
        num_bits: i32,
    ) -> Result<LongBitSet> {
        let mut bs: LongBitSet;
        if random.random_bool(0.5) {
            let bits_2_words = LongBitSet::bits2words(num_bits as i64)?;
            let mut words: Vec<i64> = Vec::with_capacity(bits_2_words as usize);
            words.resize(num_bits as usize, 0);
            bs = LongBitSet::from_bits(words, num_bits as i64)?
        } else {
            bs = LongBitSet::new(num_bits as i64)?
        }
        for e in a {
            bs.set(*e as i64);
        }
        Ok(bs)
    }

    fn make_bitset(a: &Vec<i32>) -> BitSet {
        let mut bs = BitSet::with_capacity(a.len());
        for x in a {
            bs.insert(*x as usize);
        }
        bs
    }

    fn check_prev_set_bit_array<R: Rng + ?Sized>(
        random: &mut R,
        a: Vec<i32>,
        num_bits: i32,
    ) -> Result<()> {
        let obs = make_long_bitset(random, &a, num_bits)?;
        let bs = make_bitset(&a);
        do_prev_set_bit(random, &bs, &obs);
        Ok(())
    }
    #[test]
    fn test_prev_set_bit() -> Result<()> {
        let mut random = random();

        check_prev_set_bit_array(&mut random, vec![], 0)?;
        check_prev_set_bit_array(&mut random, vec![0], 1)?;
        check_prev_set_bit_array(&mut random, vec![0, 2], 3)?;

        Ok(())
    }

    fn check_next_set_bit_array<R: Rng + ?Sized>(
        random: &mut R,
        a: Vec<i32>,
        num_bits: i32,
    ) -> Result<()> {
        let obs = make_long_bitset(random, &a, num_bits)?;
        let bs = make_bitset(&a);
        do_next_set_bit(&bs, &obs);
        Ok(())
    }
    #[test]
    fn test_next_bit_set() -> Result<()> {
        let mut random = random();
        let len = random.random_range(0..1000);
        let mut set_bits = Vec::with_capacity(len);
        for _ in 0..len {
            set_bits.push(random.random_range(0..len) as i32);
        }
        let mut num_bits = len as i32 + random.random_range(0..10);
        check_next_set_bit_array(&mut random, set_bits.clone(), num_bits)?;
        num_bits = len as i32 + random.random_range(0..10);
        check_next_set_bit_array(&mut random, vec![], num_bits)?;

        Ok(())
    }
    #[test]
    fn test_ensure_capacity() -> Result<()> {
        let mut bits = LongBitSet::new(5)?;
        bits.set(1);
        bits.set(4);
        LongBitSet::ensure_capacity(&mut bits, 8)?;
        let mut new_bits = bits.clone();
        assert!(bits.get(1));
        assert!(bits.get(4));
        bits.clear(1);
        assert!(!bits.get(1));
        assert!(new_bits.get(1));

        new_bits.set(1);
        let length = bits.length();
        LongBitSet::ensure_capacity(&mut new_bits, length - 2)?;
        assert!(new_bits.get(1));

        new_bits.set(1);
        LongBitSet::ensure_capacity(&mut new_bits, 72)?;
        assert!(new_bits.get(1));
        assert!(new_bits.get(4));
        new_bits.clear(1);
        // we grew the long[], so it's not shared
        assert!(!bits.get(1));
        assert!(!new_bits.get(1));
        Ok(())
    }
    #[test]
    #[ignore]
    fn test_huge_capacity() -> Result<()> {
        let more_than_max_int = i32::MAX as i64 + 5;

        let mut bits = LongBitSet::new(42)?;
        assert_eq!(bits.length(), 42);

        LongBitSet::ensure_capacity(&mut bits, more_than_max_int)?;
        assert!(bits.length() >= more_than_max_int);

        Ok(())
    }
    #[test]
    fn test_bits2words() -> Result<()> {
        assert_eq!(LongBitSet::bits2words(0)?, 0);
        assert_eq!(LongBitSet::bits2words(1)?, 1);
        assert_eq!(LongBitSet::bits2words(64)?, 1);
        assert_eq!(LongBitSet::bits2words(65)?, 2);
        assert_eq!(LongBitSet::bits2words(128)?, 2);
        assert_eq!(LongBitSet::bits2words(129)?, 3);

        let v1 = LongBitSet::bits2words(i32::MAX as i64 + 1)?;
        assert_eq!(v1, 1 << (31 - 6));

        let v2 = LongBitSet::bits2words(i32::MAX as i64 + 2)?;
        assert_eq!(v2, (1 << (31 - 6)) + 1);

        let v3 = LongBitSet::bits2words(1i64 << 32)?;
        assert_eq!(v3, 1 << (32 - 6));

        let v4 = LongBitSet::bits2words((1i64 << 32) + 1)?;
        assert_eq!(v4, (1 << (32 - 6)) + 1);

        // Ensure MAX_NUM_BITS doesn't throw
        let v5 = LongBitSet::bits2words(LongBitSet::MAX_NUM_BITS)?;
        assert!(v5 > 0);

        Ok(())
    }
}
