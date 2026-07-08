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
use crate::core::codecs::compound_format::CompoundFormat;
use crate::core::codecs::doc_values_format::DocValuesFormat;
use crate::core::codecs::field_infos_format::FieldInfosFormat;
use crate::core::codecs::knn_vectors_format::KnnVectorsFormat;
use crate::core::codecs::live_docs_format::LiveDocsFormat;
use crate::core::codecs::lucene101_codec::Lucene101Codec;
use crate::core::codecs::norms_format::NormsFormat;
use crate::core::codecs::points_format::PointsFormat;
use crate::core::codecs::postings_format::PostingsFormat;
use crate::core::codecs::segment_info_format::SegmentInfoFormat;
use crate::core::codecs::stored_fields_format::StoredFieldsFormat;
use crate::core::codecs::term_vectors_format::TermVectorsFormat;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::Display;

pub trait Codec: Display {
  type PostingsFormat: PostingsFormat;
  type DocValuesFormat: DocValuesFormat;
  type StoredFieldsFormat: StoredFieldsFormat;
  type TermVectorsFormat: TermVectorsFormat;
  type FieldInfosFormat: FieldInfosFormat;
  type SegmentInfoFormat: SegmentInfoFormat;
  type NormsFormat: NormsFormat;
  type LiveDocsFormat: LiveDocsFormat;
  type CompoundFormat: CompoundFormat;
  type PointsFormat: PointsFormat;
  type KnnVectorsFormat: KnnVectorsFormat;
  // type KnnVectorsFormat;
  /// Encodes/decodes postings
  fn postings_format(&self) -> Self::PostingsFormat;
  /// Encodes/decodes docvalues
  fn doc_values_format(&self) -> Self::DocValuesFormat;
  //
  /// Encodes/decodes stored fields
  fn stored_fields_format(&self) -> Self::StoredFieldsFormat;
  //
  /// Encodes/decodes term vectors
  fn term_vectors_format(&self) -> Self::TermVectorsFormat;

  /// Encodes/decodes field infos file
  fn field_infos_format(&self) -> Self::FieldInfosFormat;

  /// Encodes/decodes segment info file
  fn segment_info_format(&self) -> Self::SegmentInfoFormat;

  // /// Encodes/decodes document normalization values
  fn norms_format(&self) -> Self::NormsFormat;

  /// Encodes/decodes live docs
  fn live_docs_format(&self) -> Self::LiveDocsFormat;

  /// Encodes/decodes compound files
  fn compound_format(&self) -> Self::CompoundFormat;

  /// Encodes/decodes points index
  fn points_format(&self) -> Self::PointsFormat;

  /// Encodes/decodes numeric vector fields
  fn knn_vectors_format(&self) -> Result<Self::KnnVectorsFormat>;

  fn get_name(&self) -> &str;
}

pub type Codecs = Lucene101Codec;

/// Returns the current default codec.
///
/// This mirrors Java Lucene's `Codec.getDefault` entry point. For now the
/// default codec is fixed; when codec selection becomes configurable at a wider
/// scope this function is the single place to expose that behavior.
pub fn get_default() -> Codecs {
  Codecs::default()
}

/// Looks up a codec by name.
///
/// This mirrors Java Lucene's `Codec.forName` entry point. For now the registry
/// only contains the default codec; when `DefaultCodec` becomes an enum this
/// function should grow with the supported variants.
pub fn for_name(name: &str) -> Result<Codecs> {
  match name {
    "Lucene101" => Ok(Codecs::default()),
    _ => Err(LuceneError::illegal_argument(format!(
      "Could not load codec named \"{}\"",
      name
    ))),
  }
}

pub type DefaultPostingsFormat = <Codecs as Codec>::PostingsFormat;
pub type DefaultDocValuesFormat = <Codecs as Codec>::DocValuesFormat;
pub type DefaultStoredFieldsFormat = <Codecs as Codec>::StoredFieldsFormat;
pub type DefaultTermVectorsFormat = <Codecs as Codec>::TermVectorsFormat;
pub type DefaultFieldInfosFormat = <Codecs as Codec>::FieldInfosFormat;
pub type DefaultSegmentInfoFormat = <Codecs as Codec>::SegmentInfoFormat;
pub type DefaultNormsFormat = <Codecs as Codec>::NormsFormat;
pub type DefaultLiveDocsFormat = <Codecs as Codec>::LiveDocsFormat;
pub type DefaultCompoundFormat = <Codecs as Codec>::CompoundFormat;
pub type DefaultPointsFormat = <Codecs as Codec>::PointsFormat;
pub type DefaultKnnVectorsFormat = <Codecs as Codec>::KnnVectorsFormat;
