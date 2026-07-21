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
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::codecs::lucene99::lucene99_scalar_quantized_vectors_writer::{
  FloatVectorWrapper, build_scalar_quantizer,
};
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::vector_util::VectorUtil;

#[allow(dead_code)] // for quick search
struct TestLucene99ScalarQuantizedVectorsWriter;

#[test]
fn test_build_scalar_quantizer_cosine() -> Result<()> {
  assert_scalar_quantizer(
    [0.3234983, 0.6236096],
    Some(0.9),
    7,
    VectorSimilarityFunction::Cosine,
  )?;
  assert_scalar_quantizer(
    [0.28759837, 0.62449116],
    Some(0.0),
    7,
    VectorSimilarityFunction::Cosine,
  )?;
  assert_scalar_quantizer(
    [0.3234983, 0.6236096],
    Some(0.9),
    4,
    VectorSimilarityFunction::Cosine,
  )?;
  assert_scalar_quantizer(
    [0.37247902, 0.58848244],
    Some(0.0),
    4,
    VectorSimilarityFunction::Cosine,
  )
}

#[test]
fn test_build_scalar_quantizer_dot_product() -> Result<()> {
  assert_scalar_quantizer(
    [0.3234983, 0.6236096],
    Some(0.9),
    7,
    VectorSimilarityFunction::DotProduct,
  )?;
  assert_scalar_quantizer(
    [0.28759837, 0.62449116],
    Some(0.0),
    7,
    VectorSimilarityFunction::DotProduct,
  )?;
  assert_scalar_quantizer(
    [0.3234983, 0.6236096],
    Some(0.9),
    4,
    VectorSimilarityFunction::DotProduct,
  )?;
  assert_scalar_quantizer(
    [0.37247902, 0.58848244],
    Some(0.0),
    4,
    VectorSimilarityFunction::DotProduct,
  )
}

#[test]
fn test_build_scalar_quantizer_mip() -> Result<()> {
  assert_scalar_quantizer(
    [2.0, 20.0],
    Some(0.9),
    7,
    VectorSimilarityFunction::MaximumInnerProduct,
  )?;
  assert_scalar_quantizer(
    [2.4375, 19.0625],
    Some(0.0),
    7,
    VectorSimilarityFunction::MaximumInnerProduct,
  )?;
  assert_scalar_quantizer(
    [2.0, 20.0],
    Some(0.9),
    4,
    VectorSimilarityFunction::MaximumInnerProduct,
  )?;
  assert_scalar_quantizer(
    [2.6875, 19.0625],
    Some(0.0),
    4,
    VectorSimilarityFunction::MaximumInnerProduct,
  )
}

#[test]
fn test_build_scalar_quantizer_euclidean() -> Result<()> {
  assert_scalar_quantizer(
    [2.0, 20.0],
    Some(0.9),
    7,
    VectorSimilarityFunction::Euclidean,
  )?;
  assert_scalar_quantizer(
    [2.125, 19.375],
    Some(0.0),
    7,
    VectorSimilarityFunction::Euclidean,
  )?;
  assert_scalar_quantizer(
    [2.0, 20.0],
    Some(0.9),
    4,
    VectorSimilarityFunction::Euclidean,
  )?;
  assert_scalar_quantizer(
    [2.1875, 19.0625],
    Some(0.0),
    4,
    VectorSimilarityFunction::Euclidean,
  )
}

fn assert_scalar_quantizer(
  expected_quantiles: [f32; 2],
  confidence_interval: Option<f32>,
  bits: u8,
  vector_similarity_function: VectorSimilarityFunction,
) -> Result<()> {
  let mut vectors = Vec::with_capacity(30);
  for i in 0..30 {
    let mut vector = vec![i as f32, (i + 1) as f32, (i + 2) as f32, (i + 3) as f32];
    if vector_similarity_function == VectorSimilarityFunction::DotProduct {
      VectorUtil::l2normalize(&mut vector)?;
    }
    vectors.push(VectorValueEnum::Float(vector));
  }

  let vector_values = FloatVectorWrapper::new(&vectors);
  let scalar_quantizer = build_scalar_quantizer(
    vector_values,
    30,
    vector_similarity_function,
    confidence_interval,
    bits,
  )?;
  assert!(
    (expected_quantiles[0] - scalar_quantizer.get_lower_quantile()).abs() <= 0.0001,
    "expected lower quantile {}, got {}",
    expected_quantiles[0],
    scalar_quantizer.get_lower_quantile()
  );
  assert!(
    (expected_quantiles[1] - scalar_quantizer.get_upper_quantile()).abs() <= 0.0001,
    "expected upper quantile {}, got {}",
    expected_quantiles[1],
    scalar_quantizer.get_upper_quantile()
  );
  Ok(())
}
