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
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::random_vector_scorer::RandomVectorScorer;
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;
use std::fmt::Display;
use std::sync::Arc;

pub enum FlatVectorValuesEnum<B, F> {
  Byte(B),
  Float(F),
}

/// Provides mechanisms to score vectors stored in a flat file. This trait
/// for providing flexibility to the codec utilizing the vectors
pub trait FlatVectorsScorer: Display {
  type RandomVectorScorerSupplier<B, F>: RandomVectorScorerSupplier
  where
    B: ByteVectorValues + TryClone + Send,
    B::ByteVectorValues: Send,
    F: FloatVectorValues + TryClone + Send,
    F::FloatVectorValues: Send;
  /// Returns a `RandomVectorScorerSupplier` that can be used to score vectors
  ///
  /// # Parameters
  /// - `similarity_function`: the similarity function to use
  /// - `vector_values`: the vector values to score
  ///
  /// # Returns
  /// a `RandomVectorScorerSupplier` that can be used to score vectors
  ///
  /// # Errors
  /// Returns an error if an I/O error occurs
  fn get_random_vector_scorer_supplier<B, F>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: FlatVectorValuesEnum<B, F>,
  ) -> Result<Self::RandomVectorScorerSupplier<B, F>>
  where
    B: ByteVectorValues + TryClone + Send,
    B::ByteVectorValues: Send,
    F: FloatVectorValues + TryClone + Send,
    F::FloatVectorValues: Send;

  type RandomVectorScorerF32<T>: RandomVectorScorer
  where
    T: FloatVectorValues;
  /// Returns a `RandomVectorScorer` for the given set of vectors and target vector.
  ///
  /// # Parameters
  /// - `similarity_function`: the similarity function to use
  /// - `vector_values`: the vector values to score
  /// - `target`: the target vector
  ///
  /// # Returns
  /// a `RandomVectorScorer` for the given field and target vector.
  ///
  /// # Errors
  /// Returns an error if an I/O error occurs when reading from the index.
  fn get_random_vector_scorer_f32<K>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: K,
    target: Vec<f32>,
  ) -> Result<Self::RandomVectorScorerF32<K>>
  where
    K: FloatVectorValues;

  type RandomVectorScorerU8<T>: RandomVectorScorer
  where
    T: ByteVectorValues;
  /// Returns a `RandomVectorScorer` for the given set of vectors and target vector.
  ///
  /// # Parameters
  /// - `similarity_function`: the similarity function to use
  /// - `vector_values`: the vector values to score
  /// - `target`: the target vector
  ///
  /// # Returns
  /// a `RandomVectorScorer` for the given field and target vector.
  ///
  /// # Errors
  /// Returns an error if an I/O error occurs when reading from the index.
  fn get_random_vector_scorer_u8<K>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: K,
    target: Vec<u8>,
  ) -> Result<Self::RandomVectorScorerU8<K>>
  where
    K: ByteVectorValues;
}

impl<FV> FlatVectorsScorer for Arc<FV>
where
  FV: FlatVectorsScorer,
{
  type RandomVectorScorerSupplier<B, F>
    = FV::RandomVectorScorerSupplier<B, F>
  where
    B: ByteVectorValues + TryClone + Send,
    B::ByteVectorValues: Send,
    F: FloatVectorValues + TryClone + Send,
    F::FloatVectorValues: Send;

  fn get_random_vector_scorer_supplier<B, F>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: FlatVectorValuesEnum<B, F>,
  ) -> Result<Self::RandomVectorScorerSupplier<B, F>>
  where
    B: ByteVectorValues + TryClone + Send,
    B::ByteVectorValues: Send,
    F: FloatVectorValues + TryClone + Send,
    F::FloatVectorValues: Send,
  {
    (**self).get_random_vector_scorer_supplier(similarity_function, vector_values)
  }

  type RandomVectorScorerF32<T>
    = FV::RandomVectorScorerF32<T>
  where
    T: FloatVectorValues;

  fn get_random_vector_scorer_f32<K>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: K,
    target: Vec<f32>,
  ) -> Result<Self::RandomVectorScorerF32<K>>
  where
    K: FloatVectorValues,
  {
    (**self).get_random_vector_scorer_f32(similarity_function, vector_values, target)
  }

  type RandomVectorScorerU8<T>
    = FV::RandomVectorScorerU8<T>
  where
    T: ByteVectorValues;

  fn get_random_vector_scorer_u8<K>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: K,
    target: Vec<u8>,
  ) -> Result<Self::RandomVectorScorerU8<K>>
  where
    K: ByteVectorValues,
  {
    (**self).get_random_vector_scorer_u8(similarity_function, vector_values, target)
  }
}
