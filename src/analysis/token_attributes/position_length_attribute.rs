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

/// Determines how many positions this token spans. Very few analyzer components actually produce
/// this attribute, and indexing ignores it, but it's useful to express the graph structure naturally
/// produced by decompounding, word splitting/joining, synonym filtering, etc.
///
/// **Note:** this is optional, and most analyzers don’t change the default value (`1`).
pub trait PositionLengthAttribute: Attribute {
    /// Set the position length of this Token.
    ///
    /// The default value is `1`.
    ///
    /// # Parameters
    ///
    /// - `position_length`: how many positions this token spans.
    ///
    /// # Error
    ///
    /// Error if `position_length <= 0`.
    ///
    /// # See
    ///
    /// [`get_position_length`](PositionLengthAttribute::get_position_length)
    fn set_position_length(&mut self, position_length: i32) -> Result<()>;

    /// Returns the position length of this Token.
    ///
    /// # See
    ///
    /// [`set_position_length`](PositionLengthAttribute::set_position_length)
    fn get_position_length(&self) -> i32;
}
