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
use crate::core::codecs::Codec;
use crate::core::codecs::doc_values_consumer::DocValuesConsumerEnum2;
use crate::core::codecs::doc_values_format::DocValuesFormat;
use crate::core::codecs::doc_values_producer::DocValuesProducerEnum2;
use crate::core::codecs::knn_vectors_format::KnnVectorsFormat;
use crate::core::codecs::knn_vectors_formats::KnnVectorsFormats;
use crate::core::codecs::knn_vectors_reader::KnnVectorsReaderEnum2;
use crate::core::codecs::knn_vectors_writer::KnnVectorsWriterEnum2;
use crate::core::codecs::perfield::per_field_doc_values_format::{
  PerFieldDocValuesFormat, PerFieldDocValuesFormatBase,
};
use crate::core::codecs::perfield::per_field_knn_vectors_format::{
  PerFieldKnnVectorsFormat, PerFieldKnnVectorsFormatBase,
};
use crate::core::codecs::perfield::per_field_postings_format::{
  PerFieldPostingsFormat, PerFieldPostingsFormatBase,
};
use crate::core::index::index_reader::Identity;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::codecs::asserting_doc_values_format::AssertingDocValuesFormat;
use crate::test_framework::core::codecs::asserting_knn_vectors_format::AssertingKnnVectorsFormat;
use crate::test_framework::core::codecs::asserting_live_docs_format::AssertingLiveDocsFormat;
use crate::test_framework::core::codecs::asserting_norms_format::AssertingNormsFormat;
use crate::test_framework::core::codecs::asserting_points_format::AssertingPointsFormat;
use crate::test_framework::core::codecs::asserting_postings_format::AssertingPostingsFormat;
use crate::test_framework::core::codecs::asserting_stored_fields_format::AssertingStoredFieldsFormat;
use crate::test_framework::core::codecs::asserting_term_vectors_format::AssertingTermVectorsFormat;
use crate::test_framework::core::codecs::perfield::test_per_field_knn_vectors_format::{
  KnnVectorsFormatMaxDims32, MaxDimensionsPerFieldFormatAssertingCodec,
  MergeUsesNewFormatAssertingCodec, TwoFieldsTwoFormatsAssertingCodec,
  WriteRecordingKnnVectorsFormat,
};
use crate::test_framework::core::util::test_util::{
  DefaultCodec, DefaultDocValuesFormat, TestUtil,
};
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::thread::ThreadId;

pub(crate) fn assert_thread(object: &str, creation_thread: ThreadId) {
  let current_thread = std::thread::current().id();
  assert!(
    creation_thread == current_thread,
    "{object} are only supposed to be consumed in the thread in which they have been acquired. \
     But was acquired in {creation_thread:?} and consumed in {current_thread:?}."
  );
}

pub enum AssertingCodecDocValuesFormat {
  Default(Arc<DefaultDocValuesFormat>),
  Asserting(Arc<AssertingDocValuesFormat>),
}

impl From<DefaultDocValuesFormat> for AssertingCodecDocValuesFormat {
  fn from(format: DefaultDocValuesFormat) -> Self {
    Self::Default(Arc::new(format))
  }
}

impl From<AssertingDocValuesFormat> for AssertingCodecDocValuesFormat {
  fn from(format: AssertingDocValuesFormat) -> Self {
    Self::Asserting(Arc::new(format))
  }
}

impl Display for AssertingCodecDocValuesFormat {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Default(format) => Display::fmt(format, f),
      Self::Asserting(format) => Display::fmt(format, f),
    }
  }
}

impl HasIdentity for AssertingCodecDocValuesFormat {
  fn identity(&self) -> &Identity {
    match self {
      Self::Default(format) => format.identity(),
      Self::Asserting(format) => format.identity(),
    }
  }
}

pub type AssertingCodecDocValuesConsumer<O> = DocValuesConsumerEnum2<
  <DefaultDocValuesFormat as DocValuesFormat>::DocValuesConsumer<O>,
  <AssertingDocValuesFormat as DocValuesFormat>::DocValuesConsumer<O>,
>;

pub type AssertingCodecDocValuesProducer<I> = DocValuesProducerEnum2<
  <DefaultDocValuesFormat as DocValuesFormat>::DocValuesProducer<I>,
  <AssertingDocValuesFormat as DocValuesFormat>::DocValuesProducer<I>,
