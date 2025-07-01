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
use crate::util::error::lucene_error::{LuceneError, Result};
use std::rc::Rc;

/// A [`DocIdSetIterator`] which iterates over set bits in a bit set.
///
/// # Note
/// This is an internal API.
pub struct BitSetIterator<T>
where
    T: BitSet,
{
    pub(crate) bits: Rc<T>,
    length: i32,
    cost: i64,
    doc: i32,
}

impl<T: BitSet> BitSetIterator<T> {
    pub fn new(bits: Rc<T>, cost: i64) -> Result<BitSetIterator<T>> {
        if cost < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "cost must be >= 0, got {cost}"
            )));
        }
        let length = bits.length();
        Ok(BitSetIterator {
            bits,
            length,
            cost,
            doc: -1,
        })
    }
    // Set the current doc id that this iterator is on.
    #[allow(unused)]
    fn set_doc_id(&mut self, doc_id: i32) {
        self.doc = doc_id;
    }
    pub fn get_bit_set(&self) -> Rc<T> {
        self.bits.clone()
    }
}

impl<T: BitSet> DocIdSetIterator for BitSetIterator<T> {
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
        self.doc = self.bits.next_set_bit(target);
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

    #[allow(unused)]
    fn equal_disi_type<T1: DocIdSetIterator + 'static, T2: DocIdSetIterator + 'static>(
        _it1: &T1,
        _it2: &T2,
    ) -> bool {
        TypeId::of::<T1>() == TypeId::of::<T2>()
    }
    #[allow(unused)]
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
        iterator: impl DocIdSetIterator + 'static,
    ) -> Option<FixedBitSet> {
        todo!()
    }

    // todo
    /// If the provided iterator wraps a [`SparseFixedBitSet`] returns it,
    /// otherwise returns `None`.
    pub fn get_sparse_fixed_bit_set_or_null<B: BitSet>(
        iterator: impl DocIdSetIterator + 'static,
    ) -> Option<SparseFixedBitSet> {
        todo!()
    }
}
