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
use crate::core::codecs::knn_vectors_format::{DEFAULT_MAX_DIMENSIONS, KnnVectorsFormat};
use crate::core::codecs::knn_vectors_formats::{
  KnnVectorsFormats, KnnVectorsFormatsReader, KnnVectorsFormatsWriter,
};
use crate::core::codecs::knn_vectors_writer::KnnVectorsWriter;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::index_reader::Identity;
use crate::core::index::merge_state::MergeState;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::sorter::DocMap;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::HasIdentity;
use crate::core::util::accountable::Accountable;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::codecs::asserting_codec::{
  AssertingCodecBase, AssertingCodecDefaults, AssertingCodecDocValuesFormat,
  AssertingCodecKnnVectorsFormat, AssertingCodecPostingsFormat,
};
use parking_lot::Mutex;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub(crate) struct TestPerFieldKnnVectorsFormat;

#[derive(Clone)]
pub struct WriteRecordingKnnVectorsFormat {
  delegate: Arc<KnnVectorsFormats>,
  fields_written: Arc<Mutex<HashSet<String>>>,
  identity: Identity,
}

impl WriteRecordingKnnVectorsFormat {
  pub(crate) fn new(delegate: impl Into<KnnVectorsFormats>) -> Self {
    Self {
      delegate: Arc::new(delegate.into()),
      fields_written: Arc::new(Mutex::new(HashSet::new())),
      identity: Identity::new(),
    }
  }

  pub(crate) fn fields_written(&self) -> HashSet<String> {
    self.fields_written.lock().clone()
  }
}

impl Display for WriteRecordingKnnVectorsFormat {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Display::fmt(self.delegate.as_ref(), f)
  }
}

impl HasIdentity for WriteRecordingKnnVectorsFormat {
  fn identity(&self) -> &Identity {
    &self.identity
  }
}

impl KnnVectorsFormat for WriteRecordingKnnVectorsFormat {
  fn get_name(&self) -> &str {
    self.delegate.get_name()
  }

  type KnnVectorsWriter<O: IndexOutput> = WriteRecordingKnnVectorsWriter<O>;

  fn fields_writer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::KnnVectorsWriter<D1::IndexOutput>>
  where
    D1: Directory,
  {
    Ok(WriteRecordingKnnVectorsWriter {
      delegate: self.delegate.fields_writer(state, segment_info)?,
      fields_written: Arc::clone(&self.fields_written),
    })
  }

  type KnnVectorsReader<I: IndexInput> = KnnVectorsFormatsReader<I>;

  fn fields_reader<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::KnnVectorsReader<D1::IndexInput>>
  where
    D1: Directory,
  {
    self.delegate.fields_reader(state, segment_info)
  }

  fn get_max_dimensions(&self, _field_name: &str) -> Result<usize> {
    Ok(DEFAULT_MAX_DIMENSIONS)
  }

  fn for_name(name: &str) -> Result<Arc<Self>> {
    Ok(Arc::new(Self {
      delegate: KnnVectorsFormats::for_name(name)?,
      fields_written: Arc::new(Mutex::new(HashSet::new())),
      identity: Identity::new(),
    }))
  }
}

pub struct WriteRecordingKnnVectorsWriter<O>
where
  O: IndexOutput,
{
  delegate: KnnVectorsFormatsWriter<O>,
  fields_written: Arc<Mutex<HashSet<String>>>,
}

impl<O> KnnVectorsWriter<O> for WriteRecordingKnnVectorsWriter<O>
where
  O: IndexOutput,
{
  fn add_field<D1, D2>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field_info: Arc<FieldInfo>,
  ) -> Result<usize>
  where
    D1: Directory<IndexOutput = O>,
  {
    self.fields_written.lock().insert(field_info.name.clone());
    self
      .delegate
      .add_field(write_state, segment_info, field_info)
  }

  fn flush<DM>(&mut self, max_doc: i32, sort_map: Option<&DM>) -> Result<()>
  where
    DM: DocMap,
  {
    self.delegate.flush(max_doc, sort_map)
  }

  fn merge_one_field<D1, D2, CR>(
    &mut self,
    field_info: &Arc<FieldInfo>,
    merge_state: &MergeState<'_, D1, CR>,
    segment_write_state: &SegmentWriteState<&D2>,
  ) -> Result<()>
  where
    D2: Directory<IndexOutput = O>,
    CR: CodecReader,
  {
    self.fields_written.lock().insert(field_info.name.clone());
    self
      .delegate
      .merge_one_field(field_info, merge_state, segment_write_state)
  }

  fn finish(&mut self) -> Result<()> {
    self.delegate.finish()
  }

  fn add_value(
    &mut self,
    doc_id: i32,
    vector_value: &VectorValueEnum,
    field_vectors_writers_idx: usize,
  ) -> Result<()> {
    self
      .delegate
      .add_value(doc_id, vector_value, field_vectors_writers_idx)
  }
}

impl<O> Closeable for WriteRecordingKnnVectorsWriter<O>
where
  O: IndexOutput,
{
  fn close(&mut self) -> Result<()> {
    self.delegate.close()
  }
}

impl<O> Accountable for WriteRecordingKnnVectorsWriter<O>
where
  O: IndexOutput,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    self.delegate.ram_bytes_used()
  }
}

pub struct KnnVectorsFormatMaxDims32 {
  delegate: Arc<KnnVectorsFormats>,
  identity: Identity,
}

