/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// An implementation of a selection algorithm, i.e., computing the k-th
/// greatest value from a collection.
pub trait Selector {
  /// Reorder elements so that the element at position `k` is the same as if
  /// all elements were sorted and all other elements are partitioned
  /// around it: `[from, k)` only contains elements that are less than or
  /// equal to `k`, and `(k, to)` only contains elements that are greater
  /// than or equal to `k`.
  fn select(&mut self, _from: usize, _to: usize, _k: usize) -> Result<()> {
    Err(LuceneError::need_implemented("select() not implement"))
  }

  /// Check the validity of the `from`, `to`, and `k` indices.
  fn check_args(&self, from: usize, to: usize, k: usize) -> Result<()> {
    if k < from {
      return Err(LuceneError::illegal_argument("k must be >= from"));
    }
    if k >= to {
      return Err(LuceneError::illegal_argument("k must be < to"));
    }
    Ok(())
  }

  /// Swap values at positions `i` and `j`.
  fn swap(&mut self, _i: usize, _j: usize) -> Result<()> {
    Err(LuceneError::need_implemented("swap() not implement"))
  }
}
