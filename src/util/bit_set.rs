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
use crate::util::bit_set_type::BitSetType;
use crate::util::bits::Bits;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::sparse_fixed_bit_set::SparseFixedBitSet;

/// Base implementation for a bit set.
pub trait BitSet: Bits + Accountable {
    /// Builds a [`BitSet`] from the content of the provided [`DocIdSetIterator`].
    /// **Note**: This will fully consume the [`DocIdSetIterator`].
    fn of(it: impl DocIdSetIterator, max_doc: i32) -> Result<BitSetType> {
        let cost = it.cost()?;
        let threshold = max_doc >> 7;
        let mut set: BitSetType;
        if cost < (threshold as i64) {
            set = BitSetType::Sparse(SparseFixedBitSet::new(max_doc)?);
        } else {
            let result = FixedBitSet::new(max_doc);
            set = BitSetType::Fixed(result);
        };
        let _ = set.or(it);
        Ok(set)
    }

    /// Clears all the bits of the set.
    ///
    /// # Note
    /// Depending on the implementation, this may be significantly faster than `clear(0, length)`.
    fn clear(&mut self) {
        self.clear_range(0, self.length())
    }

    /// Sets the bit at `i`.
    fn set(&mut self, i: i32);
    /// Sets the bit at `i`, returning `true` if it was previously set.
    fn get_and_set(&mut self, i: i32) -> bool;

    /// Clears the bit at `i`.
    fn clear_with_index(&mut self, i: i32);
    /// Clears a range of bits.
    ///
    /// # Arguments
    /// * `start_index` - The lower index.
    /// * `end_index` - One-past the last bit to clear.
    fn clear_range(&mut self, start_index: i32, end_index: i32);

    /// Returns the number of bits that are set.
    ///
    /// # Note
    /// This method is likely to run in linear time.
    fn cardinality(&self) -> i32;
    /// Returns an approximation of the cardinality of this set. Some implementations may trade accuracy
    /// for speed if they have the ability to estimate the cardinality of the set without iterating
    /// over all the data. The default implementation returns [`cardinality`](BitSet::cardinality).
    fn approximate_cardinality(&self) -> i32;

    /// Returns the index of the last set bit before or on the index specified. -1 is returned if there
    /// are no more set bits.
    fn prev_set_bit(&self, index: i32) -> i32;

    /// Returns the index of the first set bit starting at the index specified.
    /// [`DocIdSetIterator::NO_MORE_DOCS`](crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS) is returned if there are no more set bits.
    fn next_set_bit(&self, index: i32) -> i32 {
        self.next_set_bit_range(index, self.length())
    }

    /// Returns the index of the first set bit from start (inclusive) until end (exclusive).
    /// [`DocIdSetIterator::NO_MORE_DOCS`](crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS) is returned if there are no more set bits.
    fn next_set_bit_range(&self, start: i32, end: i32) -> i32;

    ///Assert that the current doc is -1.
    fn check_unpositioned(iter: &impl DocIdSetIterator) -> Result<()> {
        if iter.doc_id() != -1 {
            return Err(LuceneError::illegal_state( format!("This operation only works with an unpositioned iterator, got current position = {}", iter.doc_id())));
        }
        Ok(())
    }

    /// Performs in-place OR of the bits provided by the iterator. The state of the iterator after this
    /// operation terminates is undefined.
    fn or<T: DocIdSetIterator>(&mut self, iter: T) -> Result<()>;
}
