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

use crate::core::util::error::lucene_error::Result;

/// Candidate iteration, sharing the matching iterator's cursor.
///
/// Every operation is explicit: implementations must consistently target their
/// candidate iterator, including slow advance and cost. When candidates are
/// identical to full matches, delegate to [`DocIdSetIterator`](crate::core::search::doc_id_set_iterator::DocIdSetIterator).
/// Each call borrows the owner only for that operation, so matching and scoring
/// can subsequently access the same state without retaining an iterator view.
pub trait ApproximationDISI {
  /// Candidate-iterator counterpart of [`DocIdSetIterator::doc_id`](crate::core::search::doc_id_set_iterator::DocIdSetIterator::doc_id).
  fn approx_doc_id(&self) -> i32;

  /// Candidate-iterator counterpart of [`DocIdSetIterator::next_doc`](crate::core::search::doc_id_set_iterator::DocIdSetIterator::next_doc).
  fn approx_next_doc(&mut self) -> Result<i32>;

  /// Candidate-iterator counterpart of [`DocIdSetIterator::advance`](crate::core::search::doc_id_set_iterator::DocIdSetIterator::advance).
  fn approx_advance(&mut self, target: i32) -> Result<i32>;

  /// Candidate-iterator counterpart of [`DocIdSetIterator::slow_advance`](crate::core::search::doc_id_set_iterator::DocIdSetIterator::slow_advance).
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32>;

  /// Candidate-iterator counterpart of [`DocIdSetIterator::cost`](crate::core::search::doc_id_set_iterator::DocIdSetIterator::cost).
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
