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
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::random_vector_scorer::RandomVectorScorerEnum2;
use crate::core::util::hnsw::random_vector_scorer_supplier::{
  RandomVectorScorerSupplier, RandomVectorScorerSupplierEnum2,
};

pub trait CloseableRandomVectorScorerSupplier: Closeable + RandomVectorScorerSupplier {
  fn total_vector_count(&self) -> Result<i32>;
}

pub enum CloseableRandomVectorScorerSupplierEnum2<A, B> {
  A(A),
  B(B),
}

impl<A, B> RandomVectorScorerSupplier for CloseableRandomVectorScorerSupplierEnum2<A, B>
where
  A: CloseableRandomVectorScorerSupplier,
  B: CloseableRandomVectorScorerSupplier,
{
  type Scorer<'a>
    = RandomVectorScorerEnum2<A::Scorer<'a>, B::Scorer<'a>>
  where
    Self: 'a;

  fn scorer(&self, ord: usize) -> Result<Self::Scorer<'_>> {
    match self {
      Self::A(supplier) => supplier.scorer(ord).map(RandomVectorScorerEnum2::A),
      Self::B(supplier) => supplier.scorer(ord).map(RandomVectorScorerEnum2::B),
    }
  }

  type RandomVectorScorerSupplier =
    RandomVectorScorerSupplierEnum2<A::RandomVectorScorerSupplier, B::RandomVectorScorerSupplier>;

  fn copy(&self) -> Result<Self::RandomVectorScorerSupplier> {
    match self {
      Self::A(supplier) => supplier.copy().map(RandomVectorScorerSupplierEnum2::A),
      Self::B(supplier) => supplier.copy().map(RandomVectorScorerSupplierEnum2::B),
    }
  }

  fn get_vector(
    &self,
  ) -> Result<&[crate::core::codecs::knn_field_vectors_writer::VectorValueEnum]> {
    match self {
      Self::A(supplier) => supplier.get_vector(),
      Self::B(supplier) => supplier.get_vector(),
    }
  }

  fn get_vector_mut(
    &mut self,
  ) -> Result<&mut Vec<crate::core::codecs::knn_field_vectors_writer::VectorValueEnum>> {
    match self {
      Self::A(supplier) => supplier.get_vector_mut(),
      Self::B(supplier) => supplier.get_vector_mut(),
    }
  }

  fn ram_bytes_used(&self) -> Result<i64> {
    match self {
      Self::A(supplier) => supplier.ram_bytes_used(),
      Self::B(supplier) => supplier.ram_bytes_used(),
    }
  }
}

impl<A, B> Closeable for CloseableRandomVectorScorerSupplierEnum2<A, B>
where
  A: CloseableRandomVectorScorerSupplier,
  B: CloseableRandomVectorScorerSupplier,
{
  fn close(&mut self) -> Result<()> {
    match self {
      Self::A(supplier) => supplier.close(),
      Self::B(supplier) => supplier.close(),
    }
  }
}

impl<A, B> CloseableRandomVectorScorerSupplier for CloseableRandomVectorScorerSupplierEnum2<A, B>
where
  A: CloseableRandomVectorScorerSupplier,
  B: CloseableRandomVectorScorerSupplier,
{
  fn total_vector_count(&self) -> Result<i32> {
    match self {
      Self::A(supplier) => supplier.total_vector_count(),
      Self::B(supplier) => supplier.total_vector_count(),
    }
  }
}
