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
use crate::search::doc_id_set_iterator::{DocIdSetIterator, EmptyDISI};
use crate::util::accountable::Accountable;
use crate::util::bit_doc_id_set::BitDocIdSet;
use crate::util::bits::MatchNoBits;
use crate::util::error::lucene_error::Result;
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::not_doc_id_set::{NotDocDocIdSetIterator, NotDocIdSet};

use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::util::bit_set_iterator::BitSetIterator;
use std::rc::Rc;

// Number of documents in a block
const BLOCK_SIZE: i32 = 1 << 16;
// The maximum length for an array, beyond that point we switch to a bitset
const MAX_ARRAY_LENGTH: i32 = 1 << 12;
// todo
#[allow(unused)]
const BASE_RAM_BYTES_USED: i64 = 0;
/// [`DocIdSet`] implementation inspired by [roaringbitmap.org](http://roaringbitmap.org/)
///
/// The space is divided into blocks of `2^16` bits, and each block is encoded independently. In each
/// block, if fewer than `2^12` bits are set, documents are simply stored in a `Vec<i16>`. If more than
/// `2^16 - 2^12` bits are set, the inverse of the set is encoded in a simple `Vec<i16>`. Otherwise,
/// a [`FixedBitSet`] is used.
///
/// # Note
/// This is an internal API.
pub struct RoaringDocIdSet {
    doc_id_sets: Vec<Option<DocIdSetEnum>>,
    cardinality: i32,
    #[allow(unused)]
    ram_bytes_used: i64,
}
impl RoaringDocIdSet {
    fn new(doc_id_sets: Vec<Option<DocIdSetEnum>>, cardinality: i32) -> Self {
        // todo
        let ram_bytes_used = 0;
        RoaringDocIdSet {
            doc_id_sets,
            cardinality,
            ram_bytes_used,
        }
    }

    #[allow(unused)]
    fn cardinality(&self) -> i32 {
        self.cardinality
    }
}
impl DocIdSet for RoaringDocIdSet {
    type DISIType<'a> = Iterator<'a>;

    fn iterator(&self) -> Option<Self::DISIType<'_>> {
        Some(Iterator::new(&self.doc_id_sets, self.cardinality as i64))
    }

    type BitType = MatchNoBits;

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        None
    }
}

impl Accountable for RoaringDocIdSet {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}
pub mod builder {
    use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::util::bit_doc_id_set::BitDocIdSet;
    use crate::util::bit_set::BitSet;
    use crate::util::bits::Bits;
    use crate::util::error::lucene_error::LuceneError;
    use crate::util::error::lucene_error::Result;
    use crate::util::fixed_bit_set::FixedBitSet;
    use crate::util::not_doc_id_set::NotDocIdSet;
    use crate::util::roaring_doc_id_set::{
        DocIdSetEnum, RoaringDocIdSet, ShortArrayDocIdSet, BLOCK_SIZE,
        MAX_ARRAY_LENGTH,
    };

    pub struct Builder {
        max_doc: i32,
        sets: Vec<Option<DocIdSetEnum>>,
        cardinality: i32,
        last_doc_id: i32,
        current_block: i32,
        current_block_cardinality: i32,
        // We start by filling the buffer and when it's full we copy the content of
        // the buffer to the FixedBitSet and put further documents in that bitset
        buffer: Vec<i16>,
        dense_buffer: FixedBitSet,
    }

