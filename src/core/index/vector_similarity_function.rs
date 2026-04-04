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
use strum_macros::{Display, EnumCount, FromRepr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, FromRepr, EnumCount, Display)]
#[repr(u8)]
pub enum VectorSimilarityFunction {
  Euclidean,
  DotProduct,
  Cosine,
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
}
/// Use Default for padding
impl Default for VectorSimilarityFunction {
  fn default() -> Self {
    VectorSimilarityFunction::Euclidean
  }
}
impl VectorSimilarityFunction {
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
