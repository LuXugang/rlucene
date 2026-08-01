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
use crate::core::codecs::doc_values_format::DocValuesFormat;
use crate::core::codecs::knn_vectors_format::KnnVectorsFormat;
use crate::core::codecs::perfield::per_field_doc_values_format::{
  PerFieldDocValuesFormat, PerFieldDocValuesFormatBase,
};
use crate::core::codecs::perfield::per_field_knn_vectors_format::{
  PerFieldKnnVectorsFormat, PerFieldKnnVectorsFormatBase,
};
use crate::core::codecs::perfield::per_field_postings_format::{
  PerFieldPostingsFormat, PerFieldPostingsFormatBase,
};
use crate::core::codecs::postings_format::PostingsFormat;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::codecs::asserting_doc_values_format::AssertingDocValuesFormat;
use crate::test_framework::core::codecs::asserting_knn_vectors_format::AssertingKnnVectorsFormat;
use crate::test_framework::core::codecs::asserting_live_docs_format::AssertingLiveDocsFormat;
use crate::test_framework::core::codecs::asserting_norms_format::AssertingNormsFormat;
use crate::test_framework::core::codecs::asserting_points_format::AssertingPointsFormat;
use crate::test_framework::core::codecs::asserting_postings_format::AssertingPostingsFormat;
use crate::test_framework::core::codecs::asserting_stored_fields_format::AssertingStoredFieldsFormat;
use crate::test_framework::core::codecs::asserting_term_vectors_format::AssertingTermVectorsFormat;
use crate::test_framework::core::util::test_util::{DefaultCodec, TestUtil};
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

/// Static-dispatch access to the methods that Java subclasses override on
/// [`AssertingCodec`].
pub trait AssertingCodecBase {
  type PostingsFormat: PostingsFormat;
  type DocValuesFormat: DocValuesFormat;
  type KnnVectorsFormat: KnnVectorsFormat;

  fn get_postings_format_for_field(&self, field: &str) -> Result<&Self::PostingsFormat>;

  fn get_doc_values_format_for_field(&self, field: &str) -> Result<&Self::DocValuesFormat>;

  fn get_knn_vectors_format_for_field(&self, field: &str) -> Result<&Self::KnnVectorsFormat>;
}

pub struct AssertingCodecDefaults {
  default_format: AssertingPostingsFormat,
  default_dv_format: AssertingDocValuesFormat,
  default_knn_vectors_format: AssertingKnnVectorsFormat,
}

