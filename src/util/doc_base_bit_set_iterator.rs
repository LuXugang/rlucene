/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::bit_set::BitSet;
use crate::util::bits::Bits;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::FixedBitSet;

/// A [`DocIdSetIterator`] like
/// [`BitSetIterator`](crate::util::bit_set_iterator::BitSetIterator) but has a
/// doc base in order to avoid storing previous 0s.
pub struct DocBaseBitSetIterator {
    bits: FixedBitSet,
    length: i32,
    cost: i64,
    doc_base: i32,
    doc: i32,
}

impl DocBaseBitSetIterator {
    pub fn new(bits: FixedBitSet, cost: i64, doc_base: i32) -> Result<DocBaseBitSetIterator> {
        if cost < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "cost must be >= 0, got {}",
                cost
            )));
        }
        if (doc_base & 63) != 0 {
            return Err(LuceneError::illegal_argument(format!(
                "docBase need to be a multiple of 64, got {}",
                doc_base
            )));
        }
        let length = bits.length() + doc_base;
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

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        if target >= self.length {
            self.doc = NO_MORE_DOCS;
            return Ok(self.doc);
        }
        let next = self
            .bits
            .next_set_bit(std::cmp::max(0, target - self.doc_base));
        if next == NO_MORE_DOCS {
            self.doc = NO_MORE_DOCS
        } else {
            self.doc = next + self.doc_base;
        }
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.cost)
    }
}
