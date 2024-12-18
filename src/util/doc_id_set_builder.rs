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
use crate::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::util::accountable::Accountable;
use crate::util::bit_doc_id_set::BitDocIdSet;
use crate::util::bit_set::BitSet;
use crate::util::bit_set_iterator::BitSetIterator;
use crate::util::error::runtime_error::RuntimeError;
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::int_array_doc_id_set::{IntArrayDocIdSet, IntArrayDocIdSetIterator};
use std::cmp::min;
use std::sync::Arc;

/// A builder of [`DocIdSet`]s. Initially, it uses a sparse structure to gather documents,
/// and then upgrades to a non-sparse bit set once enough hits match.
///
///
/// # Note
/// This is an internal API.
pub struct DocIdSetBuilder {
    max_doc: i32,
    threshold: i32,
    // pkg-private for testing
    multi_valued: bool,
    num_values_per_doc: f64,

    buffer: Vec<i32>,
    bit_set: Option<FixedBitSet>,
    counter: i64,
}
impl DocIdSetBuilder {
    /// Create a builder that can contain doc IDs between  0 and maxDoc.
    pub fn new(max_doc: i32) -> DocIdSetBuilder {
        Self::new_with_count(max_doc, -1, -1)
    }

    pub fn new_with_count(max_doc: i32, doc_count: i32, value_count: i64) -> DocIdSetBuilder {
        let multi_valued = doc_count < 0 || doc_count as i64 != value_count;
        let num_values_per_doc = if doc_count <= 0 || value_count < 0 {
            // assume one value per doc, this means the cost will be overestimated
            // if the docs are actually multi-valued
            1f64
        } else {
            // otherwise compute from index stats
            value_count as f64 / doc_count as f64
        };
        debug_assert!(
            num_values_per_doc >= 1f64,
            "value_count = {} doc_count = {}",
            value_count,
            doc_count
        );
        // For ridiculously small sets, we'll just use a sorted int[]
        // maxDoc >>> 7 is a good value if you want to save memory, lower values
        // such as maxDoc >>> 11 should provide faster building but at the expense
        // of using a full bitset even for quite sparse data
        Self {
            max_doc,
            multi_valued,
            num_values_per_doc,
            threshold: max_doc >> 7,
            buffer: Vec::new(),
            bit_set: None,
            counter: 0,
        }
    }
    pub fn add_disi<D: DocIdSetIterator>(&mut self, mut iter: impl DocIdSetIterator) {
        let cost = min(iter.cost(), i32::MAX as i64);
        self.grow(cost as i32);
        if self.bit_set.is_some() {
            let _ = BitSet::or(self.bit_set.as_mut().unwrap(), iter);
            return;
        }
        for _i in 0..cost {
            let doc = iter.next_doc();
            if doc == NO_MORE_DOCS {
                return;
            }
            self.add_doc(doc);
        }
        let mut doc = iter.next_doc();
        while doc != NO_MORE_DOCS {
            self.grow(1);
            self.add_doc(doc);
            doc = iter.next_doc();
        }
    }
    pub fn add_doc(&mut self, doc: i32) {
        if self.bit_set.is_none() {
            self.buffer.push(doc);
        } else {
            self.bit_set.as_mut().unwrap().set(doc);
        }
    }
    pub fn grow(&mut self, num_docs: i32) {
        if self.bit_set.is_none() {
            if self.buffer.len() as i32 + num_docs > self.threshold {
                self.upgrade_to_bitset();
                self.counter += num_docs as i64;
            }
        } else {
            self.counter += num_docs as i64;
        }
    }
    fn upgrade_to_bitset(&mut self) {
        debug_assert!(self.bit_set.is_none());
        let mut bitset = FixedBitSet::new(self.max_doc);
        let mut counter = 0i64;
        for doc in self.buffer.iter() {
            bitset.set(*doc);
            counter += 1;
        }
        self.bit_set = Some(bitset);
        self.counter = counter;
        self.buffer.clear();
    }
    pub fn build(&mut self) -> Result<DocIdSetBuilderEnum, RuntimeError> {
        if self.bit_set.is_some() {
            debug_assert!(self.counter >= 0);
            let cost = (self.counter as f64 / self.num_values_per_doc).round();
            let result = BitDocIdSet::new_with_cost(self.bit_set.take(), cost as i64)?;
            Ok(DocIdSetBuilderEnum::B(result))
        } else {
            self.buffer.sort();
            if self.multi_valued {
                self.buffer.dedup();
            } else {
                debug_assert!(self.no_dups());
            }
            self.buffer.push(NO_MORE_DOCS);
            let l = self.buffer.len() - 1;
            let result = IntArrayDocIdSet::new(self.buffer.clone(), l as i32)?;
            Ok(DocIdSetBuilderEnum::I(result))
        }
    }
    fn no_dups(&self) -> bool {
        for i in 1..self.buffer.len() {
            assert_eq!(self.buffer[i], self.buffer[i - 1]);
        }
        true
    }
    #[cfg(feature = "test_only")]
    pub fn get_num_values_per_doc(&self) -> f64 {
        self.num_values_per_doc
    }
    #[cfg(feature = "test_only")]
    pub fn get_multi_valued(&self) -> bool {
        self.multi_valued
    }
}

pub enum DocIdSetBuilderEnum {
    B(BitDocIdSet<FixedBitSet>),
    I(IntArrayDocIdSet),
}
impl Accountable for DocIdSetBuilderEnum {
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

impl DocIdSet for DocIdSetBuilderEnum {
    type DISIType<'a> = DocIdSetBuilderIterator<'a>;

    fn iterator(&self) -> Option<Self::DISIType<'_>> {
        match self {
            DocIdSetBuilderEnum::B(m) => Some(DocIdSetBuilderIterator::F(m.iterator()?)),
            DocIdSetBuilderEnum::I(m) => Some(DocIdSetBuilderIterator::I(m.iterator()?)),
        }
    }

    type BitType = FixedBitSet;

    fn bits(&self) -> Option<Arc<Self::BitType>> {
        match self {
            DocIdSetBuilderEnum::B(bit_doc_id_set) => Some(bit_doc_id_set.bits().unwrap()),
            DocIdSetBuilderEnum::I(_) => None,
        }
    }
}
pub enum DocIdSetBuilderIterator<'a> {
    F(BitSetIterator<'a, FixedBitSet>),
    I(IntArrayDocIdSetIterator<'a>),
}
impl DocIdSetIterator for DocIdSetBuilderIterator<'_> {
    fn doc_id(&self) -> i32 {
        match self {
            DocIdSetBuilderIterator::F(bit_set) => bit_set.doc_id(),
            DocIdSetBuilderIterator::I(int_array) => int_array.doc_id(),
        }
    }

    fn next_doc(&mut self) -> i32 {
        match self {
            DocIdSetBuilderIterator::F(bit_set) => bit_set.next_doc(),
            DocIdSetBuilderIterator::I(int_array) => int_array.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> i32 {
        match self {
            DocIdSetBuilderIterator::F(bit_set) => bit_set.advance(target),
            DocIdSetBuilderIterator::I(int_array) => int_array.advance(target),
        }
    }

    fn cost(&self) -> i64 {
        match self {
            DocIdSetBuilderIterator::F(bit_set) => bit_set.cost(),
            DocIdSetBuilderIterator::I(int_array) => int_array.cost(),
        }
    }
}
