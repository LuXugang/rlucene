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
use std::borrow::Cow;

use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::document::field::Field;
use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::knn_vector_values::{KnnVectorValues, KnnVectorValuesType};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::knn_float_vector_query::KnnFloatVectorQuery;
use crate::core::search::query::Query;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::hnsw::hnsw_builder::HnswBuilder;
use crate::core::util::hnsw::hnsw_graph_builder;
use crate::core::util::hnsw::hnsw_graph_searcher;
use crate::test::core::util::hnsw::hnsw_graph_test_case::{
  CircularByteVectorValues, CircularFloatVectorValues, HnswGraphTestCase,
  create_random_float_vectors, random_vector,
};
use crate::test::core::util::hnsw::mock_vector_values::MockVectorValues;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use rand::prelude::StdRng;
use rand::{Rng, RngExt};

#[allow(dead_code)] // for quick search
pub struct TestHnswFloatVectorGraph;

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestHnswFloatVectorGraph, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestHnswFloatVectorGraph;
  f(&case, &mut random)
}

mod hnsw_graph_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::util::hnsw::hnsw_graph_test_case::HnswGraphTestCase;
  use crate::test::core::util::hnsw::test_hnsw_float_vector_graph::run_case;

  #[test]
  fn test_random_read_write_and_merge() -> Result<()> {
    run_case(|case, random| case.test_random_read_write_and_merge(random))
  }
  #[test]
  fn test_read_write() -> Result<()> {
    run_case(|case, random| case.test_read_write(random))
  }

  #[test]
  fn test_sorted_and_unsorted_indices_return_same_results() -> Result<()> {
    run_case(|case, random| case.test_sorted_and_unsorted_indices_return_same_results(random))
  }

  #[test]
  fn test_aknn_diverse() -> Result<()> {
    run_case(|case, random| case.test_aknn_diverse(random))
  }

  #[test]
  fn test_search_with_accept_ords() -> Result<()> {
    run_case(|case, random| case.test_search_with_accept_ords(random))
  }

  #[test]
  fn test_search_with_selective_accept_ords() -> Result<()> {
    run_case(|case, random| case.test_search_with_selective_accept_ords(random))
  }

  #[test]
  fn test_hnsw_graph_builder_initialization_from_graph_with_offset_zero() -> Result<()> {
    run_case(|case, random| {
      case.test_hnsw_graph_builder_initialization_from_graph_with_offset_zero(random)
    })
  }

  #[test]
  fn test_hnsw_graph_builder_initialization_from_graph_with_non_zero_offset() -> Result<()> {
    run_case(|case, random| {
      case.test_hnsw_graph_builder_initialization_from_graph_with_non_zero_offset(random)
    })
  }

  #[test]
  fn test_visited_limit() -> Result<()> {
    run_case(|case, random| case.test_visited_limit(random))
  }

  #[test]
  fn test_hnsw_graph_builder_invalid() -> Result<()> {
    run_case(|case, random| case.test_hnsw_graph_builder_invalid(random))
  }

  #[test]
  fn test_ram_usage_estimate() -> Result<()> {
    run_case(|case, random| case.test_ram_usage_estimate(random))
  }

  #[test]
  fn test_diversity() -> Result<()> {
    run_case(|case, random| case.test_diversity(random))
  }

  #[test]
  fn test_diversity_fallback() -> Result<()> {
    run_case(|case, random| case.test_diversity_fallback(random))
  }

  #[test]
  fn test_random() -> Result<()> {
    run_case(|case, random| case.test_random(random))
  }

  #[test]
  fn test_on_heap_hnsw_graph_search() -> Result<()> {
    run_case(|case, random| case.test_on_heap_hnsw_graph_search(random))
  }

  #[test]
  fn test_concurrent_merge_builder() -> Result<()> {
    run_case(|case, random| case.test_concurrent_merge_builder(random))
  }

  #[test]
  fn test_all_nodes_visited_in_single_level() -> Result<()> {
    run_case(|case, random| case.test_all_nodes_visited_in_single_level(random))
  }
}

impl HnswGraphTestCase<Vec<f32>> for TestHnswFloatVectorGraph {
  fn similarity_function<R>(&self, random: &mut R) -> VectorSimilarityFunction
  where
    R: Rng + ?Sized,
  {
    VectorSimilarityFunction::random(random)
  }

  fn get_vector_encoding(&self) -> VectorEncoding {
    VectorEncoding::FLOAT32(4)
  }

  fn knn_query(&self, field: &str, vector: Vec<f32>, k: usize) -> Result<Query> {
    Ok(KnnFloatVectorQuery::new(field, vector, k)?.into())
  }

  fn random_vector<R>(&self, random: &mut R, dim: usize) -> Vec<f32>
  where
    R: Rng + ?Sized,
  {
    random_vector(random, dim)
  }

  type KnnVectorValues = MockVectorValues;

