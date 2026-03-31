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
use crate::core::codecs::knn_vectors_reader::KnnVectorsReader;
use crate::core::index::dummy::dummy_byte_vector_values::DummyByteVectorValues;
use crate::core::index::dummy::dummy_float_vector_values::DummyFloatVectorValues;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::util::bits::Bits;

pub struct DummyKnnVectorsReader;
impl KnnVectorsReader for DummyKnnVectorsReader {
  fn check_integrity(&self) -> crate::core::util::error::lucene_error::Result<()> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  type FloatVectorValues = DummyFloatVectorValues;

  fn get_float_vector_values(
    &self,
    _field: &str,
  ) -> crate::core::util::error::lucene_error::Result<Self::FloatVectorValues> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  type ByteVectorValues = DummyByteVectorValues;

  fn get_byte_vector_values(
    &self,
    _field: &str,
  ) -> crate::core::util::error::lucene_error::Result<Self::ByteVectorValues> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn search_f32<B, K>(
    &self,
    _field: &str,
    _target: Vec<f32>,
    _knn_collector: &mut K,
    _accept_docs: Option<B>,
  ) -> crate::core::util::error::lucene_error::Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn search_u8<B, K>(
    &self,
    _field: &str,
    _target: Vec<u8>,
    _knn_collector: &mut K,
    _accept_docs: Option<B>,
  ) -> crate::core::util::error::lucene_error::Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn get_merge_instance(&self) -> crate::core::util::error::lucene_error::Result<Option<Self>>
  where
    Self: Sized,
  {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn finish_merge(&self) -> crate::core::util::error::lucene_error::Result<()> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }
}
