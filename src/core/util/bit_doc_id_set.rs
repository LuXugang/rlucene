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
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::util::accountable::Accountable;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
//TODO

const BASE_RAM_BYTES_USED: i64 = 0;

/// [`DocIdSet`] implementation backed by a [`BitSet`].
///
/// # Note
/// This is an internal API.
pub struct BitDocIdSet<T>
where
  T: BitSet,
{
  set: T,
  pub(crate) cost: i64,
}
/// Wraps the given [`BitSet`] as a [`DocIdSet`].
/// The provided [`BitSet`] must not be modified afterwards.
impl<T: BitSet> BitDocIdSet<T> {
  pub fn with_cost(set: Option<T>, cost: i64) -> Result<BitDocIdSet<T>> {
    if cost < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "cost must be >= 0, got {cost}"
      )));
    }
    match set {
      None => Err(LuceneError::illegal_argument("set must not be None")),
      Some(v) => Ok(BitDocIdSet { set: v, cost }),
    }
  }
  /// Same as [`BitDocIdSet`] but uses the set's
  /// [`BitSet::approximate_cardinality`] as a cost.
  pub fn new(set: Option<T>) -> Result<BitDocIdSet<T>> {
    let cost = match set.as_ref() {
      None => return Err(LuceneError::illegal_argument("set must not be None")),
      Some(s) => s.approximate_cardinality(),
    };
    Self::with_cost(set, cost as i64)
  }
}

impl<T> Accountable for BitDocIdSet<T>
where
  T: BitSet + Clone,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    self.set.ram_bytes_used()
  }
}

impl<T> DocIdSet for BitDocIdSet<T>
where
  T: BitSet + Clone,
{
  type DocIdSetIterator = BitSetIterator<T>;

  fn iterator(&self) -> Result<Self::DocIdSetIterator> {
    BitSetIterator::new(self.set.clone(), self.cost)
  }

  type Bits = T;

  fn bits(&self) -> Option<Self::Bits> {
    Some(self.set.clone())
  }
}
