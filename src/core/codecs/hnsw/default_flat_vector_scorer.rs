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
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::knn_vector_values::{KnnVectorValues, KnnVectorValuesType};
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::random_vector_scorer::{RandomVectorScorer, RandomVectorScorerEnum2};
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;
use std::fmt::{Display, Formatter};

/// Default implementation of [`FlatVectorsScorer`].
#[derive(Default, Clone, Debug)]
pub struct DefaultFlatVectorScorer;

impl Display for DefaultFlatVectorScorer {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl FlatVectorsScorer for DefaultFlatVectorScorer {
  type RandomVectorScorerSupplier<B, F>
    = RandomVectorScorerSupplierEnum<B, F>
  where
    B: ByteVectorValues + Clone,
    F: FloatVectorValues + Clone;

  fn get_random_vector_scorer_supplier<B, F>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: KnnVectorValuesType<B, F>,
  ) -> Result<Self::RandomVectorScorerSupplier<B, F>>
  where
    B: ByteVectorValues + Clone,
    F: FloatVectorValues + Clone,
  {
    let v = match vector_values {
      KnnVectorValuesType::Byte(b) => {
        debug_assert!(KnnVectorValues::get_encoding(&b) == VectorEncoding::BYTE(1));
        RandomVectorScorerSupplierEnum::Byte(ByteScoringSupplier::new(b, similarity_function)?)
      },
      KnnVectorValuesType::Float(f) => {
        debug_assert!(KnnVectorValues::get_encoding(&f) == VectorEncoding::FLOAT32(4));
        RandomVectorScorerSupplierEnum::Float(FloatScoringSupplier::new(f, similarity_function)?)
      },
    };
    Ok(v)
  }

  type RandomVectorScorerF32<T>
    = FloatVectorScorer<T>
  where
    T: FloatVectorValues;

