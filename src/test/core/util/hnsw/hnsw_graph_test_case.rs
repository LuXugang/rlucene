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
use std::collections::HashSet;
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
use crate::core::util::hnsw::hnsw_graph::HnswGraph;
use crate::core::util::vector_util::VectorUtil;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use rand::{Rng, RngExt};

pub trait HnswGraphTestCase<T> {
  fn similarity_function(&self) -> VectorSimilarityFunction;

  fn flat_vector_scorer(&self) -> DefaultFlatVectorScorer {
    DefaultFlatVectorScorer
  }

  fn get_vector_encoding(&self) -> VectorEncoding;

  fn knn_query(&self, field: &str, vector: T, k: usize) -> Query;

  fn random_vector(&self, dim: usize) -> T;

  type KnnVectorValues: KnnVectorValues;

  fn vector_values(&self, size: usize, dimension: usize) -> Self::KnnVectorValues;

  fn vector_values_from_values(&self, values: Vec<Vec<f32>>) -> Self::KnnVectorValues;

  fn vector_values_from_reader<LR>(
    &self,
    reader: &LR,
    field_name: &str,
  ) -> Result<Self::KnnVectorValues>
  where
    LR: LeafReader;

  fn vector_values_with_pregenerated(
    &self,
    size: usize,
    dimension: usize,
    pregenerated_vector_values: Self::KnnVectorValues,
    pregenerated_offset: usize,
  ) -> Self::KnnVectorValues;

  fn knn_vector_field(
    &self,
    name: &str,
    vector: T,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Field>;
  type CircularKnnVectorValues: KnnVectorValues;
  fn circular_vector_values(&self, n_doc: usize) -> Self::CircularKnnVectorValues;

  fn get_target_vector(&self) -> T;

  fn build_scorer_supplier<B, F>(
    &self,
    vectors: KnnVectorValuesType<B, F>,
  ) -> Result<<DefaultFlatVectorScorer as FlatVectorsScorer>::RandomVectorScorerSupplier<B, F>>
  where
    B: ByteVectorValues + Clone,
    F: FloatVectorValues + Clone,
  {
    self
      .flat_vector_scorer()
      .get_random_vector_scorer_supplier(self.similarity_function(), vectors)
  }
}
#[derive(Clone)]
pub struct CircularByteVectorValues {
  size: usize,
  doc: i32,
}

impl CircularByteVectorValues {
  pub fn new(size: usize) -> Self {
    Self { size,doc:-1}
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

  type Bits<B> = BitsImpl1<B> where B: Bits;

  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Option<Self::Bits<B>>
  where
      B: Bits
  {
    self.default_get_accept_ords(accept_docs)
  }

  type DocIndexIterator = DummyDocIndexIterator;
}

impl ByteVectorValues for CircularByteVectorValues {
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    Ok(Cow::Owned(VectorValueEnum::Byte(self.vector_value_bytes(ord))))
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

  type Bits<B> = BitsImpl1<B> where B: Bits;

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

pub fn create_random_float_vectors<R>(size: usize, dimension: usize, random: &mut R) -> Vec<Vec<f32>>
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
