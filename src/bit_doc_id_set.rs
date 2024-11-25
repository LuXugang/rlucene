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
use crate::accountable::Accountable;
use crate::bit_sets::bit_set::BitSet;
use crate::doc_id_set::DocIdSet;
use crate::util::error::runtime_error::RuntimeError;
use crate::BitSetIterator;
use std::rc::Rc;

//TODO
const _BASE_RAM_BYTES_USED: i64 = 0;

/**
 * Implementation of the DocIdSet interface on top of a {@link BitSet}.
 */
pub struct BitDocIdSet<T: BitSet> {
    set: Option<Rc<T>>,
    pub(crate) cost: i64,
}
/**
 * Wrap the given BitSet as a DocIdSet. The provided BitSet must not be
 * modified afterwards.
 */
impl<T: BitSet> BitDocIdSet<T> {
    pub fn new_with_cost(set: Option<T>, cost: i64) -> Result<BitDocIdSet<T>, RuntimeError> {
        if cost < 0 {
            return Err(RuntimeError::argument(format!(
                "cost must be >= 0, got {}",
                cost
            )));
        }
        Ok(BitDocIdSet {
            set: Some(Rc::new(set.unwrap())),
            cost,
        })
    }
    /**
     * Same as new_with_cost(BitSet, long) but uses the set's
     * BitSet#approximateCardinality() approximate cardinality as a cost.
     */
    pub fn new(set: Option<T>) -> Result<BitDocIdSet<T>, RuntimeError> {
        let cost = set.as_ref().unwrap().approximate_cardinality();
        Self::new_with_cost(set, cost as i64)
    }
}

impl<T> Accountable for BitDocIdSet<T>
where
    T: BitSet + Clone,
{
    fn ram_bytes_used(&self) -> i64 {
        todo!()
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

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        self.set.clone()
    }
}
