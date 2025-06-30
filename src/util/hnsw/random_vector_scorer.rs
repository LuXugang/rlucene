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
use crate::util::bits::Bits;
use crate::util::error::lucene_error::Result;

/// A trait for scoring random nodes in batches against an abstract query.
pub trait RandomVectorScorer {
    /// Returns the score between the query and the provided node.
    ///
    /// # Arguments
    ///
    /// * `node` - a random node in the graph
    ///
    /// # Errors
    ///
    /// Returns an error if the scoring fails (e.g., I/O error).
    fn score(&self, node: i32) -> Result<f32>;

    /// Returns the maximum possible ordinal for this scorer.
    fn max_ord(&self) -> i32;

    /// Translates a vector ordinal to the correct document ID.  
    /// By default, this is an identity function.
    ///
    /// # Arguments
    ///
    /// * `ord` - The vector ordinal.
    ///
    /// # Returns
    ///
    /// The document ID for the given vector ordinal.
    fn ord_to_doc(&self, ord: i32) -> i32 {
        ord
    }

    type Bits: Bits;
    type BitsR: Bits;
    /// Returns the [`Bits`] representing live documents.  
    /// By default, this is an identity function.
    ///
    /// # Arguments
    ///
    /// * `accept_docs` - The accept docs.
    ///
    /// # Returns
    ///
    /// The accept docs.
    fn get_accept_ords(&self, accept_docs: Self::Bits) -> Self::Bits;
}
