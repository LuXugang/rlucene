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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::util::accountable::Accountable;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use std::rc::Rc;
use std::sync::Arc;

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
    /// [`DocIdSetIterator::NO_MORE_DOCS`](NO_MORE_DOCS) is returned if there are no more set bits.
    fn next_set_bit(&self, index: i32) -> i32 {
        self.next_set_bit_range(index, self.length())
    }

    /// Returns the index of the first set bit from start (inclusive) until end
    /// (exclusive).
    /// [`DocIdSetIterator::NO_MORE_DOCS`](NO_MORE_DOCS) is returned if there are no more set bits.
    fn next_set_bit_range(&self, start: i32, end: i32) -> i32;

    /// Performs in-place OR of the bits provided by the iterator. The state of
    /// the iterator after this operation terminates is undefined.
    fn or<T: DocIdSetIterator>(&mut self, iter: &mut T) -> Result<()>;

    fn default_or<T: DocIdSetIterator>(&mut self, iter: &mut T) -> Result<()> {
        check_unpositioned(iter)?;
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

// BitSet
pub enum Either2BitSet<A, B> {
    A(A),
    B(B),
}

impl<A, B> Bits for Either2BitSet<A, B>
where
    A: BitSet,
    B: BitSet,
{
    fn get(&self, index: i32) -> bool {
        match self {
            Either2BitSet::A(t) => t.get(index),
            Either2BitSet::B(s) => s.get(index),
        }
    }

    fn length(&self) -> i32 {
        match self {
            Either2BitSet::A(t) => t.length(),
            Either2BitSet::B(s) => s.length(),
        }
    }

    fn copy_of(&self) -> FixedBitSet {
        match self {
            Either2BitSet::A(t) => t.copy_of(),
            Either2BitSet::B(s) => s.copy_of(),
        }
    }
}

impl<A, B> Accountable for Either2BitSet<A, B>
where
    A: BitSet,
    B: BitSet,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        match self {
            Either2BitSet::A(t) => t.ram_bytes_used(),
            Either2BitSet::B(s) => s.ram_bytes_used(),
        }
    }
}

impl<A, B> BitSet for Either2BitSet<A, B>
where
    A: BitSet,
    B: BitSet,
{
    fn clear(&mut self) {
        match self {
            Either2BitSet::A(t) => t.clear(),
            Either2BitSet::B(s) => s.clear(),
        }
    }

    fn set(&mut self, i: i32) {
        match self {
            Either2BitSet::A(t) => t.set(i),
            Either2BitSet::B(s) => s.set(i),
        }
    }

    fn get_and_set(&mut self, i: i32) -> bool {
        match self {
            Either2BitSet::A(t) => t.get_and_set(i),
            Either2BitSet::B(s) => s.get_and_set(i),
        }
    }

    fn clear_with_index(&mut self, i: i32) {
        match self {
            Either2BitSet::A(t) => t.clear_with_index(i),
            Either2BitSet::B(s) => s.clear_with_index(i),
        }
    }

    fn clear_range(&mut self, start_index: i32, end_index: i32) {
        match self {
            Either2BitSet::A(t) => t.clear_range(start_index, end_index),
            Either2BitSet::B(s) => s.clear_range(start_index, end_index),
        }
    }

    fn cardinality(&self) -> i32 {
        match self {
            Either2BitSet::A(t) => t.cardinality(),
            Either2BitSet::B(s) => s.cardinality(),
        }
    }

    fn approximate_cardinality(&self) -> i32 {
        match self {
            Either2BitSet::A(t) => t.approximate_cardinality(),
            Either2BitSet::B(s) => s.approximate_cardinality(),
        }
    }

    fn prev_set_bit(&self, index: i32) -> i32 {
        match self {
            Either2BitSet::A(t) => t.prev_set_bit(index),
            Either2BitSet::B(s) => s.prev_set_bit(index),
        }
    }

    fn next_set_bit(&self, index: i32) -> i32 {
        match self {
            Either2BitSet::A(t) => t.next_set_bit(index),
            Either2BitSet::B(s) => s.next_set_bit(index),
        }
    }

    fn next_set_bit_range(&self, start: i32, end: i32) -> i32 {
        match self {
            Either2BitSet::A(t) => t.next_set_bit_range(start, end),
            Either2BitSet::B(s) => s.next_set_bit_range(start, end),
        }
    }

    fn or<T: DocIdSetIterator>(&mut self, iter: &mut T) -> Result<()> {
        match self {
            Either2BitSet::A(t) => t.or(iter),
            Either2BitSet::B(s) => s.or(iter),
        }
    }

    fn ensure_capacity(&mut self, _num_bits: i32) {
        match self {
            Either2BitSet::A(t) => t.ensure_capacity(_num_bits),
            Either2BitSet::B(s) => s.ensure_capacity(_num_bits),
        }
    }
}

