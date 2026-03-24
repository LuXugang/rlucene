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
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;

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
  fn score(&self, node: usize) -> Result<f32>;

  /// Returns the maximum possible ordinal for this scorer.
  fn max_ord(&self) -> usize;

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
  fn ord_to_doc(&self, ord: usize) -> usize {
    ord
  }

  type Bits<B>: Bits
  where
    B: Bits;
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
  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Result<Option<Self::Bits<B>>>
  where
    B: Bits;
}
