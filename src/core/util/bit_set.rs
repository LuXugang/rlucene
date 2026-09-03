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
use crate::core::index::index_reader::Identity;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::accountable::Accountable;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet;
use std::rc::Rc;
use std::sync::Arc;

/// Rust-side specialization hooks for [`BitSet`].
pub trait BitSetExtensions {
  fn as_fixed_bit_set(&self) -> Option<&FixedBitSet> {
    None
  }

  fn as_sparse_fixed_bit_set(&self) -> Option<&SparseFixedBitSet> {
    None
  }
}

/// Base implementation for a bit set.
pub trait BitSet: Bits + Accountable + BitSetExtensions {
  /// Clears all the bits of the set.
  ///
  /// # Note
  /// Depending on the implementation, this may be significantly faster than
  /// `clear(0, length)`.
  fn clear(&mut self) -> Result<()> {
    self.clear_range(0, self.length())
  }

  /// Sets the bit at `i`.
  fn set(&mut self, i: usize) -> Result<()>;
  /// Sets the bit at `i`, returning `true` if it was previously set.
  fn get_and_set(&mut self, i: usize) -> Result<bool>;

  /// Clears the bit at `i`.
  fn clear_with_index(&mut self, i: usize) -> Result<()>;
  /// Clears a range of bits.
  ///
  /// # Arguments
  /// * `start_index` - The lower index.
  /// * `end_index` - One-past the last bit to clear.
  fn clear_range(&mut self, start_index: usize, end_index: usize) -> Result<()>;

  /// Returns the number of bits that are set.
  ///
  /// # Note
  /// This method is likely to run in linear time.
  fn cardinality(&self) -> usize;
  /// Returns an approximation of the cardinality of this set. Some
  /// implementations may trade accuracy for speed if they have the
  /// ability to estimate the cardinality of the set without iterating
  /// over all the data. The default implementation returns
  /// [`cardinality`](BitSet::cardinality).
  fn approximate_cardinality(&self) -> usize;

  /// Returns the index of the last set bit before or on the index specified.
  /// -1 is returned if there are no more set bits.
  fn prev_set_bit(&self, index: usize) -> Option<usize>;

  /// Returns the index of the first set bit starting at the index specified.
  /// [`DocIdSetIterator::NO_MORE_DOCS`](NO_MORE_DOCS) is returned if there are no more set bits.
  fn next_set_bit(&self, index: usize) -> usize {
    self.next_set_bit_range(index, self.length())
  }

  /// Returns the index of the first set bit from start (inclusive) until end
  /// (exclusive).
  /// [`DocIdSetIterator::NO_MORE_DOCS`](NO_MORE_DOCS) is returned if there are no more set bits.
  fn next_set_bit_range(&self, start: usize, end: usize) -> usize;

  /// Performs in-place OR of the bits provided by the iterator. The state of
  /// the iterator after this operation terminates is undefined.
  fn or<T>(&mut self, iter: &mut T) -> Result<()>
  where
    T: DocIdSetIterator;

  fn default_or<T>(&mut self, iter: &mut T) -> Result<()>
  where
    T: DocIdSetIterator,
  {
    check_unpositioned(iter)?;
    loop {
      let doc = iter.next_doc()?.try_convert()?;
      if doc == NO_MORE_DOCS as usize {
        break;
      }
      self.set(doc)?;
    }
    Ok(())
  }

  fn ensure_capacity(&mut self, _num_bits: usize) -> Result<()> {
    Ok(())
  }
}

// BitSet
pub enum BitSetEnum2<A, B> {
  A(A),
  B(B),
}

impl<A, B> HasIdentity for BitSetEnum2<A, B>
where
  A: HasIdentity,
  B: HasIdentity,
{
  fn identity(&self) -> &Identity {
    match self {
      BitSetEnum2::A(t) => t.identity(),
      BitSetEnum2::B(s) => s.identity(),
    }
  }
}

impl<A, B> Bits for BitSetEnum2<A, B>
where
  A: BitSet,
  B: BitSet,
{
  fn get(&self, index: usize) -> Result<bool> {
    match self {
      BitSetEnum2::A(t) => t.get(index),
      BitSetEnum2::B(s) => s.get(index),
    }
  }

  fn length(&self) -> usize {
    match self {
      BitSetEnum2::A(t) => t.length(),
      BitSetEnum2::B(s) => s.length(),
    }
  }

  fn copy_of(&self) -> Result<FixedBitSet> {
    match self {
      BitSetEnum2::A(t) => t.copy_of(),
      BitSetEnum2::B(s) => s.copy_of(),
    }
  }
}

