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
use crate::core::codecs::hnsw::flat_vectors_writer::FlatVectorsWriter;
use crate::core::codecs::knn_field_vectors_writer::{KnnFieldVectorsWriter, VectorValueEnum};
use crate::core::codecs::knn_vectors_writer::KnnVectorsWriter;
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_writer::{
  DefaultRandomVectorScorerSupplier, FieldWriter,
};
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::docs_with_field_set::DocsWithFieldSet;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::merge_state::MergeState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::sorter::DocMap;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::accountable::Accountable;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::closeable_random_vector_scorer_supplier::CloseableRandomVectorScorerSupplier;
use crate::core::util::hnsw::dummy::dummy_random_vector_scorer::DummyRandomVectorScorer;
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;
use std::marker::PhantomData;
use std::sync::Arc;

/// Writes scalar quantized vector values to index segments.
pub struct Lucene99ScalarQuantizedVectorsWriter<O, F>
where
  O: IndexOutput,
  F: FlatVectorsScorer,
{
  flat_vector_scorer: F,
  _output: PhantomData<O>,
}

impl<O, F> Lucene99ScalarQuantizedVectorsWriter<O, F>
where
  O: IndexOutput,
  F: FlatVectorsScorer,
{
  #[allow(dead_code)]
  pub(crate) fn new(flat_vector_scorer: F) -> Self {
    Self {
      flat_vector_scorer,
      _output: PhantomData,
    }
  }
}

impl<O, F> Accountable for Lucene99ScalarQuantizedVectorsWriter<O, F>
where
  O: IndexOutput,
  F: FlatVectorsScorer,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }
}

impl<O, F> Closeable for Lucene99ScalarQuantizedVectorsWriter<O, F>
where
  O: IndexOutput,
  F: FlatVectorsScorer,
{
}

impl<O, F> KnnVectorsWriter for Lucene99ScalarQuantizedVectorsWriter<O, F>
where
  O: IndexOutput,
  F: FlatVectorsScorer,
{
}

impl<O, F> FlatVectorsWriter for Lucene99ScalarQuantizedVectorsWriter<O, F>
where
  O: IndexOutput,
  F: FlatVectorsScorer,
{
  type FlatVectorsScorer = F;

  fn get_flat_vector_scorer(&self) -> &Self::FlatVectorsScorer {
    &self.flat_vector_scorer
  }

  fn flat_add_field(&mut self, _field_info: Arc<FieldInfo>) -> Result<usize> {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }

  fn flat_flush<DM, F1>(
    &mut self,
    _max_doc: i32,
    _sort_map: Option<&DM>,
    _fields: &[FieldWriter<DefaultRandomVectorScorerSupplier<F1>>],
  ) -> Result<()>
  where
    DM: DocMap,
    F1: FlatVectorsWriter,
  {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }

  type FlatFieldVectorsWriter = Lucene99ScalarQuantizedFieldVectorsWriter;

  fn get_fields_mut(&mut self) -> &mut [Self::FlatFieldVectorsWriter] {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }

  type CloseableRandomVectorScorerSupplier<'a, I, D>
    = Lucene99ScalarQuantizedCloseableRandomVectorScorerSupplier
  where
    I: IndexInput + 'a,
    D: Directory,
    Self: 'a,
    D: 'a,
    I: 'a;

  fn merge_one_field_to_index<'a, D1, D2, CR>(
    &'a mut self,
    _field_info: &FieldInfo,
    _merge_state: &MergeState<'_, D1, CR>,
    _segment_write_state: &SegmentWriteState<'a, &D2>,
  ) -> Result<Self::CloseableRandomVectorScorerSupplier<'a, D2::IndexInput, D2>>
  where
    D1: Directory,
    D2: Directory,
    CR: CodecReader,
  {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }
}

pub struct Lucene99ScalarQuantizedFieldVectorsWriter;

impl Accountable for Lucene99ScalarQuantizedFieldVectorsWriter {
  fn ram_bytes_used(&self) -> Result<i64> {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }
}

impl KnnFieldVectorsWriter for Lucene99ScalarQuantizedFieldVectorsWriter {
  fn copy_value(&self, _vector_value: &VectorValueEnum) -> Result<VectorValueEnum> {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }
}

impl FlatFieldVectorsWriter for Lucene99ScalarQuantizedFieldVectorsWriter {
  fn get_docs_with_field_set(&self) -> &DocsWithFieldSet {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }

  fn finish(&mut self) -> Result<()> {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }

  fn is_finished(&self) -> bool {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }

  fn flat_add_value<F>(
    &mut self,
    _doc_id: i32,
    _vector_value: &VectorValueEnum,
    _vector: &mut Vec<VectorValueEnum>,
  ) -> Result<()> {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }
}

pub struct Lucene99ScalarQuantizedCloseableRandomVectorScorerSupplier;

impl RandomVectorScorerSupplier for Lucene99ScalarQuantizedCloseableRandomVectorScorerSupplier {
  type Scorer<'a>
    = DummyRandomVectorScorer
  where
    Self: 'a;

  fn scorer(&self, _ord: usize) -> Result<Self::Scorer<'_>> {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }

  type RandomVectorScorerSupplier = Self;

  fn copy(&self) -> Result<Self::RandomVectorScorerSupplier>
  where
    Self: Sized,
  {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }
}

impl Closeable for Lucene99ScalarQuantizedCloseableRandomVectorScorerSupplier {}

impl CloseableRandomVectorScorerSupplier
  for Lucene99ScalarQuantizedCloseableRandomVectorScorerSupplier
{
  fn total_vector_count(&self) -> Result<i32> {
    todo!("Lucene99ScalarQuantizedVectorsWriter is not implemented yet")
  }
}