  fn vector_values<R>(&self, size: usize, dimension: usize, random: &mut R) -> Self::KnnVectorValues
  where
    R: Rng + ?Sized,
  {
    let values = create_random_float_vectors(size, dimension, random);
    MockVectorValues::from_values(values, random.random())
  }

  fn vector_values_from_values<R>(
    &self,
    values: Vec<Vec<f32>>,
    random: &mut R,
  ) -> Self::KnnVectorValues
  where
    R: Rng + ?Sized,
  {
    MockVectorValues::from_values(values, random.random())
  }

  fn vector_values_from_reader<LR, R>(
    &self,
    reader: &LR,
    field_name: &str,
    random: &mut R,
  ) -> Result<Self::KnnVectorValues>
  where
    LR: LeafReader,
    R: Rng + ?Sized,
  {
    let vector_values = reader
      .get_float_vector_values(field_name)?
      .expect("float vector values should exist");
    let mut vectors = vec![Vec::new(); reader.max_doc()? as usize];
    for i in 0..vector_values.size() {
      let ord = vector_values.ord_to_doc(i)?;
      let value = vector_values.vector_value(i)?;
      let floats = value.as_ref().as_floats()?;
      vectors[ord] = ArrayUtil::copy_of_sub_array(floats, 0, vector_values.dimension());
    }
    Ok(MockVectorValues::from_values(vectors, random.random()))
  }

  fn vector_values_with_pregenerated<R>(
    &self,
    size: usize,
    dimension: usize,
    pregenerated_vector_values: Self::KnnVectorValues,
    pregenerated_offset: usize,
    random: &mut R,
  ) -> Self::KnnVectorValues
  where
    R: Rng + ?Sized,
  {
    let pregenerated_size = pregenerated_vector_values.size();
    let random_vectors = create_random_float_vectors(size - pregenerated_size, dimension, random);
    let mut vectors = vec![Vec::new(); size];
    let pregenerated_values = pregenerated_vector_values.values;

    vectors[..pregenerated_offset].clone_from_slice(&random_vectors[..pregenerated_offset]);

    for (current_ord, value) in pregenerated_values.into_iter().enumerate() {
      vectors[pregenerated_offset + current_ord] = value;
    }

    for (dst, value) in vectors[(pregenerated_offset + pregenerated_size)..]
      .iter_mut()
      .zip(random_vectors.into_iter().skip(pregenerated_offset))
    {
      *dst = value;
    }

    MockVectorValues::from_values(vectors, random.random())
  }

  fn knn_vector_field(
    &self,
    name: &str,
    vector: Vec<f32>,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Field> {
    let field_type =
      KnnFloatVectorField::create_field_type(vector.len() as i32, similarity_function)?;
    Ok(Field::new(name, VectorValueEnum::Float(vector), field_type))
  }

  type CircularKnnVectorValues = CircularFloatVectorValues;

  fn circular_vector_values(&self, n_doc: usize) -> Self::CircularKnnVectorValues {
    CircularFloatVectorValues::new(n_doc)
  }

  fn get_target_vector(&self) -> Vec<f32> {
    vec![1.0, 0.0]
  }

  fn vector_value<'a, K>(
    &self,
    vectors: &'a Self::KnnVectorValues,
    ord: usize,
  ) -> Result<Cow<'a, VectorValueEnum>> {
    vectors.vector_value(ord)
  }
}

// 测试未通过
fn test_search_with_skewed_accept_ords() -> Result<()> {
  let case = TestHnswFloatVectorGraph;
  let mut random = random();
  let n_doc = 1000;
  let similarity_function = VectorSimilarityFunction::Euclidean;
  let vectors = case.circular_vector_values(n_doc);
  let scorer_supplier = case.build_scorer_supplier(
    KnnVectorValuesType::<CircularByteVectorValues, CircularFloatVectorValues>::Float(
      vectors.clone(),
    ),
    &mut random,
  )?;
  let mut builder = hnsw_graph_builder::create(scorer_supplier, 16, 100, random.random::<u64>())?;
  let hnsw = builder.build(vectors.size())?;

  let mut accept_ords = FixedBitSet::new(n_doc);
  for i in 500..n_doc {
    accept_ords.set(i);
  }

  let mut scorer = case.flat_vector_scorer().get_random_vector_scorer_f32(
    similarity_function,
    vectors,
    case.get_target_vector(),
  )?;
  let mut nn = hnsw_graph_searcher::search_with_top_k(
    &mut scorer,
    10,
    hnsw,
    Some(&mut accept_ords),
    usize::MAX,
  )?;

  let nodes = nn.top_docs()?;
  assert_eq!(
    10,
    nodes.score_docs.len(),
    "Number of found results is not equal to [10]."
  );

  let mut sum = 0_i32;
  for node in &nodes.score_docs {
    assert!(
      accept_ords.get(node.doc as usize)?,
      "the results include a deleted document: {node}"
    );
    sum += node.doc;
  }

  assert!(sum < 5100, "sum(result docs)={sum}");
  Ok(())
}