>;

impl DocValuesFormat for AssertingCodecDocValuesFormat {
  fn get_name(&self) -> &str {
    match self {
      Self::Default(format) => format.get_name(),
      Self::Asserting(format) => format.get_name(),
    }
  }

  type DocValuesConsumer<O: IndexOutput> = AssertingCodecDocValuesConsumer<O>;

  fn fields_consumer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::DocValuesConsumer<D1::IndexOutput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Default(format) => format
        .fields_consumer(state, segment_info)
        .map(DocValuesConsumerEnum2::A),
      Self::Asserting(format) => format
        .fields_consumer(state, segment_info)
        .map(DocValuesConsumerEnum2::B),
    }
  }

  type DocValuesProducer<I: IndexInput> = AssertingCodecDocValuesProducer<I>;

  fn fields_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::DocValuesProducer<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Default(format) => format
        .fields_producer(state, segment_info)
        .map(DocValuesProducerEnum2::A),
      Self::Asserting(format) => format
        .fields_producer(state, segment_info)
        .map(DocValuesProducerEnum2::B),
    }
  }

  fn for_name(name: &str) -> Result<Arc<Self>> {
    match name {
      "Lucene90" => {
        DefaultDocValuesFormat::for_name(name).map(|format| Arc::new(Self::Default(format)))
      },
      "Asserting" => {
        AssertingDocValuesFormat::for_name(name).map(|format| Arc::new(Self::Asserting(format)))
      },
      _ => Err(LuceneError::illegal_argument(format!(
        "Could not load doc values format named \"{name}\""
      ))),
    }
  }
}

pub enum AssertingCodecKnnVectorsFormat {
  Asserting(Arc<AssertingKnnVectorsFormat>),
  Source(Arc<KnnVectorsFormats>),
  WriteRecording(Arc<WriteRecordingKnnVectorsFormat>),
  MaxDims32(Arc<KnnVectorsFormatMaxDims32>),
}

impl From<AssertingKnnVectorsFormat> for AssertingCodecKnnVectorsFormat {
  fn from(format: AssertingKnnVectorsFormat) -> Self {
    Self::Asserting(Arc::new(format))
  }
}

impl From<KnnVectorsFormats> for AssertingCodecKnnVectorsFormat {
  fn from(format: KnnVectorsFormats) -> Self {
    Self::Source(Arc::new(format))
  }
}

impl From<WriteRecordingKnnVectorsFormat> for AssertingCodecKnnVectorsFormat {
  fn from(format: WriteRecordingKnnVectorsFormat) -> Self {
    Self::WriteRecording(Arc::new(format))
  }
}

impl From<KnnVectorsFormatMaxDims32> for AssertingCodecKnnVectorsFormat {
  fn from(format: KnnVectorsFormatMaxDims32) -> Self {
    Self::MaxDims32(Arc::new(format))
  }
}

impl Display for AssertingCodecKnnVectorsFormat {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Asserting(format) => Display::fmt(format.as_ref(), f),
      Self::Source(format) => Display::fmt(format.as_ref(), f),
      Self::WriteRecording(format) => Display::fmt(format.as_ref(), f),
      Self::MaxDims32(format) => Display::fmt(format.as_ref(), f),
    }
  }
}

impl HasIdentity for AssertingCodecKnnVectorsFormat {
  fn identity(&self) -> &Identity {
    match self {
      Self::Asserting(format) => format.identity(),
      Self::Source(format) => format.identity(),
      Self::WriteRecording(format) => format.identity(),
      Self::MaxDims32(format) => format.identity(),
    }
  }
}

pub type AssertingCodecKnnVectorsWriter<O> = KnnVectorsWriterEnum2<
  <AssertingKnnVectorsFormat as KnnVectorsFormat>::KnnVectorsWriter<O>,
  KnnVectorsWriterEnum2<
    <KnnVectorsFormats as KnnVectorsFormat>::KnnVectorsWriter<O>,
    <WriteRecordingKnnVectorsFormat as KnnVectorsFormat>::KnnVectorsWriter<O>,
  >,
>;

pub type AssertingCodecKnnVectorsReader<I> = KnnVectorsReaderEnum2<
  <AssertingKnnVectorsFormat as KnnVectorsFormat>::KnnVectorsReader<I>,
  <KnnVectorsFormats as KnnVectorsFormat>::KnnVectorsReader<I>,
