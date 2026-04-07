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
use crate::core::codecs::CodecUtil;
use crate::core::codecs::hnsw::flat_field_vectors_writer::FlatFieldVectorsWriter;
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::codecs::hnsw::flat_vectors_writer::FlatVectorsWriter;
use crate::core::codecs::knn_field_vectors_writer::{KnnFieldVectorsWriter, VectorValueEnum};
use crate::core::codecs::knn_vectors_writer::{KnnVectorsWriter, map_old_ord_to_new_ord};
use crate::core::codecs::lucene95::ord_to_doc_disi_reader_configuration::OrdToDocDISIReaderConfiguration;
use crate::core::codecs::lucene99::lucene99_flat_vectors_format::DIRECT_MONOTONIC_BLOCK_SHIFT;
use crate::core::codecs::lucene99::lucene99_flat_vectors_format::{
  META_CODEC_NAME, META_EXTENSION, VECTOR_DATA_CODEC_NAME, VECTOR_DATA_EXTENSION, VERSION_CURRENT,
};
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_writer::{
  DefaultRandomVectorScorerSupplier, FieldWriter,
};
use crate::core::index::IndexFileNames;
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::docs_with_field_set::DocsWithFieldSet;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::knn_vector_values::DocIndexIterator;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::sorter::DocMap;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::store::IndexOutput;
use crate::core::store::directory::Directory;
use crate::core::util::accountable::Accountable;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;
use std::sync::Arc;

