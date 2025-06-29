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
use std::borrow::Cow;

use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::terms_enum::TermsEnum;
use crate::index::BytesRef;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::ToInt;

/// A per-document `byte[]` with presorted values. This is fundamentally an
/// iterator over the `int` ord values per document, with random access APIs to
/// resolve an `int` ord to `BytesRef`.
///
/// Per-document values in a `SortedDocValues` are deduplicated, dereferenced,
/// and sorted into a dictionary of unique values. A pointer to the dictionary
/// value (ordinal) can be retrieved for each document. Ordinals are dense and
/// in increasing sorted order.
pub trait SortedDocValues: DocValuesIterator {
    /// Returns the ordinal for the current docID.
    ///
    /// This method must only be called after `advance_exact(doc_id)` returns
    /// `true`.
    ///
    /// # Returns
    /// A dense ordinal (starts at 0, then increments in sorted order).
    fn ord_value(&mut self) -> Result<i32>;

    /// Resolves the provided ordinal to the associated dictionary value.
    ///
    /// The returned `BytesRef` may be reused across calls,
    /// so if you want to keep it, make sure to deep-copy the value.
    ///
    /// # Arguments
    /// * `ord` - An ordinal in the range `[0, get_value_count())`
    ///
    /// # Returns
    /// The dictionary value corresponding to the ordinal.
    fn lookup_ord(&mut self, _ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        Err(LuceneError::need_implemented(
            "this method is not implemented",
        ))
    }

    /// Returns the number of unique sorted values in this doc values set.
    ///
    /// This is equivalent to one plus the maximum ordinal.
    fn get_value_count(&mut self) -> Result<i32> {
        Err(LuceneError::need_implemented(
            "this method is not implemented",
        ))
    }
    /// If `key` exists, returns its ordinal, else returns `-insertion_point -
    /// 1`, like `Arrays.binarySearch`.
    ///
    /// # Arguments
    /// * `key` - Key to look up
    ///
    /// # Returns
    /// * Ordinal of the key if found, otherwise `-insertion_point - 1`
    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        let mut low = 0;
        let mut high = self.get_value_count()? - 1;

        while low <= high {
            let mid = (low + high) >> 1;
            let term = self.lookup_ord(mid)?;
            let cmp = term.as_ref().cmp(key).to_int();
            if cmp < 0 {
                low = mid + 1;
            } else if cmp > 0 {
                high = mid - 1;
            } else {
                return Ok(mid); // key found
            }
        }
        Ok(-(low + 1)) // key not found
    }
    type TermsEnum: TermsEnum;
    /// Returns a [`TermsEnum`] over the
    /// values. The enum supports
    /// [`TermsEnum::ord`] and
    /// [`TermsEnum::seek_exact_with_ord`].
    fn terms_enum(&mut self) -> Result<Self::TermsEnum> {
        Err(LuceneError::not_implemented(""))
    }
    // TODO:
    // intersect not Implemented
}
