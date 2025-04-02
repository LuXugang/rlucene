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
use crate::index::terms_enum::TermsEnums;
use crate::index::BytesRef;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::ToInt;

/// A per-document `byte[]` with presorted values. This is fundamentally an iterator over the `int`
/// ord values per document, with random access APIs to resolve an `int` ord to `BytesRef`.
///
/// Per-document values in a `SortedDocValues` are deduplicated, dereferenced, and sorted into a
/// dictionary of unique values. A pointer to the dictionary value (ordinal) can be retrieved for
/// each document. Ordinals are dense and in increasing sorted order.
pub trait SortedDocValues: DocValuesIterator {
    /// Returns the ordinal for the current docID.
    ///
    /// This method must only be called after `advance_exact(doc_id)` returns `true`.
    ///
    /// # Returns
    /// A dense ordinal (starts at 0, then increments in sorted order).
    fn ord_value(&self) -> Result<i32>;

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
    fn lookup_ord(&self, ord: i32) -> Result<BytesRef>;

    /// Returns the number of unique sorted values in this doc values set.
    ///
    /// This is equivalent to one plus the maximum ordinal.
    fn get_value_count(&self) -> i32;
    /// If `key` exists, returns its ordinal, else returns `-insertion_point - 1`, like `Arrays.binarySearch`.
    ///
    /// # Arguments
    /// * `key` - Key to look up
    ///
    /// # Returns
    /// * Ordinal of the key if found, otherwise `-insertion_point - 1`
    fn lookup_term(&mut self, key: &BytesRef) -> Result<i32> {
        let mut low = 0;
        let mut high = self.get_value_count() - 1;

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
    fn terms_enum(&mut self) -> Result<TermsEnums> {
        Err(LuceneError::not_implemented(""))
    }
    // TODO:
    // intersect not Implemented
}
