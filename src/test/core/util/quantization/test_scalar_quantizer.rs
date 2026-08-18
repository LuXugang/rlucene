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
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::index_reader::Identity;
use crate::core::index::knn_vector_values::{DocIndexIterator, KnnVectorValues};
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::util::HasIdentity;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::quantization::scalar_quantizer::{
  SCRATCH_SIZE, ScalarQuantizer, get_upper_and_lower_quantile,
};
use crate::core::util::vector_util::VectorUtil;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::prelude::StdRng;
use rand::{Rng, RngExt};
use std::borrow::Cow;
use std::collections::HashSet;
use strum::EnumCount;

#[allow(dead_code)] // for quick search
struct TestScalarQuantizer;

#[test]
fn test_tiny_vectors() -> Result<()> {
  let mut random = random();
  for function_ord in 0..VectorSimilarityFunction::COUNT {
    let function = VectorSimilarityFunction::from_repr(function_ord as u8).unwrap();
    let dims = random.random_range(0..9) + 1;
    let num_vecs = random.random_range(0..9) + 10;
    let mut floats = random_floats(&mut random, num_vecs, dims);
    if function == VectorSimilarityFunction::Cosine {
      for v in &mut floats {
        VectorUtil::l2normalize(v)?;
      }
    }
    for bits in [4, 7] {
      let float_vector_values = from_floats(floats.clone());
      let actual_function = if function == VectorSimilarityFunction::Cosine {
        VectorSimilarityFunction::DotProduct
      } else {
        function
      };
      if random.random_bool(0.5) {
        ScalarQuantizer::from_vectors(&float_vector_values, 0.9, num_vecs, bits)?;
      } else {
        ScalarQuantizer::from_vectors_auto_interval(
          &float_vector_values,
          actual_function,
          num_vecs,
          bits,
        )?;
      }
      // We simply assert that we created a scalar quantizer and didn't trip any assertions
      // the quality of the quantization might be poor, but this is expected as sampling size is
      // tiny
    }
  }
  Ok(())
}

#[test]
fn test_nan_and_inf_value_failure() {
  let mut random = random();
  for function_ord in 0..VectorSimilarityFunction::COUNT {
    let function = VectorSimilarityFunction::from_repr(function_ord as u8).unwrap();
    let dims = random.random_range(0..9) + 1;
    let num_vecs = random.random_range(0..9) + 10;
    let mut floats = vec![vec![0.0; dims]; num_vecs];
    for v in &mut floats {
      for value in v {
        *value = if random.random_bool(0.5) {
          f32::NAN
        } else {
          f32::INFINITY
        };
      }
    }
    for bits in [4, 7] {
      let float_vector_values = from_floats(floats.clone());
      assert!(matches!(
        ScalarQuantizer::from_vectors(&float_vector_values, 0.9, num_vecs, bits),
        Err(LuceneError::IllegalState(_))
      ));
      let actual_function = if function == VectorSimilarityFunction::Cosine {
        VectorSimilarityFunction::DotProduct
      } else {
        function
      };
      assert!(matches!(
        ScalarQuantizer::from_vectors_auto_interval(
          &float_vector_values,
          actual_function,
          num_vecs,
          bits,
        ),
        Err(LuceneError::IllegalState(_))
      ));
    }
  }
}

