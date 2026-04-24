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
use crate::core::index::dummy::dummy_doc_index_iterator::DummyDocIndexIterator;
use crate::core::index::dummy::dummy_knn_vector_values::DummyKnnVectorsWriter;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::knn_vector_values::KnnVectorValues;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::util::bits::Bits;
use crate::core::util::dummy::dummy_bits::DummyBits;
use std::borrow::Cow;

#[derive(Clone)]
pub struct DummyFloatVectorValues;

impl KnnVectorValues for DummyFloatVectorValues {
  fn dimension(&self) -> usize {
    dummy_unreachable!()
  }
  fn size(&self) -> usize {
    dummy_unreachable!()
  }
  fn ord_to_doc(&self, _ord: usize) -> crate::core::util::error::lucene_error::Result<usize> {
    dummy_unreachable!()
  }

  type KnnVectorValues = DummyKnnVectorsWriter;

  fn get_encoding(&self) -> VectorEncoding {
    dummy_unreachable!()
  }

  type Bits<'a, B>
    = DummyBits
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, _accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    dummy_unreachable!()
  }

  type DocIndexIterator = DummyDocIndexIterator;
}

impl FloatVectorValues for DummyFloatVectorValues {
  fn vector_value(
    &self,
    _ord: usize,
  ) -> crate::core::util::error::lucene_error::Result<Cow<'_, VectorValueEnum>> {
    dummy_unreachable!()
  }

  type FloatVectorValues = Self;

  fn float_copy(
    &self,
  ) -> crate::core::util::error::lucene_error::Result<Option<Self::FloatVectorValues>> {
    dummy_unreachable!()
  }

  type VectorScorer = DummyVectorScorer;

  fn scorer(
    &self,
    _target: Vec<f32>,
  ) -> crate::core::util::error::lucene_error::Result<Option<Self::VectorScorer>> {
    dummy_unreachable!()
  }

  fn get_encoding(&self) -> VectorEncoding {
    dummy_unreachable!()
  }
}
