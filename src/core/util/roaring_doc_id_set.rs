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
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, EmptyDISI};
use crate::core::util::accountable::Accountable;
use crate::core::util::bit_doc_id_set::BitDocIdSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::bits::MatchNoBits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::not_doc_id_set::{NotDocDocIdSetIterator, NotDocIdSet};
use std::sync::Arc;

// Number of documents in a block
const BLOCK_SIZE: usize = 1 << 16;
// The maximum length for an array, beyond that point we switch to a bitset
const MAX_ARRAY_LENGTH: usize = 1 << 12;
// todo

const BASE_RAM_BYTES_USED: i64 = 0;
/// [`DocIdSet`] implementation inspired by [roaringbitmap.org](http://roaringbitmap.org/)
///
/// The space is divided into blocks of `2^16` bits, and each block is encoded
/// independently. In each block, if fewer than `2^12` bits are set, documents
/// are simply stored in a `Vec<i16>`. If more than `2^16 - 2^12` bits are set,
/// the inverse of the set is encoded in a simple `Vec<i16>`. Otherwise,
/// a [`FixedBitSet`] is used.
///
/// # Note
/// This is an internal API.
pub struct RoaringDocIdSet {
    doc_id_sets: Vec<Option<Arc<DocIdSetEnum>>>,
    cardinality: usize,

    ram_bytes_used: i64,
}
impl RoaringDocIdSet {
    fn new(doc_id_sets: Vec<Option<DocIdSetEnum>>, cardinality: usize) -> Self {
        // todo
        let doc_id_sets: Vec<Option<Arc<DocIdSetEnum>>> = doc_id_sets
            .into_iter()
            .map(|opt| opt.map(Arc::new))
            .collect();
        let ram_bytes_used = 0;
        RoaringDocIdSet {
            doc_id_sets,
            cardinality,
            ram_bytes_used,
        }
    }

    pub(crate) fn cardinality(&self) -> usize {
        self.cardinality
    }
}
impl DocIdSet for RoaringDocIdSet {
    type DocIdSetIterator = Iterator;

    fn iterator(&self) -> Result<Self::DocIdSetIterator> {
        Ok(Iterator::new(
            self.doc_id_sets.clone(),
            self.cardinality as i64,
        ))
    }

    type BitType = MatchNoBits;

    fn bits(&self) -> Option<Arc<Self::BitType>> {
        None
    }
}

impl Accountable for RoaringDocIdSet {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}
pub mod builder {
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::core::util::bit_doc_id_set::BitDocIdSet;
    use crate::core::util::bit_set::BitSet;
    use crate::core::util::bits::Bits;
    use crate::core::util::error::lucene_error::LuceneError;
    use crate::core::util::error::lucene_error::Result;
    use crate::core::util::fixed_bit_set::FixedBitSet;
    use crate::core::util::not_doc_id_set::NotDocIdSet;
    use crate::core::util::roaring_doc_id_set::{
        BLOCK_SIZE, DocIdSetEnum, MAX_ARRAY_LENGTH, RoaringDocIdSet, ShortArrayDocIdSet,
    };

    pub struct Builder {
        max_doc: usize,
        sets: Vec<Option<DocIdSetEnum>>,
        cardinality: usize,
        last_doc_id: i32,
        current_block: i32,
        current_block_cardinality: usize,
        // We start by filling the buffer and when it's full we copy the
        // content of the buffer to the FixedBitSet and put further
        // documents in that bitset
        buffer: Vec<i16>,
        dense_buffer: FixedBitSet,
    }

