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
use crate::search::doc_id_set_iterator::{AllDocIdSetIterator, DocIdSetIterator};
use crate::util::accountable::Accountable;
use crate::util::bit_set::BitSet;
use crate::util::bit_set_iterator::BitSetIterator;
use crate::util::bits::{Bits, MatchNoBits};
use crate::util::error::runtime_error::RuntimeError;
use crate::util::fixed_bit_set::FixedBitSet;
use std::rc::Rc;

//TODO
#[allow(dead_code)]
const BASE_RAM_BYTES_USED: i64 = 0;
/// Accumulator for documents that have a value for a field.
/// This is optimized for the case where all documents have a value.
pub struct DocsWithFieldSet<FixedBitSet> {
    set: FixedBitSet,
    cardinality: i32,
    last_doc_id: i32,
    _marker: std::marker::PhantomData<FixedBitSet>,
}
impl Default for DocsWithFieldSet<FixedBitSet> {
    fn default() -> Self {
        Self::new()
    }
}

impl DocsWithFieldSet<FixedBitSet> {
    pub fn new() -> DocsWithFieldSet<FixedBitSet> {
        let set = FixedBitSet::new(0);
        DocsWithFieldSet {
            set,
            cardinality: 0,
            last_doc_id: -1,
            _marker: Default::default(),
        }
    }
    /**
     * Add a document to the set
     *
     * @param docID – document ID to be added
     */
    pub fn add(&mut self, doc_id: i32) -> Result<(), RuntimeError> {
        if doc_id <= self.last_doc_id {
            return Err(RuntimeError::illegal_argument(format!(
                "Out of order doc ids: last= {}, next= {}",
                self.last_doc_id, doc_id
            )));
        }
        if self.set.length() != 0 {
            FixedBitSet::ensure_capacity(&mut self.set, doc_id);
            self.set.set(doc_id);
        } else if doc_id != self.cardinality {
            self.set = FixedBitSet::new(doc_id + 1);
            self.set.set_with_range(0, self.cardinality);
            self.set.set(doc_id);
        }
        self.last_doc_id = doc_id;
        self.cardinality += 1;
        Ok(())
    }
    /// Returns the number of documents in this set.
    pub fn cardinality(&self) -> i32 {
        self.cardinality
    }
}
pub enum DocsWithFieldSetEnum<'a, T: BitSet> {
    Dense(AllDocIdSetIterator),
    Sparse(BitSetIterator<'a, T>),
}
impl<T: BitSet> DocIdSetIterator for DocsWithFieldSetEnum<'_, T> {
    fn doc_id(&self) -> i32 {
        match self {
            DocsWithFieldSetEnum::Dense(d) => d.doc_id(),
            DocsWithFieldSetEnum::Sparse(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> i32 {
        match self {
            DocsWithFieldSetEnum::Dense(d) => d.next_doc(),
            DocsWithFieldSetEnum::Sparse(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> i32 {
        match self {
            DocsWithFieldSetEnum::Dense(d) => d.advance(target),
            DocsWithFieldSetEnum::Sparse(s) => s.advance(target),
        }
    }

    fn cost(&self) -> i64 {
        match self {
            DocsWithFieldSetEnum::Dense(d) => d.cost(),
            DocsWithFieldSetEnum::Sparse(s) => s.cost(),
        }
    }
}

impl<T: BitSet> Accountable for DocsWithFieldSet<T> {
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

impl<T: BitSet> DocIdSet for DocsWithFieldSet<T> {
    type DISIType<'b> = DocsWithFieldSetEnum<'b, T> where T: 'b;

    fn iterator(&self) -> Option<Self::DISIType<'_>> {
        if self.set.length() != 0 {
            debug_assert!(self.cardinality > 0);
            Some(DocsWithFieldSetEnum::Sparse(
                BitSetIterator::new(&self.set, self.cardinality as i64).unwrap(),
            ))
        } else {
            Some(DocsWithFieldSetEnum::Dense(AllDocIdSetIterator::new(
                self.cardinality,
            )))
        }
    }

    type BitType = MatchNoBits;

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        None
    }
}