#[test]
fn test_quantize_and_de_quantize_7_bit() -> Result<()> {
  let mut random = random();
  let dims = 128;
  let num_vecs = 100;
  let similarity_function = VectorSimilarityFunction::DotProduct;

  let floats = random_floats(&mut random, num_vecs, dims);
  let float_vector_values = from_floats(floats.clone());
  let scalar_quantizer = ScalarQuantizer::from_vectors(&float_vector_values, 1.0, num_vecs, 7)?;
  let mut dequantized = vec![0.0; dims];
  let mut quantized = vec![0; dims];
  let mut requantized = vec![0; dims];
  let mut max_dim_value = i32::MIN;
  let mut min_dim_value = i32::MAX;
  for vector in floats.iter().take(num_vecs) {
    scalar_quantizer.quantize(vector, &mut quantized, similarity_function);
    scalar_quantizer.de_quantize(&quantized, &mut dequantized);
    scalar_quantizer.quantize(&dequantized, &mut requantized, similarity_function);
    for j in 0..dims {
      let value = quantized[j] as i8 as i32;
      if value > max_dim_value {
        max_dim_value = value;
      }
      if value < min_dim_value {
        min_dim_value = value;
      }
      assert_approx_eq(dequantized[j], vector[j], 0.02);
      assert_eq!(quantized[j], requantized[j]);
    }
  }
  // int7 should always quantize to 0-127
  assert!(min_dim_value >= 0);
  assert!(max_dim_value <= 127);
  Ok(())
}

#[test]
fn test_quantiles() {
  let mut random = random();
  let mut percs = (0..1000).map(|i| i as f32).collect::<Vec<_>>();
  shuffle_array(&mut random, &mut percs);
  let upper_and_lower = get_upper_and_lower_quantile(&mut percs, 0.9);
  assert_approx_eq(50.0, upper_and_lower[0], 1e-7);
  assert_approx_eq(949.0, upper_and_lower[1], 1e-7);
  shuffle_array(&mut random, &mut percs);
  let upper_and_lower = get_upper_and_lower_quantile(&mut percs, 0.95);
  assert_approx_eq(25.0, upper_and_lower[0], 1e-7);
  assert_approx_eq(974.0, upper_and_lower[1], 1e-7);
  shuffle_array(&mut random, &mut percs);
  let upper_and_lower = get_upper_and_lower_quantile(&mut percs, 0.99);
  assert_approx_eq(5.0, upper_and_lower[0], 1e-7);
  assert_approx_eq(994.0, upper_and_lower[1], 1e-7);
}

#[test]
fn test_edge_case() {
  let mut arr = [1.0, 1.0, 1.0, 1.0, 1.0];
  let upper_and_lower = get_upper_and_lower_quantile(&mut arr, 0.9);
  assert_approx_eq(1.0, upper_and_lower[0], 1e-7);
  assert_approx_eq(1.0, upper_and_lower[1], 1e-7);
}

#[test]
fn test_scalar_with_sampling() -> Result<()> {
  let mut random = random();
  let num_vecs = random.random_range(0..128) + 5;
  let dims = 64;
  let floats = random_floats(&mut random, num_vecs, dims);
  // Should not throw
  {
    let num_deleted = random.random_range(0..num_vecs - 1) + 1;
    let float_vector_values =
      from_floats_with_random_deletions(&mut random, floats.clone(), num_deleted);
    ScalarQuantizer::from_vectors_with_sample_size(
      &float_vector_values,
      0.99,
      float_vector_values.num_live_vectors,
      7,
      (float_vector_values.num_live_vectors - 1).max(SCRATCH_SIZE + 1),
    )?;
  }
  {
    let num_deleted = random.random_range(0..num_vecs - 1) + 1;
    let float_vector_values =
      from_floats_with_random_deletions(&mut random, floats.clone(), num_deleted);
    ScalarQuantizer::from_vectors_with_sample_size(
      &float_vector_values,
      0.99,
      float_vector_values.num_live_vectors,
      7,
      (float_vector_values.num_live_vectors - 1).max(SCRATCH_SIZE + 1),
    )?;
  }
  {
    let num_deleted = random.random_range(0..num_vecs - 1) + 1;
    let float_vector_values =
      from_floats_with_random_deletions(&mut random, floats.clone(), num_deleted);
    ScalarQuantizer::from_vectors_with_sample_size(
      &float_vector_values,
      0.99,
      float_vector_values.num_live_vectors,
      7,
      (float_vector_values.num_live_vectors - 1).max(SCRATCH_SIZE + 1),
    )?;
  }
  {
    let num_deleted = random.random_range(0..num_vecs - 1) + 1;
    let float_vector_values = from_floats_with_random_deletions(&mut random, floats, num_deleted);
    ScalarQuantizer::from_vectors_with_sample_size(
      &float_vector_values,
      0.99,
      float_vector_values.num_live_vectors,
      7,
      (random.random_range(0..float_vector_values.floats.len() - 1) + 1).max(SCRATCH_SIZE + 1),
    )?;
  }
  Ok(())
}

