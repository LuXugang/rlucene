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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::{BitSetIteratorAccess, DocIdSetIteratorExtensions};
use crate::core::util::error::lucene_error::Result;

/// Computes the similarity score between a given query vector and different
/// document vectors. This is used for exact searching and scoring.
/// Advance this scorer directly to select the document used by `score`.
pub trait VectorScorer: DocIdSetIterator {
  /// Compute the score for the current document ID.
  ///
  /// # Returns
  /// The score for the current document ID
  ///
  /// # Errors
  /// Returns an error if score computation fails.
  fn score(&self) -> Result<f32>;
}

/// Concrete vector scorer for either indexed vector encoding.
pub enum VectorScorerEnum2<A, B> {
  Byte(A),
  Float(B),
}
impl<A: VectorScorer, B: VectorScorer> VectorScorer for VectorScorerEnum2<A, B> {
  fn score(&self) -> Result<f32> {
    match self {
      Self::Byte(s) => s.score(),
      Self::Float(s) => s.score(),
    }
  }
}

impl<A: VectorScorer, B: VectorScorer> DocIdSetIteratorExtensions for VectorScorerEnum2<A, B> {}

impl<A: VectorScorer, B: VectorScorer> BitSetIteratorAccess for VectorScorerEnum2<A, B> {}

impl<A: VectorScorer, B: VectorScorer> DocIdSetIterator for VectorScorerEnum2<A, B> {
  fn doc_id(&self) -> i32 {
    match self {
      Self::Byte(s) => s.doc_id(),
      Self::Float(s) => s.doc_id(),
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    match self {
      Self::Byte(s) => s.next_doc(),
      Self::Float(s) => s.next_doc(),
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    match self {
      Self::Byte(s) => s.advance(target),
      Self::Float(s) => s.advance(target),
    }
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    match self {
      Self::Byte(s) => s.slow_advance(target),
      Self::Float(s) => s.slow_advance(target),
    }
  }

  fn cost(&self) -> Result<i64> {
    match self {
      Self::Byte(s) => s.cost(),
      Self::Float(s) => s.cost(),
    }
  }
}
pub type LeafVectorScorer<LR> = VectorScorerEnum2<
  <<LR as crate::core::index::leaf_reader::LeafReader>::ByteVectorValues as crate::core::index::byte_vector_values::ByteVectorValues>::VectorScorer,
  <<LR as crate::core::index::leaf_reader::LeafReader>::FloatVectorValues as crate::core::index::float_vector_values::FloatVectorValues>::VectorScorer,
>;
