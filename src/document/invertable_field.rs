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
use strum_macros::{EnumCount, FromRepr};

/// Describes how an
/// [`IndexableField`](crate::index::indexable_field::IndexableField) should be
/// inverted for indexing terms and postings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, FromRepr, Hash, EnumCount)]
#[repr(u8)]
pub enum InvertableType {
    /// The field should be treated as a single value whose binary content is
    /// returned by
    /// [`IndexableField::binary_value()`](crate::index::indexable_field::IndexableField::binary_value).
    /// The term frequency is assumed to be one. If you need to index
    /// multiple values, you should pass multiple
    /// [`IndexableField`](crate::index::indexable_field::IndexableField)
    /// instances to the
    /// [`IndexWriter`](crate::index::index_writer::IndexWriter). If the same
    /// value is provided multiple times, the term frequency will be equal
    /// to the number of times that this value occurred in the same document.
    BINARY,

    /// The field should be inverted through its
    /// [`IndexableField::token_stream()`](crate::index::indexable_field::IndexableField::token_stream).
    TokenStream,
}
