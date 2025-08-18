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
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::util::accountable::Accountable;
use crate::util::bits::Bits;
use crate::util::error::lucene_error;
use crate::util::error::lucene_error::Result;
use crate::util::fixed_bit_set::FixedBitSet;

/// Base implementation for a bit set.
pub trait BitSet: Bits + Accountable {
    /// Clears all the bits of the set.
    ///
    /// # Note
    /// Depending on the implementation, this may be significantly faster than
    /// `clear(0, length)`.
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
    /// Returns an approximation of the cardinality of this set. Some
    /// implementations may trade accuracy for speed if they have the
    /// ability to estimate the cardinality of the set without iterating
    /// over all the data. The default implementation returns
    /// [`cardinality`](BitSet::cardinality).
    fn approximate_cardinality(&self) -> i32;

    /// Returns the index of the last set bit before or on the index specified.
    /// -1 is returned if there are no more set bits.
    fn prev_set_bit(&self, index: i32) -> i32;

    /// Returns the index of the first set bit starting at the index specified.
    /// [`DocIdSetIterator::NO_MORE_DOCS`](crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS) is returned if there are no more set bits.
    fn next_set_bit(&self, index: i32) -> i32 {
        self.next_set_bit_range(index, self.length())
    }

    /// Returns the index of the first set bit from start (inclusive) until end
    /// (exclusive).
    /// [`DocIdSetIterator::NO_MORE_DOCS`](crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS) is returned if there are no more set bits.
    fn next_set_bit_range(&self, start: i32, end: i32) -> i32;

    /// Performs in-place OR of the bits provided by the iterator. The state of
    /// the iterator after this operation terminates is undefined.
    fn or<T: DocIdSetIterator>(&mut self, iter: &mut T) -> Result<()>;

    fn default_or<T: DocIdSetIterator>(&mut self, iter: &mut T) -> Result<()> {
        bit_set_util::check_unpositioned(iter)?;
        loop {
            let doc = iter.next_doc()?;
            if doc == NO_MORE_DOCS {
                break;
            }
            self.set(doc);
        }
        Ok(())
    }

    fn ensure_capacity(&mut self, _num_bits: i32) {}
}
pub mod bit_set_util {
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::util::bit_set::BitSet;
    use crate::util::bit_set::EitherBitSet;
    use crate::util::error::lucene_error::LuceneError;
    use crate::util::error::lucene_error::Result;
    use crate::util::fixed_bit_set::FixedBitSet;
    use crate::util::sparse_fixed_bit_set::SparseFixedBitSet;

    /// Builds a [`BitSet`] from the content of the provided
    /// [`DocIdSetIterator`]. **Note**: This will fully consume the
    /// [`DocIdSetIterator`].
    pub fn of(
        it: &mut impl DocIdSetIterator,
        max_doc: i32,
    ) -> Result<EitherBitSet<SparseFixedBitSet, FixedBitSet>> {
        let cost = it.cost()?;
        let threshold = max_doc >> 7;
        let mut set;
        if cost < (threshold as i64) {
            set = EitherBitSet::F(SparseFixedBitSet::new(max_doc)?);
        } else {
            let result = FixedBitSet::new(max_doc);
            set = EitherBitSet::S(result);
        };
        let _ = set.or(it);
        Ok(set)
    }
    ///Assert that the current doc is -1.
    pub(crate) fn check_unpositioned(iter: &impl DocIdSetIterator) -> Result<()> {
        if iter.doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "This operation only works with an unpositioned iterator, got current position = {}",
                iter.doc_id()
            )));
        }
        Ok(())
    }
}

// BitSet
pub enum EitherBitSet<F, S> {
    F(F),
    S(S),
}

impl<F, S> Bits for EitherBitSet<F, S>
where
    F: BitSet,
    S: BitSet,
{
    fn get(&self, index: i32) -> bool {
        match self {
            EitherBitSet::F(t) => t.get(index),
            EitherBitSet::S(s) => s.get(index),
        }
    }

    fn length(&self) -> i32 {
        match self {
            EitherBitSet::F(t) => t.length(),
            EitherBitSet::S(s) => s.length(),
        }
    }

    fn copy_of(&self) -> FixedBitSet {
        match self {
            EitherBitSet::F(t) => t.copy_of(),
            EitherBitSet::S(s) => s.copy_of(),
        }
    }
}

impl<F, S> Accountable for EitherBitSet<F, S>
where
    F: BitSet,
    S: BitSet,
{
    fn ram_bytes_used(&self) -> lucene_error::Result<i64> {
        match self {
            EitherBitSet::F(t) => t.ram_bytes_used(),
            EitherBitSet::S(s) => s.ram_bytes_used(),
        }
    }
}

impl<F, S> BitSet for EitherBitSet<F, S>
where
    F: BitSet,
    S: BitSet,
{
    fn clear(&mut self) {
        match self {
            EitherBitSet::F(t) => t.clear(),
            EitherBitSet::S(s) => s.clear(),
        }
    }

    fn set(&mut self, i: i32) {
        match self {
            EitherBitSet::F(t) => t.set(i),
            EitherBitSet::S(s) => s.set(i),
        }
    }

    fn get_and_set(&mut self, i: i32) -> bool {
        match self {
            EitherBitSet::F(t) => t.get_and_set(i),
            EitherBitSet::S(s) => s.get_and_set(i),
        }
    }

    fn clear_with_index(&mut self, i: i32) {
        match self {
            EitherBitSet::F(t) => t.clear_with_index(i),
            EitherBitSet::S(s) => s.clear_with_index(i),
        }
    }

    fn clear_range(&mut self, start_index: i32, end_index: i32) {
        match self {
            EitherBitSet::F(t) => t.clear_range(start_index, end_index),
            EitherBitSet::S(s) => s.clear_range(start_index, end_index),
        }
    }

    fn cardinality(&self) -> i32 {
        match self {
            EitherBitSet::F(t) => t.cardinality(),
            EitherBitSet::S(s) => s.cardinality(),
        }
    }

    fn approximate_cardinality(&self) -> i32 {
        match self {
            EitherBitSet::F(t) => t.approximate_cardinality(),
            EitherBitSet::S(s) => s.approximate_cardinality(),
        }
    }

    fn prev_set_bit(&self, index: i32) -> i32 {
        match self {
            EitherBitSet::F(t) => t.prev_set_bit(index),
            EitherBitSet::S(s) => s.prev_set_bit(index),
        }
    }

    fn next_set_bit(&self, index: i32) -> i32 {
        match self {
            EitherBitSet::F(t) => t.next_set_bit(index),
            EitherBitSet::S(s) => s.next_set_bit(index),
        }
    }

    fn next_set_bit_range(&self, start: i32, end: i32) -> i32 {
        match self {
            EitherBitSet::F(t) => t.next_set_bit_range(start, end),
            EitherBitSet::S(s) => s.next_set_bit_range(start, end),
        }
    }

    fn or<T: DocIdSetIterator>(&mut self, iter: &mut T) -> lucene_error::Result<()> {
        match self {
            EitherBitSet::F(t) => t.or(iter),
            EitherBitSet::S(s) => s.or(iter),
        }
    }

    fn ensure_capacity(&mut self, _num_bits: i32) {
        match self {
            EitherBitSet::F(t) => t.ensure_capacity(_num_bits),
            EitherBitSet::S(s) => s.ensure_capacity(_num_bits),
        }
    }
}
