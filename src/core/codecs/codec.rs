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
use crate::core::codecs::codec_formats::{
  CodecDocValuesFormat, CodecKnnVectorsFormat, CodecLiveDocsFormat, CodecNormsFormat,
  CodecPointsFormat, CodecPostingsFormat, CodecStoredFieldsFormat, CodecTermVectorsFormat,
};
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
#[cfg(test)]
use crate::test_framework::core::codecs::asserting_codec::AssertingCodec;
use std::fmt::{Display, Formatter};

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

#[derive(Clone)]
pub enum Codecs {
  Lucene101(Lucene101Codec),
  #[cfg(test)]
  Asserting(AssertingCodec),
}

impl Default for Codecs {
  fn default() -> Self {
    Self::Lucene101(Lucene101Codec::default())
  }
}

impl From<Lucene101Codec> for Codecs {
  fn from(codec: Lucene101Codec) -> Self {
    Self::Lucene101(codec)
  }
}

#[cfg(test)]
impl From<AssertingCodec> for Codecs {
  fn from(codec: AssertingCodec) -> Self {
    Self::Asserting(codec)
  }
}

impl Display for Codecs {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Lucene101(codec) => Display::fmt(codec, f),
      #[cfg(test)]
      Self::Asserting(codec) => Display::fmt(codec, f),
    }
  }
}

impl Codec for Codecs {
  type PostingsFormat = CodecPostingsFormat;
  type DocValuesFormat = CodecDocValuesFormat;
  type StoredFieldsFormat = CodecStoredFieldsFormat;
  type TermVectorsFormat = CodecTermVectorsFormat;
  type FieldInfosFormat = <Lucene101Codec as Codec>::FieldInfosFormat;
  type SegmentInfoFormat = <Lucene101Codec as Codec>::SegmentInfoFormat;
  type NormsFormat = CodecNormsFormat;
  type LiveDocsFormat = CodecLiveDocsFormat;
  type CompoundFormat = <Lucene101Codec as Codec>::CompoundFormat;
  type PointsFormat = CodecPointsFormat;
  type KnnVectorsFormat = CodecKnnVectorsFormat;

  fn postings_format(&self) -> Self::PostingsFormat {
    match self {
      Self::Lucene101(codec) => CodecPostingsFormat::Lucene101(codec.postings_format()),
      #[cfg(test)]
      Self::Asserting(codec) => CodecPostingsFormat::Asserting(codec.postings_format()),
    }
  }

  fn doc_values_format(&self) -> Self::DocValuesFormat {
    match self {
      Self::Lucene101(codec) => CodecDocValuesFormat::Lucene101(codec.doc_values_format()),
      #[cfg(test)]
      Self::Asserting(codec) => CodecDocValuesFormat::Asserting(codec.doc_values_format()),
    }
  }

  fn stored_fields_format(&self) -> Self::StoredFieldsFormat {
    match self {
      Self::Lucene101(codec) => CodecStoredFieldsFormat::Lucene90(codec.stored_fields_format()),
      #[cfg(test)]
      Self::Asserting(codec) => CodecStoredFieldsFormat::Asserting(codec.stored_fields_format()),
    }
  }

  fn term_vectors_format(&self) -> Self::TermVectorsFormat {
    match self {
      Self::Lucene101(codec) => CodecTermVectorsFormat::Lucene90(codec.term_vectors_format()),
      #[cfg(test)]
      Self::Asserting(codec) => CodecTermVectorsFormat::Asserting(codec.term_vectors_format()),
    }
  }

  fn field_infos_format(&self) -> Self::FieldInfosFormat {
    match self {
      Self::Lucene101(codec) => codec.field_infos_format(),
      #[cfg(test)]
      Self::Asserting(codec) => codec.field_infos_format(),
    }
  }

  fn segment_info_format(&self) -> Self::SegmentInfoFormat {
    match self {
      Self::Lucene101(codec) => codec.segment_info_format(),
      #[cfg(test)]
      Self::Asserting(codec) => codec.segment_info_format(),
    }
  }

  fn norms_format(&self) -> Self::NormsFormat {
    match self {
      Self::Lucene101(codec) => CodecNormsFormat::Lucene90(codec.norms_format()),
      #[cfg(test)]
      Self::Asserting(codec) => CodecNormsFormat::Asserting(codec.norms_format()),
    }
  }

  fn live_docs_format(&self) -> Self::LiveDocsFormat {
    match self {
      Self::Lucene101(codec) => CodecLiveDocsFormat::Lucene90(codec.live_docs_format()),
      #[cfg(test)]
      Self::Asserting(codec) => CodecLiveDocsFormat::Asserting(codec.live_docs_format()),
    }
  }

  fn compound_format(&self) -> Self::CompoundFormat {
    match self {
      Self::Lucene101(codec) => codec.compound_format(),
      #[cfg(test)]
      Self::Asserting(codec) => codec.compound_format(),
    }
  }

  fn points_format(&self) -> Self::PointsFormat {
    match self {
      Self::Lucene101(codec) => CodecPointsFormat::Lucene90(codec.points_format()),
      #[cfg(test)]
      Self::Asserting(codec) => CodecPointsFormat::Asserting(codec.points_format()),
    }
  }

  fn knn_vectors_format(&self) -> Result<Self::KnnVectorsFormat> {
    match self {
      Self::Lucene101(codec) => codec
        .knn_vectors_format()
        .map(CodecKnnVectorsFormat::Lucene101),
      #[cfg(test)]
      Self::Asserting(codec) => codec
        .knn_vectors_format()
        .map(CodecKnnVectorsFormat::Asserting),
    }
  }

  fn get_name(&self) -> &str {
    match self {
      Self::Lucene101(codec) => codec.get_name(),
      #[cfg(test)]
      Self::Asserting(codec) => codec.get_name(),
    }
  }
}

/// Returns the current default codec.
///
/// This mirrors Java Lucene's `Codec.getDefault` entry point. The production
/// default remains `Lucene101`.
pub fn get_default() -> Codecs {
  Codecs::default()
}

/// Looks up a codec by name.
///
/// This mirrors Java Lucene's `Codec.forName` entry point.
pub fn for_name(name: &str) -> Result<Codecs> {
  match name {
    "Lucene101" => Ok(Codecs::default()),
    #[cfg(test)]
    "Asserting" => Ok(Codecs::Asserting(AssertingCodec::new())),
    _ => Err(LuceneError::illegal_argument(format!(
      "Could not load codec named \"{}\"",
      name
    ))),
  }
}

pub type DefaultPostingsFormat = <Lucene101Codec as Codec>::PostingsFormat;
pub type DefaultDocValuesFormat = <Lucene101Codec as Codec>::DocValuesFormat;
pub type DefaultStoredFieldsFormat = <Lucene101Codec as Codec>::StoredFieldsFormat;
pub type DefaultTermVectorsFormat = <Lucene101Codec as Codec>::TermVectorsFormat;
pub type DefaultFieldInfosFormat = <Lucene101Codec as Codec>::FieldInfosFormat;
pub type DefaultSegmentInfoFormat = <Lucene101Codec as Codec>::SegmentInfoFormat;
pub type DefaultNormsFormat = <Lucene101Codec as Codec>::NormsFormat;
pub type DefaultLiveDocsFormat = <Lucene101Codec as Codec>::LiveDocsFormat;
pub type DefaultCompoundFormat = <Lucene101Codec as Codec>::CompoundFormat;
pub type DefaultPointsFormat = <Lucene101Codec as Codec>::PointsFormat;
pub type DefaultKnnVectorsFormat = <Lucene101Codec as Codec>::KnnVectorsFormat;
