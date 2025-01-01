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
use crate::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::util::bit_set::BitSet;
use crate::util::bits::Bits;

use crate::util::error::runtime_error::RuntimeError;
use crate::util::fixed_bit_set::FixedBitSet;
use std::cmp::max;

/// A [`DocIdSetIterator`] like
/// [`BitSetIterator`](crate::util::bit_set_iterator::BitSetIterator) but has a doc base in order to avoid
/// storing previous 0s.
pub struct DocBaseBitSetIterator {
    bits: FixedBitSet,
    length: i32,
    cost: i64,
    doc_base: i32,
    doc: i32,
}

impl DocBaseBitSetIterator {
    pub fn new(
        bits: FixedBitSet,
        cost: i64,
        doc_base: i32,
    ) -> Result<DocBaseBitSetIterator, RuntimeError> {
        if cost < 0 {
            return Err(RuntimeError::illegal_argument(format!(
                "cost must be >= 0, got {}",
                cost
            )));
        }
        if (doc_base & 63) != 0 {
            return Err(RuntimeError::illegal_argument(format!(
                "docBase need to be a multiple of 64, got {}",
                doc_base
            )));
        }
        let length = bits.length();
        Ok(DocBaseBitSetIterator {
            bits,
            length,
            cost,
            doc_base,
            doc: -1,
        })
    }
    /// Gets the [`FixedBitSet`](FixedBitSet). A `docId` will exist in this
    /// [`DocIdSetIterator`](DocIdSetIterator) if the bitset
    /// contains `(docId - get_doc_base())`.
    ///
    /// # Returns
    /// The offset `docId` bitset.
    #[allow(unused)]
    fn get_bit_set(&self) -> &FixedBitSet {
        &self.bits
    }

    /// Gets the `docBase`. It is guaranteed that `docBase` is a multiple of 64.
    ///
    /// # Returns
    /// The `docBase`.
    #[allow(unused)]
    fn get_doc_base(&self) -> i32 {
        self.doc_base
    }
}

impl DocIdSetIterator for DocBaseBitSetIterator {
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
        let next = self.bits.next_set_bit(max(0, target - self.doc_base));
        if next == NO_MORE_DOCS {
            self.doc = NO_MORE_DOCS
        } else {
            self.doc = next + self.doc_base;
        }
        self.doc
    }

    fn cost(&self) -> i64 {
        self.cost
    }
}
