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
use crate::util::bytes_ref_iterator::BytesRefIterator;

/// Iterates over terms across multiple fields. The caller must check
/// [`field()`](FieldTermIterator::field) after each
/// [`next()`](BytesRefIterator::next) to see if the field changed, but `==` can
/// be used since the iterator implementation ensures it will use the same
/// `String` instance for a given field.
pub trait FieldTermIterator: BytesRefIterator {
    /// Returns the current field. This method should not be called after
    /// iteration is done. Note that you may use `==` to detect a change in
    /// field.
    fn field(&self) -> &str;

    /// Returns the del generation of the current term.
    /// Note: In some cases, this represents the current iterator (e.g., when
    /// using `MergedPrefixCodedTermsIterator`) to identify which iterator
    /// is active.
    fn del_gen(&self) -> i64;
}