impl<A, B> Accountable for BitSetEnum2<A, B>
where
  A: Accountable,
  B: Accountable,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    match self {
      BitSetEnum2::A(t) => t.ram_bytes_used(),
      BitSetEnum2::B(s) => s.ram_bytes_used(),
    }
  }
}

impl<A, B> BitSet for BitSetEnum2<A, B>
where
  A: BitSet,
  B: BitSet,
{
  fn clear(&mut self) -> Result<()> {
    match self {
      BitSetEnum2::A(t) => t.clear(),
      BitSetEnum2::B(s) => s.clear(),
    }
  }

  fn set(&mut self, i: usize) -> Result<()> {
    match self {
      BitSetEnum2::A(t) => t.set(i),
      BitSetEnum2::B(s) => s.set(i),
    }
  }

  fn get_and_set(&mut self, i: usize) -> Result<bool> {
    match self {
      BitSetEnum2::A(t) => t.get_and_set(i),
      BitSetEnum2::B(s) => s.get_and_set(i),
    }
  }

  fn clear_with_index(&mut self, i: usize) -> Result<()> {
    match self {
      BitSetEnum2::A(t) => t.clear_with_index(i),
      BitSetEnum2::B(s) => s.clear_with_index(i),
    }
  }

  fn clear_range(&mut self, start_index: usize, end_index: usize) -> Result<()> {
    match self {
      BitSetEnum2::A(t) => t.clear_range(start_index, end_index),
      BitSetEnum2::B(s) => s.clear_range(start_index, end_index),
    }
  }

  fn cardinality(&self) -> usize {
    match self {
      BitSetEnum2::A(t) => t.cardinality(),
      BitSetEnum2::B(s) => s.cardinality(),
    }
  }

  fn approximate_cardinality(&self) -> usize {
    match self {
      BitSetEnum2::A(t) => t.approximate_cardinality(),
      BitSetEnum2::B(s) => s.approximate_cardinality(),
    }
  }

  fn prev_set_bit(&self, index: usize) -> Option<usize> {
    match self {
      BitSetEnum2::A(t) => t.prev_set_bit(index),
      BitSetEnum2::B(s) => s.prev_set_bit(index),
    }
  }

  fn next_set_bit(&self, index: usize) -> usize {
    match self {
      BitSetEnum2::A(t) => t.next_set_bit(index),
      BitSetEnum2::B(s) => s.next_set_bit(index),
    }
  }

  fn next_set_bit_range(&self, start: usize, end: usize) -> usize {
    match self {
      BitSetEnum2::A(t) => t.next_set_bit_range(start, end),
      BitSetEnum2::B(s) => s.next_set_bit_range(start, end),
    }
  }

  fn or<T>(&mut self, iter: &mut T) -> Result<()>
  where
    T: DocIdSetIterator,
  {
    match self {
      BitSetEnum2::A(t) => t.or(iter),
      BitSetEnum2::B(s) => s.or(iter),
    }
  }

  fn ensure_capacity(&mut self, _num_bits: usize) -> Result<()> {
    match self {
      BitSetEnum2::A(t) => t.ensure_capacity(_num_bits),
      BitSetEnum2::B(s) => s.ensure_capacity(_num_bits),
    }
  }
}

impl<A, B> BitSetExtensions for BitSetEnum2<A, B>
where
  A: BitSet,
  B: BitSet,
{
  fn as_fixed_bit_set(&self) -> Option<&FixedBitSet> {
    match self {
      BitSetEnum2::A(t) => t.as_fixed_bit_set(),
      BitSetEnum2::B(s) => s.as_fixed_bit_set(),
    }
  }

  fn as_sparse_fixed_bit_set(&self) -> Option<&SparseFixedBitSet> {
    match self {
      BitSetEnum2::A(t) => t.as_sparse_fixed_bit_set(),
      BitSetEnum2::B(s) => s.as_sparse_fixed_bit_set(),
    }
  }
}