impl<T> Accountable for Arc<T>
where
    T: BitSet,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        (**self).ram_bytes_used()
    }
}

impl<T> BitSet for Arc<T>
where
    T: BitSet,
{
    fn clear(&mut self) {
        unreachable!()
    }

    fn set(&mut self, i: i32) {
        unreachable!()
    }

    fn get_and_set(&mut self, i: i32) -> bool {
        unreachable!()
    }

    fn clear_with_index(&mut self, i: i32) {
        unreachable!()
    }

    fn clear_range(&mut self, _start_index: i32, _end_index: i32) {
        unreachable!()
    }

    fn cardinality(&self) -> i32 {
        (**self).cardinality()
    }

    fn approximate_cardinality(&self) -> i32 {
        (**self).approximate_cardinality()
    }

    fn prev_set_bit(&self, index: i32) -> i32 {
        (**self).prev_set_bit(index)
    }

    fn next_set_bit(&self, index: i32) -> i32 {
        (**self).next_set_bit(index)
    }

    fn next_set_bit_range(&self, start: i32, end: i32) -> i32 {
        (**self).next_set_bit_range(start, end)
    }

    fn or<T1: DocIdSetIterator>(&mut self, _iter: &mut T1) -> Result<()> {
        unreachable!()
    }

    fn default_or<T1: DocIdSetIterator>(&mut self, _iter: &mut T1) -> Result<()> {
        unreachable!()
    }

    fn ensure_capacity(&mut self, _num_bits: i32) {
        unreachable!()
    }
}

impl<T> Bits for Rc<T>
where
    T: BitSet,
{
    fn get(&self, index: i32) -> bool {
        todo!()
    }

    fn length(&self) -> i32 {
        todo!()
    }
}

impl<T> Accountable for Rc<T>
where
    T: BitSet,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        (**self).ram_bytes_used()
    }
}

impl<T> BitSet for Rc<T>
where
    T: BitSet,
{
    fn clear(&mut self) {
        unreachable!()
    }

    fn set(&mut self, i: i32) {
        unreachable!()
    }

    fn get_and_set(&mut self, i: i32) -> bool {
        unreachable!()
    }

    fn clear_with_index(&mut self, i: i32) {
        unreachable!()
    }

    fn clear_range(&mut self, _start_index: i32, _end_index: i32) {
        unreachable!()
    }

    fn cardinality(&self) -> i32 {
        (**self).cardinality()
    }

    fn approximate_cardinality(&self) -> i32 {
        (**self).approximate_cardinality()
    }

    fn prev_set_bit(&self, index: i32) -> i32 {
        (**self).prev_set_bit(index)
    }

    fn next_set_bit(&self, index: i32) -> i32 {
        (**self).next_set_bit(index)
    }

    fn next_set_bit_range(&self, start: i32, end: i32) -> i32 {
        (**self).next_set_bit_range(start, end)
    }

    fn or<T1: DocIdSetIterator>(&mut self, _iter: &mut T1) -> Result<()> {
        unreachable!()
    }

    fn default_or<T1: DocIdSetIterator>(&mut self, _iter: &mut T1) -> Result<()> {
        unreachable!()
    }

    fn ensure_capacity(&mut self, _num_bits: i32) {
        unreachable!()
    }
}
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet;

/// Builds a [`BitSet`] from the content of the provided
/// [`DocIdSetIterator`]. **Note**: This will fully consume the
/// [`DocIdSetIterator`].
pub fn of(
    it: &mut impl DocIdSetIterator,
    max_doc: i32,
) -> Result<Either2BitSet<SparseFixedBitSet, FixedBitSet>> {
    let cost = it.cost()?;
    let threshold = max_doc >> 7;
    let mut set;
    if cost < (threshold as i64) {
        set = Either2BitSet::A(SparseFixedBitSet::new(max_doc)?);
    } else {
        let result = FixedBitSet::new(max_doc);
        set = Either2BitSet::B(result);
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