#[test]
fn test_from_vectors_auto_interval_4_bit() -> Result<()> {
  let mut random = random();
  let dims = 128;
  let num_vecs = 100;
  let similarity_function = VectorSimilarityFunction::DotProduct;

  let mut floats = random_floats(&mut random, num_vecs, dims);
  for v in &mut floats {
    VectorUtil::l2normalize(v)?;
  }
  let float_vector_values = from_floats(floats.clone());
  let scalar_quantizer = ScalarQuantizer::from_vectors_auto_interval(
    &float_vector_values,
    similarity_function,
    num_vecs,
    4,
  )?;
  let mut dequantized = vec![0.0; dims];
  let mut quantized = vec![0; dims];
  let mut requantized = vec![0; dims];
  let mut max_dim_value = i32::MIN;
  let mut min_dim_value = i32::MAX;
  for vector in floats.iter().take(num_vecs) {
    scalar_quantizer.quantize(vector, &mut quantized, similarity_function);
    scalar_quantizer.de_quantize(&quantized, &mut dequantized);
    scalar_quantizer.quantize(&dequantized, &mut requantized, similarity_function);
    for j in 0..dims {
      let value = quantized[j] as i8 as i32;
      if value > max_dim_value {
        max_dim_value = value;
      }
      if value < min_dim_value {
        min_dim_value = value;
      }
      assert_approx_eq(dequantized[j], vector[j], 0.2);
      assert_eq!(quantized[j], requantized[j]);
    }
  }
  // int4 should always quantize to 0-15
  assert!(min_dim_value >= 0);
  assert!(max_dim_value <= 15);
  Ok(())
}

fn shuffle_array(random: &mut StdRng, ar: &mut [f32]) {
  for i in (1..ar.len()).rev() {
    let index = random.random_range(0..i + 1);
    ar.swap(index, i);
  }
}

pub(crate) fn random_float_array(random: &mut StdRng, dims: usize) -> Vec<f32> {
  let mut arr = vec![0.0; dims];
  for value in &mut arr {
    *value = random.random_range(-1.0..1.0);
  }
  arr
}

pub(crate) fn random_floats(random: &mut StdRng, num: usize, dims: usize) -> Vec<Vec<f32>> {
  let mut floats = vec![Vec::new(); num];
  for vector in &mut floats {
    *vector = random_float_array(random, dims);
  }
  floats
}

pub(crate) fn from_floats(floats: Vec<Vec<f32>>) -> TestSimpleFloatVectorValues {
  TestSimpleFloatVectorValues::new(floats, None)
}

fn from_floats_with_random_deletions(
  random: &mut StdRng,
  floats: Vec<Vec<f32>>,
  num_deleted: usize,
) -> TestSimpleFloatVectorValues {
  let mut deleted_vectors = HashSet::new();
  for _ in 0..num_deleted {
    deleted_vectors.insert(random.random_range(0..floats.len()));
  }
  TestSimpleFloatVectorValues::new(floats, Some(deleted_vectors))
}

#[derive(Clone)]
pub(crate) struct TestSimpleFloatVectorValues {
  pub(crate) floats: Vec<Vec<f32>>,
  pub(crate) deleted_vectors: Option<HashSet<usize>>,
  pub(crate) ord_to_doc: Vec<usize>,
  pub(crate) num_live_vectors: usize,
}

