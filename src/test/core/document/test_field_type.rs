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

use std::hash::{DefaultHasher, Hash, Hasher};

use rand::Rng;
use rand::RngExt;

use crate::core::document::field_type::FieldType;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::point_values::{MAX_DIMENSIONS, MAX_INDEX_DIMENSIONS, MAX_NUM_BYTES};
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;

#[allow(dead_code)] // for quick search
struct TestFieldType;
#[test]
fn test_equals() -> Result<()> {
  let ft = FieldType::new();
  assert_eq!(ft, ft);
  assert_ne!(Some(ft.clone()), None);

  let ft2 = FieldType::new();
  assert_eq!(ft, ft2);
  let mut hasher1 = DefaultHasher::new();
  ft.hash(&mut hasher1);
  let mut hasher2 = DefaultHasher::new();
  ft2.hash(&mut hasher2);
  assert_eq!(hasher1.finish(), hasher2.finish());

  let mut ft3 = FieldType::new();
  ft3.set_index_options(IndexOptions::DocsAndFreqs)?;
  assert_ne!(ft3, ft);

  let mut ft4 = FieldType::new();
  ft4.set_doc_values_type(DocValuesType::Binary)?;
  assert_ne!(ft4, ft);

  let mut ft5 = FieldType::new();
  ft5.set_index_options(IndexOptions::DocsAndFreqsAndPositions)?;
  assert_ne!(ft5, ft);

  let mut ft6 = FieldType::new();
  ft6.set_stored(true)?;
  assert_ne!(ft6, ft);

  let mut ft7 = FieldType::new();
  ft7.set_omit_norms(true)?;
  assert_ne!(ft7, ft);

  let mut ft10 = FieldType::new();
  ft10.set_store_term_vectors(true)?;
  assert_ne!(ft10, ft);

  let mut ft11 = FieldType::new();
  ft11.set_dimensions(1, 4)?;
  assert_ne!(ft11, ft);
  Ok(())
}

#[test]
fn test_points_to_string() -> Result<()> {
  let mut ft = FieldType::new();
  ft.set_dimensions(1, MAX_NUM_BYTES)?;
  let expected = format!(
    "pointDimensionCount=1,pointIndexDimensionCount=1,pointNumBytes={}",
    MAX_NUM_BYTES
  );
  let s = ft.to_string();
  assert_eq!(s, expected);
  Ok(())
}

#[test]
fn test_attribute_map_frozen() -> Result<()> {
  // FieldType#put_attribute no need to Implement, so as this test
  Ok(())
}

#[test]
fn test_attribute_map_not_frozen() -> Result<()> {
  // FieldType#put_attribute no need to Implement, so as this test
  Ok(())
}

fn random_value_bool<R>(random: &mut R) -> bool
where
  R: Rng + ?Sized,
{
  random.random_bool(0.5)
}

// Generates a random FieldType.
fn random_field_type<R>(random: &mut R) -> Result<FieldType>
where
  R: Rng + ?Sized,
{
  let mut ft = FieldType::new();
  let max_idx_dims = MAX_INDEX_DIMENSIONS;
  let max_dims = MAX_DIMENSIONS;
  let max_bytes = MAX_NUM_BYTES;
  let dim = random.random_range(1..=max_dims);
  let idx_dim = random.random_range(1..=max_idx_dims.min(dim));
  let num_bytes = random.random_range(1..=max_bytes);
  ft.set_dimensions_with_index(dim, idx_dim, num_bytes)?;
  ft.set_stored(random_value_bool(random))?;
  ft.set_tokenized(random_value_bool(random))?;
  ft.set_store_term_vectors(random_value_bool(random))?;
  ft.set_store_term_vector_offsets(random_value_bool(random))?;
  ft.set_store_term_vector_positions(random_value_bool(random))?;
  ft.set_store_term_vector_payloads(random_value_bool(random))?;
  ft.set_omit_norms(random_value_bool(random))?;
  let options = if random_value_bool(random) {
    IndexOptions::DocsAndFreqs
  } else {
    IndexOptions::DocsAndFreqsAndPositions
  };
  ft.set_index_options(options)?;
  let dv = if random_value_bool(random) {
    DocValuesType::Binary
  } else {
    DocValuesType::None
  };
  ft.set_doc_values_type(dv)?;

  if random_value_bool(random) {
    let vec_dim = random.random_range(1..=4);
    ft.set_vector_attributes(
      vec_dim,
      VectorEncoding::FLOAT32(4),
      VectorSimilarityFunction::Euclidean,
    )?;
  }
  // ft.put_attribute("random".to_string(), "value".to_string())?;
  Ok(ft)
}

#[test]
fn test_copy_constructor() -> Result<()> {
  let mut random = random();
  let iters = 10;
  for _ in 0..iters {
    let ft = random_field_type(&mut random)?;
    let ft2 = FieldType::from_ref(&ft)?;
    assert_eq!(ft, ft2);
  }
  Ok(())
}
