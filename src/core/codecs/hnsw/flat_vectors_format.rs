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
use crate::core::codecs::hnsw::flat_vectors_reader::FlatVectorsReader;
use crate::core::codecs::hnsw::flat_vectors_writer::FlatVectorsWriter;
use crate::core::codecs::knn_vectors_format::KnnVectorsFormat;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IOContext, IndexInput, IndexOutput};
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;
/// Encodes/decodes per-document vectors and provides a scoring interface for the flat stored vectors
pub trait FlatVectorsFormat: KnnVectorsFormat {
  type FlatVectorsWriter<T: IndexInput>: FlatVectorsWriter;
  /// Returns a [`KnnVectorsWriter`] to write the vectors to the index.
  fn fields_writer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field_infos: Arc<FieldInfos>,
    context: &IOContext,
  ) -> Result<Self::FlatVectorsWriter<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory;
  type FlatVectorsReader<T: IndexOutput>: FlatVectorsReader;
  /// Returns a [`KnnVectorsReader`] to write the vectors to the index.
  fn fields_reader<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &mut SegmentInfo<D2>,
  ) -> Result<Self::FlatVectorsReader<D1::IndexOutput>>
  where
    D1: Directory,
    D2: Directory;
}
