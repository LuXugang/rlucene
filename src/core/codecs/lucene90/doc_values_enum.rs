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
pub mod norms {
  use crate::core::codecs::lucene90_norms_producer::{DenseNormsIterator, SparseNormsIterator};
  use crate::core::index::doc_values::EmptyNumeric;
  use crate::core::index::doc_values_iterator::DocValuesIterator;
  use crate::core::index::numeric_doc_values::{NumericDocValues, NumericDocValuesVisitor};
  use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
  use crate::core::store::IndexInput;
  use crate::core::store::random_access_input::RandomAccessInput;
  use crate::core::util::error::lucene_error::Result;
  pub enum Lucene90NormNumericDocValuesEnum<I, R = <I as IndexInput>::RandomAccessSlice>
  where
    I: IndexInput,
    R: RandomAccessInput,
  {
    Dense(DenseNormsIterator<R>),
    Sparse(SparseNormsIterator<I, R>),
    Empty(EmptyNumeric),
  }

  impl<I, R: RandomAccessInput> DocValuesIterator for Lucene90NormNumericDocValuesEnum<I, R>
  where
    I: IndexInput,
  {
    #[inline]
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
      match self {
        Self::Dense(dense) => dense.advance_exact(target),
        Self::Sparse(sparse) => sparse.advance_exact(target),
        Self::Empty(empty) => empty.advance_exact(target),
      }
    }
  }

  impl<I, R: RandomAccessInput> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
    for Lucene90NormNumericDocValuesEnum<I, R>
  where
    I: IndexInput,
  {
  }
  impl<I, R: RandomAccessInput> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
    for Lucene90NormNumericDocValuesEnum<I, R>
  where
    I: IndexInput,
  {
  }

  impl<I, R: RandomAccessInput> DocIdSetIterator for Lucene90NormNumericDocValuesEnum<I, R>
  where
    I: IndexInput,
  {
    fn doc_id(&self) -> i32 {
      match self {
        Self::Dense(dense) => dense.doc_id(),
        Self::Sparse(sparse) => sparse.doc_id(),
        Self::Empty(empty) => empty.doc_id(),
      }
    }

    fn next_doc(&mut self) -> Result<i32> {
      match self {
        Self::Dense(dense) => dense.next_doc(),
        Self::Sparse(sparse) => sparse.next_doc(),
        Self::Empty(empty) => empty.next_doc(),
      }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
      match self {
        Self::Dense(dense) => dense.advance(target),
        Self::Sparse(sparse) => sparse.advance(target),
        Self::Empty(empty) => empty.advance(target),
      }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
      match self {
        Self::Dense(dense) => dense.slow_advance(target),
        Self::Sparse(sparse) => sparse.slow_advance(target),
        Self::Empty(empty) => empty.slow_advance(target),
      }
    }

    fn cost(&self) -> Result<i64> {
      match self {
        Self::Dense(dense) => dense.cost(),
        Self::Sparse(sparse) => sparse.cost(),
        Self::Empty(empty) => empty.cost(),
      }
    }
  }

  impl<I, R: RandomAccessInput> NumericDocValues for Lucene90NormNumericDocValuesEnum<I, R>
  where
    I: IndexInput,
  {
    #[inline(always)]
    fn long_value(&mut self) -> Result<i64> {
      match self {
        Self::Dense(dense) => dense.long_value(),
        Self::Sparse(sparse) => sparse.long_value(),
        Self::Empty(empty) => empty.long_value(),
      }
    }

    fn visit<V: NumericDocValuesVisitor>(&mut self, visitor: V) -> V::Output {
      match self {
        Self::Dense(dense) => dense.visit(visitor),
        Self::Sparse(sparse) => sparse.visit(visitor),
        Self::Empty(empty) => empty.visit(visitor),
      }
    }
  }
}
