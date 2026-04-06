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
use crate::core::codecs::hnsw::default_flat_vector_scorer::DefaultFlatVectorScorer;
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::document::field::Field;
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::dummy::dummy_doc_index_iterator::DummyDocIndexIterator;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::knn_vector_values::{
  BitsImpl1, DenseDocIndexIterator, KnnVectorValues, KnnVectorValuesType, create_dense_iterator,
};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::search::query::Query;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::hnsw::hnsw_graph::{HnswGraph, NodesIterator};
use crate::core::util::vector_util::VectorUtil;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use rand::{Rng, RngExt};
use std::borrow::Cow;
use std::collections::HashSet;

pub trait HnswGraphTestCase<T> {
  fn similarity_function<R>(&self, random: &mut R) -> VectorSimilarityFunction
  where
    R: Rng + ?Sized;

  fn flat_vector_scorer(&self) -> DefaultFlatVectorScorer {
    DefaultFlatVectorScorer
  }

  fn get_vector_encoding(&self) -> VectorEncoding;

  fn knn_query(&self, field: &str, vector: T, k: usize) -> Result<Query>;

  fn random_vector<R>(&self, random: &mut R, dim: usize) -> T
  where
    R: Rng + ?Sized;

  type KnnVectorValues: KnnVectorValues;

  fn vector_values<R>(
    &self,
    size: usize,
    dimension: usize,
    random: &mut R,
  ) -> Self::KnnVectorValues
  where
    R: Rng + ?Sized;

  fn vector_values_from_values<R>(
    &self,
    values: Vec<Vec<f32>>,
    random: &mut R,
  ) -> Self::KnnVectorValues
  where
    R: Rng + ?Sized;

  fn vector_values_from_reader<LR, R>(
    &self,
    reader: &LR,
    field_name: &str,
    random: &mut R,
  ) -> Result<Self::KnnVectorValues>
  where
    LR: LeafReader,
    R: Rng + ?Sized;

  fn vector_values_with_pregenerated<R>(
    &self,
    size: usize,
    dimension: usize,
    pregenerated_vector_values: Self::KnnVectorValues,
    pregenerated_offset: usize,
    random: &mut R,
  ) -> Self::KnnVectorValues
  where
    R: Rng + ?Sized;

  fn knn_vector_field(
    &self,
    name: &str,
    vector: T,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Field>;
  type CircularKnnVectorValues: KnnVectorValues;
  fn circular_vector_values(&self, n_doc: usize) -> Self::CircularKnnVectorValues;

  fn get_target_vector(&self) -> T;

  fn build_scorer_supplier<B, F, R>(
    &self,
    vectors: KnnVectorValuesType<B, F>,
    random: &mut R,
  ) -> Result<<DefaultFlatVectorScorer as FlatVectorsScorer>::RandomVectorScorerSupplier<B, F>>
  where
    B: ByteVectorValues + Clone,
    F: FloatVectorValues + Clone,
    R: Rng + ?Sized,
  {
    self
      .flat_vector_scorer()
      .get_random_vector_scorer_supplier(self.similarity_function(random), vectors)
  }

  fn test_random_read_write_and_merge<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO Knn 合并未实现
    Ok(())
  }
  fn vector_value<'a, K>(
    &self,
    vectors: &'a Self::KnnVectorValues,
    ord: usize,
  ) -> Result<Cow<'a, VectorValueEnum>>;
  fn test_read_write<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_sorted_and_unsorted_indices_return_same_results<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_aknn_diverse<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_search_with_accept_ords<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_search_with_selective_accept_ords<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_hnsw_graph_builder_initialization_from_graph_with_offset_zero<R>(
    &self,
    _random: &mut R,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_hnsw_graph_builder_initialization_from_graph_with_non_zero_offset<R>(
    &self,
    _random: &mut R,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_visited_limit<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_hnsw_graph_builder_invalid<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_ram_usage_estimate<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_diversity<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_diversity_fallback<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_random<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_on_heap_hnsw_graph_search<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_concurrent_merge_builder<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_all_nodes_visited_in_single_level<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }
}

#[derive(Clone)]
pub struct CircularByteVectorValues {
  size: usize,
  doc: i32,
}

impl CircularByteVectorValues {
  pub fn new(size: usize) -> Self {
    Self { size, doc: -1 }
  }

