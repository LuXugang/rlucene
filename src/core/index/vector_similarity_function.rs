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
      VectorSimilarityFunction::Cosine => 1,
      VectorSimilarityFunction::MaximumInnerProduct => 1,
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
  pub fn compare_f32(&self, _v1: &[f32], _v2: &[f32]) -> f32 {
    todo!()
  }

  pub fn compare_u8(&self, _v1: &[u8], _v2: &[u8]) -> f32 {
    todo!()
  }
}
