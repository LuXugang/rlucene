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
use crate::util::error::lucene_error::Result;

/// The start and end character offset of a token.
pub trait OffsetAttribute: Attribute {
    /// Returns this token's starting offset, the position of the first
    /// character in the source text.
    ///
    /// See also: [`Self::set_offset`]
    fn start_offset(&self) -> i32;

    /// Sets the starting and ending offset.
    ///
    /// # Errors
    ///
    /// Implementations should throw errors if `start_offset` or `end_offset`
    /// are negative, or if `start_offset > end_offset`.
    ///
    /// See also: [`Self::start_offset`], [`Self::end_offset`]
    fn set_offset(&mut self, start_offset: i32, end_offset: i32) -> Result<()>;

    /// Returns this token's ending offset, one greater than the position of the
    /// last character in the source text.
    ///
    /// The length of the token in the source text is `end_offset() -
    /// start_offset()`.
    ///
    /// See also: [`Self::set_offset`]
    fn end_offset(&self) -> i32;
}
