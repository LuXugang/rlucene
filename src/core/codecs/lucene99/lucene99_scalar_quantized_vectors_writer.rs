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
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::codecs::knn_vectors_writer::{KnnVectorsWriter, map_old_ord_to_new_ord};
use crate::core::codecs::lucene95::ord_to_doc_disi_reader_configuration::OrdToDocDISIReaderConfiguration;
use crate::core::codecs::lucene99::lucene99_flat_vectors_format::DIRECT_MONOTONIC_BLOCK_SHIFT;
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_writer::{
  DefaultRandomVectorScorerSupplier, FieldWriter as HnswFieldWriter,
};
use crate::core::codecs::lucene99::lucene99_scalar_quantized_vectors_format::{
  DYNAMIC_CONFIDENCE_INTERVAL, Lucene99ScalarQuantizedVectorsFormat, META_CODEC_NAME,
  META_EXTENSION, QUANTIZED_VECTOR_COMPONENT, VECTOR_DATA_CODEC_NAME, VECTOR_DATA_EXTENSION,
  VERSION_ADD_BITS, VERSION_CURRENT,
};
use crate::core::codecs::lucene99::off_heap_quantized_byte_vector_values::{
  compress_bytes, compressed_array,
};
use crate::core::index::IndexFileNames;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::docs_with_field_set::DocsWithFieldSet;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::knn_vector_values::{
  BitsImpl1, DenseDocIndexIterator, DocIndexIterator, KnnVectorValues,
};
use crate::core::index::merge_state::MergeState;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::sorter::DocMap;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::TryIntoInt;
use crate::core::util::accountable::Accountable;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::bits::Bits;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::closeable_random_vector_scorer_supplier::CloseableRandomVectorScorerSupplier;
use crate::core::util::hnsw::dummy::dummy_random_vector_scorer::DummyRandomVectorScorer;
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;
use crate::core::util::info_stream::{InfoStream, InfoStreamMT};
use crate::core::util::io_utils::IOUtils;
use crate::core::util::quantization::quantized_byte_vector_values::QuantizedByteVectorValues;
use crate::core::util::quantization::scalar_quantizer::ScalarQuantizer;
use crate::core::util::ram_usage_estimator::size_of_vec;
use crate::core::util::vector_util::VectorUtil;
use std::borrow::Cow;
use std::sync::Arc;

// Used for determining when merged quantiles shifted too far from individual segment quantiles.
// When merging quantiles from various segments, we need to ensure that the new quantiles
// are not exceptionally different from an individual segments quantiles.
// This would imply that the quantization buckets would shift too much
// for floating point values and justify recalculating the quantiles. This helps preserve
// accuracy of the calculated quantiles, even in adversarial cases such as vector clustering.
// This number was determined via empirical testing
const QUANTILE_RECOMPUTE_LIMIT: f32 = 32.0;
// Used for determining if a new quantization state requires a re-quantization
// for a given segment.
// This ensures that in expectation 4/5 of the vector would be unchanged by requantization.
// Furthermore, only those values where the value is within 1/5 of the centre of a quantization
// bin will be changed. In these cases the error introduced by snapping one way or another
// is small compared to the error introduced by quantization in the first place. Furthermore,
// empirical testing showed that the relative error by not requantizing is small (compared to
// the quantization error) and the condition is sensitive enough to detect all adversarial cases,
// such as merging clustered data.
const REQUANTIZATION_LIMIT: f32 = 0.2;

/// Writes quantized vector values and metadata to index segments.
pub struct Lucene99ScalarQuantizedVectorsWriter<O, R, F>
where
  O: IndexOutput,
  R: FlatVectorsWriter,
  F: FlatVectorsScorer,
{
  fields: Vec<ScalarQuantizedFieldWriter>,
  meta: O,
  quantized_vector_data: O,
  confidence_interval: Option<f32>,
  raw_vector_delegate: R,
  flat_vector_scorer: F,
  bits: u8,
  compress: bool,
  version: i32,
  finished: bool,
  info_stream: InfoStreamMT,
}

