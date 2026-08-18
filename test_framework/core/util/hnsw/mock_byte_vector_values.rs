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
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::dummy::dummy_knn_vector_values::DummyKnnVectorsWriter;
use crate::core::index::knn_vector_values::{
  BitsImpl, DenseDocIndexIterator, KnnVectorValues, create_dense_iterator,
};
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::util::bits::Bits;
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::lucene_test_case::random_from_seed;
use rand::RngExt;
use std::borrow::Cow;

#[derive(Clone)]
pub struct MockByteVectorValues {
  dimension: usize,
  dense_values: Vec<Vec<u8>>,
  pub(crate) values: Vec<Vec<u8>>,
  num_vectors: i32,
  scratch: Vec<u8>,
  seed: u64,
}
impl TryClone for MockByteVectorValues {
  fn try_clone(&self) -> Result<Self>
  where
    Self: Sized,
  {
    Ok(self.clone())
  }
}

impl MockByteVectorValues {
  pub fn from_values(values: Vec<Vec<u8>>, seed: u64) -> Self {
    let first_non_empty = values
      .iter()
      .find(|value| !value.is_empty())
      .expect("MockByteVectorValues::from_values requires at least one non-empty vector");
    let dimension = first_non_empty.len();
    let dense_values: Vec<Vec<u8>> = values
      .iter()
      .filter(|value| !value.is_empty())
      .cloned()
      .collect();
    let num_vectors = dense_values.len() as i32;
    Self::new(values, dimension, dense_values, num_vectors, seed)
  }

  fn new(
    values: Vec<Vec<u8>>,
    dimension: usize,
    dense_values: Vec<Vec<u8>>,
    num_vectors: i32,
    seed: u64,
  ) -> Self {
    Self {
      dimension,
      dense_values,
      values,
      num_vectors,
      scratch: vec![0; dimension],
      seed,
    }
  }
}

impl KnnVectorValues for MockByteVectorValues {
  fn dimension(&self) -> usize {
    self.dimension
  }

  fn size(&self) -> usize {
    self.values.len()
  }

  type KnnVectorValues = DummyKnnVectorsWriter;

  fn get_encoding(&self) -> VectorEncoding {
    ByteVectorValues::get_encoding(self)
  }

  type Bits<'a, B>
    = BitsImpl<B, &'a Self>
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    self.default_get_accept_ords(accept_docs)
  }

  type DocIndexIterator = DenseDocIndexIterator;

  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    let size = self.size();
    Ok(create_dense_iterator(size as i32))
  }
}

impl ByteVectorValues for MockByteVectorValues {
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    let mut random = random_from_seed(self.seed);
    if random.random_bool(0.5) {
      Ok(Cow::Owned(VectorValueEnum::Byte(self.values[ord].clone())))
    } else {
      Ok(Cow::Owned(VectorValueEnum::Byte(
        self.values[ord][0..self.dimension].to_vec(),
      )))
    }
  }

  type ByteVectorValues = MockByteVectorValues;

  fn byte_copy(&self) -> Result<Option<Self::ByteVectorValues>> {
    let mut random = random_from_seed(self.seed);
    let seed = random.random();
    Ok(Some(MockByteVectorValues::new(
      self.values.clone(),
      self.dimension,
      self.dense_values.clone(),
      self.num_vectors,
      seed,
    )))
  }

  type VectorScorer = DummyVectorScorer;
}
