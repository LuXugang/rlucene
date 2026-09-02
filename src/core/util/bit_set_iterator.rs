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
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::bit_set::BitSet;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet;

/// A [`DocIdSetIterator`] which iterates over set bits in a bit set.
///
/// # Note
/// This is an internal API.
pub struct BitSetIterator<T> {
  pub(crate) bits: T,
  length: i32,
  cost: i64,
  doc: i32,
}

impl<T> BitSetIterator<T>
where
  T: BitSet,
{
  pub fn new(bits: T, cost: i64) -> Result<Self> {
    if cost < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "cost must be >= 0, got {cost}"
      )));
    }
    let length = bits.length();
    Ok(BitSetIterator {
      bits,
      length: length as i32,
      cost,
      doc: -1,
    })
  }
  // Set the current doc id that this iterator is on.

  pub fn set_doc_id(&mut self, doc_id: i32) {
    self.doc = doc_id;
  }
  pub fn get_bit_set(&self) -> &T {
    &self.bits
  }
}

impl<T> DocIdSetIterator for BitSetIterator<T>
where
  T: BitSet,
{
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.advance(self.doc + 1)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    if target >= self.length {
      self.doc = NO_MORE_DOCS;
      return Ok(self.doc);
    }
    self.doc = self.bits.next_set_bit(target as usize) as i32;
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.cost)
  }
}

impl<T> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for BitSetIterator<T>
where
  T: BitSet,
{
  fn get_fixed_bit_set(&self) -> Option<&FixedBitSet> {
    self.bits.as_fixed_bit_set()
  }

  fn get_sparse_fixed_bit_set(&self) -> Option<&SparseFixedBitSet> {
    self.bits.as_sparse_fixed_bit_set()
  }
}
