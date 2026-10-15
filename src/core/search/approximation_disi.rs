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
use crate::core::util::error::lucene_error::Result;

/// Candidate iteration, sharing the matching iterator's cursor.
///
/// Every operation is explicit: implementations must consistently target their
/// candidate iterator, including slow advance and cost. When candidates are
/// identical to full matches, delegate to [`DocIdSetIterator`].
/// Each call borrows the owner only for that operation, so matching and scoring
/// can subsequently access the same state without retaining an iterator view.
pub trait ApproximationDISI {
  /// Candidate-iterator counterpart of [`DocIdSetIterator::doc_id`].
  fn approx_doc_id(&self) -> i32;

  /// Candidate-iterator counterpart of [`DocIdSetIterator::next_doc`].
  fn approx_next_doc(&mut self) -> Result<i32>;

  /// Candidate-iterator counterpart of [`DocIdSetIterator::advance`].
  fn approx_advance(&mut self, target: i32) -> Result<i32>;

  /// Candidate-iterator counterpart of [`DocIdSetIterator::slow_advance`].
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32>;

  /// Candidate-iterator counterpart of [`DocIdSetIterator::cost`].
  fn approx_cost(&self) -> Result<i64>;
}

/// Optional random access to candidate documents backed by a bit set.
pub trait ApproximationBitSetAccess {
  /// Candidate-iterator counterpart of [`BitSetIteratorAccess::is_bit_iter`](crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::is_bit_iter).
  fn approx_is_bit_iter(&self) -> bool;

  /// Candidate-iterator counterpart of [`BitSetIteratorAccess::get`](crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::get).
  fn approx_get(&self, index: usize) -> Result<bool>;

  /// Candidate-iterator counterpart of [`BitSetIteratorAccess::set_doc_id`](crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::set_doc_id).
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()>;

  /// Candidate-iterator counterpart of [`BitSetIteratorAccess::bit_set_length`](crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::bit_set_length).
  fn approx_bit_set_length(&self) -> Result<usize>;
}

impl<T: ApproximationDISI + ?Sized> ApproximationDISI for Box<T> {
  fn approx_doc_id(&self) -> i32 {
    (**self).approx_doc_id()
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    (**self).approx_next_doc()
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    (**self).approx_advance(target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    (**self).approx_slow_advance(target)
  }
  fn approx_cost(&self) -> Result<i64> {
    (**self).approx_cost()
  }
}

impl<T: ApproximationBitSetAccess + ?Sized> ApproximationBitSetAccess for Box<T> {
  fn approx_is_bit_iter(&self) -> bool {
    (**self).approx_is_bit_iter()
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    (**self).approx_get(index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    (**self).approx_set_doc_id(doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    (**self).approx_bit_set_length()
  }
}

impl<T: ApproximationDISI + ?Sized> ApproximationDISI for &mut T {
  fn approx_doc_id(&self) -> i32 {
    (**self).approx_doc_id()
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    (**self).approx_next_doc()
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    (**self).approx_advance(target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    (**self).approx_slow_advance(target)
  }
  fn approx_cost(&self) -> Result<i64> {
    (**self).approx_cost()
  }
}

impl<T: ApproximationBitSetAccess + ?Sized> ApproximationBitSetAccess for &mut T {
  fn approx_is_bit_iter(&self) -> bool {
    (**self).approx_is_bit_iter()
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    (**self).approx_get(index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    (**self).approx_set_doc_id(doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    (**self).approx_bit_set_length()
  }
}

/// Shared candidate view for two-phase iterators that own a scorer.
/// Scorer consumers should call `approx_*` directly.
pub struct ApproximationRef<'a, S: ?Sized>(pub(crate) &'a S);

/// Mutable candidate view for two-phase iterators that own a scorer.
/// Each operation delegates directly to the scorer's candidate entry point.
pub struct ApproximationMut<'a, S: ?Sized>(pub(crate) &'a mut S);

impl<S: ApproximationDISI + ?Sized>
  crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for ApproximationRef<'_, S>
{
}
impl<S: ApproximationDISI + ApproximationBitSetAccess + ?Sized>
  crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for ApproximationRef<'_, S>
{
  fn is_bit_iter(&self) -> bool {
    self.0.approx_is_bit_iter()
  }
  fn get(&self, index: usize) -> Result<bool> {
    self.0.approx_get(index)
  }
  fn bit_set_length(&self) -> Result<usize> {
    self.0.approx_bit_set_length()
  }
}
impl<S: ApproximationDISI + ApproximationBitSetAccess + ?Sized> DocIdSetIterator
  for ApproximationRef<'_, S>
{
  fn doc_id(&self) -> i32 {
    self.0.approx_doc_id()
  }
  fn cost(&self) -> Result<i64> {
    self.0.approx_cost()
  }
  fn next_doc(&mut self) -> Result<i32> {
    Err(
      crate::core::util::error::lucene_error::LuceneError::unsupported_operation(
        "next_doc() not implement for &T",
      ),
    )
  }
  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(
      crate::core::util::error::lucene_error::LuceneError::unsupported_operation(
        "advance() not implement for &T",
      ),
    )
  }
  fn slow_advance(&mut self, _target: i32) -> Result<i32> {
    Err(
      crate::core::util::error::lucene_error::LuceneError::unsupported_operation(
        "slow_advance() not implement for &T",
      ),
    )
  }
}

impl<S: ApproximationDISI + ?Sized>
  crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for ApproximationMut<'_, S>
{
}
impl<S: ApproximationDISI + ApproximationBitSetAccess + ?Sized>
  crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for ApproximationMut<'_, S>
{
  fn is_bit_iter(&self) -> bool {
    self.0.approx_is_bit_iter()
  }
  fn get(&self, index: usize) -> Result<bool> {
    self.0.approx_get(index)
  }
  fn bit_set_length(&self) -> Result<usize> {
    self.0.approx_bit_set_length()
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.0.approx_set_doc_id(doc)
  }
}
impl<S: ApproximationDISI + ApproximationBitSetAccess + ?Sized> DocIdSetIterator
  for ApproximationMut<'_, S>
{
  fn doc_id(&self) -> i32 {
    self.0.approx_doc_id()
  }
  fn cost(&self) -> Result<i64> {
    self.0.approx_cost()
  }
  fn next_doc(&mut self) -> Result<i32> {
    self.0.approx_next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.0.approx_advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.0.approx_slow_advance(target)
  }
}
