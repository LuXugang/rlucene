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
use crate::core::util::error::lucene_error::Result;

/// Computes the similarity score between a given query vector and different
/// document vectors. This is used for exact searching and scoring.
pub trait VectorScorer {
  /// Compute the score for the current document ID.
  ///
  /// # Returns
  /// The score for the current document ID
  ///
  /// # Errors
  /// Returns an error if score computation fails.
  fn score(&self) -> Result<f32>;

  type DocIdSetIteratorRef<'a>: DocIdSetIterator
  where
    Self: 'a;
  /// Returns an iterator over the document IDs.
  fn iterator(&self) -> Self::DocIdSetIteratorRef<'_>;
  type DocIdSetIteratorMut<'a>: DocIdSetIterator
  where
    Self: 'a;
  fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_>;
}

/// Concrete vector scorer for either indexed vector encoding.
pub enum VectorScorerEnum2<A, B> {
  Byte(A),
  Float(B),
}
impl<A: VectorScorer, B: VectorScorer> VectorScorer for VectorScorerEnum2<A, B> {
  type DocIdSetIteratorRef<'a>
    = crate::core::search::doc_id_set_iterator::DocIdSetIteratorEnum2<
    A::DocIdSetIteratorRef<'a>,
    B::DocIdSetIteratorRef<'a>,
  >
  where
    Self: 'a;
  type DocIdSetIteratorMut<'a>
    = crate::core::search::doc_id_set_iterator::DocIdSetIteratorEnum2<
    A::DocIdSetIteratorMut<'a>,
    B::DocIdSetIteratorMut<'a>,
  >
  where
    Self: 'a;
  fn score(&self) -> Result<f32> {
    match self {
      Self::Byte(s) => s.score(),
      Self::Float(s) => s.score(),
    }
  }
  fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
    match self {
      Self::Byte(s) => {
        crate::core::search::doc_id_set_iterator::DocIdSetIteratorEnum2::A(s.iterator())
      },
      Self::Float(s) => {
        crate::core::search::doc_id_set_iterator::DocIdSetIteratorEnum2::B(s.iterator())
      },
    }
  }
  fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
    match self {
      Self::Byte(s) => {
        crate::core::search::doc_id_set_iterator::DocIdSetIteratorEnum2::A(s.iterator_mut())
      },
      Self::Float(s) => {
        crate::core::search::doc_id_set_iterator::DocIdSetIteratorEnum2::B(s.iterator_mut())
      },
    }
  }
}
pub type LeafVectorScorer<LR> = VectorScorerEnum2<
  <<LR as crate::core::index::leaf_reader::LeafReader>::ByteVectorValues as crate::core::index::byte_vector_values::ByteVectorValues>::VectorScorer,
  <<LR as crate::core::index::leaf_reader::LeafReader>::FloatVectorValues as crate::core::index::float_vector_values::FloatVectorValues>::VectorScorer,
>;
