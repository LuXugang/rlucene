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
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::quantization::scalar_quantizer::{
  ScalarQuantizedVectorSimilarity, ScalarQuantizer,
};
use crate::core::util::vector_util::VectorUtil;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::{Rng, RngExt};
use std::collections::HashSet;
use strum::EnumCount;

use super::test_scalar_quantizer::{
  TestSimpleFloatVectorValues, from_floats, random_float_array, random_floats,
};

#[allow(dead_code)] // for quick search
struct TestScalarQuantizedVectorSimilarity;

#[test]
fn test_non_zero_scores() -> Result<()> {
  let mut random = random();
  let quantized = [vec![0; 32], vec![0; 32]];
  for similarity_function_ord in 0..VectorSimilarityFunction::COUNT {
    let similarity_function =
      VectorSimilarityFunction::from_repr(similarity_function_ord as u8).unwrap();
    let mut multiplier = random.random_range(0.0..1.0);
    if random.random_bool(0.5) {
      multiplier = -multiplier;
    }
    for bits in [4, 7] {
      let quantized_similarity = ScalarQuantizedVectorSimilarity::from_vector_similarity(
        similarity_function,
        multiplier,
        bits,
      );
      let negative_offset_a =
        -(random.random_range(0.0..1.0) * (random.random_range(0..10) + 1) as f32);
      let negative_offset_b =
        -(random.random_range(0.0..1.0) * (random.random_range(0..10) + 1) as f32);
      let score = quantized_similarity.score(
        &quantized[0],
        negative_offset_a,
        &quantized[1],
        negative_offset_b,
      )?;
      assert!(score >= 0.0);
    }
  }
  Ok(())
}

#[test]
fn test_to_euclidean() -> Result<()> {
  let mut random = random();
  let dims = 128;
  let num_vecs = 100;

  let floats = random_floats(&mut random, num_vecs, dims);
  for confidence_interval in confidence_intervals(dims) {
    let error = ((100.0 - confidence_interval) * 0.01).max(0.01);
    let float_vector_values = from_floats(floats.clone());
    let scalar_quantizer =
      ScalarQuantizer::from_vectors(&float_vector_values, confidence_interval, num_vecs, 7)?;
    let mut quantized = vec![Vec::new(); floats.len()];
    let offsets = quantize_vectors(
      &scalar_quantizer,
      &floats,
      &mut quantized,
      VectorSimilarityFunction::Euclidean,
    );
    let query = floats[0][0..dims].to_vec();
    let quantized_similarity = ScalarQuantizedVectorSimilarity::from_vector_similarity(
      VectorSimilarityFunction::Euclidean,
      scalar_quantizer.get_constant_multiplier(),
      scalar_quantizer.get_bits(),
    );
    assert_quantized_scores(
      &floats,
      &quantized,
      &offsets,
      &query,
      error,
      VectorSimilarityFunction::Euclidean,
      &quantized_similarity,
      &scalar_quantizer,
    )?;
  }
  Ok(())
}

#[test]
fn test_to_cosine() -> Result<()> {
  let mut random = random();
  let dims = 128;
  let num_vecs = 100;

  let floats = random_floats(&mut random, num_vecs, dims);

  for confidence_interval in confidence_intervals(dims) {
    let error = ((100.0 - confidence_interval) * 0.01).max(0.01);
    let float_vector_values = from_floats_normalized(floats.clone(), None)?;
    let scalar_quantizer =
      ScalarQuantizer::from_vectors(&float_vector_values, confidence_interval, num_vecs, 7)?;
    let mut quantized = vec![Vec::new(); floats.len()];
    let offsets = quantize_vectors_normalized(
      &scalar_quantizer,
      &floats,
      &mut quantized,
      VectorSimilarityFunction::Cosine,
    )?;
    let mut query = floats[0][0..dims].to_vec();
    VectorUtil::l2normalize(&mut query)?;
    let quantized_similarity = ScalarQuantizedVectorSimilarity::from_vector_similarity(
      VectorSimilarityFunction::Cosine,
      scalar_quantizer.get_constant_multiplier(),
      scalar_quantizer.get_bits(),
    );
    assert_quantized_scores(
      &floats,
      &quantized,
      &offsets,
      &query,
      error,
      VectorSimilarityFunction::Cosine,
      &quantized_similarity,
      &scalar_quantizer,
    )?;
  }
  Ok(())
}

