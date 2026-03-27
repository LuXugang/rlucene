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
use crate::core::util::bits::{Bits, BitsEnum2};
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
  fn score(&mut self, node: usize) -> Result<f32>;

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
  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    Ok(ord)
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
pub enum RandomVectorScorerEnum2<A, B> {
  A(A),
  B(B),
}
impl<A, B> RandomVectorScorer for RandomVectorScorerEnum2<A, B>
where
  A: RandomVectorScorer,
  B: RandomVectorScorer,
{
  fn score(&mut self, node: usize) -> Result<f32> {
    match self {
      RandomVectorScorerEnum2::A(t) => t.score(node),
      RandomVectorScorerEnum2::B(s) => s.score(node),
    }
  }

  fn max_ord(&self) -> usize {
    match self {
      RandomVectorScorerEnum2::A(t) => t.max_ord(),
      RandomVectorScorerEnum2::B(s) => s.max_ord(),
    }
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    match self {
      RandomVectorScorerEnum2::A(t) => t.ord_to_doc(ord),
      RandomVectorScorerEnum2::B(s) => s.ord_to_doc(ord),
    }
  }

  type Bits<C>
    = BitsEnum2<A::Bits<C>, B::Bits<C>>
  where
    C: Bits;

  fn get_accept_ords<C>(&self, accept_docs: Option<C>) -> Result<Option<Self::Bits<C>>>
  where
    C: Bits,
  {
    Ok(match self {
      RandomVectorScorerEnum2::A(t) => t.get_accept_ords(accept_docs)?.map(BitsEnum2::A),
      RandomVectorScorerEnum2::B(t) => t.get_accept_ords(accept_docs)?.map(BitsEnum2::B),
    })
  }
}
