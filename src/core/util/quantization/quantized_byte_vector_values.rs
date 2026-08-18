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
use crate::core::codecs::lucene95::has_index_slice::HasIndexSlice;
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::knn_vector_values::DocIndexIteratorEnum2;
use crate::core::search::vector_scorer::VectorScorer;
use crate::core::search::vector_scorer::VectorScorerEnum2;
use crate::core::util::bits::BitsEnum2;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::quantization::scalar_quantizer::ScalarQuantizer;

/// A version of [`ByteVectorValues`], but additionally retrieving score correction offset for
/// Scalar quantization scores.
pub trait QuantizedByteVectorValues: ByteVectorValues + HasIndexSlice {
  fn get_scalar_quantizer(&self) -> Result<ScalarQuantizer> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_score_correction_constant(&self, ord: usize) -> Result<f32>;

  /// Return a `VectorScorer` for the given query vector.
  ///
  /// # Arguments
  /// * `query` - the query vector
  ///
  /// # Returns
  /// a `VectorScorer` instance or None
  type QuantizedVectorScorer: VectorScorer;
  fn scorer(&self, _query: &[f32]) -> Result<Option<Self::QuantizedVectorScorer>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type QuantizedByteVectorValues: QuantizedByteVectorValues;

  fn copy(&self) -> Result<Self::QuantizedByteVectorValues>;
}

crate::either_byte_vector_values!(
    pub QuantizedByteVectorValuesEnum2 {
        iter = DocIndexIteratorEnum2,
        bits = BitsEnum2,
        scorer = VectorScorerEnum2;
        A: A, B: B,
    }
);

impl<A, B> HasIndexSlice for QuantizedByteVectorValuesEnum2<A, B>
where
  A: QuantizedByteVectorValues,
  B: QuantizedByteVectorValues,
{
  fn seek(&self, pos: usize) -> Result<()> {
    match self {
      Self::A(values) => values.seek(pos),
      Self::B(values) => values.seek(pos),
    }
  }

  fn read_bytes(&self, bytes: &mut [u8], offset: usize, len: usize) -> Result<()> {
    match self {
      Self::A(values) => values.read_bytes(bytes, offset, len),
      Self::B(values) => values.read_bytes(bytes, offset, len),
    }
  }
}

impl<A, B> QuantizedByteVectorValues for QuantizedByteVectorValuesEnum2<A, B>
where
  A: QuantizedByteVectorValues,
  B: QuantizedByteVectorValues,
{
  fn get_scalar_quantizer(&self) -> Result<ScalarQuantizer> {
    match self {
      Self::A(values) => values.get_scalar_quantizer(),
      Self::B(values) => values.get_scalar_quantizer(),
    }
  }

  fn get_score_correction_constant(&self, ord: usize) -> Result<f32> {
    match self {
      Self::A(values) => values.get_score_correction_constant(ord),
      Self::B(values) => values.get_score_correction_constant(ord),
    }
  }

  type QuantizedVectorScorer =
    VectorScorerEnum2<A::QuantizedVectorScorer, B::QuantizedVectorScorer>;

  fn scorer(&self, query: &[f32]) -> Result<Option<Self::QuantizedVectorScorer>> {
    match self {
      Self::A(values) => QuantizedByteVectorValues::scorer(values, query)
        .map(|scorer| scorer.map(VectorScorerEnum2::A)),
      Self::B(values) => QuantizedByteVectorValues::scorer(values, query)
        .map(|scorer| scorer.map(VectorScorerEnum2::B)),
    }
  }

  type QuantizedByteVectorValues =
    QuantizedByteVectorValuesEnum2<A::QuantizedByteVectorValues, B::QuantizedByteVectorValues>;

  fn copy(&self) -> Result<Self::QuantizedByteVectorValues> {
    match self {
      Self::A(values) => {
        QuantizedByteVectorValues::copy(values).map(QuantizedByteVectorValuesEnum2::A)
      },
      Self::B(values) => {
        QuantizedByteVectorValues::copy(values).map(QuantizedByteVectorValuesEnum2::B)
      },
    }
  }
}
