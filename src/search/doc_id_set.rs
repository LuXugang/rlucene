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
use std::rc::Rc;

use crate::search::doc_id_set_iterator::{AllDocIdSetIterator, DocIdSetIterator, EmptyDISI};
use crate::util::accountable::Accountable;
use crate::util::bits::{Bits, MatchAllBits, MatchNoBits};
use crate::util::error::lucene_error::Result;

/// A `DocIdSet` contains a set of document IDs.
/// Implementing types must provide an [`iterator`](DocIdSet::iterator) method
/// to access the set.
pub trait DocIdSet: Accountable {
    type DocIdSetIterator: DocIdSetIterator;
    fn iterator(&self) -> Result<Option<Self::DocIdSetIterator>>;

    // TODO: somehow this struct should express the cost of
    // iteration vs the cost of random access Bits; for
    // expensive Filters (e.g. distance < 1 km) we should use
    // bits() after all other Query/Filters have matched, but
    // this is the opposite of what bits() is for now
    // (down-low filtering using e.g. FixedBitSet)

    /// Optionally provides a [`Bits`] interface for random access to matching
    /// documents.
    ///
    /// # Returns
    /// * `None` if this `DocIdSet` does not support random access.
    ///
    /// Note that, unlike [`iterator`](DocIdSet::iterator), a return value of
    /// `None` **does not** imply that no documents match the filter!
    ///
    /// The default implementation does not provide random access, so you only
    /// need to implement this method if your [`DocIdSet`] can guarantee
    /// random access to every document ID in `O(1)` time without external
    /// disk access (as the [`Bits`] interface cannot throw an `IOError`).
    /// This is generally true for bit sets like
    /// [`FixedBitSet`](crate::util::fixed_bit_set::FixedBitSet),
    /// which return themselves if used as a [`DocIdSet`].
    type BitType: Bits;
    fn bits(&self) -> Option<Rc<Self::BitType>>;
}

/// A [`DocIdSet`] that matches all document IDs up to a specified document
/// (exclusive).
struct All {
    max_doc: i32,
    bits: Option<Rc<MatchAllBits>>,
}
impl All {
    #[allow(unused)]
    fn new(max_doc: i32) -> Self {
        let bits = Some(Rc::new(MatchAllBits::new(max_doc)));
        All { max_doc, bits }
    }
}
/// A `DocIdSet` that matches all doc ids up to a specified doc (exclusive).
impl DocIdSet for All {
    type DocIdSetIterator = AllDocIdSetIterator;

    fn iterator(&self) -> Result<Option<Self::DocIdSetIterator>> {
        Ok(Some(AllDocIdSetIterator::new(self.max_doc)))
    }

    type BitType = MatchAllBits;

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        self.bits.clone()
    }
}

impl Accountable for All {
    fn ram_bytes_used(&self) -> Result<i64> {
        Ok(0)
    }
}

pub struct EmptyDocIdSet;
impl Accountable for EmptyDocIdSet {
    fn ram_bytes_used(&self) -> Result<i64> {
        Ok(0)
    }
}
impl DocIdSet for EmptyDocIdSet {
    type DocIdSetIterator = EmptyDISI;

    fn iterator(&self) -> Result<Option<Self::DocIdSetIterator>> {
        Ok(None)
    }

    type BitType = MatchNoBits;

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        None
    }
}