impl<O, R, F> Lucene99ScalarQuantizedVectorsWriter<O, R, F>
where
  O: IndexOutput,
  R: FlatVectorsWriter,
  F: FlatVectorsScorer,
{
  pub fn new<D1, D2>(
    state: &SegmentWriteState<D1>,
    confidence_interval: Option<f32>,
    bits: u8,
    compress: bool,
    raw_vector_delegate: R,
    flat_vector_scorer: F,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self>
  where
    D1: Directory<IndexOutput = O>,
    D2: Directory,
  {
    Self::with_version(
      state,
      VERSION_CURRENT,
      confidence_interval,
      bits,
      compress,
      raw_vector_delegate,
      flat_vector_scorer,
      segment_info,
    )
  }

  #[allow(clippy::too_many_arguments)]
  fn with_version<D1, D2>(
    state: &SegmentWriteState<D1>,
    version: i32,
    confidence_interval: Option<f32>,
    bits: u8,
    compress: bool,
    mut raw_vector_delegate: R,
    flat_vector_scorer: F,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self>
  where
    D1: Directory<IndexOutput = O>,
    D2: Directory,
  {
    let meta_file_name =
      IndexFileNames::segment_file_name(&segment_info.name, &state.segment_suffix, META_EXTENSION);

    let quantized_vector_data_file_name = IndexFileNames::segment_file_name(
      &segment_info.name,
      &state.segment_suffix,
      VECTOR_DATA_EXTENSION,
    );

    let mut meta = match state
      .directory
      .create_output(&meta_file_name, state.context)
    {
      Ok(meta) => meta,
      Err(err) => {
        return IOUtils::use_or_suppress_result::<Self>(Err(err), raw_vector_delegate.close());
      },
    };
    let mut quantized_vector_data = match state
      .directory
      .create_output(&quantized_vector_data_file_name, state.context)
    {
      Ok(quantized_vector_data) => quantized_vector_data,
      Err(err) => {
        let output_close_result = IOUtils::close(std::iter::once(&mut meta), Closeable::close);
        let close_result =
          IOUtils::use_or_suppress_result(output_close_result, raw_vector_delegate.close());
        return IOUtils::use_or_suppress_result::<Self>(Err(err), close_result);
      },
    };

    let result = (|| -> Result<()> {
      CodecUtil::write_index_header(
        &mut meta,
        META_CODEC_NAME,
        version,
        segment_info.get_id(),
        &state.segment_suffix,
      )?;
      CodecUtil::write_index_header(
        &mut quantized_vector_data,
        VECTOR_DATA_CODEC_NAME,
        version,
        segment_info.get_id(),
        &state.segment_suffix,
      )?;
      Ok(())
    })();

    if let Err(err) = result {
      let output_close_result =
        IOUtils::close([&mut meta, &mut quantized_vector_data], Closeable::close);
      let close_result =
        IOUtils::use_or_suppress_result(output_close_result, raw_vector_delegate.close());
      return IOUtils::use_or_suppress_result::<Self>(Err(err), close_result);
    }

    Ok(Self {
      fields: Vec::new(),
      meta,
      quantized_vector_data,
      confidence_interval,
      raw_vector_delegate,
      flat_vector_scorer,
      bits,
      compress,
      version,
      finished: false,
      info_stream: state.info_stream.clone(),
    })
  }
  #[allow(clippy::too_many_arguments)]
  fn write_field<FW>(
    meta: &mut O,
    quantized_vector_data: &mut O,
    field_data: &ScalarQuantizedFieldWriter,
    flat_field_vectors_writers: &mut [FW],
    max_doc: i32,
    vectors: &[VectorValueEnum],
    scalar_quantizer: &ScalarQuantizer,
    version: i32,
  ) -> Result<()>
  where
    FW: FlatFieldVectorsWriter,
  {
    // write vector values
    let vector_data_offset = quantized_vector_data.align_file_pointer(BitUtil::FLOAT_BYTES)?;
    write_quantized_vectors(quantized_vector_data, field_data, vectors, scalar_quantizer)?;
    let vector_data_length = quantized_vector_data.get_file_pointer()? - vector_data_offset;

    write_meta(
      meta,
      quantized_vector_data,
      field_data.field_info.as_ref(),
      max_doc,
      vector_data_offset as i64,
      vector_data_length as i64,
      field_data.confidence_interval,
      field_data.bits,
      field_data.compress,
      scalar_quantizer.get_lower_quantile(),
      scalar_quantizer.get_upper_quantile(),
      field_data.get_docs_with_field_set(flat_field_vectors_writers)?,
      version,
    )
  }
  #[allow(clippy::too_many_arguments)]
  fn write_sorting_field<DM, FW>(
    meta: &mut O,
    quantized_vector_data: &mut O,
    field_data: &ScalarQuantizedFieldWriter,
    flat_field_vectors_writers: &mut [FW],
    max_doc: i32,
    sort_map: &DM,
    vectors: &[VectorValueEnum],
    scalar_quantizer: &ScalarQuantizer,
    version: i32,
  ) -> Result<()>
  where
    DM: DocMap,
    FW: FlatFieldVectorsWriter,
  {
    let docs_with_field = field_data.get_docs_with_field_set(flat_field_vectors_writers)?;
    let mut ord_map = vec![0usize; docs_with_field.cardinality() as usize]; // new ord to old ord

    let mut new_docs_with_field = DocsWithFieldSet::new();
    map_old_ord_to_new_ord(
      docs_with_field,
      sort_map,
      None,
      Some(&mut ord_map),
      Some(&mut new_docs_with_field),
    )?;
    new_docs_with_field.finish();

    // write vector values
    let vector_data_offset = quantized_vector_data.align_file_pointer(BitUtil::FLOAT_BYTES)?;
    write_sorted_quantized_vectors(
      quantized_vector_data,
      field_data,
      &ord_map,
      vectors,
      scalar_quantizer,
    )?;
    let quantized_vector_length = quantized_vector_data.get_file_pointer()? - vector_data_offset;
    write_meta(
      meta,
      quantized_vector_data,
      field_data.field_info.as_ref(),
      max_doc,
      vector_data_offset as i64,
      quantized_vector_length as i64,
      field_data.confidence_interval,
      field_data.bits,
      field_data.compress,
      scalar_quantizer.get_lower_quantile(),
      scalar_quantizer.get_upper_quantile(),
      &new_docs_with_field,
      version,
    )
  }
}

impl<O, R, F> Accountable for Lucene99ScalarQuantizedVectorsWriter<O, R, F>
where
  O: IndexOutput,
  R: FlatVectorsWriter,
  F: FlatVectorsScorer,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    let mut total = self
      .raw_vector_delegate
      .ram_bytes_used()?
      .saturating_add(size_of_vec(&self.fields));
    for field in &self.fields {
      total = total.saturating_add(field.ram_bytes_used()?);
    }
    Ok(total)
  }
}

