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
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet;

/// Random access supported by Java's `BitSetIterator` specialization.
/// Ordinary iterators keep the defaults; Rust-only ownership wrappers forward
/// these methods only when they represent the same Java iterator.
pub trait BitSetIteratorAccess {
  fn is_bit_iter(&self) -> bool {
    false
  }

  fn get(&self, _index: usize) -> Result<bool> {
    Err(LuceneError::unsupported_operation(
      "Iterator does not support bit-set access",
    ))
  }

  fn set_doc_id(&mut self, _doc: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "Iterator does not support setting its document ID",
    ))
  }

  fn bit_set_length(&self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(
      "Iterator does not expose a bit-set length",
    ))
  }
}

/// Rust-side specialization hooks for [`DocIdSetIterator`].
pub trait DocIdSetIteratorExtensions {
  /// Returns the wrapped fixed bit set when this iterator has the same shape
  /// as Java's `BitSetIterator.getFixedBitSetOrNull` specialization.
  fn get_fixed_bit_set(&self) -> Option<&FixedBitSet> {
    None
  }

  /// Returns the wrapped sparse fixed bit set when this iterator has the same
  /// shape as Java's `BitSetIterator.getSparseFixedBitSetOrNull`
  /// specialization.
  fn get_sparse_fixed_bit_set(&self) -> Option<&SparseFixedBitSet> {
    None
  }

  /// Returns Java's [`DocBaseBitSetIterator`](crate::core::util::doc_base_bit_set_iterator::DocBaseBitSetIterator) specialization as its document
  /// base and backing bit set.
  fn get_doc_base_fixed_bit_set(&self) -> Option<(usize, &FixedBitSet)> {
    None
  }
}

/// This trait defines methods to iterate over a set of non-decreasing document
/// IDs. It assumes implementations iterate on document IDs, and
/// therefore [`NO_MORE_DOCS`] is set to its constant value to be used as a
/// sentinel object.
///
/// Implementations of this trait are expected to treat `i32::MAX` as an
/// invalid value.
pub trait DocIdSetIterator: DocIdSetIteratorExtensions + BitSetIteratorAccess {
  /// Returns the following:
  ///
  /// - `-1` if [`next_doc`](DocIdSetIterator::next_doc) or
  ///   [`advance`](DocIdSetIterator::advance) has not been called yet.
  /// - [`NO_MORE_DOCS`]if the iterator has been exhausted.
  /// - Otherwise, it returns the document ID it is currently on.
  fn doc_id(&self) -> i32;
  /// Advances to the next document in the set and returns the document ID it
  /// is currently on, or [`NO_MORE_DOCS`] if there are no more documents
  /// in the set.
  ///
  /// # Note
  /// After the iterator has been exhausted, you should not call this method,
  /// as it may result in undefined behavior.
  fn next_doc(&mut self) -> Result<i32>;
  /// Advances to the first document beyond the current one whose document
  /// number is greater than or equal to the `target`, and returns the
  /// document number itself. If `target` is greater than the
  /// highest document number in the set, the iterator is exhausted, and
  /// [`NO_MORE_DOCS`] is returned.
  ///
  /// # Undefined Behavior
  /// The behavior of this method is **undefined** when called with `target <=
  /// current`, or after the iterator has been exhausted. Both cases may
  /// result in unpredictable behavior.
  ///
  /// # Behavior for `target > current`
  /// When `target > current`, it behaves similarly to:
  ///
  /// ```text
  /// fn advance(target: i32) -> i32 {
  ///     let mut doc;
  ///     while {
  ///         doc = next_doc();
  ///         doc < target
  ///     } {}
  ///     doc
  /// }
  /// ```
  ///
  /// Some implementations may be significantly more efficient than this.
  ///
  /// # Note
  /// This method may be called with [`NO_MORE_DOCS`] for efficiency
  /// by some Scorers. If your implementation cannot efficiently determine
  /// that it should exhaust, it is recommended to check for this value in
  /// each call to this method.
  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::not_implemented(""))
  }
  /// A slow (linear) implementation of [`advance`](DocIdSetIterator::advance)
  /// that relies on [`next_doc`](DocIdSetIterator::next_doc) to move
  /// beyond the target position.
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    debug_assert!(self.doc_id() < target);
    let mut doc;
    loop {
      doc = self.next_doc()?;
      if doc >= target {
        break;
      }
    }
    Ok(doc)
  }
  /// Returns the estimated cost of this [`DocIdSetIterator`].
  /// This is generally an upper bound on the number of documents this
  /// iterator might match, but it may also be a rough heuristic, a
  /// hardcoded value, or otherwise completely inaccurate.
  fn cost(&self) -> Result<i64> {
    Err(LuceneError::not_implemented(""))
  }
}