    impl Builder {
        pub fn new(max_doc: i32) -> Builder {
            let buffer: Vec<i16> =
                Vec::with_capacity(MAX_ARRAY_LENGTH as usize);
            let sets_length = (max_doc + (1 << 16) - 1) >> 16;
            let mut sets = Vec::with_capacity(sets_length as usize);
            // not want to impl Copy of DocIdSetEnum
            for _i in 0..sets_length {
                sets.push(None);
            }
            Builder {
                max_doc,
                sets,
                cardinality: 0,
                last_doc_id: -1,
                current_block: -1,
                current_block_cardinality: 0,
                buffer,
                dense_buffer: FixedBitSet::new(0),
            }
        }
        /// Add a new doc-id to this builder. NOTE: doc ids must be added in order.
        pub fn add(&mut self, doc_id: i32) -> Result<()> {
            if doc_id <= self.last_doc_id {
                return Err(LuceneError::illegal_argument(format!(
                    "Doc ids must be added in-order, got {} which is <= lastDocID=",
                    self.last_doc_id
                )));
            }
            let block = doc_id >> 16;
            if block != self.current_block {
                let _ = self.flush();
                self.current_block = block;
            }

            if self.current_block_cardinality < MAX_ARRAY_LENGTH {
                // self.buffer[self.current_block_cardinality as usize] = doc_id as i16;
                self.buffer.push(doc_id as i16);
            } else {
                if self.dense_buffer.length() == 0 {
                    // the buffer is full, let's move to a fixed bit set
                    let num_bits =
                        std::cmp::min(1 << 16, self.max_doc - (block << 16));
                    self.dense_buffer = FixedBitSet::new(num_bits);
                    for i in 0..self.buffer.len() {
                        self.dense_buffer.set(self.buffer[i] as i32 & 0xFFFF);
                    }
                }
                self.dense_buffer.set(doc_id & 0xFFFF);
            }
            self.last_doc_id = doc_id;
            self.current_block_cardinality += 1;
            Ok(())
        }
        /// Add the content of the provided DocIdSetIterator.
        pub fn add_disi<T: DocIdSetIterator>(
            &mut self,
            mut disi: T,
        ) -> Result<()> {
            let mut doc = disi.next_doc()?;
            while doc != NO_MORE_DOCS {
                let _ = self.add(doc);
                doc = disi.next_doc()?;
            }
            Ok(())
        }
        pub fn build(&mut self) -> RoaringDocIdSet {
            let _ = self.flush();
            RoaringDocIdSet::new(
                std::mem::take(&mut self.sets),
                self.cardinality,
            )
        }
        fn flush(&mut self) -> Result<()> {
            debug_assert!(self.current_block_cardinality <= BLOCK_SIZE);
            if self.current_block_cardinality <= MAX_ARRAY_LENGTH {
                // use sparse encoding
                debug_assert_eq!(self.dense_buffer.length(), 0);
                if self.current_block_cardinality > 0 {
                    let sparse =
                        Some(DocIdSetEnum::Sparse(ShortArrayDocIdSet::new(
                            std::mem::take(&mut self.buffer),
                        )));
                    debug_assert!(self.buffer.is_empty());
                    self.sets[self.current_block as usize] = sparse;
                }
            } else {
                assert_ne!(self.dense_buffer.length(), 0);
                debug_assert_eq!(
                    self.dense_buffer.cardinality(),
                    self.current_block_cardinality
                );
                if self.dense_buffer.length() == BLOCK_SIZE
                    && BLOCK_SIZE - self.current_block_cardinality
                        < MAX_ARRAY_LENGTH
                {
                    let capacity =
                        (BLOCK_SIZE - self.current_block_cardinality) as usize;
                    let mut excluded_docs: Vec<i16> = vec![0; capacity];
                    self.dense_buffer.flip_range(0, self.dense_buffer.length());
                    let mut excluded_doc = -1;
                    for excluded_doc_ref in excluded_docs.iter_mut() {
                        excluded_doc =
                            self.dense_buffer.next_set_bit(excluded_doc + 1);
                        assert_ne!(excluded_doc, NO_MORE_DOCS);
                        *excluded_doc_ref = excluded_doc as i16;
                    }

                    debug_assert!(
                        excluded_doc + 1 == self.dense_buffer.length()
                            || self.dense_buffer.next_set_bit(excluded_doc + 1)
                                == NO_MORE_DOCS
                    );
                    let dense: Option<DocIdSetEnum> =
                        Some(DocIdSetEnum::Dense(NotDocIdSet::new(
                            BLOCK_SIZE,
                            ShortArrayDocIdSet::new(excluded_docs),
                        )));
                    self.buffer.clear();
                    self.sets[self.current_block as usize] = dense;
                } else {
                    let result = BitDocIdSet::with_cost(
                        Some(std::mem::take(&mut self.dense_buffer)),
                        self.current_block_cardinality as i64,
                    )?;

                    let medium: Option<DocIdSetEnum> =
                        Some(DocIdSetEnum::Medium(result));
                    self.buffer.clear();
                    self.sets[self.current_block as usize] = medium;
                }
                self.dense_buffer = FixedBitSet::new(0);
            }
            self.cardinality += self.current_block_cardinality;
            self.dense_buffer = FixedBitSet::new(0);
            self.current_block_cardinality = 0;
            Ok(())
        }
    }
}

// todo
#[allow(unused)]
const SHORT_ARRAY_DOC_ID_SET_BASE_RAM_BYTES_USED: i64 = 0;

pub struct ShortArrayDocIdSet {
    doc_ids: Vec<i16>,
}
impl ShortArrayDocIdSet {
    fn new(doc_ids: Vec<i16>) -> ShortArrayDocIdSet {
        ShortArrayDocIdSet { doc_ids }
    }
}