impl<O, R, F> Closeable for Lucene99ScalarQuantizedVectorsWriter<O, R, F>
where
  O: IndexOutput,
  R: FlatVectorsWriter,
  F: FlatVectorsScorer,
{
  fn close(&mut self) -> Result<()> {
    let output_close_result = IOUtils::close(
      [&mut self.meta, &mut self.quantized_vector_data],
      Closeable::close,
    );
    IOUtils::use_or_suppress_result(output_close_result, self.raw_vector_delegate.close())
  }
}

impl<O, R, F> KnnVectorsWriter for Lucene99ScalarQuantizedVectorsWriter<O, R, F>
where
  O: IndexOutput,
  R: FlatVectorsWriter,
  F: FlatVectorsScorer,
{
  fn merge_one_field<D1, D2, CR>(
    &mut self,
    _field_info: &Arc<FieldInfo>,
    _merge_state: &MergeState<'_, D1, CR>,
    _segment_write_state: &SegmentWriteState<&D2>,
  ) -> Result<()>
  where
    D1: Directory,
    D2: Directory,
    CR: CodecReader,
  {
    Err(LuceneError::unsupported_operation(
      "Lucene99ScalarQuantizedVectorsWriter merge is not implemented yet",
    ))
  }

  fn finish(&mut self) -> Result<()> {
    if self.finished {
      return Err(LuceneError::illegal_state("already finished"));
    }
    self.finished = true;
    self.raw_vector_delegate.finish()?;
    // write end of fields marker
    self.meta.write_int(-1)?;
    CodecUtil::write_footer(&mut self.meta)?;

    CodecUtil::write_footer(&mut self.quantized_vector_data)?;

    Ok(())
  }
}