>;

impl KnnVectorsFormat for AssertingCodecKnnVectorsFormat {
  fn get_name(&self) -> &str {
    match self {
      Self::Asserting(format) => format.get_name(),
      Self::Source(format) => format.get_name(),
      Self::WriteRecording(format) => format.get_name(),
      Self::MaxDims32(format) => format.get_name(),
    }
  }

  type KnnVectorsWriter<O: IndexOutput> = AssertingCodecKnnVectorsWriter<O>;

  fn fields_writer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::KnnVectorsWriter<D1::IndexOutput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Asserting(format) => format
        .fields_writer(state, segment_info)
        .map(KnnVectorsWriterEnum2::A),
      Self::Source(format) => format
        .fields_writer(state, segment_info)
        .map(|writer| KnnVectorsWriterEnum2::B(KnnVectorsWriterEnum2::A(writer))),
      Self::WriteRecording(format) => format
        .fields_writer(state, segment_info)
        .map(|writer| KnnVectorsWriterEnum2::B(KnnVectorsWriterEnum2::B(writer))),
      Self::MaxDims32(format) => format
        .fields_writer(state, segment_info)
        .map(|writer| KnnVectorsWriterEnum2::B(KnnVectorsWriterEnum2::A(writer))),
    }
  }

  type KnnVectorsReader<I: IndexInput> = AssertingCodecKnnVectorsReader<I>;

  fn fields_reader<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::KnnVectorsReader<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Asserting(format) => format
        .fields_reader(state, segment_info)
        .map(KnnVectorsReaderEnum2::A),
      Self::Source(format) => format
        .fields_reader(state, segment_info)
        .map(KnnVectorsReaderEnum2::B),
      Self::WriteRecording(format) => format
        .fields_reader(state, segment_info)
        .map(KnnVectorsReaderEnum2::B),
      Self::MaxDims32(format) => format
        .fields_reader(state, segment_info)
        .map(KnnVectorsReaderEnum2::B),
    }
  }

  fn get_max_dimensions(&self, field_name: &str) -> Result<usize> {
    match self {
      Self::Asserting(format) => format.get_max_dimensions(field_name),
      Self::Source(format) => format.get_max_dimensions(field_name),
      Self::WriteRecording(format) => format.get_max_dimensions(field_name),
      Self::MaxDims32(format) => format.get_max_dimensions(field_name),
    }
  }

  fn for_name(name: &str) -> Result<Arc<Self>> {
    match name {
      "Asserting" => {
        AssertingKnnVectorsFormat::for_name(name).map(|format| Arc::new(Self::Asserting(format)))
      },
      _ => KnnVectorsFormats::for_name(name).map(|format| Arc::new(Self::Source(format))),
    }
  }
}

/// Static-dispatch access to the methods that Java subclasses override on
/// [`AssertingCodec`].
pub trait AssertingCodecBase {
  fn get_postings_format_for_field(&self, field: &str) -> Result<&AssertingPostingsFormat>;

  fn get_doc_values_format_for_field(&self, field: &str) -> Result<&AssertingCodecDocValuesFormat>;

  fn get_knn_vectors_format_for_field(
    &self,
    field: &str,
  ) -> Result<&AssertingCodecKnnVectorsFormat>;
}

pub struct AssertingCodecDefaults {
  default_format: AssertingPostingsFormat,
  default_dv_format: AssertingCodecDocValuesFormat,
  default_knn_vectors_format: AssertingCodecKnnVectorsFormat,
}

impl Default for AssertingCodecDefaults {
  fn default() -> Self {
    Self {
      default_format: AssertingPostingsFormat::new(),
      default_dv_format: AssertingDocValuesFormat::new().into(),
      default_knn_vectors_format: AssertingKnnVectorsFormat::new()
        .expect("default KNN vectors format parameters are valid")
        .into(),
    }
  }
}

impl AssertingCodecDefaults {
  /// Returns the postings format that should be used for writing new segments
  /// of `field`.
  ///
  /// The default implementation always returns `Asserting`.
  pub fn get_postings_format_for_field(&self, _field: &str) -> Result<&AssertingPostingsFormat> {
    Ok(&self.default_format)
  }

