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
use crate::bit_sets::bit_set::BitSet;
use crate::bit_sets::fixed_bit_set::FixedBitSet;
use crate::bit_sets::sparse_fixed_bit_set::SparseFixedBitSet;
use crate::{DocIdSetIterator, NO_MORE_DOCS};
use std::any::TypeId;

pub struct BitSetIterator<'a, T: BitSet> {
    bits: &'a T,
    length: i32,
    cost: i64,
    doc: i32,
}

impl<'a, T: BitSet> BitSetIterator<'a, T> {
    pub fn new(bits: &'a T, cost: i64) -> Result<BitSetIterator<T>, String> {
        if cost < 0 {
            return Err(format!("cost must be >= 0: got {}", cost));
        }
        let length = bits.length();
        Ok(BitSetIterator {
            bits,
            length,
            cost,
            doc: -1,
        })
    }

    pub fn get_bit_set(&self) -> &T {
        self.bits
    }

    /** Set the current doc id that this iterator is on. */
    fn set_doc_id(&mut self, doc_id: i32) {
        self.doc = doc_id;
    }
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
    //todo
    pub fn try_get_bit_set<B: BitSet + 'static>(
        _iterator: impl DocIdSetIterator + 'static,
        _bit_set: B,
    ) -> Option<B> {
        todo!()
    }

    // todo
    pub fn get_fixed_bit_set_or_null<B: BitSet>(
        iterator: impl DocIdSetIterator + 'static,
    ) -> Option<FixedBitSet> {
        Self::try_get_bit_set(iterator, FixedBitSet::default())
    }

    // todo
    pub fn get_sparse_fixed_bit_set_or_null<B: BitSet>(
        iterator: impl DocIdSetIterator + 'static,
    ) -> Option<SparseFixedBitSet> {
        Self::try_get_bit_set(iterator, SparseFixedBitSet::default())
    }
}

impl<'a, T: BitSet> DocIdSetIterator for BitSetIterator<'a, T> {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> i32 {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> i32 {
        if target >= self.length {
            self.doc = NO_MORE_DOCS;
            return self.doc;
        }
        self.doc = self.bits.next_set_bit(target);
        self.doc
    }

    fn cost(&self) -> i64 {
        self.cost
    }
}