impl<O, R, F> FlatVectorsWriter for Lucene99ScalarQuantizedVectorsWriter<O, R, F>
where
  O: IndexOutput,
  R: FlatVectorsWriter,
  F: FlatVectorsScorer,
{
  type FlatVectorsScorer = F;

  fn get_flat_vector_scorer(&self) -> &Self::FlatVectorsScorer {
    &self.flat_vector_scorer
  }

  fn flat_add_field(&mut self, field_info: Arc<FieldInfo>) -> Result<usize> {
    let raw_vector_delegate = self
      .raw_vector_delegate
      .flat_add_field(field_info.clone())?;
    if *field_info.get_vector_encoding() == VectorEncoding::FLOAT32(BitUtil::FLOAT_BYTES) {
      if self.bits <= 4 && field_info.get_vector_dimension() % 2 != 0 {
        return Err(LuceneError::illegal_argument(format!(
          "bits={} is not supported for odd vector dimensions; vector dimension={}",
          self.bits,
          field_info.get_vector_dimension()
        )));
      }
      let quantized_writer = ScalarQuantizedFieldWriter::new(
        self.confidence_interval,
        self.bits,
        self.compress,
        field_info,
        self.info_stream.clone(),
        raw_vector_delegate,
      );
      self.fields.push(quantized_writer);
    }
    Ok(raw_vector_delegate)
  }

  fn flat_flush<DM, F1>(
    &mut self,
    max_doc: i32,
    sort_map: Option<&DM>,
    fields: &[HnswFieldWriter<DefaultRandomVectorScorerSupplier<F1>>],
  ) -> Result<()>
  where
    DM: DocMap,
    F1: FlatVectorsWriter,
  {
    self
      .raw_vector_delegate
      .flat_flush::<DM, F1>(max_doc, sort_map, fields)?;

    for field_idx in 0..self.fields.len() {
      let field = &self.fields[field_idx];
      let vectors = fields
        .get(field.flat_field_vectors_writer_idx)
        .ok_or_else(|| LuceneError::illegal_state("Invalid flat field vectors writer index"))?
        .hnsw_graph_builder
        .get_scorer_supplier()
        .get_vector()?;
      let scalar_quantizer = {
        let flat_field_vectors_writers = self.raw_vector_delegate.get_fields_mut();
        field.create_quantizer(flat_field_vectors_writers, vectors)?
      };
      let flat_field_vectors_writers = self.raw_vector_delegate.get_fields_mut();
      if let Some(sm) = sort_map {
        Self::write_sorting_field(
          &mut self.meta,
          &mut self.quantized_vector_data,
          field,
          flat_field_vectors_writers,
          max_doc,
          sm,
          vectors,
          &scalar_quantizer,
          self.version,
        )?;
      } else {
        Self::write_field(
          &mut self.meta,
          &mut self.quantized_vector_data,
          field,
          flat_field_vectors_writers,
          max_doc,
          vectors,
          &scalar_quantizer,
          self.version,
        )?;
      }
      self.fields[field_idx].finish(self.raw_vector_delegate.get_fields_mut())?;
    }

    Ok(())
  }

  type FlatFieldVectorsWriter = R::FlatFieldVectorsWriter;

  fn get_fields_mut(&mut self) -> &mut [Self::FlatFieldVectorsWriter] {
    self.raw_vector_delegate.get_fields_mut()
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
    Err(LuceneError::unsupported_operation(
      "Lucene99ScalarQuantizedVectorsWriter mergeOneFieldToIndex is not implemented yet",
    ))
  }
}

