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
use crate::core::util::error::lucene_error::Result;
use crate::core::util::vector_util::{VECTOR_UTIL, VectorUtil};
#[cfg(test)]
use rand::{Rng, RngExt};
#[cfg(test)]
use strum::EnumCount;
use strum_macros::{Display, EnumCount, FromRepr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, FromRepr, EnumCount, Display)]
#[repr(u8)]
/// Vector similarity function; used in search to return top K most
/// similar vectors to a target vector. This is a label describing the
/// method used during indexing and searching of the vectors in order
/// to determine the nearest neighbors.
pub enum VectorSimilarityFunction {
  /// Euclidean distance
  Euclidean,
  /// Dot product. NOTE: this similarity is intended as an optimized
  /// way to perform cosine similarity. In order to use it, all vectors
  /// must be normalized, including both document and query vectors.
  /// Using dot product with vectors that are not normalized can result
  /// in errors or poor search results. Floating point vectors must be
  /// normalized to be of unit length, while byte vectors should simply
  /// all have the same norm.
  DotProduct,
  /// Cosine similarity. NOTE: the preferred way to perform cosine
  /// similarity is to normalize all vectors to unit length, and
  /// instead use [VectorSimilarityFunction::DotProduct]. You should
  /// only use this function if you need to preserve the original
  /// vectors and cannot normalize them in advance. The similarity
  /// score is normalised to assure it is positive.
  Cosine,
  /// Maximum inner product. This is like
  /// [VectorSimilarityFunction::DotProduct], but does not require
  /// normalization of the inputs. Should be used when the embedding
  /// vectors store useful information within the vector magnitude.
  MaximumInnerProduct,
}
impl VectorSimilarityFunction {
  pub fn ordinal(&self) -> i32 {
    match self {
      VectorSimilarityFunction::Euclidean => 0,
      VectorSimilarityFunction::DotProduct => 1,
      VectorSimilarityFunction::Cosine => 2,
      VectorSimilarityFunction::MaximumInnerProduct => 3,
    }
  }
  #[cfg(test)]
  pub fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
    let v = rng.random_range(0..Self::COUNT) as u8;
    Self::from_repr(v).unwrap()
  }
}
/// Use Default for padding
impl Default for VectorSimilarityFunction {
  fn default() -> Self {
    VectorSimilarityFunction::Euclidean
  }
}
impl VectorSimilarityFunction {
  /// Calculates a similarity score between the two vectors with a
  /// specified function. Higher similarity scores correspond to
  /// closer vectors.
  pub fn compare_f32(&self, v1: &[f32], v2: &[f32]) -> Result<f32> {
    match self {
      VectorSimilarityFunction::Euclidean => {
        let distance = VECTOR_UTIL.square_distance_f32(v1, v2)?;
        Ok(1.0 / (1.0 + distance))
      },
      VectorSimilarityFunction::DotProduct => {
        let dot = VECTOR_UTIL.dot_product_f32(v1, v2)?;
        Ok(((1.0 + dot) / 2.0).max(0.0))
      },
      VectorSimilarityFunction::Cosine => {
        let cosine = VECTOR_UTIL.cosine_f32(v1, v2)?;
        Ok(((1.0 + cosine) / 2.0).max(0.0))
      },
      VectorSimilarityFunction::MaximumInnerProduct => {
        let dot = VECTOR_UTIL.dot_product_f32(v1, v2)?;
        Ok(VectorUtil::scale_max_inner_product_score(dot))
      },
    }
  }

  /// Calculates a similarity score between the two vectors with a
  /// specified function. Higher similarity scores correspond to
  /// closer vectors. Each (signed) byte represents a vector dimension.
  pub fn compare_u8(&self, v1: &[u8], v2: &[u8]) -> Result<f32> {
    match self {
      VectorSimilarityFunction::Euclidean => {
        let distance = VECTOR_UTIL.square_distance_u8(v1, v2)? as f32;
        Ok(1.0 / (1.0 + distance))
      },
      VectorSimilarityFunction::DotProduct => VECTOR_UTIL.dot_product_score(v1, v2),
      VectorSimilarityFunction::Cosine => {
        let cosine = VECTOR_UTIL.cosine_u8(v1, v2)?;
        Ok((1.0 + cosine) / 2.0)
      },
      VectorSimilarityFunction::MaximumInnerProduct => {
        let dot = VECTOR_UTIL.dot_product_u8(v1, v2)? as f32;
        Ok(VectorUtil::scale_max_inner_product_score(dot))
      },
    }
  }
}
