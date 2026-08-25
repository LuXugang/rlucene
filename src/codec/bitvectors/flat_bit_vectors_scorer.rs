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
use crate::core::codecs::hnsw::flat_vectors_scorer::{FlatVectorValuesEnum, FlatVectorsScorer};
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::knn_vector_values::KnnVectorValues;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::bits::Bits;
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::dummy::dummy_random_vector_scorer::DummyRandomVectorScorer;
use crate::core::util::hnsw::random_vector_scorer::{RandomVectorScorer, RandomVectorScorerEnum2};
use crate::core::util::hnsw::random_vector_scorer_supplier::{
  RandomVectorScorerSupplier, vector_values_ram_bytes_used,
};
use crate::core::util::vector_util::VECTOR_UTIL;
use std::fmt::{Display, Formatter};

/// A bit vector scorer for scoring byte vectors.
#[derive(Clone, Debug, Default)]
pub struct FlatBitVectorsScorer;

impl FlatVectorsScorer for FlatBitVectorsScorer {
  type RandomVectorScorerSupplier<B, F>
    = BitRandomVectorScorerSupplier<B>
  where
    B: ByteVectorValues + TryClone + Send,
    B::ByteVectorValues: Send,
    F: FloatVectorValues + TryClone + Send,
    F::FloatVectorValues: Send;

  fn get_random_vector_scorer_supplier<B, F>(
    &self,
    _similarity_function: VectorSimilarityFunction,
    vector_values: FlatVectorValuesEnum<B, F>,
  ) -> Result<Self::RandomVectorScorerSupplier<B, F>>
  where
    B: ByteVectorValues + TryClone + Send,
    B::ByteVectorValues: Send,
    F: FloatVectorValues + TryClone + Send,
    F::FloatVectorValues: Send,
  {
    match vector_values {
      FlatVectorValuesEnum::Byte(byte_vector_values) => {
        debug_assert!(
          KnnVectorValues::get_encoding(&byte_vector_values) == VectorEncoding::BYTE(1)
        );
        BitRandomVectorScorerSupplier::new(byte_vector_values)
      },
      FlatVectorValuesEnum::Float(_) => Err(LuceneError::illegal_argument(
        "vectorValues must be an instance of ByteVectorValues",
      )),
    }
  }

  type RandomVectorScorerF32<T>
    = DummyRandomVectorScorer
  where
    T: FloatVectorValues;

  fn get_random_vector_scorer_f32<K>(
    &self,
    _similarity_function: VectorSimilarityFunction,
    _vector_values: K,
    _target: Vec<f32>,
  ) -> Result<Self::RandomVectorScorerF32<K>>
  where
    K: FloatVectorValues,
  {
    Err(LuceneError::illegal_argument(
      "bit vectors do not support f32 slice targets",
    ))
  }

  type RandomVectorScorerU8<T>
    = BitRandomVectorScorer<T>
  where
    T: ByteVectorValues;

  fn get_random_vector_scorer_u8<K>(
    &self,
    _similarity_function: VectorSimilarityFunction,
    vector_values: K,
    target: Vec<u8>,
  ) -> Result<Self::RandomVectorScorerU8<K>>
  where
    K: ByteVectorValues,
  {
    debug_assert!(KnnVectorValues::get_encoding(&vector_values) == VectorEncoding::BYTE(1));
    Ok(BitRandomVectorScorer::new(vector_values, target))
  }
}

pub struct BitRandomVectorScorer<B> {
  vector_values: B,
  bit_dimensions: usize,
  query: Vec<u8>,
}

impl<B> BitRandomVectorScorer<B>
where
  B: KnnVectorValues,
{
  pub(crate) fn new(vector_values: B, query: Vec<u8>) -> Self {
    Self {
      bit_dimensions: vector_values.dimension() * u8::BITS as usize,
      vector_values,
      query,
    }
  }
}

