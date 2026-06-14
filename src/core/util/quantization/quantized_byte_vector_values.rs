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
  fn scorer(&self, _query: &[f32]) -> Result<Self::VectorScorer> {
    Err(LuceneError::unsupported_operation(""))
  }

  type QuantizedByteVectorValues: QuantizedByteVectorValues;

  fn copy(&self) -> Result<&Self::QuantizedByteVectorValues>;
}
