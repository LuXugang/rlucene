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
use std::rc::Rc;

use crate::search::doc_id_set::DocIdSet;
use crate::search::doc_id_set_iterator::{AllDocIdSetIterator, DocIdSetIterator};
use crate::util::accountable::Accountable;
use crate::util::bit_set::BitSet;
use crate::util::bit_set_iterator::BitSetIterator;
use crate::util::bits::MatchNoBits;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::FixedBitSet;

/// Accumulator for documents that have a value for a field.
/// This is optimized for the case where all documents have a value.
pub struct DocsWithFieldSet {
    set: Option<FixedBitSet>,
    cardinality: i32,
    last_doc_id: i32,
}
impl Default for DocsWithFieldSet {
    fn default() -> Self {
        Self::new()
    }
}

impl DocsWithFieldSet {
    pub fn new() -> DocsWithFieldSet {
        DocsWithFieldSet {
            set: None,
            cardinality: 0,
            last_doc_id: -1,
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
        if self.set.is_some() {
            let set = self.set.as_mut().unwrap();
            set.ensure_capacity(doc_id);
            set.set(doc_id);
        } else if doc_id != self.cardinality {
            let mut set = FixedBitSet::new(doc_id + 1);
            set.set_with_range(0, self.cardinality);
            set.set(doc_id);
            self.set = Some(set);
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
pub enum DocsWithFieldSetEnum<'a> {
    Dense(AllDocIdSetIterator),
    Sparse(BitSetIterator<'a, FixedBitSet>),
}
impl DocIdSetIterator for DocsWithFieldSetEnum<'_> {
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

impl Accountable for DocsWithFieldSet {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

impl DocIdSet for DocsWithFieldSet {
    type DocIdSetIterator<'b> = DocsWithFieldSetEnum<'b>;

    fn iterator(&self) -> Option<Self::DocIdSetIterator<'_>> {
        if self.set.is_some() {
            debug_assert!(self.cardinality > 0);
            Some(DocsWithFieldSetEnum::Sparse(
                BitSetIterator::new(self.set.as_ref().unwrap(), self.cardinality as i64).unwrap(),
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

mod dwfs_util {
    //TODO
    #[allow(unused)]
    pub(super) const BASE_RAM_BYTES_USED: i64 = 0;
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use crate::index::docs_with_field_set::DocsWithFieldSet;
    use crate::search::doc_id_set::DocIdSet;
    use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::test::util::lucene_test_case::random;
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;

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
