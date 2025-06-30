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
use crate::util::error::lucene_error::Result;
use crate::util::hnsw::random_vector_scorer::RandomVectorScorer;

/// A supplier that creates  [`RandomVectorScorer`] from an ordinal.
pub trait RandomVectorScorerSupplier {
    type Scorer: RandomVectorScorer;
    /// This creates a [`RandomVectorScorer`] for scoring random nodes in
    /// batches against the given ordinal.
    ///
    /// # Arguments
    ///
    /// * `ord` - The ordinal of the node to compare.
    ///
    /// # Returns
    ///
    /// A new [`RandomVectorScorer`].
    fn scorer(&self, ord: i32) -> Result<Self::Scorer>;

    /// Make a copy of the supplier, which will copy the underlying
    /// `vectorValues` so the copy is safe to be used in other threads.
    fn copy(&self) -> Result<Self>
    where
        Self: Sized;
}