impl Accountable for ShortArrayDocIdSet {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

impl DocIdSet for ShortArrayDocIdSet {
    type DISIType<'b> = ShortArrayDISI<'b>;

    fn iterator(&self) -> Option<Self::DISIType<'_>> {
        Some(ShortArrayDISI::new(&self.doc_ids))
    }

    type BitType = MatchNoBits;

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        None
    }
}

pub struct ShortArrayDISI<'a> {
    i: i32,
    doc: i32,
    doc_ids: &'a Vec<i16>,
}
impl<'a> ShortArrayDISI<'a> {
    fn new(doc_ids: &'a Vec<i16>) -> Self {
        ShortArrayDISI {
            i: -1,
            doc: -1,
            doc_ids,
        }
    }
    fn doc_id_index(&self, i: i32) -> i32 {
        self.doc_ids[i as usize] as i32 & 0xFFFF
    }
}
impl DocIdSetIterator for ShortArrayDISI<'_> {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.i += 1;
        if self.i as usize >= self.doc_ids.len() {
            self.doc = NO_MORE_DOCS;
            return Ok(self.doc);
        }
        self.doc = self.doc_id_index(self.i);
        Ok(self.doc)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        let mut lo = self.i + 1;
        let mut hi = self.doc_ids.len() as i32 - 1;
        while lo <= hi {
            let mid = (lo + hi) >> 1;
            let mid_doc = self.doc_id_index(mid);
            if mid_doc < _target {
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }
        if lo == self.doc_ids.len() as i32 {
            self.i = self.doc_ids.len() as i32;
            self.doc = NO_MORE_DOCS;
        } else {
            self.i = lo;
            self.doc = self.doc_id_index(self.i);
        }
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.doc_ids.len() as i64)
    }
}

pub struct Iterator<'a> {
    block: i32,
    doc: i32,
    set_length: usize,
    sub: Option<DocIdSetIteratorEnum<'a>>,
    doc_id_sets: &'a Vec<Option<DocIdSetEnum>>,
    cardinality: i64,
}
impl<'a> Iterator<'a> {
    fn new(
        doc_id_sets: &'a Vec<Option<DocIdSetEnum>>,
        cardinality: i64,
    ) -> Self {
        let set_length = doc_id_sets.len();
        Iterator {
            block: -1,
            doc: -1,
            set_length,
            sub: Some(DocIdSetIteratorEnum::Empty(EmptyDISI::new())),
            doc_id_sets,
            cardinality,
        }
    }
    fn first_doc_from_next_block(&mut self) -> Result<i32> {
        loop {
            self.block += 1;
            if self.block >= self.set_length as i32 {
                self.doc = NO_MORE_DOCS;
                break;
            } else if self.doc_id_sets[self.block as usize].is_some() {
                self.sub = self.doc_id_sets[self.block as usize]
                    .as_ref()
                    .unwrap()
                    .iterator();
                let sub_next = self.sub.as_mut().unwrap().next_doc()?;
                debug_assert!(sub_next != NO_MORE_DOCS);
                self.doc = (self.block << 16) | sub_next;
                break;
            }
        }
        Ok(self.doc)
    }
}
impl DocIdSetIterator for Iterator<'_> {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        let sub_next = self.sub.as_mut().unwrap().next_doc()?;
        if sub_next == NO_MORE_DOCS {
            return self.first_doc_from_next_block();
        }
        self.doc = (self.block << 16) | sub_next;
        Ok(self.doc)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        let target_block = _target >> 16;
        if target_block != self.block {
            self.block = target_block;
            if self.block > self.doc_id_sets.len() as i32 {
                self.sub = None;
                self.doc = NO_MORE_DOCS;
                return Ok(self.doc);
            }
            if self.doc_id_sets[self.block as usize].is_none() {
                return self.first_doc_from_next_block();
            }
            self.sub = self.doc_id_sets[self.block as usize]
                .as_ref()
                .unwrap()
                .iterator()
        }
        let sub_next = self.sub.as_mut().unwrap().advance(_target & 0xFFFF)?;
        if sub_next == NO_MORE_DOCS {
            return self.first_doc_from_next_block();
        }
        self.doc = (self.block << 16) | sub_next;
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.cardinality)
    }
}

enum DocIdSetEnum {
    Sparse(ShortArrayDocIdSet),
    Medium(BitDocIdSet<FixedBitSet>),
    Dense(NotDocIdSet<ShortArrayDocIdSet>),
}
impl Accountable for DocIdSetEnum {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

impl DocIdSet for DocIdSetEnum {
    type DISIType<'a>
        = DocIdSetIteratorEnum<'a>
    where
        Self: 'a;