/// Writes vector values to index segments.
pub struct Lucene99FlatVectorsWriter<O, F> {
  meta: O,
  vector_data: O,
  fields: Vec<FlatFieldWriter>,
  finished: bool,
  flat_vectors_scorer: F,
}
impl<O, F> Lucene99FlatVectorsWriter<O, F>
where
  O: IndexOutput,
  F: FlatVectorsScorer,
{
  pub fn new<D1, D2>(
    state: &SegmentWriteState<D1>,
    scorer: F,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self>
  where
    D1: Directory<IndexOutput = O>,
    D2: Directory,
  {
    let meta_file_name =
      IndexFileNames::segment_file_name(&segment_info.name, &state.segment_suffix, META_EXTENSION);

    let vector_data_file_name = IndexFileNames::segment_file_name(
      &segment_info.name,
      &state.segment_suffix,
      VECTOR_DATA_EXTENSION,
    );

    let mut meta = state
      .directory
      .create_output(&meta_file_name, state.context)?;
    let mut vector_data = state
      .directory
      .create_output(&vector_data_file_name, state.context)?;

    CodecUtil::write_index_header(
      &mut meta,
      META_CODEC_NAME,
      VERSION_CURRENT,
      segment_info.get_id(),
      &state.segment_suffix,
    )?;
    CodecUtil::write_index_header(
      &mut vector_data,
      VECTOR_DATA_CODEC_NAME,
      VERSION_CURRENT,
      segment_info.get_id(),
      &state.segment_suffix,
    )?;

    Ok(Self {
      meta,
      vector_data,
      fields: Vec::new(),
      finished: false,
      flat_vectors_scorer: scorer,
    })
  }

  fn write_float32_vectors(
    vector_data: &mut O,
    dim: usize,
    vectors: &[VectorValueEnum],
  ) -> Result<()> {
    let byte_size = BitUtil::FLOAT_BYTES;
    let mut buffer = vec![0u8; dim * byte_size];

    for vector in vectors.iter() {
      debug_assert_eq!(vector.len(), dim);
      vector.write_float(&mut buffer)?;
      vector_data.write_bytes_range(&buffer, 0, buffer.len())?;
    }

    Ok(())
  }

  fn write_byte_vectors(vector_data: &mut O, vectors: &[VectorValueEnum]) -> Result<()> {
    for vector in vectors.iter() {
      vector_data.write_bytes_range(vector.as_bytes()?, 0, vector.len())?;
    }

    Ok(())
  }
  fn write_sorted_float32_vectors(
    vector_data: &mut O,
    field_data: &FlatFieldWriter,
    ord_map: &[usize],
    vectors: &[VectorValueEnum],
  ) -> Result<usize>
  where
    O: IndexOutput,
  {
    let vector_data_offset = vector_data.align_file_pointer(BitUtil::FLOAT_BYTES)?;

    let dim = field_data.dim;
    let byte_size = BitUtil::FLOAT_BYTES;
    let mut buffer = vec![0u8; dim * byte_size];

    for &ord in ord_map {
      let vector = &vectors[ord];
      debug_assert_eq!(vector.len(), dim);
      vector.write_float(&mut buffer)?;
      vector_data.write_bytes_range(&buffer, 0, buffer.len())?;
    }

    Ok(vector_data_offset)
  }
  fn write_sorted_byte_vectors(
    vector_data: &mut O,
    ord_map: &[usize],
    vectors: &[VectorValueEnum],
  ) -> Result<usize>
  where
    O: IndexOutput,
  {
    let vector_data_offset = vector_data.align_file_pointer(BitUtil::FLOAT_BYTES)?;
    for &ord in ord_map {
      let vector = &vectors[ord];
      vector_data.write_bytes_range(vector.as_bytes()?, 0, vector.len())?;
    }
    Ok(vector_data_offset)
  }
  fn write_field(
    &mut self,
    field_data_idx: usize,
    max_doc: i32,
    vectors: &[VectorValueEnum],
  ) -> Result<()>
  where
    O: IndexOutput,
  {
    let field_data = self.fields.get_mut(field_data_idx).ok_or_else(|| {
      LuceneError::illegal_argument(format!("Invalid field_data_idx: {}", field_data_idx))
    })?;
    field_data.docs_with_field.finish();
    let vector_data_offset = self.vector_data.align_file_pointer(BitUtil::FLOAT_BYTES)?;

    match field_data.field_info.get_vector_encoding() {
      VectorEncoding::BYTE(_) => {
        Self::write_byte_vectors(&mut self.vector_data, vectors)?;
      },
      VectorEncoding::FLOAT32(_) => {
        Self::write_float32_vectors(&mut self.vector_data, field_data.dim, vectors)?;
      },
    }

    let vector_data_length = self.vector_data.get_file_pointer() - vector_data_offset;

    write_meta(
      &mut self.meta,
      &mut self.vector_data,
      &field_data.field_info,
      max_doc,
      vector_data_offset as i64,
      vector_data_length as i64,
      field_data.get_docs_with_field_set(),
    )?;

    Ok(())
  }
  fn write_sorting_field<DM>(
    &mut self,
    field_data_idx: usize,
    max_doc: i32,
    sort_map: &DM,
    vectors: &[VectorValueEnum],
  ) -> Result<()>
  where
    DM: DocMap,
  {
    let field_data = self.fields.get(field_data_idx).ok_or_else(|| {
      LuceneError::illegal_argument(format!("Invalid field_data_idx: {}", field_data_idx))
    })?;

    let cardinality = field_data.get_docs_with_field_set().cardinality() as usize;

    // new ord -> old ord
    let mut ord_map = vec![0usize; cardinality];

    let mut new_docs_with_field = DocsWithFieldSet::new();

    map_old_ord_to_new_ord(
      field_data.get_docs_with_field_set(),
      sort_map,
      None,
      Some(&mut ord_map),
      Some(&mut new_docs_with_field),
    )?;

    // write vector values
    let vector_data_offset = match field_data.field_info.get_vector_encoding() {
      VectorEncoding::BYTE(_) => {
        Self::write_sorted_byte_vectors(&mut self.vector_data, &ord_map, vectors)?
      },
      VectorEncoding::FLOAT32(_) => {
        Self::write_sorted_float32_vectors(&mut self.vector_data, field_data, &ord_map, vectors)?
      },
    };
    let vector_data_length = self.vector_data.get_file_pointer() - vector_data_offset;

    write_meta(
      &mut self.meta,
      &mut self.vector_data,
      &field_data.field_info,
      max_doc,
      vector_data_offset as i64,
      vector_data_length as i64,
      &new_docs_with_field,
    )?;

    Ok(())
  }
}
impl<O, F> Accountable for Lucene99FlatVectorsWriter<O, F>
where
  F: FlatVectorsScorer,
  O: IndexOutput,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    todo!()
  }
}