#[allow(clippy::too_many_arguments)]
fn write_meta<O>(
  meta: &mut O,
  quantized_vector_data: &mut O,
  field: &FieldInfo,
  max_doc: i32,
  vector_data_offset: i64,
  vector_data_length: i64,
  confidence_interval: Option<f32>,
  bits: u8,
  compress: bool,
  lower_quantile: f32,
  upper_quantile: f32,
  docs_with_field: &DocsWithFieldSet,
  version: i32,
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
  let count = docs_with_field.cardinality();
  meta.write_int(count)?;
  if count > 0 {
    debug_assert!(lower_quantile.is_finite() && upper_quantile.is_finite());
    if version >= VERSION_ADD_BITS {
      meta.write_int(
        confidence_interval
          .map(|value| value.to_bits() as i32)
          .unwrap_or(-1),
      )?;
      meta.write_byte(bits)?;
      meta.write_byte(if compress { 1 } else { 0 })?;
    } else {
      debug_assert!(
        confidence_interval.is_none() || confidence_interval != Some(DYNAMIC_CONFIDENCE_INTERVAL)
      );
      let confidence_interval = confidence_interval.unwrap_or_else(|| {
        Lucene99ScalarQuantizedVectorsFormat::calculate_default_confidence_interval(
          field.get_vector_dimension() as usize,
        )
      });
      meta.write_int(confidence_interval.to_bits() as i32)?;
    }
    meta.write_int(lower_quantile.to_bits() as i32)?;
    meta.write_int(upper_quantile.to_bits() as i32)?;
  }
  // write docIDs
  OrdToDocDISIReaderConfiguration::write_stored_meta(
    DIRECT_MONOTONIC_BLOCK_SHIFT,
    meta,
    quantized_vector_data,
    count,
    max_doc,
    docs_with_field,
  )
}

fn write_quantized_vectors<O>(
  quantized_vector_data: &mut O,
  field_data: &ScalarQuantizedFieldWriter,
  vectors: &[VectorValueEnum],
  scalar_quantizer: &ScalarQuantizer,
) -> Result<()>
where
  O: IndexOutput,
{
  let mut vector = vec![0u8; field_data.field_info.get_vector_dimension() as usize];
  let mut compressed_vector = if field_data.compress {
    compressed_array(
      field_data.field_info.get_vector_dimension() as usize,
      field_data.bits,
    )
  } else {
    None
  };
  let mut copy = if field_data.normalize {
    Some(vec![
      0f32;
      field_data.field_info.get_vector_dimension() as usize
    ])
  } else {
    None
  };
  debug_assert!(vectors.is_empty() || scalar_quantizer.get_bits() == field_data.bits);
  for v in vectors {
    let borrowed;
    let vector_value = if field_data.normalize {
      let copy = copy
        .as_mut()
        .ok_or_else(|| LuceneError::illegal_state("missing normalized vector buffer"))?;
      copy.copy_from_slice(v.as_floats()?);
      VectorUtil::l2normalize(copy)?;
      borrowed = copy.as_slice();
      borrowed
    } else {
      v.as_floats()?
    };

    let offset_correction = scalar_quantizer.quantize(
      vector_value,
      &mut vector,
      *field_data.field_info.get_vector_similarity_function(),
    );
    if let Some(compressed_vector) = compressed_vector.as_mut() {
      compress_bytes(&vector, compressed_vector)?;
      quantized_vector_data.write_bytes_range(compressed_vector, 0, compressed_vector.len())?;
    } else {
      quantized_vector_data.write_bytes_range(&vector, 0, vector.len())?;
    }
    let offset_buffer = offset_correction.to_le_bytes();
    quantized_vector_data.write_bytes_range(&offset_buffer, 0, offset_buffer.len())?;
  }
  Ok(())
}

fn write_sorted_quantized_vectors<O>(
  quantized_vector_data: &mut O,
  field_data: &ScalarQuantizedFieldWriter,
  ord_map: &[usize],
  vectors: &[VectorValueEnum],
  scalar_quantizer: &ScalarQuantizer,
) -> Result<()>
where
  O: IndexOutput,
{
  let mut vector = vec![0u8; field_data.field_info.get_vector_dimension() as usize];
  let mut compressed_vector = if field_data.compress {
    compressed_array(
      field_data.field_info.get_vector_dimension() as usize,
      field_data.bits,
    )
  } else {
    None
  };
  let mut copy = if field_data.normalize {
    Some(vec![
      0f32;
      field_data.field_info.get_vector_dimension() as usize
    ])
  } else {
    None
  };
  for &ordinal in ord_map {
    let v = vectors
      .get(ordinal)
      .ok_or_else(|| LuceneError::illegal_state("Invalid vector ordinal"))?;
    let borrowed;
    let vector_value = if field_data.normalize {
      let copy = copy
        .as_mut()
        .ok_or_else(|| LuceneError::illegal_state("missing normalized vector buffer"))?;
      copy.copy_from_slice(v.as_floats()?);
      VectorUtil::l2normalize(copy)?;
      borrowed = copy.as_slice();
      borrowed
    } else {
      v.as_floats()?
    };
    let offset_correction = scalar_quantizer.quantize(
      vector_value,
      &mut vector,
      *field_data.field_info.get_vector_similarity_function(),
    );
    if let Some(compressed_vector) = compressed_vector.as_mut() {
      compress_bytes(&vector, compressed_vector)?;
      quantized_vector_data.write_bytes_range(compressed_vector, 0, compressed_vector.len())?;
    } else {
      quantized_vector_data.write_bytes_range(&vector, 0, vector.len())?;
    }
    let offset_buffer = offset_correction.to_le_bytes();
    quantized_vector_data.write_bytes_range(&offset_buffer, 0, offset_buffer.len())?;
  }
  Ok(())
}