  /// Returns the doc values format that should be used for writing new
  /// segments of `field`.
  ///
  /// The default implementation always returns `Asserting`.
  pub fn get_doc_values_format_for_field(
    &self,
    _field: &str,
  ) -> Result<&AssertingCodecDocValuesFormat> {
    Ok(&self.default_dv_format)
  }

  /// Returns the vectors format that should be used for writing new segments
  /// of `field`.
  ///
  /// The default implementation always returns `Asserting`.
  pub fn get_knn_vectors_format_for_field(
    &self,
    _field: &str,
  ) -> Result<&AssertingCodecKnnVectorsFormat> {
    Ok(&self.default_knn_vectors_format)
  }
}

impl AssertingCodecBase for AssertingCodecDefaults {
  fn get_postings_format_for_field(&self, field: &str) -> Result<&AssertingPostingsFormat> {
    AssertingCodecDefaults::get_postings_format_for_field(self, field)
  }

  fn get_doc_values_format_for_field(&self, field: &str) -> Result<&AssertingCodecDocValuesFormat> {
    AssertingCodecDefaults::get_doc_values_format_for_field(self, field)
  }

  fn get_knn_vectors_format_for_field(
    &self,
    field: &str,
  ) -> Result<&AssertingCodecKnnVectorsFormat> {
    AssertingCodecDefaults::get_knn_vectors_format_for_field(self, field)
  }
}

pub(crate) struct AlwaysDocValuesFormatAssertingCodec {
  defaults: AssertingCodecDefaults,
  format: AssertingCodecDocValuesFormat,
}

impl AlwaysDocValuesFormatAssertingCodec {
  fn new(format: AssertingCodecDocValuesFormat) -> Self {
    Self {
      defaults: AssertingCodecDefaults::default(),
      format,
    }
  }
}

impl AssertingCodecBase for AlwaysDocValuesFormatAssertingCodec {
  fn get_postings_format_for_field(&self, field: &str) -> Result<&AssertingPostingsFormat> {
    self.defaults.get_postings_format_for_field(field)
  }

  fn get_doc_values_format_for_field(
    &self,
    _field: &str,
  ) -> Result<&AssertingCodecDocValuesFormat> {
    Ok(&self.format)
  }

  fn get_knn_vectors_format_for_field(
    &self,
    field: &str,
  ) -> Result<&AssertingCodecKnnVectorsFormat> {
    self.defaults.get_knn_vectors_format_for_field(field)
  }
}

pub(crate) struct AlwaysKnnVectorsFormatAssertingCodec {
  defaults: AssertingCodecDefaults,
  format: AssertingCodecKnnVectorsFormat,
}

impl AlwaysKnnVectorsFormatAssertingCodec {
  fn new(format: AssertingCodecKnnVectorsFormat) -> Self {
    Self {
      defaults: AssertingCodecDefaults::default(),
      format,
    }
  }
}

impl AssertingCodecBase for AlwaysKnnVectorsFormatAssertingCodec {
  fn get_postings_format_for_field(&self, field: &str) -> Result<&AssertingPostingsFormat> {
    self.defaults.get_postings_format_for_field(field)
  }

  fn get_doc_values_format_for_field(&self, field: &str) -> Result<&AssertingCodecDocValuesFormat> {
    self.defaults.get_doc_values_format_for_field(field)
  }

  fn get_knn_vectors_format_for_field(
    &self,
    _field: &str,
  ) -> Result<&AssertingCodecKnnVectorsFormat> {
    Ok(&self.format)
  }
}

pub(crate) enum AssertingCodecHook {
  Default(AssertingCodecDefaults),
  AlwaysDocValuesFormat(AlwaysDocValuesFormatAssertingCodec),
  AlwaysKnnVectorsFormat(AlwaysKnnVectorsFormatAssertingCodec),
  TwoFieldsTwoFormats(TwoFieldsTwoFormatsAssertingCodec),
  MergeUsesNewFormat(MergeUsesNewFormatAssertingCodec),
  MaxDimensionsPerFieldFormat(MaxDimensionsPerFieldFormatAssertingCodec),
}

impl Default for AssertingCodecHook {
  fn default() -> Self {
    Self::Default(AssertingCodecDefaults::default())
  }
}