impl<B> RandomVectorScorer for BitRandomVectorScorer<B>
where
  B: ByteVectorValues,
{
  fn score(&self, node: usize) -> Result<f32> {
    let vector_value = self.vector_values.vector_value(node)?;
    Ok(
      (self.bit_dimensions as i32
        - VECTOR_UTIL.xor_bit_count(self.query.as_slice(), vector_value.as_bytes()?)?) as f32
        / self.bit_dimensions as f32,
    )
  }

  fn max_ord(&self) -> usize {
    self.vector_values.size()
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    self.vector_values.ord_to_doc(ord)
  }

  type Bits<'a, B1>
    = <B as KnnVectorValues>::Bits<'a, B1>
  where
    B1: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B1>(
    &'a self,
    accept_docs: Option<B1>,
  ) -> Result<Option<Self::Bits<'a, B1>>>
  where
    B1: Bits,
  {
    Ok(self.vector_values.get_accept_ords(accept_docs))
  }
}

pub struct BitRandomVectorScorerSupplier<B>
where
  B: ByteVectorValues,
{
  vector_values: B,
  vector_values1: Option<<B as ByteVectorValues>::ByteVectorValues>,
  vector_values2: Option<<B as ByteVectorValues>::ByteVectorValues>,
}

impl<B> BitRandomVectorScorerSupplier<B>
where
  B: ByteVectorValues,
{
  pub fn new(vector_values: B) -> Result<Self> {
    let vector_values1 = vector_values.byte_copy()?;
    let vector_values2 = vector_values.byte_copy()?;
    if vector_values1.is_some() != vector_values2.is_some() {
      return Err(LuceneError::illegal_state(
        "ByteVectorValues copy must consistently return a value",
      ));
    }
    Ok(Self {
      vector_values,
      vector_values1,
      vector_values2,
    })
  }
}

impl<B> RandomVectorScorerSupplier for BitRandomVectorScorerSupplier<B>
where
  B: ByteVectorValues + TryClone + Send,
  B::ByteVectorValues: Send,
{
  type Scorer<'a>
    = RandomVectorScorerEnum2<
    BitRandomVectorScorer<&'a B>,
    BitRandomVectorScorer<&'a <B as ByteVectorValues>::ByteVectorValues>,
  >
  where
    Self: 'a,
    B: 'a,
    <B as ByteVectorValues>::ByteVectorValues: 'a;

  fn scorer(&self, ord: usize) -> Result<Self::Scorer<'_>> {
    match (&self.vector_values1, &self.vector_values2) {
      (Some(vector_values1), Some(vector_values2)) => {
        let query = vector_values1.vector_value(ord)?;
        Ok(RandomVectorScorerEnum2::B(BitRandomVectorScorer::new(
          vector_values2,
          query.as_bytes()?.to_vec(),
        )))
      },
      (None, None) => {
        let query = self.vector_values.vector_value(ord)?;
        Ok(RandomVectorScorerEnum2::A(BitRandomVectorScorer::new(
          &self.vector_values,
          query.as_bytes()?.to_vec(),
        )))
      },
      _ => Err(LuceneError::illegal_state(
        "ByteVectorValues copy must consistently return a value",
      )),
    }
  }

  type RandomVectorScorerSupplier = Self;

  fn copy(&self) -> Result<Self>
  where
    Self: Sized,
  {
    BitRandomVectorScorerSupplier::new(self.vector_values.try_clone()?)
  }

  fn get_vector_mut(&mut self) -> Result<&mut Vec<VectorValueEnum>> {
    self.vector_values.get_vectors_mut()
  }

  fn get_vector(&self) -> Result<&[VectorValueEnum]> {
    self.vector_values.get_vectors()
  }

  fn ram_bytes_used(&self) -> Result<i64> {
    let Ok(vectors) = self.vector_values.get_vectors() else {
      return Ok(0);
    };
    let capacity = self
      .vector_values
      .get_vectors_capacity()
      .unwrap_or(vectors.len());
    vector_values_ram_bytes_used(vectors, capacity)
  }
}

impl Display for FlatBitVectorsScorer {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "FlatBitVectorsScorer()")
  }
}
