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
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::util::error::lucene_error::Result;
/// A list of per-document numeric values, sorted according to i64's cmp.
pub trait SortedNumericDocValues: DocValuesIterator {
    /// Iterates to the next value in the current document. Do not call this
    /// more than
    /// [`doc_value_count`](SortedNumericDocValues::doc_value_count) times for
    /// the document.
    fn next_value(&mut self) -> Result<i64>;

    /// Retrieves the number of values for the current document. This must
    /// always be greater than zero. It is illegal to call this method after
    /// [`advance_exact(int)`](DocValuesIterator::advance_exact) returned
    /// `false`.
    fn doc_value_count(&mut self) -> Result<i32>;

    fn is_single_valued(&self) -> bool {
        false
    }
    type NumericDocValues: NumericDocValues;
    fn get_numeric_doc_values(&mut self) -> Result<Option<Self::NumericDocValues>> {
        Ok(None)
    }
}
