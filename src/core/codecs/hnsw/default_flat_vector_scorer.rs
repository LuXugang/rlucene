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
use crate::core::index::dummy::dummy_float_vector_values::DummyFloatVectorValues;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::knn_vector_values::KnnVectorValues;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::bits::Bits;
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::random_vector_scorer::RandomVectorScorer;
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;
use std::fmt::{Display, Formatter};

#[derive(Default)]
pub struct DefaultFlatVectorScorer;

impl Display for DefaultFlatVectorScorer {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl FlatVectorsScorer for DefaultFlatVectorScorer {
  type RandomVectorScorerSupplier = FloatScoringSupplier<DummyFloatVectorValues>;

  fn get_random_vector_scorer_supplier<K>(
    &self,
    _similarity_function: VectorSimilarityFunction,
    _vector_values: &K,
  ) -> Result<Self::RandomVectorScorerSupplier>
  where
    K: KnnVectorValues,
  {
    todo!()
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

pub struct FloatScoringSupplier<FV>
where
  FV: FloatVectorValues,
{
  vectors: FV,
  vectors1: Option<<FV as FloatVectorValues>::FloatVectorValues>,
  similarity_function: VectorSimilarityFunction,
}
impl<FV> FloatScoringSupplier<FV>
where
  FV: FloatVectorValues,
{
  pub(crate) fn new(vectors: FV, similarity_function: VectorSimilarityFunction) -> Result<Self> {
    let vectors1 = FloatVectorValues::copy(&vectors)?;
    Ok(Self {
      vectors,
      vectors1,
      similarity_function,
    })
  }
}
impl<FV> RandomVectorScorerSupplier for FloatScoringSupplier<FV>
where
  FV: FloatVectorValues,
{
  type Scorer = RandomVectorScorerF32Impl<FV>;

  fn scorer(&self, _ord: usize) -> Result<Self::Scorer> {
    todo!()
  }

  fn copy(&self) -> Result<Self>
  where
    Self: Sized,
  {
    todo!()
  }
}
pub struct RandomVectorScorerF32Impl<FV>
where
  FV: FloatVectorValues,
{
  vectors: FV,
  vectors1: Option<<FV as FloatVectorValues>::FloatVectorValues>,
  similarity_function: VectorSimilarityFunction,
}
impl<FV> RandomVectorScorerF32Impl<FV>
where
  FV: FloatVectorValues,
{
  pub(crate) fn new(
    vectors: FV,
    vectors1: Option<<FV as FloatVectorValues>::FloatVectorValues>,
    similarity_function: VectorSimilarityFunction,
  ) -> Self {
    Self {
      vectors,
      vectors1,
      similarity_function,
    }
  }
}
impl<FV> RandomVectorScorer for RandomVectorScorerF32Impl<FV>
where
  FV: FloatVectorValues,
{
  fn score(&self, _node: usize) -> Result<f32> {
    todo!()
  }

  fn max_ord(&self) -> usize {
    todo!()
  }

  fn ord_to_doc(&self, _ord: usize) -> usize {
    todo!()
  }

  type Bits<B>
    = DummyBits
  where
    B: Bits;

  fn get_accept_ords<B>(&self, _accept_docs: Option<B>) -> Result<Option<Self::Bits<B>>>
  where
    B: Bits,
  {
    todo!()
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
    Ok(
      self
        .similarity_function
        .compare_f32(self.query.as_slice(), self.values.vector_value(node)),
    )
  }

  fn max_ord(&self) -> usize {
    self.values.size()
  }

  fn ord_to_doc(&self, ord: usize) -> usize {
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
    Ok(
      self
        .similarity_function
        .compare_u8(self.query.as_slice(), self.values.vector_value(node)),
    )
  }

  fn max_ord(&self) -> usize {
    self.values.size()
  }

  fn ord_to_doc(&self, ord: usize) -> usize {
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