impl TestSimpleFloatVectorValues {
  pub(crate) fn new(values: Vec<Vec<f32>>, deleted_vectors: Option<HashSet<usize>>) -> Self {
    let num_live_vectors = deleted_vectors
      .as_ref()
      .map_or(values.len(), |deleted_vectors| {
        values.len() - deleted_vectors.len()
      });
    let mut ord_to_doc = vec![0; num_live_vectors];
    if let Some(deleted_vectors) = &deleted_vectors {
      let mut ord = 0;
      for doc in 0..values.len() {
        if !deleted_vectors.contains(&doc) {
          ord_to_doc[ord] = doc;
          ord += 1;
        }
      }
    } else {
      for (i, doc) in ord_to_doc.iter_mut().enumerate() {
        *doc = i;
      }
    }
    Self {
      floats: values,
      deleted_vectors,
      ord_to_doc,
      num_live_vectors,
    }
  }
}

impl KnnVectorValues for TestSimpleFloatVectorValues {
  fn dimension(&self) -> usize {
    self.floats[0].len()
  }

  fn size(&self) -> usize {
    self.floats.len()
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    Ok(self.ord_to_doc[ord])
  }

  type KnnVectorValues = Self;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    Ok(self.clone())
  }

  fn get_encoding(&self) -> VectorEncoding {
    VectorEncoding::FLOAT32(4)
  }

  type Bits<'a, B>
    = TestSimpleBits
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, _accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    None
  }

  type DocIndexIterator = TestSimpleDocIndexIterator;

  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    Ok(TestSimpleDocIndexIterator::new(
      self.floats.len(),
      self.deleted_vectors.clone(),
    ))
  }
}

impl FloatVectorValues for TestSimpleFloatVectorValues {
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    Ok(Cow::Owned(VectorValueEnum::Float(
      self.floats[self.ord_to_doc(ord)?].clone(),
    )))
  }

  type FloatVectorValues = Self;

  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    Ok(Some(self.clone()))
  }

  type VectorScorer = DummyVectorScorer;

  fn scorer(&self, _target: Vec<f32>) -> Result<Option<Self::VectorScorer>> {
    Err(LuceneError::unsupported_operation(""))
  }
}

pub(crate) struct TestSimpleDocIndexIterator {
  deleted_vectors: Option<HashSet<usize>>,
  ord: i32,
  doc: i32,
  len: usize,
}

impl TestSimpleDocIndexIterator {
  fn new(len: usize, deleted_vectors: Option<HashSet<usize>>) -> Self {
    Self {
      deleted_vectors,
      ord: -1,
      doc: -1,
      len,
    }
  }
}

impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for TestSimpleDocIndexIterator
{
}
impl DocIdSetIterator for TestSimpleDocIndexIterator {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    while self.doc < self.len as i32 - 1 {
      self.doc += 1;
      if self
        .deleted_vectors
        .as_ref()
        .is_none_or(|deleted_vectors| !deleted_vectors.contains(&(self.doc as usize)))
      {
        self.ord += 1;
        return Ok(self.doc);
      }
    }
    self.doc = NO_MORE_DOCS;
    Ok(self.doc)
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn cost(&self) -> Result<i64> {
    Ok(
      self
        .deleted_vectors
        .as_ref()
        .map_or(self.len, |deleted_vectors| self.len - deleted_vectors.len()) as i64,
    )
  }
}

impl DocIndexIterator for TestSimpleDocIndexIterator {
  fn index(&self) -> Result<i32> {
    Ok(self.ord)
  }
}

pub(crate) struct TestSimpleBits {
  id: Identity,
}

impl HasIdentity for TestSimpleBits {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl Bits for TestSimpleBits {
  fn get(&self, _index: usize) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn length(&self) -> usize {
    0
  }
}

fn assert_approx_eq(expected: f32, actual: f32, delta: f32) {
  assert!(
    (expected - actual).abs() <= delta,
    "expected {expected}, got {actual}, delta {delta}"
  );
}