    fn iterator(&self) -> Option<Self::DISIType<'_>> {
        match self {
            DocIdSetEnum::Sparse(s) => {
                Some(DocIdSetIteratorEnum::Sparse(s.iterator()?))
            },
            DocIdSetEnum::Medium(m) => {
                Some(DocIdSetIteratorEnum::Medium(m.iterator()?))
            },
            DocIdSetEnum::Dense(d) => {
                Some(DocIdSetIteratorEnum::Dense(d.iterator()?))
            },
        }
    }

    type BitType = MatchNoBits;

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        None
    }
}

enum DocIdSetIteratorEnum<'a> {
    Sparse(ShortArrayDISI<'a>),
    Medium(BitSetIterator<'a, FixedBitSet>),
    Dense(NotDocDocIdSetIterator<ShortArrayDISI<'a>>),
    Empty(EmptyDISI),
}
impl DocIdSetIterator for DocIdSetIteratorEnum<'_> {
    fn doc_id(&self) -> i32 {
        match self {
            DocIdSetIteratorEnum::Sparse(s) => s.doc_id(),
            DocIdSetIteratorEnum::Medium(m) => m.doc_id(),
            DocIdSetIteratorEnum::Dense(d) => d.doc_id(),
            DocIdSetIteratorEnum::Empty(e) => e.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            DocIdSetIteratorEnum::Sparse(s) => s.next_doc(),
            DocIdSetIteratorEnum::Medium(m) => m.next_doc(),
            DocIdSetIteratorEnum::Dense(d) => d.next_doc(),
            DocIdSetIteratorEnum::Empty(e) => e.next_doc(),
        }
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        match self {
            DocIdSetIteratorEnum::Sparse(s) => s.advance(_target),
            DocIdSetIteratorEnum::Medium(m) => m.advance(_target),
            DocIdSetIteratorEnum::Dense(d) => d.advance(_target),
            DocIdSetIteratorEnum::Empty(e) => e.advance(_target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            DocIdSetIteratorEnum::Sparse(s) => s.cost(),
            DocIdSetIteratorEnum::Medium(m) => m.cost(),
            DocIdSetIteratorEnum::Dense(d) => d.cost(),
            DocIdSetIteratorEnum::Empty(e) => e.cost(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::search::doc_id_set::DocIdSet;
    use crate::test::util::base_doc_id_set_test_case::{
        BaseDocIdSetTestCase, BaseDocIdSetTestCaseSupperImpl,
    };
    use crate::test::util::lucene_test_case::random;

    use crate::util::error::lucene_error::Result;
    use crate::util::roaring_doc_id_set::builder::Builder;
    use rand::prelude::StdRng;

    struct TestRoaringDocIdSet;
    #[test]
    fn test_bit_0() -> Result<()> {
        let test_case = TestRoaringDocIdSet;
        let mut random = random();
        test_case.test_bit_0(&mut random)
    }
    #[test]
    fn test_bit_1() -> Result<()> {
        let test_case = TestRoaringDocIdSet;
        let mut random = random();
        test_case.test_bit_1(&mut random)
    }
    #[test]
    fn test_bit_2() -> Result<()> {
        let test_case = TestRoaringDocIdSet;
        let mut random = random();
        test_case.test_bit_2(&mut random)
    }
    #[test]
    fn test_against_bit_set() -> Result<()> {
        let test_case = TestRoaringDocIdSet;
        let mut random = random();
        test_case.test_against_bit_set(&mut random)
    }
    #[test]
    fn test_ram_bytes_used() {
        let test_case = TestRoaringDocIdSet;
        let mut random = random();
        test_case.test_ram_bytes_used(&mut random);
    }
    impl BaseDocIdSetTestCase for TestRoaringDocIdSet {
        fn copy_of(&self, bs: &bit_set::BitSet, length: i32) -> impl DocIdSet {
            let mut builder = Builder::new(length);
            let iter = bs.iter();
            for doc in iter {
                let _ = builder.add(doc as i32);
            }
            builder.build()
        }

        fn assert_equals(
            &self,
            random: &mut StdRng,
            num_bits: i32,
            ds1: &bit_set::BitSet,
            ds2: impl DocIdSet,
        ) -> Result<()> {
            BaseDocIdSetTestCaseSupperImpl::assert_equals(
                self, random, num_bits, ds1, ds2,
            )
        }
    }
    impl BaseDocIdSetTestCaseSupperImpl for TestRoaringDocIdSet {}
}