  fn get_random_vector_scorer_f32<K>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: K,
    target: Vec<f32>,
  ) -> Result<Self::RandomVectorScorerF32<K>>
  where
    K: FloatVectorValues,
  {
    if target.len() != vector_values.dimension() {
      return Err(LuceneError::illegal_argument(format!(
        "vector query dimension: {} differs from field dimension: {}",
        target.len(),
        vector_values.dimension()
      )));
    }

    Ok(FloatVectorScorer::new(
      vector_values,
      target,
      similarity_function,
    ))
  }

  type RandomVectorScorerU8<T>
    = ByteVectorScorer<T>
  where
    T: ByteVectorValues;

  fn get_random_vector_scorer_u8<K>(
    &self,
    similarity_function: VectorSimilarityFunction,
    vector_values: K,
    target: Vec<u8>,
  ) -> Result<Self::RandomVectorScorerU8<K>>
  where
    K: ByteVectorValues,
  {
    if target.len() != vector_values.dimension() {
      return Err(LuceneError::illegal_argument(format!(
        "vector query dimension: {} differs from field dimension: {}",
        target.len(),
        vector_values.dimension()
      )));
    }

    Ok(ByteVectorScorer::new(
      vector_values,
      target,
      similarity_function,
    ))
  }
}
pub enum RandomVectorScorerSupplierEnum<BV, FV>
where
  BV: ByteVectorValues + Clone,
  FV: FloatVectorValues + Clone,
{
  Byte(ByteScoringSupplier<BV>),
  Float(FloatScoringSupplier<FV>),
}
impl<BV, FV> RandomVectorScorerSupplier for RandomVectorScorerSupplierEnum<BV, FV>
where
  BV: ByteVectorValues + Clone,
  FV: FloatVectorValues + Clone,
{
  type Scorer<'a>
    = RandomVectorScorerEnum2<RandomVectorScorerByteImpl<'a, BV>, RandomVectorScorerF32Impl<'a, FV>>
  where
    Self: 'a;

  fn scorer(&self, ord: usize) -> Result<Self::Scorer<'_>> {
    match self {
      RandomVectorScorerSupplierEnum::Byte(supplier) => {
        Ok(RandomVectorScorerEnum2::A(supplier.scorer(ord)?))
      },
      RandomVectorScorerSupplierEnum::Float(supplier) => {
        Ok(RandomVectorScorerEnum2::B(supplier.scorer(ord)?))
      },
    }
  }

  fn copy(&self) -> Result<Self>
  where
    Self: Sized,
  {
    match self {
      RandomVectorScorerSupplierEnum::Byte(supplier) => {
        Ok(RandomVectorScorerSupplierEnum::Byte(supplier.copy()?))
      },
      RandomVectorScorerSupplierEnum::Float(supplier) => {
        Ok(RandomVectorScorerSupplierEnum::Float(supplier.copy()?))
      },
    }
  }

  fn get_vector_byte_mut(&mut self) -> Result<&mut Vec<Vec<u8>>> {
    match self {
      RandomVectorScorerSupplierEnum::Byte(supplier) => supplier.get_vector_byte_mut(),
      _ => Err(LuceneError::illegal_state("should byte here")),
    }
  }

  fn get_vector_byte(&self) -> Result<&[Vec<u8>]> {
    match self {
      RandomVectorScorerSupplierEnum::Byte(supplier) => supplier.get_vector_byte(),
      _ => Err(LuceneError::illegal_state("should byte here")),
    }
  }

  fn get_vector_float_mut(&mut self) -> Result<&mut Vec<Vec<f32>>> {
    match self {
      RandomVectorScorerSupplierEnum::Float(supplier) => supplier.get_vector_float_mut(),
      _ => Err(LuceneError::illegal_state("should float here")),
    }
  }

  fn get_vector_float(&self) -> Result<&[Vec<f32>]> {
    match self {
      RandomVectorScorerSupplierEnum::Float(supplier) => supplier.get_vector_float(),
      _ => Err(LuceneError::illegal_state("should float here")),
    }
  }
}
/// RandomVectorScorerSupplier for bytes vector
pub struct ByteScoringSupplier<BV>
where
  BV: ByteVectorValues + Clone,
{
  vectors: BV,
  vectors1: <BV as ByteVectorValues>::ByteVectorValues,
  vectors2: <BV as ByteVectorValues>::ByteVectorValues,
  similarity_function: VectorSimilarityFunction,
}

impl<BV> ByteScoringSupplier<BV>
where
  BV: ByteVectorValues + Clone,
{
  pub(crate) fn new(vectors: BV, similarity_function: VectorSimilarityFunction) -> Result<Self> {
    let vectors1 = ByteVectorValues::byte_copy(&vectors)?;
    let vectors2 = ByteVectorValues::byte_copy(&vectors)?;
    Ok(Self {
      vectors,
      vectors1,
      vectors2,
      similarity_function,
    })
  }
}

impl<BV> RandomVectorScorerSupplier for ByteScoringSupplier<BV>
where
  BV: ByteVectorValues + Clone,
{
  type Scorer<'a>
    = RandomVectorScorerByteImpl<'a, BV>
  where
    Self: 'a;

  fn scorer(&self, ord: usize) -> Result<Self::Scorer<'_>> {
    Ok(RandomVectorScorerByteImpl::new(
      &self.vectors,
      &self.vectors1,
      &self.vectors2,
      self.similarity_function,
      ord,
    ))
  }

  fn copy(&self) -> Result<Self>
  where
    Self: Sized,
  {
    ByteScoringSupplier::new(self.vectors.clone(), self.similarity_function)
  }

  fn get_vector_byte_mut(&mut self) -> Result<&mut Vec<Vec<u8>>> {
    self.vectors.get_vectors_mut()
  }

  fn get_vector_byte(&self) -> Result<&[Vec<u8>]> {
    self.vectors.get_vectors()
  }
}

