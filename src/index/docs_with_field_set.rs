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
use std::rc::Rc;

use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::FixedBitSet;

//TODO
#[allow(unused)]
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
    /// Adds a document to the set.
    ///
    /// # Parameters
    /// - `doc_id`: The document ID to be added.
    pub fn add(&mut self, doc_id: i32) -> Result<()> {
        if doc_id <= self.last_doc_id {
            return Err(LuceneError::illegal_argument(format!(
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

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            DocsWithFieldSetEnum::Dense(d) => d.next_doc(),
            DocsWithFieldSetEnum::Sparse(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        match self {
            DocsWithFieldSetEnum::Dense(d) => d.advance(_target),
            DocsWithFieldSetEnum::Sparse(s) => s.advance(_target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            DocsWithFieldSetEnum::Dense(d) => d.cost(),
            DocsWithFieldSetEnum::Sparse(s) => s.cost(),
        }
    }
}

impl<T: BitSet> Accountable for DocsWithFieldSet<T> {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

impl<T: BitSet> DocIdSet for DocsWithFieldSet<T> {
    type DISIType<'b>
        = DocsWithFieldSetEnum<'b, T>
    where
        T: 'b;

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

#[cfg(test)]
mod tests {
    use crate::index::docs_with_field_set::DocsWithFieldSet;
    use crate::search::doc_id_set::DocIdSet;
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::test::util::lucene_test_case::random;

    use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;
    use rand::Rng;

    #[allow(dead_code)] // for quick search
    struct TestDocsWithFieldSet {}
    #[test]
    fn test_dense() -> Result<()> {
        let mut set = DocsWithFieldSet::new();
        let mut it = set.iterator().unwrap();
        assert_eq!(it.next_doc()?, NO_MORE_DOCS);

        let _ = set.add(0);
        it = set.iterator().unwrap();
        assert_eq!(0, it.next_doc()?);
        assert_eq!(it.next_doc()?, NO_MORE_DOCS);

        //TODO
        // let ram_bytes_used = set.ram_bytes_used();
        for i in 0..1000 {
            let _ = set.add(i);
        }
        //TODO:
        // assert_eq!(ram_bytes_used, set.ram_bytes_used());
        it = set.iterator().unwrap();
        for i in 0..1000 {
            assert_eq!(i, it.next_doc()?);
        }
        assert_eq!(NO_MORE_DOCS, it.next_doc()?);
        Ok(())
    }

    #[test]
    fn test_sparse() -> Result<()> {
        let mut random = random();
        let mut set = DocsWithFieldSet::new();
        let doc = random.random_range(0..10000);
        let _ = set.add(doc);
        let mut it = set.iterator().unwrap();
        assert_eq!(doc, it.next_doc()?);
        assert_eq!(it.next_doc()?, NO_MORE_DOCS);
        let doc2 = doc + TestUtil::next_int(&mut random, 1, 100);
        let _ = set.add(doc2);
        it = set.iterator().unwrap();
        assert_eq!(doc, it.next_doc()?);
        assert_eq!(doc2, it.next_doc()?);
        assert_eq!(it.next_doc()?, NO_MORE_DOCS);
        Ok(())
    }

    #[test]
    fn test_dense_then_sparse() -> Result<()> {
        let mut random = random();
        let dense_count = random.random_range(1..10000);
        let next_doc = dense_count + random.random_range(1..10000);
        let mut set = DocsWithFieldSet::new();
        for i in 0..dense_count {
            let _ = set.add(i);
        }
        let _ = set.add(next_doc);
        let mut it = set.iterator().unwrap();
        for i in 0..dense_count {
            assert_eq!(i, it.next_doc()?);
        }
        assert_eq!(next_doc, it.next_doc()?);
        assert_eq!(NO_MORE_DOCS, it.next_doc()?);
        Ok(())
    }
}