///An empty [`DocIdSetIterator`]
pub struct EmptyDISI {
  exhausted: bool,
}
impl Default for EmptyDISI {
  fn default() -> Self {
    Self::new()
  }
}

impl EmptyDISI {
  pub fn new() -> Self {
    Self { exhausted: false }
  }
}
impl DocIdSetIterator for EmptyDISI {
  fn doc_id(&self) -> i32 {
    if self.exhausted { NO_MORE_DOCS } else { -1 }
  }

  fn next_doc(&mut self) -> Result<i32> {
    debug_assert!(!self.exhausted);
    self.exhausted = true;
    Ok(NO_MORE_DOCS)
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    debug_assert!(!self.exhausted);
    debug_assert!(_target >= 0);
    self.exhausted = true;
    Ok(NO_MORE_DOCS)
  }

  fn cost(&self) -> Result<i64> {
    Ok(0)
  }
}

/// A [`DocIdSetIterator`] that matches all documents up to `maxDoc - 1`.  */
pub struct AllDISI {
  doc: i32,
  max_doc: i32,
}
impl AllDISI {
  pub fn new(max_doc: i32) -> Self {
    AllDISI { doc: -1, max_doc }
  }
}
impl DocIdSetIterator for AllDISI {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.advance(self.doc + 1)?;
    Ok(self.doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.doc = target;
    if self.doc >= self.max_doc {
      self.doc = NO_MORE_DOCS
    }
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.max_doc as i64)
  }
}

/// A [`DocIdSetIterator`] that matches a range of documents from `min_doc_id`
/// (inclusive) to `max_doc_id` (exclusive).
///
/// # Parameters
/// - `min_doc_id`: The minimum document ID to match (inclusive).
/// - `max_doc_id`: The maximum document ID to match (exclusive).
///
/// # See Also
/// - [`DocIdSetIterator`]
pub struct RangeDISI {
  doc: i32,
  min_doc: i32,
  max_doc: i32,
}
impl RangeDISI {
  pub fn new(min_doc: i32, max_doc: i32) -> Result<RangeDISI> {
    if min_doc >= max_doc {
      return Err(LuceneError::illegal_argument(format!(
        "minDoc must be < maxDoc but got minDoc= {min_doc} maxDoc= {max_doc}"
      )));
    }
    if min_doc < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "minDoc must be >= 0 but got minDoc= {min_doc}"
      )));
    }
    Ok(RangeDISI {
      doc: -1,
      min_doc,
      max_doc,
    })
  }
}
impl DocIdSetIterator for RangeDISI {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.advance(self.doc + 1)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    if target < self.min_doc {
      self.doc = self.min_doc;
    } else if target >= self.max_doc {
      self.doc = NO_MORE_DOCS
    } else {
      self.doc = target
    }
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok((self.max_doc - self.min_doc) as i64)
  }
}
impl<T> DocIdSetIterator for Box<T>
where
  T: DocIdSetIterator + ?Sized,
{
  fn doc_id(&self) -> i32 {
    (**self).doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    (**self).next_doc()
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    (**self).advance(_target)
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    (**self).slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    (**self).cost()
  }
}
impl<T: DocIdSetIterator + ?Sized> DocIdSetIteratorExtensions for Box<T> {
  fn get_fixed_bit_set(&self) -> Option<&FixedBitSet> {
    (**self).get_fixed_bit_set()
  }

  fn get_sparse_fixed_bit_set(&self) -> Option<&SparseFixedBitSet> {
    (**self).get_sparse_fixed_bit_set()
  }

  fn get_doc_base_fixed_bit_set(&self) -> Option<(usize, &FixedBitSet)> {
    (**self).get_doc_base_fixed_bit_set()
  }
}
impl<T: DocIdSetIterator + ?Sized> BitSetIteratorAccess for Box<T> {
  fn is_bit_iter(&self) -> bool {
    (**self).is_bit_iter()
  }
  fn get(&self, index: usize) -> Result<bool> {
    (**self).get(index)
  }
  fn bit_set_length(&self) -> Result<usize> {
    (**self).bit_set_length()
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    (**self).set_doc_id(doc)
  }
}

impl DocIdSetIteratorExtensions for EmptyDISI {}
impl BitSetIteratorAccess for EmptyDISI {}

impl DocIdSetIteratorExtensions for AllDISI {}
impl BitSetIteratorAccess for AllDISI {}

impl DocIdSetIteratorExtensions for RangeDISI {}
impl BitSetIteratorAccess for RangeDISI {}

impl<T: DocIdSetIterator + ?Sized> DocIdSetIterator for &mut T {
  fn doc_id(&self) -> i32 {
    (**self).doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    (**self).next_doc()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    (**self).advance(target)
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    (**self).slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    (**self).cost()
  }
}
impl<T: DocIdSetIterator + ?Sized> DocIdSetIteratorExtensions for &mut T {
  fn get_fixed_bit_set(&self) -> Option<&FixedBitSet> {
    (**self).get_fixed_bit_set()
  }

