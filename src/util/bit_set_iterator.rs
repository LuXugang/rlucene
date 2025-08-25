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
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::util::access::SharedReadOnly;
use crate::util::bit_set::BitSet;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::marker::PhantomData;

/// A [`DocIdSetIterator`] which iterates over set bits in a bit set.
///
/// # Note
/// This is an internal API.
pub struct BitSetIterator<T, R>
where
    T: BitSet,
    R: SharedReadOnly<T>,
{
    pub(crate) bits: R,
    length: i32,
    cost: i64,
    doc: i32,
    phantom: PhantomData<T>,
}

impl<T, R> BitSetIterator<T, R>
where
    T: BitSet,
    R: SharedReadOnly<T>,
{
    pub fn new(bits: R, cost: i64) -> Result<BitSetIterator<T, R>> {
        if cost < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "cost must be >= 0, got {cost}"
            )));
        }
        let length = bits.access(|b| b.length());
        Ok(BitSetIterator {
            bits,
            length,
            cost,
            doc: -1,
            phantom: PhantomData,
        })
    }
    // Set the current doc id that this iterator is on.

    fn set_doc_id(&mut self, doc_id: i32) {
        self.doc = doc_id;
    }
    pub fn get_bit_set(&self) -> R {
        self.bits.clone()
    }
}

impl<T, R> DocIdSetIterator for BitSetIterator<T, R>
where
    T: BitSet,
    R: SharedReadOnly<T>,
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
        self.doc = self.bits.access(|b| b.next_set_bit(target));
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.cost)
    }
}

pub mod bsi_util {
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::util::bit_set::BitSet;
    use crate::util::fixed_bit_set::FixedBitSet;
    use crate::util::sparse_fixed_bit_set::SparseFixedBitSet;
    use std::any::TypeId;

    fn equal_disi_type<T1: DocIdSetIterator + 'static, T2: DocIdSetIterator + 'static>(
        _it1: &T1,
        _it2: &T2,
    ) -> bool {
        TypeId::of::<T1>() == TypeId::of::<T2>()
    }

    fn equal_bit_set_type<T1: BitSet + 'static, T2: BitSet + 'static>(
        _it1: &T1,
        _it2: &T2,
    ) -> bool {
        TypeId::of::<T1>() == TypeId::of::<T2>()
    }
    //TODO
    pub fn try_get_bit_set<B: BitSet + 'static>(
        _iterator: impl DocIdSetIterator + 'static,
        _bit_set: B,
    ) -> Option<B> {
        todo!()
    }

    // todo
    /// If the provided iterator wraps a [`FixedBitSet`], returns it, otherwise
    /// returns `None`.
    pub fn get_fixed_bit_set_or_null<B: BitSet>(
        _iterator: impl DocIdSetIterator + 'static,
    ) -> Option<FixedBitSet> {
        todo!()
    }

    // todo
    /// If the provided iterator wraps a [`SparseFixedBitSet`] returns it,
    /// otherwise returns `None`.
    pub fn get_sparse_fixed_bit_set_or_null<B: BitSet>(
        _iterator: impl DocIdSetIterator + 'static,
    ) -> Option<SparseFixedBitSet> {
        todo!()
    }
}
