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
use crate::util::attribute::Attribute;

/// Sets the custom term frequency of a term within one document.
///
/// If this attribute is present in the analysis chain for a given field,
/// that field must be indexed with
/// [`IndexOptions::DocsAndFreqs`](crate::index::index_options::IndexOptions).
///
/// See also: [`IndexOptions`](crate::index::index_options::IndexOptions)
pub trait TermFrequencyAttribute: Attribute {
    /// Sets the custom term frequency of the current term within one document.
    fn set_term_frequency(&mut self, term_frequency: i32);

    /// Returns the custom term frequency.
    fn get_term_frequency(&self) -> i32;
}