pub struct RandomVectorScorerByteImpl<'a, BV>
where
  BV: ByteVectorValues,
{
  vectors: &'a BV,
  vectors1: &'a <BV as ByteVectorValues>::ByteVectorValues,
  vectors2: &'a <BV as ByteVectorValues>::ByteVectorValues,
  similarity_function: VectorSimilarityFunction,
  ord: usize,
}

impl<'a, BV> RandomVectorScorerByteImpl<'a, BV>
where
  BV: ByteVectorValues,
{
  pub(crate) fn new(
    vectors: &'a BV,
    vectors1: &'a <BV as ByteVectorValues>::ByteVectorValues,
    vectors2: &'a <BV as ByteVectorValues>::ByteVectorValues,
    similarity_function: VectorSimilarityFunction,
    ord: usize,
  ) -> Self {
    Self {
      vectors,
      vectors1,
      vectors2,
      similarity_function,
      ord,
    }
  }
}

impl<BV> RandomVectorScorer for RandomVectorScorerByteImpl<'_, BV>
where
  BV: ByteVectorValues,
{
  fn score(&self, node: usize) -> Result<f32> {
    let ord_vector = self.vectors1.vector_value(self.ord)?;
    let node_vector = self.vectors2.vector_value(node)?;
    Ok(
      self
        .similarity_function
        .compare_u8(ord_vector.as_ref(), node_vector.as_ref()),
    )
  }

  fn max_ord(&self) -> usize {
    self.vectors.size()
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    self.vectors.ord_to_doc(ord)
  }

  type Bits<B>
    = <BV as KnnVectorValues>::Bits<B>
  where
    B: Bits;

  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Result<Option<Self::Bits<B>>>
  where
    B: Bits,
  {
    Ok(self.vectors.get_accept_ords(accept_docs))
  }
}
/// RandomVectorScorerSupplier for Float vector
pub struct FloatScoringSupplier<FV>
where
  FV: FloatVectorValues + Clone,
{
  vectors: FV,
  vectors1: <FV as FloatVectorValues>::FloatVectorValues,
  vectors2: <FV as FloatVectorValues>::FloatVectorValues,
  similarity_function: VectorSimilarityFunction,
}
impl<FV> FloatScoringSupplier<FV>
where
  FV: FloatVectorValues + Clone,
{
  pub(crate) fn new(vectors: FV, similarity_function: VectorSimilarityFunction) -> Result<Self> {
    let vectors1 = FloatVectorValues::float_copy(&vectors)?;
    let vectors2 = FloatVectorValues::float_copy(&vectors)?;
    Ok(Self {
      vectors,
      vectors1,
      vectors2,
      similarity_function,
    })
  }
}
impl<FV> RandomVectorScorerSupplier for FloatScoringSupplier<FV>
where
  FV: FloatVectorValues + Clone,
{
  type Scorer<'a>
    = RandomVectorScorerF32Impl<'a, FV>
  where
    Self: 'a;

  fn scorer(&self, ord: usize) -> Result<Self::Scorer<'_>> {
    Ok(RandomVectorScorerF32Impl::new(
      &self.vectors,
      &self.vectors1,
      &self.vectors2,
      self.similarity_function,
      ord,
    ))
  }

  fn copy(&self) -> Result<Self>
  where
    Self: Sized,
  {
    FloatScoringSupplier::new(self.vectors.clone(), self.similarity_function)
  }

  fn get_vector_float_mut(&mut self) -> Result<&mut Vec<Vec<f32>>> {
    self.vectors.get_vectors_mut()
  }

  fn get_vector_float(&self) -> Result<&[Vec<f32>]> {
    self.vectors.get_vectors()
  }
}
pub struct RandomVectorScorerF32Impl<'a, FV>
where
  FV: FloatVectorValues,
{
  vectors: &'a FV,
  vectors1: &'a <FV as FloatVectorValues>::FloatVectorValues,
  vectors2: &'a <FV as FloatVectorValues>::FloatVectorValues,
  similarity_function: VectorSimilarityFunction,
  ord: usize,
}
impl<'a, FV> RandomVectorScorerF32Impl<'a, FV>
where
  FV: FloatVectorValues,
{
  pub(crate) fn new(
    vectors: &'a FV,
    vectors1: &'a <FV as FloatVectorValues>::FloatVectorValues,
    vectors2: &'a <FV as FloatVectorValues>::FloatVectorValues,
    similarity_function: VectorSimilarityFunction,
    ord: usize,
  ) -> Self {
    Self {
      vectors,
      vectors1,
      vectors2,
      similarity_function,
      ord,
    }
  }
}
impl<FV> RandomVectorScorer for RandomVectorScorerF32Impl<'_, FV>
where
  FV: FloatVectorValues,
{
  fn score(&self, node: usize) -> Result<f32> {
    let ord_vector = self.vectors1.vector_value(self.ord)?;
    let node_vector = self.vectors2.vector_value(node)?;
    Ok(
      self
        .similarity_function
        .compare_f32(ord_vector.as_ref(), node_vector.as_ref()),
    )
  }

  fn max_ord(&self) -> usize {
    self.vectors.size()
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    self.vectors.ord_to_doc(ord)
  }

  type Bits<B>
    = <FV as KnnVectorValues>::Bits<B>
  where
    B: Bits;

  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Result<Option<Self::Bits<B>>>
  where
    B: Bits,
  {
    Ok(self.vectors.get_accept_ords(accept_docs))
  }
}