impl AssertingCodecBase for AssertingCodecHook {
  fn get_postings_format_for_field(&self, field: &str) -> Result<&AssertingPostingsFormat> {
    match self {
      Self::Default(defaults) => defaults.get_postings_format_for_field(field),
      Self::AlwaysDocValuesFormat(hook) => hook.get_postings_format_for_field(field),
      Self::AlwaysKnnVectorsFormat(hook) => hook.get_postings_format_for_field(field),
      Self::TwoFieldsTwoFormats(hook) => hook.get_postings_format_for_field(field),
      Self::MergeUsesNewFormat(hook) => hook.get_postings_format_for_field(field),
      Self::MaxDimensionsPerFieldFormat(hook) => hook.get_postings_format_for_field(field),
    }
  }

  fn get_doc_values_format_for_field(&self, field: &str) -> Result<&AssertingCodecDocValuesFormat> {
    match self {
      Self::Default(defaults) => defaults.get_doc_values_format_for_field(field),
      Self::AlwaysDocValuesFormat(hook) => hook.get_doc_values_format_for_field(field),
      Self::AlwaysKnnVectorsFormat(hook) => hook.get_doc_values_format_for_field(field),
      Self::TwoFieldsTwoFormats(hook) => hook.get_doc_values_format_for_field(field),
      Self::MergeUsesNewFormat(hook) => hook.get_doc_values_format_for_field(field),
      Self::MaxDimensionsPerFieldFormat(hook) => hook.get_doc_values_format_for_field(field),
    }
  }

  fn get_knn_vectors_format_for_field(
    &self,
    field: &str,
  ) -> Result<&AssertingCodecKnnVectorsFormat> {
    match self {
      Self::Default(defaults) => defaults.get_knn_vectors_format_for_field(field),
      Self::AlwaysDocValuesFormat(hook) => hook.get_knn_vectors_format_for_field(field),
      Self::AlwaysKnnVectorsFormat(hook) => hook.get_knn_vectors_format_for_field(field),
      Self::TwoFieldsTwoFormats(hook) => hook.get_knn_vectors_format_for_field(field),
      Self::MergeUsesNewFormat(hook) => hook.get_knn_vectors_format_for_field(field),
      Self::MaxDimensionsPerFieldFormat(hook) => hook.get_knn_vectors_format_for_field(field),
    }
  }
}

pub struct AssertingCodecPostingsFormatBase {
  hook: Arc<AssertingCodecHook>,
}

impl PerFieldPostingsFormatBase for AssertingCodecPostingsFormatBase {
  type Format = AssertingPostingsFormat;

  fn get_postings_format_for_field(&self, field: &str) -> Result<&Self::Format> {
    self.hook.get_postings_format_for_field(field)
  }
}

pub struct AssertingCodecDocValuesFormatBase {
  hook: Arc<AssertingCodecHook>,
}

impl PerFieldDocValuesFormatBase for AssertingCodecDocValuesFormatBase {
  type Format = AssertingCodecDocValuesFormat;

  fn get_doc_values_format_for_field(&self, field: &str) -> Result<&Self::Format> {
    self.hook.get_doc_values_format_for_field(field)
  }
}

pub struct AssertingCodecKnnVectorsFormatBase {
  hook: Arc<AssertingCodecHook>,
}

impl PerFieldKnnVectorsFormatBase for AssertingCodecKnnVectorsFormatBase {
  type Format = AssertingCodecKnnVectorsFormat;

  fn get_knn_vectors_format_for_field(&self, field: &str) -> Result<&Self::Format> {
    self.hook.get_knn_vectors_format_for_field(field)
  }
}

/// Acts like the default codec but with additional asserts.
pub struct AssertingCodec {
  delegate: DefaultCodec,
  postings: PerFieldPostingsFormat<AssertingCodecPostingsFormatBase>,
  doc_values: PerFieldDocValuesFormat<AssertingCodecDocValuesFormatBase>,
  knn_vectors_format: PerFieldKnnVectorsFormat<AssertingCodecKnnVectorsFormatBase>,
  hook: Arc<AssertingCodecHook>,
}

impl Default for AssertingCodec {
  fn default() -> Self {
    Self::new()
  }
}

impl AssertingCodec {
  pub fn new() -> Self {
    Self::with_hook(AssertingCodecHook::default())
  }

  pub(crate) fn with_doc_values_format(format: impl Into<AssertingCodecDocValuesFormat>) -> Self {
    Self::with_hook(AssertingCodecHook::AlwaysDocValuesFormat(
      AlwaysDocValuesFormatAssertingCodec::new(format.into()),
    ))
  }