    impl Builder {
        pub fn new(max_doc: usize) -> Builder {
            let buffer: Vec<i16> = Vec::with_capacity(MAX_ARRAY_LENGTH);
            let sets_length = (max_doc + (1 << 16) - 1) >> 16;
            let mut sets = Vec::with_capacity(sets_length);
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
        /// Add a new doc-id to this builder. NOTE: doc ids must be added in
        /// order.
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
                // self.buffer[self.current_block_cardinality as usize] = doc_id
                // as i16;
                self.buffer.push(doc_id as i16);
            } else {
                if self.dense_buffer.length() == 0 {
                    // the buffer is full, let's move to a fixed bit set
                    let num_bits = std::cmp::min(1 << 16, self.max_doc - (block << 16) as usize);
                    self.dense_buffer = FixedBitSet::new(num_bits);
                    for i in 0..self.buffer.len() {
                        self.dense_buffer.set(self.buffer[i] as usize & 0xFFFF);
                    }
                }
                self.dense_buffer.set(doc_id as usize & 0xFFFF);
            }
            self.last_doc_id = doc_id;
            self.current_block_cardinality += 1;
            Ok(())
        }
        /// Add the content of the provided DocIdSetIterator.
        pub fn add_disi<T: DocIdSetIterator>(&mut self, mut disi: T) -> Result<()> {
            let mut doc = disi.next_doc()?;
            while doc != NO_MORE_DOCS {
                let _ = self.add(doc);
                doc = disi.next_doc()?;
            }
            Ok(())
        }
        pub fn build(&mut self) -> RoaringDocIdSet {
            let _ = self.flush();
            RoaringDocIdSet::new(std::mem::take(&mut self.sets), self.cardinality)
        }
        fn flush(&mut self) -> Result<()> {
            debug_assert!(self.current_block_cardinality <= BLOCK_SIZE);
            if self.current_block_cardinality <= MAX_ARRAY_LENGTH {
                // use sparse encoding
                debug_assert_eq!(self.dense_buffer.length(), 0);
                if self.current_block_cardinality > 0 {
                    let sparse = Some(DocIdSetEnum::Sparse(ShortArrayDocIdSet::new(
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
                    && BLOCK_SIZE - self.current_block_cardinality < MAX_ARRAY_LENGTH
                {
                    let capacity = BLOCK_SIZE - self.current_block_cardinality;
                    let mut excluded_docs: Vec<i16> = vec![0; capacity];
                    self.dense_buffer.flip_range(0, self.dense_buffer.length());
                    let mut excluded_doc: usize = 0;

                    for excluded_doc_ref in excluded_docs.iter_mut() {
                        let v = self.dense_buffer.next_set_bit(excluded_doc);
                        assert_ne!(v, NO_MORE_DOCS as usize);
                        *excluded_doc_ref = v as i16;
                        excluded_doc = v + 1;
                    }

                    debug_assert!(
                        excluded_doc == self.dense_buffer.length()
                            || self.dense_buffer.next_set_bit(excluded_doc)
                                == NO_MORE_DOCS as usize
                    );
                    let dense: Option<DocIdSetEnum> = Some(DocIdSetEnum::Dense(NotDocIdSet::new(
                        BLOCK_SIZE as i32,
                        ShortArrayDocIdSet::new(excluded_docs),
                    )));
                    self.buffer.clear();
                    self.sets[self.current_block as usize] = dense;
                } else {
                    let result = BitDocIdSet::with_cost(
                        Some(std::mem::take(&mut self.dense_buffer)),
                        self.current_block_cardinality as i64,
                    )?;

                    let medium: Option<DocIdSetEnum> = Some(DocIdSetEnum::Medium(result));
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

const SHORT_ARRAY_DOC_ID_SET_BASE_RAM_BYTES_USED: i64 = 0;

pub struct ShortArrayDocIdSet {
    doc_ids: Arc<Vec<i16>>,
}
impl ShortArrayDocIdSet {
    fn new(doc_ids: Vec<i16>) -> ShortArrayDocIdSet {
        ShortArrayDocIdSet {
            doc_ids: Arc::new(doc_ids),
        }
    }
}

impl Accountable for ShortArrayDocIdSet {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

impl DocIdSet for ShortArrayDocIdSet {
    type DocIdSetIterator = ShortArrayDISI;

    fn iterator(&self) -> Result<Self::DocIdSetIterator> {
        Ok(ShortArrayDISI::new(self.doc_ids.clone()))
    }

    type BitType = MatchNoBits;

    fn bits(&self) -> Option<Arc<Self::BitType>> {
        None
    }
}

pub struct ShortArrayDISI {
    i: i32,
    doc: i32,
    doc_ids: Arc<Vec<i16>>,
}
impl ShortArrayDISI {
    fn new(doc_ids: Arc<Vec<i16>>) -> Self {
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
impl DocIdSetIterator for ShortArrayDISI {
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

    fn advance(&mut self, target: i32) -> Result<i32> {
        let mut lo = self.i + 1;
        let mut hi = self.doc_ids.len() as i32 - 1;
        while lo <= hi {
            let mid = (lo + hi) >> 1;
            let mid_doc = self.doc_id_index(mid);
            if mid_doc < target {
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

pub struct Iterator {
    block: i32,
    doc: i32,
    set_length: usize,
    sub: Option<DocIdSetIteratorEnum>,
    doc_id_sets: Vec<Option<Arc<DocIdSetEnum>>>,
    cardinality: i64,
}
impl Iterator {
    fn new(doc_id_sets: Vec<Option<Arc<DocIdSetEnum>>>, cardinality: i64) -> Self {
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
                self.sub = Some(
                    self.doc_id_sets[self.block as usize]
                        .as_ref()
                        .unwrap()
                        .iterator()?,
                );
                let sub_next = self.sub.as_mut().unwrap().next_doc()?;
                debug_assert!(sub_next != NO_MORE_DOCS);
                self.doc = (self.block << 16) | sub_next;
                break;
            }
        }
        Ok(self.doc)
    }
}
impl DocIdSetIterator for Iterator {
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

    fn advance(&mut self, target: i32) -> Result<i32> {
        let target_block = target >> 16;
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
            self.sub = Some(
                self.doc_id_sets[self.block as usize]
                    .as_ref()
                    .unwrap()
                    .iterator()?,
            )
        }
        let sub_next = self.sub.as_mut().unwrap().advance(target & 0xFFFF)?;
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
    type DocIdSetIterator = DocIdSetIteratorEnum;

    fn iterator(&self) -> Result<Self::DocIdSetIterator> {
        match self {
            DocIdSetEnum::Sparse(s) => Ok(DocIdSetIteratorEnum::Sparse(s.iterator()?)),
            DocIdSetEnum::Medium(m) => Ok(DocIdSetIteratorEnum::Medium(m.iterator()?)),
            DocIdSetEnum::Dense(d) => Ok(DocIdSetIteratorEnum::Dense(d.iterator()?)),
        }
    }

    type BitType = MatchNoBits;

    fn bits(&self) -> Option<Arc<Self::BitType>> {
        None
    }
}

enum DocIdSetIteratorEnum {
    Sparse(ShortArrayDISI),
    Medium(BitSetIterator<Arc<FixedBitSet>>),
    Dense(NotDocDocIdSetIterator<ShortArrayDISI>),
    Empty(EmptyDISI),
}
impl DocIdSetIterator for DocIdSetIteratorEnum {
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

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            DocIdSetIteratorEnum::Sparse(s) => s.advance(target),
            DocIdSetIteratorEnum::Medium(m) => m.advance(target),
            DocIdSetIteratorEnum::Dense(d) => d.advance(target),
            DocIdSetIteratorEnum::Empty(e) => e.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            DocIdSetIteratorEnum::Sparse(s) => s.slow_advance(target),
            DocIdSetIteratorEnum::Medium(m) => m.slow_advance(target),
            DocIdSetIteratorEnum::Dense(d) => d.slow_advance(target),
            DocIdSetIteratorEnum::Empty(e) => e.slow_advance(target),
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

    use rand::Rng;

    use crate::core::search::doc_id_set::DocIdSet;
    use crate::core::util::error::lucene_error::Result;
    use crate::core::util::roaring_doc_id_set::builder::Builder;
    use crate::test::util::base_doc_id_set_test_case::{
        BaseDocIdSetTestCase, BaseDocIdSetTestCaseSupperImpl,
    };
    use crate::test::util::lucene_test_case::lucene_test_case_util::random;

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
        fn copy_of(&self, bs: &bit_set::BitSet, length: usize) -> impl DocIdSet {
            let mut builder = Builder::new(length);
            let iter = bs.iter();
            for doc in iter {
                let _ = builder.add(doc as i32);
            }
            builder.build()
        }

        fn assert_equals<R: Rng + ?Sized>(
            &self,
            random: &mut R,
            num_bits: usize,
            ds1: &bit_set::BitSet,
            ds2: impl DocIdSet,
        ) -> Result<()> {
            BaseDocIdSetTestCaseSupperImpl::assert_equals(self, random, num_bits, ds1, ds2)
        }
    }
    impl BaseDocIdSetTestCaseSupperImpl for TestRoaringDocIdSet {}
}