impl<T> BitSet for Arc<T>
where
  T: BitSet,
{
  fn clear(&mut self) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Arc",
    ))
  }

  fn set(&mut self, _i: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Arc",
    ))
  }

  fn get_and_set(&mut self, _i: usize) -> Result<bool> {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Arc",
    ))
  }

  fn clear_with_index(&mut self, _i: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Arc",
    ))
  }

  fn clear_range(&mut self, _start_index: usize, _end_index: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Arc",
    ))
  }

  fn cardinality(&self) -> usize {
    (**self).cardinality()
  }

  fn approximate_cardinality(&self) -> usize {
    (**self).approximate_cardinality()
  }

  fn prev_set_bit(&self, index: usize) -> Option<usize> {
    (**self).prev_set_bit(index)
  }

  fn next_set_bit(&self, index: usize) -> usize {
    (**self).next_set_bit(index)
  }

  fn next_set_bit_range(&self, start: usize, end: usize) -> usize {
    (**self).next_set_bit_range(start, end)
  }

  fn or<T1>(&mut self, _iter: &mut T1) -> Result<()>
  where
    T1: DocIdSetIterator,
  {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Arc",
    ))
  }

  fn default_or<T1>(&mut self, _iter: &mut T1) -> Result<()>
  where
    T1: DocIdSetIterator,
  {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Arc",
    ))
  }

  fn ensure_capacity(&mut self, _num_bits: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Arc",
    ))
  }
}

impl<T> BitSetExtensions for Arc<T>
where
  T: BitSet,
{
  fn as_fixed_bit_set(&self) -> Option<&FixedBitSet> {
    (**self).as_fixed_bit_set()
  }

  fn as_sparse_fixed_bit_set(&self) -> Option<&SparseFixedBitSet> {
    (**self).as_sparse_fixed_bit_set()
  }
}

impl<T> HasIdentity for Rc<T>
where
  T: HasIdentity,
{
  fn identity(&self) -> &Identity {
    (**self).identity()
  }
}

impl<T> Bits for Rc<T>
where
  T: BitSet,
{
  fn get(&self, index: usize) -> Result<bool> {
    (**self).get(index)
  }

  fn length(&self) -> usize {
    (**self).length()
  }
}

impl<T> BitSet for Rc<T>
where
  T: BitSet,
{
  fn clear(&mut self) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Rc",
    ))
  }

  fn set(&mut self, _i: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Rc",
    ))
  }

  fn get_and_set(&mut self, _i: usize) -> Result<bool> {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Rc",
    ))
  }

  fn clear_with_index(&mut self, _i: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Rc",
    ))
  }

  fn clear_range(&mut self, _start_index: usize, _end_index: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Rc",
    ))
  }

  fn cardinality(&self) -> usize {
    (**self).cardinality()
  }

  fn approximate_cardinality(&self) -> usize {
    (**self).approximate_cardinality()
  }

  fn prev_set_bit(&self, index: usize) -> Option<usize> {
    (**self).prev_set_bit(index)
  }

  fn next_set_bit(&self, index: usize) -> usize {
    (**self).next_set_bit(index)
  }

  fn next_set_bit_range(&self, start: usize, end: usize) -> usize {
    (**self).next_set_bit_range(start, end)
  }

  fn or<T1>(&mut self, _iter: &mut T1) -> Result<()>
  where
    T1: DocIdSetIterator,
  {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Rc",
    ))
  }

  fn default_or<T1>(&mut self, _iter: &mut T1) -> Result<()>
  where
    T1: DocIdSetIterator,
  {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Rc",
    ))
  }

  fn ensure_capacity(&mut self, _num_bits: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "cannot mutate a BitSet through Rc",
    ))
  }
}

impl<T> BitSetExtensions for Rc<T>
where
  T: BitSet,
{
  fn as_fixed_bit_set(&self) -> Option<&FixedBitSet> {
    (**self).as_fixed_bit_set()
  }

  fn as_sparse_fixed_bit_set(&self) -> Option<&SparseFixedBitSet> {
    (**self).as_sparse_fixed_bit_set()
  }
}
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::{HasIdentity, TryIntoInt};

/// Builds a [`BitSet`] from the content of the provided
/// [`DocIdSetIterator`]. **Note**: This will fully consume the
/// [`DocIdSetIterator`].
pub fn of(it: &mut impl DocIdSetIterator, max_doc: usize) -> Result<SparseFixedBitSetBitSet> {
  let cost = it.cost()?;
  let threshold = max_doc >> 7;
  let mut set;
  if cost < (threshold as i64) {
    set = BitSetEnum2::A(SparseFixedBitSet::new(max_doc)?);
  } else {
    let result = FixedBitSet::new(max_doc);
    set = BitSetEnum2::B(result);
  };
  set.or(it)?;
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

pub type SparseFixedBitSetBitSet = BitSetEnum2<SparseFixedBitSet, FixedBitSet>;
