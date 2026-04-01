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
use once_cell::sync::Lazy;
use std::fmt::Display;

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
use crate::core::util::error::lucene_error::Result;

pub static LATEST_CODEC: Lazy<Lucene101Codec> = Lazy::new(|| Lucene101Codec);
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

pub type DefaultCodec = Lucene101Codec;
pub type DefaultPostingsFormat = <DefaultCodec as Codec>::PostingsFormat;
pub type DefaultDocValuesFormat = <DefaultCodec as Codec>::DocValuesFormat;
pub type DefaultStoredFieldsFormat = <DefaultCodec as Codec>::StoredFieldsFormat;
pub type DefaultTermVectorsFormat = <DefaultCodec as Codec>::TermVectorsFormat;
pub type DefaultFieldInfosFormat = <DefaultCodec as Codec>::FieldInfosFormat;
pub type DefaultSegmentInfoFormat = <DefaultCodec as Codec>::SegmentInfoFormat;
pub type DefaultNormsFormat = <DefaultCodec as Codec>::NormsFormat;
pub type DefaultLiveDocsFormat = <DefaultCodec as Codec>::LiveDocsFormat;
pub type DefaultCompoundFormat = <DefaultCodec as Codec>::CompoundFormat;
pub type DefaultPointsFormat = <DefaultCodec as Codec>::PointsFormat;
pub type DefaultKnnVectorsFormat = <DefaultCodec as Codec>::KnnVectorsFormat;
