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
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_writer::EncodedFieldWriter;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::merge_state::MergeState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::sorter::DocMap;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::{VectorSimilarity, VectorSimilarityFunction};
use crate::core::store::directory::Directory;
use crate::core::store::index_output::IndexOutput;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::closeable_random_vector_scorer_supplier::{
  CloseableRandomVectorScorerSupplier, CloseableRandomVectorScorerSupplierEnum2,
};
use std::sync::Arc;

/// Vectors' writer for a field that allows additional indexing logic to be implemented by the caller
pub trait FlatVectorsWriter: KnnVectorsWriter<Self::IndexOutput> {
  type IndexOutput: IndexOutput;
  type FlatVectorsScorer: FlatVectorsScorer;
  fn get_flat_vector_scorer(&self) -> &Self::FlatVectorsScorer;
  /// Add a new field for indexing
  ///
  /// # Arguments
  ///
  /// * `field_info` - fieldInfo of the field to add
  ///
  /// # Returns
  ///
  /// A writer for the field.
  ///
  /// # Errors
  ///
  /// Returns an error if an I/O error occurs when adding the field.
  fn flat_add_field(&mut self, field_info: Arc<FieldInfo>) -> Result<usize>;

  /// Flushes all buffered data on disk.
  fn flat_flush<DM, FS>(
    &mut self,
    max_doc: i32,
    sort_map: Option<&DM>,
    fields: &[EncodedFieldWriter<FS>],
  ) -> Result<()>
  where
    DM: DocMap,
    FS: FlatVectorsScorer;

  type FlatFieldVectorsWriter: FlatFieldVectorsWriter;
  fn get_fields_mut(&mut self) -> &mut [Self::FlatFieldVectorsWriter];

  type ByteMergeScorerSupplier<'a, D, Sim: VectorSimilarity>: CloseableRandomVectorScorerSupplier
  where
    D: Directory,
    Self: 'a,
    D: 'a;
  type FloatMergeScorerSupplier<'a, D, Sim: VectorSimilarity>: CloseableRandomVectorScorerSupplier
  where
    D: Directory,
    Self: 'a,
    D: 'a;

  fn merge_one_field_to_index_byte<'a, D1, D2, CR, Sim: VectorSimilarity>(
    &'a mut self,
    field_info: &FieldInfo,
    merge_state: &MergeState<'_, D1, CR>,
    segment_write_state: &SegmentWriteState<'a, &'a D2>,
    similarity: Sim,
  ) -> Result<Self::ByteMergeScorerSupplier<'a, D2, Sim>>
  where
    D2: Directory<IndexOutput = Self::IndexOutput>,
    CR: CodecReader;

  fn merge_one_field_to_index_float<'a, D1, D2, CR, Sim: VectorSimilarity>(
    &'a mut self,
    field_info: &FieldInfo,
    merge_state: &MergeState<'_, D1, CR>,
    segment_write_state: &SegmentWriteState<'a, &'a D2>,
    similarity: Sim,
  ) -> Result<Self::FloatMergeScorerSupplier<'a, D2, Sim>>
  where
    D2: Directory<IndexOutput = Self::IndexOutput>,
    CR: CodecReader;

  /// Writes the flat vectors and selects the concrete supplier for graph construction.
  fn merge_one_field_to_index<'a, D1, D2, CR, Sim: VectorSimilarity>(
    &'a mut self,
    field_info: &FieldInfo,
    merge_state: &MergeState<'_, D1, CR>,
    segment_write_state: &SegmentWriteState<'a, &'a D2>,
    similarity: Sim,
  ) -> Result<MergeScorerSupplier<'a, Self, D2, Sim>>
  where
    D2: Directory<IndexOutput = Self::IndexOutput>,
    CR: CodecReader,
  {
    match field_info.get_vector_encoding() {
      VectorEncoding::BYTE(_) => self
        .merge_one_field_to_index_byte(field_info, merge_state, segment_write_state, similarity)
        .map(CloseableRandomVectorScorerSupplierEnum2::A),
      VectorEncoding::FLOAT32(_) => self
        .merge_one_field_to_index_float(field_info, merge_state, segment_write_state, similarity)
        .map(CloseableRandomVectorScorerSupplierEnum2::B),
    }
  }
}

pub type FlatVectorsWriterSs<F, BV, FV> =
  crate::core::codecs::hnsw::flat_vectors_scorer::RandomVectorScorerSupplierFor<
    <F as FlatVectorsWriter>::FlatVectorsScorer,
    BV,
    FV,
  >;

pub type MergeScorerSupplier<'a, F, D, Sim = VectorSimilarityFunction> =
  CloseableRandomVectorScorerSupplierEnum2<
    <F as FlatVectorsWriter>::ByteMergeScorerSupplier<'a, D, Sim>,
    <F as FlatVectorsWriter>::FloatMergeScorerSupplier<'a, D, Sim>,
  >;
