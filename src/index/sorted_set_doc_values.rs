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
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::terms_enums::TermsEnums;
use crate::index::BytesRef;
use crate::store::IndexInput;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::ToInt;

/// A multi-valued version of [`SortedDocValues`](crate::index::sorted_doc_values::SortedDocValues).
///
/// Per-Document values in a `SortedSetDocValues` are deduplicated, dereferenced, and sorted into a
/// dictionary of unique values. A pointer to the dictionary value (ordinal) can be retrieved for
/// each document. Ordinals are dense and in increasing sorted order.
pub trait SortedSetDocValues<I>: DocValuesIterator
where
    I: IndexInput,
{
    /// Returns the next ordinal for the current document. It is illegal to call this method after
    /// [`advance_exact(int)`](DocValuesIterator::advance_exact) returned `false`. It is illegal to call this more than
    /// [`doc_value_count()`](SortedSetDocValues::doc_value_count) times for the currently-positioned doc.
    ///
    /// # Returns
    /// Next ordinal for the document. Ordinals are dense, start at 0, then increment by 1 for
    /// the next value in sorted order.
    fn next_ord(&mut self) -> Result<i32>;

    /// Retrieves the number of unique ords for the current document. This must always be greater than
    /// zero. It is illegal to call this method after [`advance_exact(int)`](DocValuesIterator::advance_exact) returned `false`.
    fn doc_value_count(&mut self) -> Result<i32>;

    /// Retrieves the value for the specified ordinal. The returned [`BytesRef`] may be re-used
    /// across calls to `lookup_ord`, so make sure to [`BytesRef::deep_copy_of`] it if you
    /// want to keep it around.
    ///
    /// # Arguments
    /// * `ord` - Ordinal to lookup
    ///
    /// See also: [`next_ord`](SortedSetDocValues::next_ord)
    fn lookup_ord(&mut self, _ord: i32) -> Result<BytesRef> {
        Err(LuceneError::need_implemented(
            "this method is not implemented",
        ))
    }
    /// Returns the number of unique values.
    ///
    /// # Returns
    /// Number of unique values in this `SortedDocValues`. This is also equivalent to one plus
    /// the maximum ordinal.
    fn get_value_count(&self) -> Result<i32> {
        Err(LuceneError::need_implemented(
            "this method is not implemented",
        ))
    }
    /// If `key` exists, returns its ordinal, else returns `-insertion_point - 1`, like `Arrays.binarySearch`.
    ///
    /// # Arguments
    /// * `key` - Key to look up
    ///
    /// # Returns
    /// * Ordinal of the key if found, otherwise `-insertion_point - 1`
    fn lookup_term(&mut self, key: &BytesRef) -> Result<i32> {
        let mut low = 0;
        let mut high = self.get_value_count()? - 1;

        while low <= high {
            let mid = (low + high) >> 1;
            let term = self.lookup_ord(mid)?;
            let cmp = term.cmp(key).to_int();
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
    /// Returns a [`TermsEnum`](crate::index::terms_enum::TermsEnum) over the values.
    /// The enum supports [`TermsEnum::ord()`](crate::index::terms_enum::TermsEnum::ord) and [`TermsEnum::seek_exact_with_ord()`](crate::index::terms_enum::TermsEnum::seek_exact_with_ord).
    fn terms_enum(&mut self) -> Result<TermsEnums<I>> {
        Err(LuceneError::not_implemented(""))
    }
    // TODO:
    // intersect not Implemented
}
