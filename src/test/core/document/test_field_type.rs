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

use crate::test_framework::core::util::lucene_test_case::random;
use std::hash::{DefaultHasher, Hash, Hasher};

use rand::Rng;
use rand::RngExt;
use strum::EnumCount;

use crate::core::document::field_type::FieldType;
use crate::core::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::point_values::{MAX_DIMENSIONS, MAX_INDEX_DIMENSIONS, MAX_NUM_BYTES};
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};

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
  ft.set_dimensions(1, BitUtil::INT_BYTES)?;
  let expected = "pointDimensionCount=1,pointIndexDimensionCount=1,pointNumBytes=4";
  let s = ft.to_string();
  assert_eq!(s, expected);
  Ok(())
}

#[test]
fn test_attribute_map_frozen() -> Result<()> {
  let mut ft = FieldType::new();
  ft.put_attribute("dummy", "d")?;
  ft.freeze();
  assert!(matches!(
    ft.put_attribute("dummy", "a"),
    Err(error) if error.is_illegal_state_error()
  ));
  Ok(())
}

#[test]
fn test_attribute_map_not_frozen() -> Result<()> {
  let mut ft = FieldType::new();
  ft.put_attribute("dummy", "d")?;
  ft.put_attribute("dummy", "a")?;
  let attributes = ft
    .get_attributes()
    .expect("put_attribute must create the attribute map");
  assert_eq!(attributes.len(), 1);
  assert_eq!(attributes.get("dummy").map(String::as_str), Some("a"));
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
  // Java discovers setters through reflection; call each setter explicitly here.
  ft.set_stored(random_value_bool(random))?;
  ft.set_tokenized(random_value_bool(random))?;
  ft.set_store_term_vectors(random_value_bool(random))?;
  ft.set_store_term_vector_offsets(random_value_bool(random))?;
  ft.set_store_term_vector_positions(random_value_bool(random))?;
  ft.set_store_term_vector_payloads(random_value_bool(random))?;
  ft.set_omit_norms(random_value_bool(random))?;
  ft.set_index_options(
    IndexOptions::from_repr(random.random_range(0..IndexOptions::COUNT) as u8).unwrap(),
  )?;
  // setDimensions is handled specially as values must be in bounds.
  ft.set_dimensions(
    random.random_range(1..=MAX_INDEX_DIMENSIONS),
    random.random_range(1..=MAX_NUM_BYTES),
  )?;
  let dim = random.random_range(1..=MAX_DIMENSIONS);
  let idx_dim = 1 + (dim - 1).min(random.random_range(0..MAX_INDEX_DIMENSIONS));
  let num_bytes = random.random_range(1..=MAX_NUM_BYTES);
  ft.set_dimensions_with_index(dim, idx_dim, num_bytes)?;
  ft.set_vector_attributes(
    random.random_range(1..=100),
    VectorEncoding::random(random),
    VectorSimilarityFunction::random(random),
  )?;
  ft.set_doc_values_type(
    DocValuesType::from_repr(random.random_range(0..DocValuesType::COUNT) as u8).unwrap(),
  )?;
  ft.set_doc_values_skip_index_type(if random_value_bool(random) {
    DocValuesSkipIndexType::Range
  } else {
    DocValuesSkipIndexType::None
  })?;
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
