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
use crate::core::index::dummy::dummy_doc_index_iterator::DummyDocIndexIterator;
use crate::core::index::dummy::dummy_knn_vector_values::DummyKnnVectorsWriter;
use crate::core::index::knn_vector_values::KnnVectorValues;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::util::bits::Bits;
use crate::core::util::dummy::dummy_bits::DummyBits;
use std::borrow::Cow;

#[derive(Clone)]
pub struct DummyByteVectorValues;

impl KnnVectorValues for DummyByteVectorValues {
  fn dimension(&self) -> usize {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn size(&self) -> usize {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  type KnnVectorValues = DummyKnnVectorsWriter;

  fn get_encoding(&self) -> VectorEncoding {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  type Bits<B>
    = DummyBits
  where
    B: Bits;

  fn get_accept_ords<B>(&self, _accept_docs: Option<B>) -> Option<Self::Bits<B>>
  where
    B: Bits,
  {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  type DocIndexIterator = DummyDocIndexIterator;
}

impl ByteVectorValues for DummyByteVectorValues {
  fn vector_value(
    &self,
    _ord: usize,
  ) -> crate::core::util::error::lucene_error::Result<Cow<'_, VectorValueEnum>> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  type ByteVectorValues = DummyByteVectorValues;

  type VectorScorer = DummyVectorScorer;

  fn scorer(
    &self,
    _query: Vec<u8>,
  ) -> crate::core::util::error::lucene_error::Result<Self::VectorScorer> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn get_encoding(&self) -> VectorEncoding {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }
}
