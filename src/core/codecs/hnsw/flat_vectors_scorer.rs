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
use crate::core::index::knn_vector_values::KnnVectorValues;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::random_vector_scorer::RandomVectorScorer;
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;

/// Provides mechanisms to score vectors that are stored in a flat file The purpose of this class is
/// for providing flexibility to the codec utilizing the vectors
///
/// @lucene.experimental
pub trait FlatVectorsScorer {
  type RandomVectorScorerSupplier: RandomVectorScorerSupplier;
  /// Returns a [`RandomVectorScorerSupplier`] that can be used to score vectors
  ///
  /// # Parameters
  /// - `similarity_function`: the similarity function to use
  /// - `vector_values`: the vector values to score
  ///
  /// # Returns
  /// a [`RandomVectorScorerSupplier`] that can be used to score vectors
  ///
  /// # Errors
  /// Returns an error if an I/O error occurs
  fn get_random_vector_scorer_supplier<K>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: &K,
  ) -> Result<Self::RandomVectorScorerSupplier>
  where
    K: KnnVectorValues;

  type RandomVectorScorer: RandomVectorScorer;
  /// Returns a [`RandomVectorScorer`] for the given set of vectors and target vector.
  ///
  /// # Parameters
  /// - `similarity_function`: the similarity function to use
  /// - `vector_values`: the vector values to score
  /// - `target`: the target vector
  ///
  /// # Returns
  /// a [`RandomVectorScorer`] for the given field and target vector.
  ///
  /// # Errors
  /// Returns an error if an I/O error occurs when reading from the index.
  fn get_random_vector_scorer_f32<K>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: &K,
    target: &[f32],
  ) -> Result<Self::RandomVectorScorer>
  where
    K: KnnVectorValues;

  /// Returns a [`RandomVectorScorer`] for the given set of vectors and target vector.
  ///
  /// # Parameters
  /// - `similarity_function`: the similarity function to use
  /// - `vector_values`: the vector values to score
  /// - `target`: the target vector
  ///
  /// # Returns
  /// a [`RandomVectorScorer`] for the given field and target vector.
  ///
  /// # Errors
  /// Returns an error if an I/O error occurs when reading from the index.
  fn get_random_vector_scorer_u8<K>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: &K,
    target: &[u8],
  ) -> Result<Self::RandomVectorScorer>
  where
    K: KnnVectorValues;
}