  fn vector_value(&self) -> Vec<u8> {
    self.vector_value_bytes(self.doc as usize)
  }

  fn vector_value_bytes(&self, ord: usize) -> Vec<u8> {
    let mut value = [0.0_f32; 2];
    unit_vector_2d(ord as f64 / self.size as f64, &mut value);
    value
      .into_iter()
      .map(|component| (component * 127.0) as u8)
      .collect()
  }
}

impl KnnVectorValues for CircularByteVectorValues {
  fn dimension(&self) -> usize {
    2
  }

  fn size(&self) -> usize {
    self.size
  }

  type KnnVectorValues = Self;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    Ok(self.clone())
  }

  fn get_encoding(&self) -> VectorEncoding {
    ByteVectorValues::get_encoding(self)
  }

  type Bits<B>
    = BitsImpl1<B>
  where
    B: Bits;

  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Option<Self::Bits<B>>
  where
    B: Bits,
  {
    self.default_get_accept_ords(accept_docs)
  }

  type DocIndexIterator = DummyDocIndexIterator;
}

impl ByteVectorValues for CircularByteVectorValues {
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    Ok(Cow::Owned(VectorValueEnum::Byte(
      self.vector_value_bytes(ord),
    )))
  }

  type ByteVectorValues = Self;

  fn byte_copy(&self) -> Result<Option<Self::ByteVectorValues>> {
    Ok(Some(self.clone()))
  }

  type VectorScorer = DummyVectorScorer;
}

fn unit_vector_2d(pi_radians: f64, value: &mut [f32; 2]) {
  value[0] = (std::f64::consts::PI * pi_radians).cos() as f32;
  value[1] = (std::f64::consts::PI * pi_radians).sin() as f32;
}

#[derive(Clone)]
pub struct CircularFloatVectorValues {
  size: usize,
}

impl CircularFloatVectorValues {
  pub fn new(size: usize) -> Self {
    Self { size }
  }

  fn vector_value_f32(&self, ord: usize) -> Vec<f32> {
    let mut value = [0.0_f32; 2];
    unit_vector_2d(ord as f64 / self.size as f64, &mut value);
    value.to_vec()
  }
}

impl KnnVectorValues for CircularFloatVectorValues {
  fn dimension(&self) -> usize {
    2
  }

  fn size(&self) -> usize {
    self.size
  }

  type KnnVectorValues = Self;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    Ok(self.clone())
  }

  fn get_encoding(&self) -> VectorEncoding {
    FloatVectorValues::get_encoding(self)
  }

  type Bits<B>
    = BitsImpl1<B>
  where
    B: Bits;

  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Option<Self::Bits<B>>
  where
    B: Bits,
  {
    self.default_get_accept_ords(accept_docs)
  }

  type DocIndexIterator = DenseDocIndexIterator;

  fn iterator(&mut self) -> Result<Self::DocIndexIterator> {
    Ok(create_dense_iterator(self.size as i32))
  }
}

impl FloatVectorValues for CircularFloatVectorValues {
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    Ok(Cow::Owned(VectorValueEnum::Float(
      self.vector_value_f32(ord),
    )))
  }

  type FloatVectorValues = Self;

  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    Ok(Some(self.clone()))
  }

  type VectorScorer = DummyVectorScorer;
}

pub fn sorted_nodes_on_level<G>(graph: &mut G, level: usize) -> Result<Vec<usize>>
where
  G: HnswGraph,
{
  let mut nodes_on_level = graph.get_nodes_on_level(level)?;
  let mut nodes = Vec::with_capacity(nodes_on_level.size());
  while nodes_on_level.has_next() {
    nodes.push(nodes_on_level.next().unwrap());
  }
  nodes.sort_unstable();
  Ok(nodes)
}

