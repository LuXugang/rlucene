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
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::codecs::knn_vectors_writer::KnnVectorsWriter;
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_writer::{
  DefaultRandomVectorScorerSupplier, FieldWriterType,
};
use crate::core::index::field_info::FieldInfo;
use crate::core::index::sorter::DocMap;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

pub trait FlatVectorsWriter: KnnVectorsWriter {
  type FlatVectorsScorer: FlatVectorsScorer;
  fn get_flat_vector_scorer(&self) -> &Self::FlatVectorsScorer;

  fn flat_add_field(&mut self, field_info: Arc<FieldInfo>) -> Result<usize>;

  /// Flushes all buffered data on disk.
  fn flat_flush<DM, F, V>(
    &mut self,
    max_doc: i32,
    sort_map: Option<&DM>,
    fields: &[FieldWriterType<DefaultRandomVectorScorerSupplier<F>, V>],
  ) -> Result<()>
  where
    DM: DocMap,
    F: FlatVectorsWriter,
    V: Clone;

  type FlatFieldVectorsWriter: FlatFieldVectorsWriter;
  fn get_fields_mut(&mut self) -> &mut [Self::FlatFieldVectorsWriter];
}

pub type FlatVectorsWriterSs<F, BV, FV> =
  <<F as FlatVectorsWriter>::FlatVectorsScorer as FlatVectorsScorer>::RandomVectorScorerSupplier<
    BV,
    FV,
  >;