/// Writes the vector values to the output and returns a set of documents that contains vectors.
pub fn write_quantized_vector_data<O, Q>(
  output: &mut O,
  quantized_byte_vector_values: &Q,
  bits: u8,
  compress: bool,
) -> Result<DocsWithFieldSet>
where
  O: IndexOutput,
  Q: QuantizedByteVectorValues,
{
  let mut docs_with_field = DocsWithFieldSet::new();
  let mut compressed_vector = if compress {
    compressed_array(quantized_byte_vector_values.dimension(), bits)
  } else {
    None
  };
  let mut iter = quantized_byte_vector_values.iterator()?;
  loop {
    let doc = iter.next_doc()?;
    if doc == NO_MORE_DOCS {
      break;
    }
    // write vector
    let ord: usize = iter.index()?.try_convert()?;
    let binary_value = quantized_byte_vector_values.vector_value(ord)?;
    let binary_value = binary_value.as_bytes()?;
    debug_assert_eq!(
      binary_value.len(),
      quantized_byte_vector_values.dimension(),
      "dim={} len={}",
      quantized_byte_vector_values.dimension(),
      binary_value.len()
    );
    if let Some(compressed_vector) = compressed_vector.as_mut() {
      compress_bytes(binary_value, compressed_vector)?;
      output.write_bytes_range(compressed_vector, 0, compressed_vector.len())?;
    } else {
      output.write_bytes_range(binary_value, 0, binary_value.len())?;
    }
    output.write_int(
      quantized_byte_vector_values
        .get_score_correction_constant(ord)?
        .to_bits() as i32,
    )?;
    docs_with_field.add(doc)?;
  }
  docs_with_field.finish();
  Ok(docs_with_field)
}

pub struct ScalarQuantizedFieldWriter {
  field_info: Arc<FieldInfo>,
  confidence_interval: Option<f32>,
  bits: u8,
  compress: bool,
  info_stream: InfoStreamMT,
  normalize: bool,
  finished: bool,
  flat_field_vectors_writer_idx: usize,
}

impl ScalarQuantizedFieldWriter {
  fn new(
    confidence_interval: Option<f32>,
    bits: u8,
    compress: bool,
    field_info: Arc<FieldInfo>,
    info_stream: InfoStreamMT,
    flat_field_vectors_writer_idx: usize,
  ) -> Self {
    Self {
      confidence_interval,
      bits,
      normalize: *field_info.get_vector_similarity_function() == VectorSimilarityFunction::Cosine,
      field_info,
      info_stream,
      compress,
      finished: false,
      flat_field_vectors_writer_idx,
    }
  }

  fn is_finished<FW>(&self, flat_field_vectors_writers: &mut [FW]) -> Result<bool>
  where
    FW: FlatFieldVectorsWriter,
  {
    Ok(
      self.finished && {
        let flat_field_vectors_writer = flat_field_vectors_writers
          .get(self.flat_field_vectors_writer_idx)
          .ok_or_else(|| LuceneError::illegal_state("Invalid flat field vectors writer index"))?;
        flat_field_vectors_writer.is_finished()
      },
    )
  }