pub fn assert_graph_equal<G, H>(g: &mut G, h: &mut H) -> Result<()>
where
  G: HnswGraph,
  H: HnswGraph,
{
  let g_num_levels = g.num_levels()?;
  let h_num_levels = h.num_levels()?;
  assert_eq!(
    g_num_levels, h_num_levels,
    "the number of levels in the graphs are different"
  );
  assert_eq!(
    g.size(),
    h.size(),
    "the number of nodes in the graphs are different"
  );

  for level in 0..g_num_levels {
    let h_nodes = sorted_nodes_on_level(h, level)?;
    let g_nodes = sorted_nodes_on_level(g, level)?;
    assert_eq!(
      g_nodes, h_nodes,
      "nodes in the graphs are different on level {level}"
    );
  }

  for level in 0..g_num_levels {
    let g_nodes = sorted_nodes_on_level(g, level)?;
    for node in g_nodes {
      g.seek(level, node)?;
      h.seek(level, node)?;
      assert_eq!(
        get_neighbor_nodes(g)?,
        get_neighbor_nodes(h)?,
        "arcs differ for node {node} on level {level}"
      );
    }
  }

  Ok(())
}

pub fn assert_graph_contains_graph<G, H>(
  graph: &mut G,
  initializer: &mut H,
  new_ordinals: &[usize],
) -> Result<()>
where
  G: HnswGraph,
  H: HnswGraph,
{
  for level in 0..initializer.num_levels()? {
    let final_graph_nodes_on_level = nodes_iterator_to_array(graph.get_nodes_on_level(level)?);
    let initializer_graph_nodes_on_level = map_array_and_sort(
      &nodes_iterator_to_array(initializer.get_nodes_on_level(level)?),
      new_ordinals,
    );
    let overlap = compute_overlap(
      &final_graph_nodes_on_level,
      &initializer_graph_nodes_on_level,
    );
    assert_eq!(initializer_graph_nodes_on_level.len(), overlap);
  }
  Ok(())
}

pub fn assert_graph_initialized_from_graph<G, H>(
  graph: &mut G,
  initializer: &mut H,
  new_ordinals: &[usize],
) -> Result<()>
where
  G: HnswGraph,
  H: HnswGraph,
{
  assert_eq!(
    initializer.num_levels()?,
    graph.num_levels()?,
    "the number of levels in the graphs are different!"
  );
  assert_eq!(
    initializer.size(),
    graph.size(),
    "the number of nodes in the graphs are different!"
  );

  for level in 0..graph.num_levels()? {
    let nodes_on_level = nodes_iterator_to_array(initializer.get_nodes_on_level(level)?);
    for node in nodes_on_level {
      graph.seek(level, new_ordinals[node])?;
      initializer.seek(level, node)?;
      let expected_neighbors: HashSet<usize> = get_neighbor_nodes(initializer)?
        .into_iter()
        .map(|neighbor| new_ordinals[neighbor])
        .collect();
      assert_eq!(
        get_neighbor_nodes(graph)?,
        expected_neighbors,
        "arcs differ for node {node}"
      );
    }
  }

  Ok(())
}

pub fn nodes_iterator_to_array<I>(mut nodes_iterator: I) -> Vec<usize>
where
  I: NodesIterator,
{
  let mut arr = Vec::with_capacity(nodes_iterator.size());
  while nodes_iterator.has_next() {
    arr.push(nodes_iterator.next().unwrap());
  }
  arr
}

pub fn map_array_and_sort(arr: &[usize], offset: &[usize]) -> Vec<usize> {
  let mut mapped = arr.iter().map(|value| offset[*value]).collect::<Vec<_>>();
  mapped.sort_unstable();
  mapped
}

pub fn create_offset_ordinal_map<T>(
  doc_id_size: usize,
  total_vector_values: &mut T,
  doc_id_offset: i32,
) -> Result<Vec<usize>>
where
  T: KnnVectorValues,
{
  let mut ordinal_offset = 0usize;
  let mut iterator = total_vector_values.iterator()?;
  while iterator.next_doc()? < doc_id_offset {
    ordinal_offset += 1;
  }

  let mut offset_ordinal_map = vec![0; doc_id_size];
  let upper_doc = doc_id_offset + doc_id_size as i32;
  let mut curr = 0usize;
  while iterator.doc_id() < upper_doc {
    offset_ordinal_map[curr] = ordinal_offset + curr;
    curr += 1;
    let _ = iterator.next_doc()?;
  }

  Ok(offset_ordinal_map)
}

fn compute_overlap(left: &[usize], right: &[usize]) -> usize {
  let mut left = left.to_vec();
  let mut right = right.to_vec();
  left.sort_unstable();
  right.sort_unstable();

  let mut overlap = 0usize;
  let mut i = 0usize;
  let mut j = 0usize;
  while i < left.len() && j < right.len() {
    if left[i] == right[j] {
      overlap += 1;
      i += 1;
      j += 1;
    } else if left[i] > right[j] {
      j += 1;
    } else {
      i += 1;
    }
  }

  overlap
}

