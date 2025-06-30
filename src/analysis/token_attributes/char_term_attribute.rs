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

/// The term text of a `Token`.
pub trait CharTermAttribute: Attribute {
    /// Copies the contents of `buffer[offset..offset+length]` into the internal term buffer.
    ///
    /// # Parameters
    ///
    /// - `buffer`: the source character slice  
    /// - `offset`: index of first character to copy  
    /// - `length`: number of characters to copy  
    fn copy_buffer(&mut self, buffer: &[char], offset: usize, length: usize);

    /// Returns the internal term buffer which you can directly alter.
    ///
    /// If the buffer is too small for your token, use [`resize_buffer`] to grow it.
    /// After altering the buffer be sure to call [`set_length`] to record the number
    /// of valid characters placed into it.
    ///
    /// **Note:** the returned slice may be larger than the valid length.
    fn buffer(&mut self) -> &mut [char];

    /// Grows the term buffer to at least `new_size`, preserving existing content.
    ///
    /// # Returns
    ///
    /// A mutable slice to the new buffer (with `len() >= new_size`).
    fn resize_buffer(&mut self, new_size: usize) -> &mut [char];

    /// Sets the number of valid characters (length of the term) in the term buffer.
    ///
    /// Use this to truncate the buffer or to synchronize with external buffer manipulation.
    /// To grow the buffer, call [`resize_buffer`] first.
    ///
    /// # Returns
    ///
    /// `self` for chaining.
    fn set_length(&mut self, length: usize) -> &mut Self;

    /// Resets the term buffer to zero length.
    ///
    /// Use before appending via the `Appendable` interface.
    ///
    /// # Returns
    ///
    /// `self` for chaining.
    fn set_empty(&mut self) -> &mut Self;

    /// Appends the given `CharSequence` to this term.
    ///
    /// # Returns
    ///
    /// `self` for chaining.
    fn append(&mut self, csq: &str) -> &mut Self;

    /// Appends the subsequence `csq[start..end]` to this term.
    ///
    /// # Returns
    ///
    /// `self` for chaining.
    fn append_range(&mut self, csq: &str, start: usize, end: usize) -> &mut Self;

    /// Appends a single character `c` to this term.
    ///
    /// # Returns
    ///
    /// `self` for chaining.
    fn append_char(&mut self, c: char) -> &mut Self;

    /// Appends the specified `&str` to this term.
    ///
    /// # Returns
    ///
    /// `self` for chaining.
    fn append_str(&mut self, s: &str) -> &mut Self;

    /// Appends the specified `StringBuilder` to this term.
    ///
    /// # Returns
    ///
    /// `self` for chaining.
    fn append_string_builder(&mut self, sb: &String) -> &mut Self;

    /// Appends the contents of another `CharTermAttribute` to this term.
    ///
    /// # Returns
    ///
    /// `self` for chaining.
    fn append_term_attribute(&mut self, term_att: &impl CharTermAttribute) -> &mut Self;
}