  fn finish<FW>(&mut self, flat_field_vectors_writers: &mut [FW]) -> Result<()>
  where
    FW: FlatFieldVectorsWriter,
  {
    if self.finished {
      return Ok(());
    }
    debug_assert!({
      let flat_field_vectors_writer = flat_field_vectors_writers
        .get(self.flat_field_vectors_writer_idx)
        .ok_or_else(|| LuceneError::illegal_state("Invalid flat field vectors writer index"))?;
      flat_field_vectors_writer.is_finished()
    });
    self.finished = true;
    Ok(())
  }

  fn create_quantizer<FW>(
    &self,
    flat_field_vectors_writers: &mut [FW],
    vectors: &[VectorValueEnum],
  ) -> Result<ScalarQuantizer>
  where
    FW: FlatFieldVectorsWriter,
  {
    debug_assert!({
      let flat_field_vectors_writer = flat_field_vectors_writers
        .get(self.flat_field_vectors_writer_idx)
        .ok_or_else(|| LuceneError::illegal_state("Invalid flat field vectors writer index"))?;
      flat_field_vectors_writer.is_finished()
    });
    if vectors.is_empty() {
      return ScalarQuantizer::new(0.0, 0.0, self.bits);
    }
    let quantizer = build_scalar_quantizer(
      FloatVectorWrapper::new(vectors),
      vectors.len(),
      *self.field_info.get_vector_similarity_function(),
      self.confidence_interval,
      self.bits,
    )?;
    if self.info_stream.is_enabled(QUANTIZED_VECTOR_COMPONENT) {
      self.info_stream.message(
        QUANTIZED_VECTOR_COMPONENT,
        &format!(
          "quantized field= confidenceInterval={:?} bits={} minQuantile={} maxQuantile={}",
          self.confidence_interval,
          self.bits,
          quantizer.get_lower_quantile(),
          quantizer.get_upper_quantile()
        ),
      )?;
    }
    Ok(quantizer)
  }

  fn get_docs_with_field_set<'a, FW>(
    &self,
    flat_field_vectors_writers: &'a mut [FW],
  ) -> Result<&'a DocsWithFieldSet>
  where
    FW: FlatFieldVectorsWriter,
  {
    let flat_field_vectors_writer = flat_field_vectors_writers
      .get(self.flat_field_vectors_writer_idx)
      .ok_or_else(|| LuceneError::illegal_state("Invalid flat field vectors writer index"))?;
    Ok(flat_field_vectors_writer.get_docs_with_field_set())
  }
}

impl Accountable for ScalarQuantizedFieldWriter {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(0)
  }
}

fn build_scalar_quantizer<FVV>(
  float_vector_values: FVV,
  num_vectors: usize,
  vector_similarity_function: VectorSimilarityFunction,
  confidence_interval: Option<f32>,
  bits: u8,
) -> Result<ScalarQuantizer>
where
  FVV: FloatVectorValues,
{
  if vector_similarity_function == VectorSimilarityFunction::Cosine {
    let float_vector_values = NormalizedFloatVectorValues::new(float_vector_values);
    if confidence_interval == Some(DYNAMIC_CONFIDENCE_INTERVAL) {
      return ScalarQuantizer::from_vectors_auto_interval(
        &float_vector_values,
        VectorSimilarityFunction::DotProduct,
        num_vectors,
        bits,
      );
    }
    return ScalarQuantizer::from_vectors(
      &float_vector_values,
      confidence_interval.unwrap_or_else(|| {
        Lucene99ScalarQuantizedVectorsFormat::calculate_default_confidence_interval(
          float_vector_values.dimension(),
        )
      }),
      num_vectors,
      bits,
    );
  }
  if confidence_interval == Some(DYNAMIC_CONFIDENCE_INTERVAL) {
    return ScalarQuantizer::from_vectors_auto_interval(
      &float_vector_values,
      vector_similarity_function,
      num_vectors,
      bits,
    );
  }
  ScalarQuantizer::from_vectors(
    &float_vector_values,
    confidence_interval.unwrap_or_else(|| {
      Lucene99ScalarQuantizedVectorsFormat::calculate_default_confidence_interval(
        float_vector_values.dimension(),
      )
    }),
    num_vectors,
    bits,
  )
}

#[derive(Clone, Copy)]
struct FloatVectorWrapper<'a> {
  vector_list: &'a [VectorValueEnum],
}