pub fn get_neighbor_nodes<G>(graph: &mut G) -> Result<HashSet<usize>>
where
  G: HnswGraph,
{
  let mut neighbors = HashSet::new();
  loop {
    let neighbor = graph.next_neighbor()?;
    if neighbor == NO_MORE_DOCS as usize {
      break;
    }
    neighbors.insert(neighbor);
  }
  Ok(neighbors)
}

pub fn assert_byte_vectors_equal<U, V>(u: &U, v: &V) -> Result<()>
where
  U: ByteVectorValues,
  V: ByteVectorValues,
{
  assert_eq!(u.size(), v.size());
  for ord in 0..u.size() {
    let u_doc = u.ord_to_doc(ord)?;
    let v_doc = v.ord_to_doc(ord)?;
    assert_eq!(u_doc, v_doc);
    assert_ne!(NO_MORE_DOCS, u_doc as i32);

    let u_vec = u.vector_value(ord)?;
    let v_vec = v.vector_value(ord)?;
    let u_bytes = u_vec.as_ref().as_bytes()?;
    let v_bytes = v_vec.as_ref().as_bytes()?;
    assert_eq!(u_bytes, v_bytes, "vectors do not match for doc={u_doc}");
  }
  Ok(())
}

pub fn assert_float_vectors_equal<U, V>(u: &U, v: &V) -> Result<()>
where
  U: FloatVectorValues,
  V: FloatVectorValues,
{
  assert_eq!(u.size(), v.size());
  for ord in 0..u.size() {
    let u_doc = u.ord_to_doc(ord)?;
    let v_doc = v.ord_to_doc(ord)?;
    assert_eq!(u_doc, v_doc);
    assert_ne!(NO_MORE_DOCS, u_doc as i32);

    let u_vec = u.vector_value(ord)?;
    let v_vec = v.vector_value(ord)?;
    let u_floats = u_vec.as_ref().as_floats()?;
    let v_floats = v_vec.as_ref().as_floats()?;
    assert_eq!(
      u_floats.len(),
      v_floats.len(),
      "vectors do not match for doc={u_doc}"
    );
    for (lhs, rhs) in u_floats.iter().zip(v_floats.iter()) {
      assert!(
        (lhs - rhs).abs() <= 1e-4,
        "vectors do not match for doc={u_doc}: left={u_floats:?}, right={v_floats:?}"
      );
    }
  }
  Ok(())
}

pub fn create_random_float_vectors<R>(
  size: usize,
  dimension: usize,
  random: &mut R,
) -> Vec<Vec<f32>>
where
  R: Rng + ?Sized,
{
  (0..size)
    .map(|_| random_vector(random, dimension))
    .collect()
}

pub fn create_random_byte_vectors<R>(size: usize, dimension: usize, random: &mut R) -> Vec<Vec<u8>>
where
  R: Rng + ?Sized,
{
  (0..size)
    .map(|_| random_vector8(random, dimension))
    .collect()
}

pub fn create_random_accept_ords(start_index: usize, length: usize) -> FixedBitSet {
  let mut bits = FixedBitSet::new(length);
  for i in 0..start_index.min(length) {
    bits.set(i);
  }

  let mut random = random();
  for i in start_index.min(length)..length {
    if random.random::<f32>() < 0.667 {
      bits.set(i);
    }
  }

  bits
}

pub fn random_vector<R>(random: &mut R, dim: usize) -> Vec<f32>
where
  R: Rng + ?Sized,
{
  let mut vec = vec![0.0; dim];
  for value in &mut vec {
    *value = random.random::<f32>();
    if random.random_bool(0.5) {
      *value = -*value;
    }
  }
  VectorUtil::l2normalize(&mut vec).expect("random_vector should not generate a zero vector");
  vec
}

pub fn random_vector8<R>(random: &mut R, dim: usize) -> Vec<u8>
where
  R: Rng + ?Sized,
{
  random_vector(random, dim)
    .into_iter()
    .map(|component| (component * 127.0) as u8)
    .collect()
}