impl KnnVectorsFormatMaxDims32 {
  pub(crate) fn new(delegate: impl Into<KnnVectorsFormats>) -> Self {
    Self {
      delegate: Arc::new(delegate.into()),
      identity: Identity::new(),
    }
  }
}

impl Display for KnnVectorsFormatMaxDims32 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Display::fmt(self.delegate.as_ref(), f)
  }
}

impl HasIdentity for KnnVectorsFormatMaxDims32 {
  fn identity(&self) -> &Identity {
    &self.identity
  }
}

impl KnnVectorsFormat for KnnVectorsFormatMaxDims32 {
  fn get_name(&self) -> &str {
    self.delegate.get_name()
  }

  type KnnVectorsWriter<O: IndexOutput> = KnnVectorsFormatsWriter<O>;

  fn fields_writer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::KnnVectorsWriter<D1::IndexOutput>>
  where
    D1: Directory,
  {
    self.delegate.fields_writer(state, segment_info)
  }

  type KnnVectorsReader<I: IndexInput> = KnnVectorsFormatsReader<I>;

  fn fields_reader<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::KnnVectorsReader<D1::IndexInput>>
  where
    D1: Directory,
  {
    self.delegate.fields_reader(state, segment_info)
  }

  fn get_max_dimensions(&self, _field_name: &str) -> Result<usize> {
    Ok(32)
  }

  fn for_name(name: &str) -> Result<Arc<Self>> {
    Ok(Arc::new(Self {
      delegate: KnnVectorsFormats::for_name(name)?,
      identity: Identity::new(),
    }))
  }
}

pub(crate) struct TwoFieldsTwoFormatsAssertingCodec {
  defaults: AssertingCodecDefaults,
  format1: AssertingCodecKnnVectorsFormat,
  format2: AssertingCodecKnnVectorsFormat,
}

impl TwoFieldsTwoFormatsAssertingCodec {
  pub(crate) fn new(
    format1: AssertingCodecKnnVectorsFormat,
    format2: AssertingCodecKnnVectorsFormat,
  ) -> Self {
    Self {
      defaults: AssertingCodecDefaults::default(),
      format1,
      format2,
    }
  }
}

impl AssertingCodecBase for TwoFieldsTwoFormatsAssertingCodec {
  fn get_postings_format_for_field(&self, field: &str) -> Result<&AssertingCodecPostingsFormat> {
    self.defaults.get_postings_format_for_field(field)
  }

  fn get_doc_values_format_for_field(&self, field: &str) -> Result<&AssertingCodecDocValuesFormat> {
    self.defaults.get_doc_values_format_for_field(field)
  }

  fn get_knn_vectors_format_for_field(
    &self,
    field: &str,
  ) -> Result<&AssertingCodecKnnVectorsFormat> {
    if field == "field1" {
      Ok(&self.format1)
    } else {
      Ok(&self.format2)
    }
  }
}

pub(crate) struct MergeUsesNewFormatAssertingCodec {
  defaults: AssertingCodecDefaults,
  format1: AssertingCodecKnnVectorsFormat,
  format2: AssertingCodecKnnVectorsFormat,
}

impl MergeUsesNewFormatAssertingCodec {
  pub(crate) fn new(
    format1: AssertingCodecKnnVectorsFormat,
    format2: AssertingCodecKnnVectorsFormat,
  ) -> Self {
    Self {
      defaults: AssertingCodecDefaults::default(),
      format1,
      format2,
    }
  }
}

impl AssertingCodecBase for MergeUsesNewFormatAssertingCodec {
  fn get_postings_format_for_field(&self, field: &str) -> Result<&AssertingCodecPostingsFormat> {
    self.defaults.get_postings_format_for_field(field)
  }

  fn get_doc_values_format_for_field(&self, field: &str) -> Result<&AssertingCodecDocValuesFormat> {
    self.defaults.get_doc_values_format_for_field(field)
  }

  fn get_knn_vectors_format_for_field(
    &self,
    field: &str,
  ) -> Result<&AssertingCodecKnnVectorsFormat> {
    if field == "field1" {
      Ok(&self.format1)
    } else {
      Ok(&self.format2)
    }
  }
}

pub(crate) struct MaxDimensionsPerFieldFormatAssertingCodec {
  defaults: AssertingCodecDefaults,
  format1: AssertingCodecKnnVectorsFormat,
  format2: AssertingCodecKnnVectorsFormat,
}

impl MaxDimensionsPerFieldFormatAssertingCodec {
  pub(crate) fn new(
    format1: AssertingCodecKnnVectorsFormat,
    format2: AssertingCodecKnnVectorsFormat,
  ) -> Self {
    Self {
      defaults: AssertingCodecDefaults::default(),
      format1,
      format2,
    }
  }
}

impl AssertingCodecBase for MaxDimensionsPerFieldFormatAssertingCodec {
  fn get_postings_format_for_field(&self, field: &str) -> Result<&AssertingCodecPostingsFormat> {
    self.defaults.get_postings_format_for_field(field)
  }

  fn get_doc_values_format_for_field(&self, field: &str) -> Result<&AssertingCodecDocValuesFormat> {
    self.defaults.get_doc_values_format_for_field(field)
  }

  fn get_knn_vectors_format_for_field(
    &self,
    field: &str,
  ) -> Result<&AssertingCodecKnnVectorsFormat> {
    if field == "field1" {
      Ok(&self.format1)
    } else {
      Ok(&self.format2)
    }
  }
}