  pub(crate) fn with_knn_vectors_format(format: impl Into<KnnVectorsFormats>) -> Self {
    Self::with_hook(AssertingCodecHook::AlwaysKnnVectorsFormat(
      AlwaysKnnVectorsFormatAssertingCodec::new(format.into().into()),
    ))
  }

  pub(crate) fn with_hook(hook: AssertingCodecHook) -> Self {
    let hook = Arc::new(hook);
    Self {
      delegate: TestUtil::get_default_codec(),
      postings: PerFieldPostingsFormat::new(AssertingCodecPostingsFormatBase {
        hook: Arc::clone(&hook),
      }),
      doc_values: PerFieldDocValuesFormat::new(AssertingCodecDocValuesFormatBase {
        hook: Arc::clone(&hook),
      }),
      knn_vectors_format: PerFieldKnnVectorsFormat::new(AssertingCodecKnnVectorsFormatBase {
        hook: Arc::clone(&hook),
      }),
      hook,
    }
  }

  pub fn get_postings_format_for_field(&self, field: &str) -> Result<&AssertingPostingsFormat> {
    self.hook.get_postings_format_for_field(field)
  }

  pub fn get_doc_values_format_for_field(
    &self,
    field: &str,
  ) -> Result<&AssertingCodecDocValuesFormat> {
    self.hook.get_doc_values_format_for_field(field)
  }

  pub fn get_knn_vectors_format_for_field(
    &self,
    field: &str,
  ) -> Result<&AssertingCodecKnnVectorsFormat> {
    self.hook.get_knn_vectors_format_for_field(field)
  }
}

impl Clone for AssertingCodec {
  fn clone(&self) -> Self {
    Self {
      delegate: self.delegate.clone(),
      postings: self.postings.clone(),
      doc_values: self.doc_values.clone(),
      knn_vectors_format: self.knn_vectors_format.clone(),
      hook: Arc::clone(&self.hook),
    }
  }
}

impl Codec for AssertingCodec {
  type PostingsFormat = PerFieldPostingsFormat<AssertingCodecPostingsFormatBase>;
  type DocValuesFormat = PerFieldDocValuesFormat<AssertingCodecDocValuesFormatBase>;
  type StoredFieldsFormat = AssertingStoredFieldsFormat;
  type TermVectorsFormat = AssertingTermVectorsFormat;
  type FieldInfosFormat = <DefaultCodec as Codec>::FieldInfosFormat;
  type SegmentInfoFormat = <DefaultCodec as Codec>::SegmentInfoFormat;
  type NormsFormat = AssertingNormsFormat;
  type LiveDocsFormat = AssertingLiveDocsFormat;
  type CompoundFormat = <DefaultCodec as Codec>::CompoundFormat;
  type PointsFormat = AssertingPointsFormat;
  type KnnVectorsFormat = PerFieldKnnVectorsFormat<AssertingCodecKnnVectorsFormatBase>;

  fn postings_format(&self) -> Self::PostingsFormat {
    self.postings.clone()
  }

  fn doc_values_format(&self) -> Self::DocValuesFormat {
    self.doc_values.clone()
  }

  fn stored_fields_format(&self) -> Self::StoredFieldsFormat {
    AssertingStoredFieldsFormat::new()
  }

  fn term_vectors_format(&self) -> Self::TermVectorsFormat {
    AssertingTermVectorsFormat::new()
  }

  fn field_infos_format(&self) -> Self::FieldInfosFormat {
    self.delegate.field_infos_format()
  }

  fn segment_info_format(&self) -> Self::SegmentInfoFormat {
    self.delegate.segment_info_format()
  }

  fn norms_format(&self) -> Self::NormsFormat {
    AssertingNormsFormat::new()
  }

  fn live_docs_format(&self) -> Self::LiveDocsFormat {
    AssertingLiveDocsFormat::new()
  }

  fn compound_format(&self) -> Self::CompoundFormat {
    self.delegate.compound_format()
  }

  fn points_format(&self) -> Self::PointsFormat {
    AssertingPointsFormat::new()
  }

  fn knn_vectors_format(&self) -> Result<Self::KnnVectorsFormat> {
    Ok(self.knn_vectors_format.clone())
  }

  fn get_name(&self) -> &str {
    "Asserting"
  }
}

impl Display for AssertingCodec {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "Asserting({})", self.delegate)
  }
}
