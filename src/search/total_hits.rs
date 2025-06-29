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
use std::fmt;
/// Description of the total number of hits of a query. The total hit count
/// can't generally be computed accurately without visiting all matches, which
/// is costly for queries that match lots of documents. Given that it is often
/// enough to have a lower bound of the number of hits, such as "there are more
/// than 1000 hits", Lucene has options to stop counting as soon as a threshold
/// has been reached in order to improve query times.
///
/// # Parameters
///
/// - `value`: The value of the total hit count. Must be interpreted in the
///   context of [`Relation`].
/// - `relation`: Whether `value` is the exact hit count (in which case
///   [`Relation`] is equal to [`Relation::EqualTo`]), or a lower bound of the
///   total hit count (in which case [`Relation`] is equal to
///   [`Relation::GreaterThanOrEqualTo`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotalHits {
    pub value: usize,
    pub relation: Relation,
}

impl TotalHits {
    pub fn new(value: usize, relation: Relation) -> Self {
        Self { value, relation }
    }
}

impl fmt::Display for TotalHits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.relation {
            Relation::EqualTo => write!(f, "{} hits", self.value),
            Relation::GreaterThanOrEqualTo => write!(f, "{}+ hits", self.value),
        }
    }
}
/// How the `TotalHits::value` should be interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// The total hit count is equal to `TotalHits::value`.
    EqualTo,
    /// The total hit count is greater than or equal to `TotalHits::value`.
    GreaterThanOrEqualTo,
}