pub struct FloatVectorScorer<FV>
where
  FV: FloatVectorValues,
{
  values: FV,
  query: Vec<f32>,
  similarity_function: VectorSimilarityFunction,
}

impl<FV> FloatVectorScorer<FV>
where
  FV: FloatVectorValues,
{
  pub(crate) fn new(
    values: FV,
    query: Vec<f32>,
    similarity_function: VectorSimilarityFunction,
  ) -> Self {
    Self {
      values,
      query,
      similarity_function,
    }
  }
}

impl<FV> RandomVectorScorer for FloatVectorScorer<FV>
where
  FV: FloatVectorValues,
{
  fn score(&self, node: usize) -> Result<f32> {
    let value = self.values.vector_value(node)?;
    Ok(
      self
        .similarity_function
        .compare_f32(self.query.as_slice(), value.as_ref()),
    )
  }

  fn max_ord(&self) -> usize {
    self.values.size()
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    self.values.ord_to_doc(ord)
  }

  type Bits<B>
    = <FV as KnnVectorValues>::Bits<B>
  where
    B: Bits;

  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Result<Option<Self::Bits<B>>>
  where
    B: Bits,
  {
    Ok(self.values.get_accept_ords(accept_docs))
  }
}
pub struct ByteVectorScorer<BV>
where
  BV: ByteVectorValues,
{
  values: BV,
  query: Vec<u8>,
  similarity_function: VectorSimilarityFunction,
}
impl<BV> ByteVectorScorer<BV>
where
  BV: ByteVectorValues,
{
  pub(crate) fn new(
    values: BV,
    query: Vec<u8>,
    similarity_function: VectorSimilarityFunction,
  ) -> Self {
    Self {
      values,
      query,
      similarity_function,
    }
  }
}
impl<BV> RandomVectorScorer for ByteVectorScorer<BV>
where
  BV: ByteVectorValues,
{
  fn score(&self, node: usize) -> Result<f32> {
    let value = self.values.vector_value(node)?;
    Ok(
      self
        .similarity_function
        .compare_u8(self.query.as_slice(), value.as_ref()),
    )
  }

  fn max_ord(&self) -> usize {
    self.values.size()
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    self.values.ord_to_doc(ord)
  }

  type Bits<B>
    = <BV as KnnVectorValues>::Bits<B>
  where
    B: Bits;

  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Result<Option<Self::Bits<B>>>
  where
    B: Bits,
  {
    Ok(self.values.get_accept_ords(accept_docs))
  }
}