impl Default for AssertingCodecDefaults {
  fn default() -> Self {
    Self {
      default_format: AssertingPostingsFormat::new(),
      default_dv_format: AssertingDocValuesFormat::new(),
      default_knn_vectors_format: AssertingKnnVectorsFormat::new()
        .expect("default KNN vectors format parameters are valid"),
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
  pub fn get_doc_values_format_for_field(&self, _field: &str) -> Result<&AssertingDocValuesFormat> {
    Ok(&self.default_dv_format)
  }

  /// Returns the vectors format that should be used for writing new segments
  /// of `field`.
  ///
  /// The default implementation always returns `Asserting`.
  pub fn get_knn_vectors_format_for_field(
    &self,
    _field: &str,
  ) -> Result<&AssertingKnnVectorsFormat> {
    Ok(&self.default_knn_vectors_format)
  }
}

impl AssertingCodecBase for AssertingCodecDefaults {
  type PostingsFormat = AssertingPostingsFormat;
  type DocValuesFormat = AssertingDocValuesFormat;
  type KnnVectorsFormat = AssertingKnnVectorsFormat;

  fn get_postings_format_for_field(&self, field: &str) -> Result<&Self::PostingsFormat> {
    AssertingCodecDefaults::get_postings_format_for_field(self, field)
  }

  fn get_doc_values_format_for_field(&self, field: &str) -> Result<&Self::DocValuesFormat> {
    AssertingCodecDefaults::get_doc_values_format_for_field(self, field)
  }

  fn get_knn_vectors_format_for_field(&self, field: &str) -> Result<&Self::KnnVectorsFormat> {
    AssertingCodecDefaults::get_knn_vectors_format_for_field(self, field)
  }
}

pub struct AssertingCodecPostingsFormatBase<B>
where
  B: AssertingCodecBase,
{
  hook: Arc<B>,
}

impl<B> PerFieldPostingsFormatBase for AssertingCodecPostingsFormatBase<B>
where
  B: AssertingCodecBase,
{
  type Format = B::PostingsFormat;

  fn get_postings_format_for_field(&self, field: &str) -> Result<&Self::Format> {
    self.hook.get_postings_format_for_field(field)
  }
}

pub struct AssertingCodecDocValuesFormatBase<B>
where
  B: AssertingCodecBase,
{
  hook: Arc<B>,
}

impl<B> PerFieldDocValuesFormatBase for AssertingCodecDocValuesFormatBase<B>
where
  B: AssertingCodecBase,
{
  type Format = B::DocValuesFormat;

  fn get_doc_values_format_for_field(&self, field: &str) -> Result<&Self::Format> {
    self.hook.get_doc_values_format_for_field(field)
  }
}

pub struct AssertingCodecKnnVectorsFormatBase<B>
where
  B: AssertingCodecBase,
{
  hook: Arc<B>,
}

impl<B> PerFieldKnnVectorsFormatBase for AssertingCodecKnnVectorsFormatBase<B>
where
  B: AssertingCodecBase,
{
  type Format = B::KnnVectorsFormat;

  fn get_knn_vectors_format_for_field(&self, field: &str) -> Result<&Self::Format> {
    self.hook.get_knn_vectors_format_for_field(field)
  }
}

/// Acts like the default codec but with additional asserts.
pub struct AssertingCodec<B = AssertingCodecDefaults>
where
  B: AssertingCodecBase,
{
  delegate: DefaultCodec,
  postings: PerFieldPostingsFormat<AssertingCodecPostingsFormatBase<B>>,
  doc_values: PerFieldDocValuesFormat<AssertingCodecDocValuesFormatBase<B>>,
  knn_vectors_format: PerFieldKnnVectorsFormat<AssertingCodecKnnVectorsFormatBase<B>>,
  hook: Arc<B>,
}

impl Default for AssertingCodec<AssertingCodecDefaults> {
  fn default() -> Self {
    Self::new()
  }
}

impl AssertingCodec<AssertingCodecDefaults> {
  pub fn new() -> Self {
    Self::with_hook(AssertingCodecDefaults::default())
  }
}

impl<B> AssertingCodec<B>
where
  B: AssertingCodecBase,
{
  pub(crate) fn with_hook(hook: B) -> Self {
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

  pub fn get_postings_format_for_field(&self, field: &str) -> Result<&B::PostingsFormat> {
    self.hook.get_postings_format_for_field(field)
  }

  pub fn get_doc_values_format_for_field(&self, field: &str) -> Result<&B::DocValuesFormat> {
    self.hook.get_doc_values_format_for_field(field)
  }

  pub fn get_knn_vectors_format_for_field(&self, field: &str) -> Result<&B::KnnVectorsFormat> {
    self.hook.get_knn_vectors_format_for_field(field)
  }
}

impl<B> Clone for AssertingCodec<B>
where
  B: AssertingCodecBase,
{
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

impl<B> Codec for AssertingCodec<B>
where
  B: AssertingCodecBase,
{
  type PostingsFormat = PerFieldPostingsFormat<AssertingCodecPostingsFormatBase<B>>;
  type DocValuesFormat = PerFieldDocValuesFormat<AssertingCodecDocValuesFormatBase<B>>;
  type StoredFieldsFormat = AssertingStoredFieldsFormat;
  type TermVectorsFormat = AssertingTermVectorsFormat;
  type FieldInfosFormat = <DefaultCodec as Codec>::FieldInfosFormat;
  type SegmentInfoFormat = <DefaultCodec as Codec>::SegmentInfoFormat;
  type NormsFormat = AssertingNormsFormat;
  type LiveDocsFormat = AssertingLiveDocsFormat;
  type CompoundFormat = <DefaultCodec as Codec>::CompoundFormat;
  type PointsFormat = AssertingPointsFormat;
  type KnnVectorsFormat = PerFieldKnnVectorsFormat<AssertingCodecKnnVectorsFormatBase<B>>;

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

impl<B> Display for AssertingCodec<B>
where
  B: AssertingCodecBase,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "Asserting({})", self.delegate)
  }
}
