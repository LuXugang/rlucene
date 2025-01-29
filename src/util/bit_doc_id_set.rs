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
use crate::search::doc_id_set::DocIdSet;
use crate::util::accountable::Accountable;
use crate::util::bit_set::BitSet;
use crate::util::bit_set_iterator::BitSetIterator;

use crate::util::error::lucene_error::LuceneError;
use std::sync::Arc;

//TODO
#[allow(unused)]
const BASE_RAM_BYTES_USED: i64 = 0;

/// Implementation of the [`DocIdSet`] interface on top of a [`BitSet`].
///
/// # Note
/// This is an internal API.
pub struct BitDocIdSet<T: BitSet> {
    set: Option<Arc<T>>,
    pub(crate) cost: i64,
}
/// Wraps the given [`BitSet`] as a [`DocIdSet`].
/// The provided [`BitSet`] must not be modified afterwards.
impl<T: BitSet> BitDocIdSet<T> {
    pub fn with_cost(set: Option<T>, cost: i64) -> Result<BitDocIdSet<T>, LuceneError> {
        if cost < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "cost must be >= 0, got {}",
                cost
            )));
        }
        Ok(BitDocIdSet {
            set: Some(Arc::new(set.unwrap())),
            cost,
        })
    }
    /// Same as [`BitDocIdSet`] but uses the set's [`BitSet::approximate_cardinality`]
    /// as a cost.
    pub fn new(set: Option<T>) -> Result<BitDocIdSet<T>, LuceneError> {
        let cost = set.as_ref().unwrap().approximate_cardinality();
        Self::with_cost(set, cost as i64)
    }
}

impl<T> Accountable for BitDocIdSet<T>
where
    T: BitSet + Clone,
{
    fn ram_bytes_used(&self) -> i64 {
        self.set.as_ref().unwrap().ram_bytes_used()
    }
}

impl<T> DocIdSet for BitDocIdSet<T>
where
    T: BitSet + Clone + 'static,
{
    type DISIType<'a> = BitSetIterator<'a, T>;

    fn iterator(&self) -> Option<Self::DISIType<'_>> {
        self.set
            .as_ref()
            .map(|set| BitSetIterator::new(&**set, self.cost).unwrap())
    }

    type BitType = T;

    fn bits(&self) -> Option<Arc<Self::BitType>> {
        self.set.clone()
    }
}
