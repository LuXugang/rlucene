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

/// Determines the position of this token relative to the previous `Token` in a `TokenStream`,  
/// used in phrase searching.
///
/// The default value is `1`.
///
/// Some common uses for this are:
/// - Set it to zero to put multiple terms in the same position. This is useful if, e.g., a word  
///   has multiple stems. Searches for phrases including either stem will match. In this case,  
///   all but the first stem's increment should be set to zero: the increment of the first  
///   instance should be one. Repeating a token with an increment of zero can also be used to  
///   boost the scores of matches on that token.
/// - Set it to values greater than one to inhibit exact phrase matches. If, for example, one  
///   does not want phrases to match across removed stop words, then one could build a stop word  
///   filter that removes stop words and also sets the increment to the number of stop words  
///   removed before each non-stop word. Then exact phrase queries will only match when the terms  
///   occur with no intervening stop words.
///
/// # See
/// [`PostingsEnum`](crate::index::postings_enum::PostingsEnum)
pub trait PositionIncrementAttribute: Attribute {
    /// Set the position increment. The default value is `1`.
    ///
    /// # Parameters
    ///
    /// - `position_increment`: the distance from the prior term; must be non-negative.
    ///
    /// # Error
    ///
    /// Error if `position_increment < 0`.
    fn set_position_increment(&mut self, position_increment: i32) -> Result<()>;

    /// Returns the position increment of this Token.
    ///
    /// # See
    ///
    /// [`set_position_increment`](PositionIncrementAttribute::set_position_increment)
    fn get_position_increment(&self) -> i32;
}
