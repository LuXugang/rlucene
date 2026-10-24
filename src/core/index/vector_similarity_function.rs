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
use crate::core::util::CoreHelper;
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
    [
      Self::Euclidean,
      Self::DotProduct,
      Self::Cosine,
      Self::MaximumInnerProduct,
    ][rng.random_range(0..Self::COUNT)]
  }
}
/// Use Default for padding
impl Default for VectorSimilarityFunction {
  fn default() -> Self {
    VectorSimilarityFunction::Euclidean
  }
}
/// Scoring policy selected before entering a vector scoring loop.
/// Concrete zero-sized policies preserve the same arithmetic as the metadata enum.
pub trait VectorSimilarity: Copy + Send + Sync + std::fmt::Debug {
  /// Built-in pure policies accept identical float bits from immutable indexed
  /// storage. Custom policies retain their original buffer and slice identity.
  const SUPPORTS_MAPPED_FLOAT_INPUT: bool = false;

  fn function(self) -> VectorSimilarityFunction;
  fn compare_f32(&self, v1: &[f32], v2: &[f32]) -> Result<f32>;
  fn compare_u8(&self, v1: &[u8], v2: &[u8]) -> Result<f32>;
}

/// Concrete `Euclidean` scoring policy.
#[derive(Clone, Copy, Debug, Default)]
pub struct EuclideanSimilarity;

impl VectorSimilarity for EuclideanSimilarity {
  const SUPPORTS_MAPPED_FLOAT_INPUT: bool = true;

  fn function(self) -> VectorSimilarityFunction {
    VectorSimilarityFunction::Euclidean
  }
  #[inline]
  fn compare_f32(&self, v1: &[f32], v2: &[f32]) -> Result<f32> {
    let distance = VECTOR_UTIL.square_distance_f32(v1, v2)?;
    Ok(1.0 / (1.0 + distance))
  }
  #[inline]
  fn compare_u8(&self, v1: &[u8], v2: &[u8]) -> Result<f32> {
    let distance = VECTOR_UTIL.square_distance_u8(v1, v2)? as f32;
    Ok(1.0 / (1.0 + distance))
  }
}

/// Concrete `DotProduct` scoring policy.
#[derive(Clone, Copy, Debug, Default)]
pub struct DotProductSimilarity;

impl VectorSimilarity for DotProductSimilarity {
  const SUPPORTS_MAPPED_FLOAT_INPUT: bool = true;

  fn function(self) -> VectorSimilarityFunction {
    VectorSimilarityFunction::DotProduct
  }
  fn compare_f32(&self, v1: &[f32], v2: &[f32]) -> Result<f32> {
    let dot = VECTOR_UTIL.dot_product_f32(v1, v2)?;
    Ok(CoreHelper::max_f32((1.0 + dot) / 2.0, 0.0))
  }
  fn compare_u8(&self, v1: &[u8], v2: &[u8]) -> Result<f32> {
    VECTOR_UTIL.dot_product_score(v1, v2)
  }
}

/// Concrete `Cosine` scoring policy.
#[derive(Clone, Copy, Debug, Default)]
pub struct CosineSimilarity;

impl VectorSimilarity for CosineSimilarity {
  const SUPPORTS_MAPPED_FLOAT_INPUT: bool = true;

  fn function(self) -> VectorSimilarityFunction {
    VectorSimilarityFunction::Cosine
  }
  fn compare_f32(&self, v1: &[f32], v2: &[f32]) -> Result<f32> {
    let cosine = VECTOR_UTIL.cosine_f32(v1, v2)?;
    Ok(CoreHelper::max_f32((1.0 + cosine) / 2.0, 0.0))
  }
  fn compare_u8(&self, v1: &[u8], v2: &[u8]) -> Result<f32> {
    let cosine = VECTOR_UTIL.cosine_u8(v1, v2)?;
    Ok((1.0 + cosine) / 2.0)
  }
}

/// Concrete `MaximumInnerProduct` scoring policy.
#[derive(Clone, Copy, Debug, Default)]
pub struct MaximumInnerProductSimilarity;

impl VectorSimilarity for MaximumInnerProductSimilarity {
  const SUPPORTS_MAPPED_FLOAT_INPUT: bool = true;

  fn function(self) -> VectorSimilarityFunction {
    VectorSimilarityFunction::MaximumInnerProduct
  }
  fn compare_f32(&self, v1: &[f32], v2: &[f32]) -> Result<f32> {
    let dot = VECTOR_UTIL.dot_product_f32(v1, v2)?;
    Ok(VectorUtil::scale_max_inner_product_score(dot))
  }
  fn compare_u8(&self, v1: &[u8], v2: &[u8]) -> Result<f32> {
    let dot = VECTOR_UTIL.dot_product_u8(v1, v2)? as f32;
    Ok(VectorUtil::scale_max_inner_product_score(dot))
  }
}

impl VectorSimilarityFunction {
  /// Compares vectors using the similarity recorded in field metadata.
  pub fn compare_f32(&self, v1: &[f32], v2: &[f32]) -> Result<f32> {
    match self {
      Self::Euclidean => EuclideanSimilarity.compare_f32(v1, v2),
      Self::DotProduct => DotProductSimilarity.compare_f32(v1, v2),
      Self::Cosine => CosineSimilarity.compare_f32(v1, v2),
      Self::MaximumInnerProduct => MaximumInnerProductSimilarity.compare_f32(v1, v2),
    }
  }
  /// Compares vectors using the similarity recorded in field metadata.
  pub fn compare_u8(&self, v1: &[u8], v2: &[u8]) -> Result<f32> {
    match self {
      Self::Euclidean => EuclideanSimilarity.compare_u8(v1, v2),
      Self::DotProduct => DotProductSimilarity.compare_u8(v1, v2),
      Self::Cosine => CosineSimilarity.compare_u8(v1, v2),
      Self::MaximumInnerProduct => MaximumInnerProductSimilarity.compare_u8(v1, v2),
    }
  }
}

impl VectorSimilarity for VectorSimilarityFunction {
  const SUPPORTS_MAPPED_FLOAT_INPUT: bool = true;

  fn function(self) -> VectorSimilarityFunction {
    self
  }
  fn compare_f32(&self, v1: &[f32], v2: &[f32]) -> Result<f32> {
    VectorSimilarityFunction::compare_f32(self, v1, v2)
  }
  fn compare_u8(&self, v1: &[u8], v2: &[u8]) -> Result<f32> {
    VectorSimilarityFunction::compare_u8(self, v1, v2)
  }
}

/// Dispatches at a field or query boundary, keeping a concrete policy inside each arm.
#[macro_export]
macro_rules! with_vector_similarity {
  ($function:expr, $similarity:ident, $body:expr) => {{
    use $crate::core::index::vector_similarity_function::*;
    match $function {
      VectorSimilarityFunction::Euclidean => {
        let $similarity = EuclideanSimilarity;
        $body
      },
      VectorSimilarityFunction::DotProduct => {
        let $similarity = DotProductSimilarity;
        $body
      },
      VectorSimilarityFunction::Cosine => {
        let $similarity = CosineSimilarity;
        $body
      },
      VectorSimilarityFunction::MaximumInnerProduct => {
        let $similarity = MaximumInnerProductSimilarity;
        $body
      },
    }
  }};
}