impl<O, F> KnnVectorsWriter for Lucene99FlatVectorsWriter<O, F>
where
  O: IndexOutput,
  F: FlatVectorsScorer,
{
  fn finish(&mut self) -> Result<()> {
    if self.finished {
      return Err(LuceneError::illegal_state("already finished"));
    }
    self.finished = true;

    // write end of fields marker
    self.meta.write_int(-1)?;
    CodecUtil::write_footer(&mut self.meta)?;

    CodecUtil::write_footer(&mut self.vector_data)?;

    Ok(())
  }
}

impl<O, F> FlatVectorsWriter for Lucene99FlatVectorsWriter<O, F>
where
  O: IndexOutput,
  F: FlatVectorsScorer,
{
  type FlatVectorsScorer = F;

  fn get_flat_vector_scorer(&self) -> &Self::FlatVectorsScorer {
    &self.flat_vectors_scorer
  }

  fn flat_add_field(&mut self, field_info: Arc<FieldInfo>) -> Result<usize> {
    let len = self.fields.len();
    let new_field = FlatFieldWriter::new(field_info, len);
    self.fields.push(new_field);
    Ok(len)
  }

  fn flat_flush<DM, F1>(
    &mut self,
    max_doc: i32,
    sort_map: Option<&DM>,
    fields: &[FieldWriter<DefaultRandomVectorScorerSupplier<F1>>],
  ) -> Result<()>
  where
    DM: DocMap,
    F1: FlatVectorsWriter,
  {
    for idx in 0..self.fields.len() {
      let fields = &fields[idx];
      let ss = fields.hnsw_graph_builder.get_scorer_supplier();
      let vectors = ss.get_vector()?;
      if let Some(sm) = sort_map {
        self.write_sorting_field(idx, max_doc, sm, vectors)?;
      } else {
        self.write_field(idx, max_doc, vectors)?;
      }
      self.fields[idx].finish()?;
    }
    Ok(())
  }

  type FlatFieldVectorsWriter = FlatFieldWriter;

  fn get_fields_mut(&mut self) -> &mut [Self::FlatFieldVectorsWriter] {
    self.fields.as_mut()
  }
}

