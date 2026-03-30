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
use crate::core::codecs::hnsw::flat_field_vectors_writer::FlatFieldVectorsWriter;
use crate::core::codecs::knn_field_vectors_writer::{KnnFieldVectorsWriter, VectorValueEnum};
use crate::core::util::accountable::Accountable;

pub struct DummyKnnFieldVectorsWriter;

impl Accountable for DummyKnnFieldVectorsWriter {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }
}

impl KnnFieldVectorsWriter for DummyKnnFieldVectorsWriter {
  fn add_value<F>(
    &mut self,
    _doc_id: i32,
    _vector_value: &VectorValueEnum,
    _flat_field_vectors_writers: &mut [F],
  ) -> crate::core::util::error::lucene_error::Result<()>
  where
    F: FlatFieldVectorsWriter,
  {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }

  fn copy_value(
    &self,
    _vector_value: &VectorValueEnum,
  ) -> crate::core::util::error::lucene_error::Result<VectorValueEnum> {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }
}
