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
use std::hash::Hash;

use crate::core::util::accountable::Accountable;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// BitSet of fixed length (`numBits`), backed by accessible
/// ([`get_bits`](LongBitSet::get_bits)) `&[i64]`, accessed with a `long` index.
/// Use it only if you intend to store more than 2.1B bits, otherwise you should
/// use [`FixedBitSet`](crate::core::util::fixed_bit_set::FixedBitSet).
#[derive(Debug)]
pub struct LongBitSet {
  bits: Vec<i64>,  // Array of longs holding the bits
  num_bits: usize, // The number of bits in use
  num_words: i32,  /* The exact number of longs needed to hold numBits (<=
                    * bits.length)  */
}
impl LongBitSet {
  pub const MAX_NUM_BITS: usize = 64 * ArrayUtil::MAX_ARRAY_LENGTH;
  /// If the given [`LongBitSet`] is large enough to hold `num_bits + 1`,
  /// returns the given bitset, otherwise returns a new [`LongBitSet`]
  /// which can hold the requested number of bits.
  ///
  /// **NOTE:** the returned bitset reuses the underlying `long[]` of the
  /// given `bits` if possible. Also, calling
  /// [`length()`](LongBitSet::length) on the returned bitset may return a
  /// value greater than `num_bits`.
  pub fn ensure_capacity(bits: &mut LongBitSet, num_bits: usize) -> Result<()> {
    if num_bits < bits.num_bits {
    } else {
      let num_words = Self::bits2words(num_bits)?;
      let length = bits.bits.len();
      if num_words as usize >= length {
        ArrayUtil::grow_with_len(&mut bits.bits, (num_words + 1) as usize);
      }
      debug_assert!(bits.bits.len() <= i32::MAX as usize);
      bits.num_bits = (bits.bits.len()) << 6;
      bits.num_words = Self::bits2words(bits.num_bits)?;
    }
    Ok(())
  }
  /// Returns the number of 64-bit words needed to hold `num_bits`.
  pub fn bits2words(num_bits: usize) -> Result<i32> {
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
  pub fn new(num_bits: usize) -> Result<Self> {
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
  pub fn from_bits(stored_bits: Vec<i64>, num_bits: usize) -> Result<Self> {
    let num_words = Self::bits2words(num_bits)?;
    if num_words as usize > stored_bits.len() {
      return Err(LuceneError::illegal_argument(format!(
        "The given long array is too small to hold {num_bits} bits"
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
  pub fn length(&self) -> usize {
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
  pub fn cardinality(&self) -> usize {
    // Depends on the ghost bits being clear!
    self.bits[..self.num_words as usize]
      .iter()
      .map(|v| v.count_ones() as usize)
      .sum()
  }

  pub fn get(&self, index: usize) -> bool {
    debug_assert!(
      index < self.num_bits,
      "index = {}, num_bits = {}",
      index,
      self.num_bits
    );
    let i = index >> 6;
    // signed shift will keep a negative index and force an
    // array-index-out-of-bounds-error, removing the need for an
    // explicit check.
    let bitmask = 1i64 << index;
    (self.bits[i] & bitmask) != 0
  }

  pub fn set(&mut self, index: usize) {
    debug_assert!(
      index < self.num_bits,
      "index = {}, num_bits = {}",
      index,
      self.num_bits
    );
    let word_num = index >> 6;
    let bitmask = 1i64 << index;
    self.bits[word_num] |= bitmask;
  }

  /// Returns the previous value of the bit at `index`, and sets it.
  pub fn get_and_set(&mut self, index: usize) -> bool {
    debug_assert!(
      index < self.num_bits,
      "index = {}, num_bits = {}",
      index,
      self.num_bits
    );
    let word_num = index >> 6;
    let bitmask = 1i64 << index;
    let val = (self.bits[word_num] & bitmask) != 0;
    self.bits[word_num] |= bitmask;
    val
  }
  pub fn clear(&mut self, index: usize) {
    debug_assert!(
      index < self.num_bits,
      "index = {}, num_bits = {}",
      index,
      self.num_bits
    );
    let word_num = index >> 6;
    let bitmask = 1i64 << index;
    self.bits[word_num] &= !bitmask;
  }

  /// Returns the previous value of the bit at `index`, and clears it.
  pub fn get_and_clear(&mut self, index: usize) -> bool {
    debug_assert!(
      index < self.num_bits,
      "index = {}, num_bits = {}",
      index,
      self.num_bits
    );
    let word_num = index >> 6;
    let bitmask = 1i64 << index;
    let val = (self.bits[word_num] & bitmask) != 0;
    self.bits[word_num] &= !bitmask;
    val
  }

  /// Returns the index of the first set bit starting at the given `index`.
  /// Returns -1 if no such bit is found.
  ///
  /// Depends on ghost bits being clear!
  pub fn next_set_bit(&self, index: usize) -> Option<usize> {
    debug_assert!(
      index < self.num_bits,
      "index = {}, num_bits = {}",
      index,
      self.num_bits
    );

    let mut i = index >> 6;
    let sub_index = index & 63;
    let mut word = self.bits[i] >> sub_index;

    if word != 0 {
      return Some(index + word.trailing_zeros() as usize);
    }

    i += 1;
    while i < self.num_words as usize {
      word = self.bits[i];
      if word != 0 {
        return Some(i << 6 | word.trailing_zeros() as usize);
      }
      i += 1;
    }
    None
  }
  /// Returns the index of the last set bit before or on the given `index`.
  /// Returns -1 if there are no more set bits.
  pub fn prev_set_bit(&self, index: usize) -> Option<usize> {
    debug_assert!(
      index < self.num_bits,
      "index = {}, num_bits = {}",
      index,
      self.num_bits
    );

    let i = index >> 6;
    let sub_index = index & 0x3f;
    let word = self.bits[i] << (63 - sub_index);

    if word != 0 {
      return Some((i << 6) + sub_index - word.leading_zeros() as usize);
    }

    let mut v = i.checked_sub(1);

    while let Some(i) = v {
      let word = self.bits[i];
      if word != 0 {
        return Some((i << 6) + 63 - word.leading_zeros() as usize);
      }
      v = i.checked_sub(1);
    }
    None
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
  pub fn flip(&mut self, start_index: usize, end_index: usize) {
    debug_assert!(
      start_index < self.num_bits,
      "start_index = {}, num_bits = {}",
      start_index,
      self.num_bits
    );
    debug_assert!(
      end_index <= self.num_bits,
      "end_index = {}, num_bits = {}",
      end_index,
      self.num_bits
    );

    if end_index <= start_index {
      return;
    }

    let start_word = start_index >> 6;
    let end_word = (end_index - 1) >> 6;

    let start_mask = -1_i64 << (start_index % 64);
    let shift: u32 = ((0usize).wrapping_sub(end_index) & 63) as u32;
    let end_mask: u64 = u64::MAX >> shift;

    if start_word == end_word {
      self.bits[start_word] ^= start_mask & end_mask as i64;
      return;
    }

    self.bits[start_word] ^= start_mask;

    for i in (start_word + 1)..end_word {
      self.bits[i] = !self.bits[i];
    }

    self.bits[end_word] ^= end_mask as i64;
  }
  /// Flip the bit at the provided index.
  pub fn flip_one(&mut self, index: usize) {
    debug_assert!(
      index < self.num_bits,
      "index = {}, num_bits = {}",
      index,
      self.num_bits
    );
    let word_num = index >> 6;
    let bitmask = 1i64 << index; // mod 64 is implicit
    self.bits[word_num] ^= bitmask;
  }

  /// Sets a range of bits in [start_index, end_index)
  ///
  /// - `start_index`: lower index (inclusive)
  /// - `end_index`: one-past the last bit to set
  pub fn set_range(&mut self, start_index: usize, end_index: usize) {
    debug_assert!(
      start_index < self.num_bits,
      "start_index = {}, num_bits = {}",
      start_index,
      self.num_bits
    );
    debug_assert!(
      end_index <= self.num_bits,
      "end_index = {}, num_bits = {}",
      end_index,
      self.num_bits
    );

    if end_index <= start_index {
      return;
    }

    let start_word = start_index >> 6;
    let end_word = (end_index - 1) >> 6;

    let start_mask = -1_i64 << (start_index % 64);
    let shift: u32 = ((0usize).wrapping_sub(end_index) & 63) as u32;
    let end_mask: u64 = u64::MAX >> shift;

    if start_word == end_word {
      self.bits[start_word] |= start_mask & end_mask as i64;
      return;
    }

    self.bits[start_word] |= start_mask;
    for i in (start_word + 1)..end_word {
      self.bits[i] = -1;
    }
    self.bits[end_word] |= end_mask as i64;
  }
  /// Clears a range of bits in [start_index, end_index)
  ///
  /// - `start_index`: lower index (inclusive)
  /// - `end_index`: one-past the last bit to clear
  pub fn clear_range(&mut self, start_index: usize, end_index: usize) {
    debug_assert!(
      start_index < self.num_bits,
      "start_index = {}, num_bits = {}",
      start_index,
      self.num_bits
    );
    debug_assert!(
      end_index <= self.num_bits,
      "end_index = {}, num_bits = {}",
      end_index,
      self.num_bits
    );

    if end_index <= start_index {
      return;
    }

    let start_word = start_index >> 6;
    let end_word = (end_index - 1) >> 6;

    let mut start_mask = -1_i64 << (start_index % 64);
    let shift: u32 = ((0usize).wrapping_sub(end_index) & 63) as u32;
    let mut end_mask: u64 = u64::MAX >> shift;

    // Invert masks since we are clearing
    start_mask = !start_mask;
    end_mask = !end_mask;

    if start_word == end_word {
      self.bits[start_word] &= start_mask | end_mask as i64;
      return;
    }

    self.bits[start_word] &= start_mask;
    for i in (start_word + 1)..end_word {
      self.bits[i] = 0;
    }
    self.bits[end_word] &= end_mask as i64;
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
  fn hash<H>(&self, state: &mut H)
  where
    H: std::hash::Hasher,
  {
    self.bits.hash(state);
    self.num_bits.hash(state);
  }
}
