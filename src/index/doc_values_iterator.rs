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
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::Result;

pub trait DocValuesIterator: DocIdSetIterator {
    /// Advances the iterator to exactly `target` and returns whether `target`
    /// has a value.
    ///
    /// # Parameters
    /// - `target`: The target document ID to advance to. `Target` must be
    ///   greater than or equal to the current document ID
    ///   ([`doc_id()`](DocIdSetIterator::doc_id)) and must be a valid document
    ///   ID (i.e., `target >= 0` and `target < max_doc`).
    ///
    /// # Returns
    /// `true` if `target` has a value, otherwise returns `false`.
    ///
    /// # Note
    /// After this method returns, [`doc_id()`](DocIdSetIterator::doc_id)
    /// will return the value of `target`.
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        unimplemented!("advance_exact needs to be implemented if you need to use it")
    }
}
