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
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::knn_vector_values::KnnVectorValuesEnm2;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

/// Optimized scalar quantized implementation of [`FlatVectorsScorer`] for quantized vectors
/// stored in the Lucene99 format.
#[derive(Clone, Debug)]
pub struct Lucene99ScalarQuantizedVectorScorer<F>
where
  F: FlatVectorsScorer,
{
  non_quantized_delegate: F,
}

impl<F> Lucene99ScalarQuantizedVectorScorer<F>
where
  F: FlatVectorsScorer,
{
  pub fn new(flat_vectors_scorer: F) -> Self {
    Self {
      non_quantized_delegate: flat_vectors_scorer,
    }
  }
}

impl<F> Display for Lucene99ScalarQuantizedVectorScorer<F>
where
  F: FlatVectorsScorer,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "ScalarQuantizedVectorScorer(nonQuantizedDelegate={})",
      self.non_quantized_delegate
    )
  }
}

impl<F> FlatVectorsScorer for Lucene99ScalarQuantizedVectorScorer<F>
where
  F: FlatVectorsScorer,
{
  type RandomVectorScorerSupplier<B, FV>
    = F::RandomVectorScorerSupplier<B, FV>
  where
    B: ByteVectorValues + TryClone,
    FV: FloatVectorValues + TryClone;

  fn get_random_vector_scorer_supplier<B, FV>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: KnnVectorValuesEnm2<B, FV>,
  ) -> Result<Self::RandomVectorScorerSupplier<B, FV>>
  where
    B: ByteVectorValues + TryClone,
    FV: FloatVectorValues + TryClone,
  {
    // It is possible to get to this branch during initial indexing and flush.
    self
      .non_quantized_delegate
      .get_random_vector_scorer_supplier(similarity_function, vector_values)
  }

  type RandomVectorScorerF32<T>
    = F::RandomVectorScorerF32<T>
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
    // It is possible to get to this branch during initial indexing and flush.
    self.non_quantized_delegate.get_random_vector_scorer_f32(
      similarity_function,
      vector_values,
      target,
    )
  }

  type RandomVectorScorerU8<T>
    = F::RandomVectorScorerU8<T>
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
    self.non_quantized_delegate.get_random_vector_scorer_u8(
      similarity_function,
      vector_values,
      target,
    )
  }
}