  fn get_sparse_fixed_bit_set(&self) -> Option<&SparseFixedBitSet> {
    (**self).get_sparse_fixed_bit_set()
  }

  fn get_doc_base_fixed_bit_set(&self) -> Option<(usize, &FixedBitSet)> {
    (**self).get_doc_base_fixed_bit_set()
  }
}
impl<T: DocIdSetIterator + ?Sized> BitSetIteratorAccess for &mut T {
  fn is_bit_iter(&self) -> bool {
    (**self).is_bit_iter()
  }
  fn get(&self, index: usize) -> Result<bool> {
    (**self).get(index)
  }
  fn bit_set_length(&self) -> Result<usize> {
    (**self).bit_set_length()
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    (**self).set_doc_id(doc)
  }
}

impl<T: DocIdSetIterator + ?Sized> DocIdSetIterator for &T {
  fn doc_id(&self) -> i32 {
    (**self).doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    Err(LuceneError::not_implemented(
      "next_doc() not implement for &T",
    ))
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::not_implemented(
      "advance() not implement for &T",
    ))
  }

  fn slow_advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::not_implemented(
      "slow_advance() not implement for &T",
    ))
  }

  fn cost(&self) -> Result<i64> {
    (**self).cost()
  }
}
impl<T: DocIdSetIterator + ?Sized> DocIdSetIteratorExtensions for &T {
  fn get_fixed_bit_set(&self) -> Option<&FixedBitSet> {
    (**self).get_fixed_bit_set()
  }

  fn get_sparse_fixed_bit_set(&self) -> Option<&SparseFixedBitSet> {
    (**self).get_sparse_fixed_bit_set()
  }

  fn get_doc_base_fixed_bit_set(&self) -> Option<(usize, &FixedBitSet)> {
    (**self).get_doc_base_fixed_bit_set()
  }
}
impl<T: DocIdSetIterator + ?Sized> BitSetIteratorAccess for &T {
  fn is_bit_iter(&self) -> bool {
    (**self).is_bit_iter()
  }
  fn get(&self, index: usize) -> Result<bool> {
    (**self).get(index)
  }
  fn bit_set_length(&self) -> Result<usize> {
    (**self).bit_set_length()
  }
}

/// When returned by
/// [`next_doc`](DocIdSetIterator::next_doc),
/// [`advance`](DocIdSetIterator::advance),
/// and [`doc_id`](DocIdSetIterator::doc_id),
/// it means there are no more documents in the iterator.
pub const NO_MORE_DOCS: i32 = i32::MAX;
#[macro_export]
macro_rules! either_docidsetiterator_named {
    ($vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> DocIdSetIterator for $name<$( $T ),+>
        where
            $( $T: DocIdSetIterator ),+
        {

            fn doc_id(&self) -> i32 {
                match self {
                    $( Self::$Variant(inner) => inner.doc_id(), )+
                }
            }


            fn next_doc(&mut self) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.next_doc(), )+
                }
            }


            fn advance(&mut self, target: i32) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.advance(target), )+
                }
            }


            fn slow_advance(&mut self, target: i32) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.slow_advance(target), )+
                }
            }


            fn cost(&self) -> Result<i64> {
                match self {
                    $( Self::$Variant(inner) => inner.cost(), )+
                }
            }
        }

        impl<$( $T ),+> $crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
          for $name<$( $T ),+>
        where
            $( $T: $crate::core::search::doc_id_set_iterator::DocIdSetIterator ),+
        {}
        impl<$( $T ),+> $crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
          for $name<$( $T ),+>
        where
            $( $T: $crate::core::search::doc_id_set_iterator::DocIdSetIterator ),+
        {
            fn is_bit_iter(&self) -> bool {
                match self { $( Self::$Variant(inner) => inner.is_bit_iter(), )+ }
            }
            fn get(&self, index: usize) -> Result<bool> {
                match self { $( Self::$Variant(inner) => inner.get(index), )+ }
            }
            fn set_doc_id(&mut self, doc: i32) -> Result<()> {
                match self { $( Self::$Variant(inner) => inner.set_doc_id(doc), )+ }
            }
            fn bit_set_length(&self) -> Result<usize> {
                match self { $( Self::$Variant(inner) => inner.bit_set_length(), )+ }
            }
        }

    };
}
either_docidsetiterator_named!(pub DocIdSetIteratorEnum2 { A: A, B: B});
either_docidsetiterator_named!(pub DocIdSetIteratorEnum3 { A: A, B: B,C:C});
either_docidsetiterator_named!(pub DocIdSetIteratorEnum4 { A: A, B: B,C:C,D:D});
either_docidsetiterator_named!(pub DocIdSetIteratorEnum5 { A: A, B: B, C: C, D: D, E: E });
