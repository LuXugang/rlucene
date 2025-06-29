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
use crate::util::error::lucene_error::{LuceneError, Result};

/// An implementation of a selection algorithm, i.e., computing the k-th
/// greatest value from a collection.
pub trait Selector {
    /// Reorder elements so that the element at position `k` is the same as if
    /// all elements were sorted and all other elements are partitioned
    /// around it: `[from, k)` only contains elements that are less than or
    /// equal to `k`, and `(k, to)` only contains elements that are greater
    /// than or equal to `k`.
    fn select(&mut self, _from: i32, _to: i32, _k: i32) -> Result<()> {
        Err(LuceneError::need_implemented("select() is not implemented"))
    }

    /// Check the validity of the `from`, `to`, and `k` indices.
    fn check_args(&self, from: i32, to: i32, k: i32) -> Result<()> {
        if k < from {
            return Err(LuceneError::illegal_argument("k must be >= from"));
        }
        if k >= to {
            return Err(LuceneError::illegal_argument("k must be < to"));
        }
        Ok(())
    }

    /// Swap values at positions `i` and `j`.
    fn swap(&mut self, _i: i32, _j: i32) -> Result<()> {
        Err(LuceneError::need_implemented("swap() is not implemented"))
    }
}