fn write_meta<O>(
  meta: &mut O,
  vector_data: &mut O,
  field: &FieldInfo,
  max_doc: i32,
  vector_data_offset: i64,
  vector_data_length: i64,
  docs_with_field: &DocsWithFieldSet,
) -> Result<()>
where
  O: IndexOutput,
{
  meta.write_int(field.number)?;
  meta.write_int(field.get_vector_encoding().ordinal())?;
  meta.write_int(field.get_vector_similarity_function().ordinal())?;

  meta.write_vlong(vector_data_offset)?;
  meta.write_vlong(vector_data_length)?;
  meta.write_vint(field.get_vector_dimension())?;

  // write docIDs
  let count = docs_with_field.cardinality();
  meta.write_int(count)?;
  OrdToDocDISIReaderConfiguration::write_stored_meta(
    DIRECT_MONOTONIC_BLOCK_SHIFT,
    meta,
    vector_data,
    count,
    max_doc,
    docs_with_field,
  )?;

  Ok(())
}
/// Writes the byte vector values to the output and returns a set of documents that contains vectors.
fn write_byte_vector_data<O, V>(
  output: &mut O,
  byte_vector_values: &mut V,
) -> Result<DocsWithFieldSet>
where
  O: IndexOutput,
  V: ByteVectorValues,
{
  let mut docs_with_field = DocsWithFieldSet::new();

  let dim = byte_vector_values.dimension() * VectorEncoding::BYTE(1).byte_size();
  let mut iter = byte_vector_values.iterator()?;

  loop {
    let doc = iter.next_doc()?;
    if doc == NO_MORE_DOCS {
      break;
    }
    let value = byte_vector_values.vector_value(iter.index()? as usize)?;
    debug_assert_eq!(value.len(), dim);
    output.write_bytes_range(value.as_bytes()?, 0, value.len())?;
    docs_with_field.add(doc)?;
  }
  Ok(docs_with_field)
}
/// Writes the vector values to the output and returns a set of documents that contains vectors.
fn write_vector_data<O, V>(output: &mut O, float_vector_values: &mut V) -> Result<DocsWithFieldSet>
where
  O: IndexOutput,
  V: FloatVectorValues,
{
  let mut docs_with_field = DocsWithFieldSet::new();

  let dim = float_vector_values.dimension();
  let byte_size = BitUtil::FLOAT_BYTES;
  let mut buffer = vec![0u8; dim * byte_size];

  let mut iter = float_vector_values.iterator()?;
  loop {
    let doc = iter.next_doc()?;
    if doc == NO_MORE_DOCS {
      break;
    }
    let value = float_vector_values.vector_value(iter.index()? as usize)?;
    for (i, &v) in value.as_floats()?.iter().enumerate() {
      let bytes = v.to_le_bytes();
      let start = i * byte_size;
      buffer[start..start + byte_size].copy_from_slice(&bytes);
    }
    output.write_bytes_range(&buffer, 0, buffer.len())?;
    docs_with_field.add(doc)?;
  }
  Ok(docs_with_field)
}
pub struct FlatFieldWriter {
  field_info: Arc<FieldInfo>,
  dim: usize,
  docs_with_field: DocsWithFieldSet,
  finished: bool,
  last_doc_id: i32,
  idx: usize,
}
impl FlatFieldWriter {
  pub fn new(field_info: Arc<FieldInfo>, idx: usize) -> Self {
    let dim = field_info.get_vector_dimension() as usize;
    Self {
      field_info,
      dim,
      docs_with_field: DocsWithFieldSet::new(),
      finished: false,
      last_doc_id: -1,
      idx,
    }
  }
}

impl Accountable for FlatFieldWriter {
  fn ram_bytes_used(&self) -> Result<i64> {
    // TODO: memory calculation not implement
    Ok(0)
  }
}

impl KnnFieldVectorsWriter for FlatFieldWriter {
  fn copy_value(&self, vector_value: &VectorValueEnum) -> Result<VectorValueEnum> {
    Ok(vector_value.copy_value(0, self.dim))
  }
}
impl FlatFieldVectorsWriter for FlatFieldWriter {
  fn get_docs_with_field_set(&self) -> &DocsWithFieldSet {
    &self.docs_with_field
  }

  fn finish(&mut self) -> Result<()> {
    if self.finished {
      return Ok(());
    }
    self.finished = true;
    Ok(())
  }

  fn is_finished(&self) -> bool {
    self.finished
  }

  fn flat_add_value<F>(
    &mut self,
    doc_id: i32,
    vector_value: &VectorValueEnum,
    vector: &mut Vec<VectorValueEnum>,
  ) -> Result<()> {
    if self.finished {
      return Err(LuceneError::illegal_state(
        "already finished, cannot add more values",
      ));
    }

    if doc_id == self.last_doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "VectorValuesField \"{}\" appears more than once in this document (only one value is allowed per field)",
        self.field_info.name
      )));
    }

    debug_assert!(doc_id > self.last_doc_id);

    let copy = self.copy_value(vector_value)?;

    self.docs_with_field.add(doc_id)?;
    vector.push(copy);

    self.last_doc_id = doc_id;

    Ok(())
  }
}