impl<'a> FloatVectorWrapper<'a> {
  fn new(vector_list: &'a [VectorValueEnum]) -> Self {
    Self { vector_list }
  }
}

impl KnnVectorValues for FloatVectorWrapper<'_> {
  fn dimension(&self) -> usize {
    self.vector_list[0].len()
  }

  fn size(&self) -> usize {
    self.vector_list.len()
  }

  type KnnVectorValues = Self;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    Ok(*self)
  }

  fn get_encoding(&self) -> VectorEncoding {
    FloatVectorValues::get_encoding(self)
  }

  type Bits<'a, B>
    = BitsImpl1<B>
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    self.default_get_accept_ords(accept_docs)
  }

  type DocIndexIterator = DenseDocIndexIterator;

  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    Ok(DenseDocIndexIterator::new(self.vector_list.len() as i32))
  }
}

impl FloatVectorValues for FloatVectorWrapper<'_> {
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    if ord >= self.vector_list.len() {
      return Err(LuceneError::io(std::io::Error::other(format!(
        "vector ord {} out of bounds",
        ord
      ))));
    }
    Ok(Cow::Borrowed(&self.vector_list[ord]))
  }

  type FloatVectorValues = Self;

  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    Ok(Some(*self))
  }

  type VectorScorer = DummyVectorScorer;
}

struct NormalizedFloatVectorValues<FVV>
where
  FVV: FloatVectorValues,
{
  values: FVV,
}

impl<FVV> NormalizedFloatVectorValues<FVV>
where
  FVV: FloatVectorValues,
{
  fn new(values: FVV) -> Self {
    Self { values }
  }
}

impl<FVV> KnnVectorValues for NormalizedFloatVectorValues<FVV>
where
  FVV: FloatVectorValues,
{
  fn dimension(&self) -> usize {
    self.values.dimension()
  }

  fn size(&self) -> usize {
    self.values.size()
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    self.values.ord_to_doc(ord)
  }

  type KnnVectorValues = Self;

  fn get_encoding(&self) -> VectorEncoding {
    KnnVectorValues::get_encoding(&self.values)
  }

  type Bits<'a, B>
    = FVV::Bits<'a, B>
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    self.values.get_accept_ords(accept_docs)
  }

  type DocIndexIterator = FVV::DocIndexIterator;

  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    self.values.iterator()
  }
}

impl<FVV> FloatVectorValues for NormalizedFloatVectorValues<FVV>
where
  FVV: FloatVectorValues,
{
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    let vector_value = self.values.vector_value(ord)?;
    let mut normalized_vector = vector_value.as_floats()?.to_vec();
    VectorUtil::l2normalize(&mut normalized_vector)?;
    Ok(Cow::Owned(VectorValueEnum::Float(normalized_vector)))
  }

  type FloatVectorValues = Self;

  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    Ok(None)
  }

  type VectorScorer = DummyVectorScorer;
}

pub struct Lucene99ScalarQuantizedCloseableRandomVectorScorerSupplier;

impl RandomVectorScorerSupplier for Lucene99ScalarQuantizedCloseableRandomVectorScorerSupplier {
  type Scorer<'a>
    = DummyRandomVectorScorer
  where
    Self: 'a;

  fn scorer(&self, _ord: usize) -> Result<Self::Scorer<'_>> {
    Err(LuceneError::unsupported_operation(
      "Lucene99ScalarQuantizedCloseableRandomVectorScorerSupplier is not implemented yet",
    ))
  }

  type RandomVectorScorerSupplier = Self;

  fn copy(&self) -> Result<Self::RandomVectorScorerSupplier>
  where
    Self: Sized,
  {
    Err(LuceneError::unsupported_operation(
      "Lucene99ScalarQuantizedCloseableRandomVectorScorerSupplier is not implemented yet",
    ))
  }
}

impl Closeable for Lucene99ScalarQuantizedCloseableRandomVectorScorerSupplier {}

impl CloseableRandomVectorScorerSupplier
  for Lucene99ScalarQuantizedCloseableRandomVectorScorerSupplier
{
  fn total_vector_count(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(
      "Lucene99ScalarQuantizedCloseableRandomVectorScorerSupplier is not implemented yet",
    ))
  }
}