#[test]
fn test_to_dot_product() -> Result<()> {
  let mut random = random();
  let dims = 128;
  let num_vecs = 100;

  let mut floats = random_floats(&mut random, num_vecs, dims);
  for fs in &mut floats {
    VectorUtil::l2normalize(fs)?;
  }
  for confidence_interval in confidence_intervals(dims) {
    let error = ((100.0 - confidence_interval) * 0.01).max(0.01);
    let float_vector_values = from_floats(floats.clone());
    let scalar_quantizer =
      ScalarQuantizer::from_vectors(&float_vector_values, confidence_interval, num_vecs, 7)?;
    let mut quantized = vec![Vec::new(); floats.len()];
    let offsets = quantize_vectors(
      &scalar_quantizer,
      &floats,
      &mut quantized,
      VectorSimilarityFunction::DotProduct,
    );
    let mut query = random_float_array(&mut random, dims);
    VectorUtil::l2normalize(&mut query)?;
    let quantized_similarity = ScalarQuantizedVectorSimilarity::from_vector_similarity(
      VectorSimilarityFunction::DotProduct,
      scalar_quantizer.get_constant_multiplier(),
      scalar_quantizer.get_bits(),
    );
    assert_quantized_scores(
      &floats,
      &quantized,
      &offsets,
      &query,
      error,
      VectorSimilarityFunction::DotProduct,
      &quantized_similarity,
      &scalar_quantizer,
    )?;
  }
  Ok(())
}

#[test]
fn test_to_max_inner_product() -> Result<()> {
  let mut random = random();
  let dims = 128;
  let num_vecs = 100;

  let floats = random_floats(&mut random, num_vecs, dims);
  for confidence_interval in confidence_intervals(dims) {
    let error = ((100.0 - confidence_interval) * 0.5).max(0.5);
    let float_vector_values = from_floats(floats.clone());
    let scalar_quantizer =
      ScalarQuantizer::from_vectors(&float_vector_values, confidence_interval, num_vecs, 7)?;
    let mut quantized = vec![Vec::new(); floats.len()];
    let offsets = quantize_vectors(
      &scalar_quantizer,
      &floats,
      &mut quantized,
      VectorSimilarityFunction::MaximumInnerProduct,
    );
    let query = random_float_array(&mut random, dims);
    let quantized_similarity = ScalarQuantizedVectorSimilarity::from_vector_similarity(
      VectorSimilarityFunction::MaximumInnerProduct,
      scalar_quantizer.get_constant_multiplier(),
      scalar_quantizer.get_bits(),
    );
    assert_quantized_scores(
      &floats,
      &quantized,
      &offsets,
      &query,
      error,
      VectorSimilarityFunction::MaximumInnerProduct,
      &quantized_similarity,
      &scalar_quantizer,
    )?;
  }
  Ok(())
}
#[allow(clippy::too_many_arguments)]
fn assert_quantized_scores(
  floats: &[Vec<f32>],
  quantized: &[Vec<u8>],
  stored_offsets: &[f32],
  query: &[f32],
  error: f32,
  similarity_function: VectorSimilarityFunction,
  quantized_similarity: &ScalarQuantizedVectorSimilarity,
  scalar_quantizer: &ScalarQuantizer,
) -> Result<()> {
  for i in 0..floats.len() {
    let stored_offset = stored_offsets[i];
    let mut quantized_query = vec![0; query.len()];
    let query_offset = scalar_quantizer.quantize(query, &mut quantized_query, similarity_function);
    let original = similarity_function.compare_f32(query, &floats[i])?;
    let quantized_score =
      quantized_similarity.score(&quantized_query, query_offset, &quantized[i], stored_offset)?;
    assert_approx_eq(
      original,
      quantized_score,
      error,
      "Not within acceptable error",
    );
  }
  Ok(())
}
fn quantize_vectors(
  scalar_quantizer: &ScalarQuantizer,
  floats: &[Vec<f32>],
  quantized: &mut [Vec<u8>],
  similarity_function: VectorSimilarityFunction,
) -> Vec<f32> {
  let mut offsets = vec![0.0; floats.len()];
  for (i, v) in floats.iter().enumerate() {
    quantized[i] = vec![0; v.len()];
    offsets[i] = scalar_quantizer.quantize(v, &mut quantized[i], similarity_function);
  }
  offsets
}

fn quantize_vectors_normalized(
  scalar_quantizer: &ScalarQuantizer,
  floats: &[Vec<f32>],
  quantized: &mut [Vec<u8>],
  similarity_function: VectorSimilarityFunction,
) -> Result<Vec<f32>> {
  let mut offsets = vec![0.0; floats.len()];
  for (i, f) in floats.iter().enumerate() {
    let mut v = f.clone();
    VectorUtil::l2normalize(&mut v)?;
    quantized[i] = vec![0; v.len()];
    offsets[i] = scalar_quantizer.quantize(&v, &mut quantized[i], similarity_function);
  }
  Ok(offsets)
}

fn from_floats_normalized(
  floats: Vec<Vec<f32>>,
  deleted_vectors: Option<HashSet<usize>>,
) -> Result<TestSimpleFloatVectorValues> {
  let mut normalized = floats;
  for v in &mut normalized {
    VectorUtil::l2normalize(v)?;
  }
  Ok(TestSimpleFloatVectorValues::new(
    normalized,
    deleted_vectors,
  ))
}

fn confidence_intervals(dims: usize) -> [f32; 5] {
  [0.9, 0.95, 0.99, 1.0 - 1.0 / (dims as f32 + 1.0), 1.0]
}

fn assert_approx_eq(expected: f32, actual: f32, delta: f32, message: &str) {
  assert!(
    (expected - actual).abs() <= delta,
    "{message} [{delta}]: expected {expected}, got {actual}"
  );
}
