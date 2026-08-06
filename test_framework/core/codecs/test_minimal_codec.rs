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
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::test_util::{DefaultCodec, TestUtil};
use std::fmt::{Display, Formatter};

#[allow(dead_code)] // for quick search
struct TestMinimalCodec;

/// Minimal codec implementation for working with the most basic documents.
#[derive(Clone)]
pub struct MinimalCodec {
  wrapped_codec: DefaultCodec,
  name: &'static str,
}

impl Default for MinimalCodec {
  fn default() -> Self {
    Self::new()
  }
}

impl MinimalCodec {
  pub fn new() -> Self {
    Self::with_name("MinimalCodec")
  }

  fn with_name(name: &'static str) -> Self {
    Self {
      wrapped_codec: TestUtil::get_default_codec(),
      name,
    }
  }
}

impl Display for MinimalCodec {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.name)
  }
}

impl Codec for MinimalCodec {
  type PostingsFormat = <DefaultCodec as Codec>::PostingsFormat;
  type DocValuesFormat = <DefaultCodec as Codec>::DocValuesFormat;
  type StoredFieldsFormat = <DefaultCodec as Codec>::StoredFieldsFormat;
  type TermVectorsFormat = <DefaultCodec as Codec>::TermVectorsFormat;
  type FieldInfosFormat = <DefaultCodec as Codec>::FieldInfosFormat;
  type SegmentInfoFormat = <DefaultCodec as Codec>::SegmentInfoFormat;
  type NormsFormat = <DefaultCodec as Codec>::NormsFormat;
  type LiveDocsFormat = <DefaultCodec as Codec>::LiveDocsFormat;
  type CompoundFormat = <DefaultCodec as Codec>::CompoundFormat;
  type PointsFormat = <DefaultCodec as Codec>::PointsFormat;
  type KnnVectorsFormat = <DefaultCodec as Codec>::KnnVectorsFormat;

  fn postings_format(&self) -> Self::PostingsFormat {
    panic!("unsupported operation")
  }

  fn doc_values_format(&self) -> Self::DocValuesFormat {
    panic!("unsupported operation")
  }

  fn stored_fields_format(&self) -> Self::StoredFieldsFormat {
    // TODO: avoid calling this when no stored fields are written or read
    self.wrapped_codec.stored_fields_format()
  }

  fn term_vectors_format(&self) -> Self::TermVectorsFormat {
    panic!("unsupported operation")
  }

  fn field_infos_format(&self) -> Self::FieldInfosFormat {
    self.wrapped_codec.field_infos_format()
  }

  fn segment_info_format(&self) -> Self::SegmentInfoFormat {
    self.wrapped_codec.segment_info_format()
  }

  fn norms_format(&self) -> Self::NormsFormat {
    panic!("unsupported operation")
  }

  fn live_docs_format(&self) -> Self::LiveDocsFormat {
    panic!("unsupported operation")
  }

  fn compound_format(&self) -> Self::CompoundFormat {
    panic!("unsupported operation")
  }

  fn points_format(&self) -> Self::PointsFormat {
    panic!("unsupported operation")
  }

  fn knn_vectors_format(&self) -> Result<Self::KnnVectorsFormat> {
    panic!("unsupported operation")
  }

  fn get_name(&self) -> &str {
    self.name
  }
}

/// Minimal codec implementation for working with the most basic documents, supporting compound
/// formats.
#[derive(Clone)]
pub struct MinimalCompoundCodec {
  base: MinimalCodec,
}

impl Default for MinimalCompoundCodec {
  fn default() -> Self {
    Self::new()
  }
}

impl MinimalCompoundCodec {
  pub fn new() -> Self {
    Self {
      base: MinimalCodec::with_name("MinimalCompoundCodec"),
    }
  }
}

impl Display for MinimalCompoundCodec {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.base, f)
  }
}

impl Codec for MinimalCompoundCodec {
  type PostingsFormat = <MinimalCodec as Codec>::PostingsFormat;
  type DocValuesFormat = <MinimalCodec as Codec>::DocValuesFormat;
  type StoredFieldsFormat = <MinimalCodec as Codec>::StoredFieldsFormat;
  type TermVectorsFormat = <MinimalCodec as Codec>::TermVectorsFormat;
  type FieldInfosFormat = <MinimalCodec as Codec>::FieldInfosFormat;
  type SegmentInfoFormat = <MinimalCodec as Codec>::SegmentInfoFormat;
  type NormsFormat = <MinimalCodec as Codec>::NormsFormat;
  type LiveDocsFormat = <MinimalCodec as Codec>::LiveDocsFormat;
  type CompoundFormat = <MinimalCodec as Codec>::CompoundFormat;
  type PointsFormat = <MinimalCodec as Codec>::PointsFormat;
  type KnnVectorsFormat = <MinimalCodec as Codec>::KnnVectorsFormat;

  fn postings_format(&self) -> Self::PostingsFormat {
    self.base.postings_format()
  }

  fn doc_values_format(&self) -> Self::DocValuesFormat {
    self.base.doc_values_format()
  }

  fn stored_fields_format(&self) -> Self::StoredFieldsFormat {
    self.base.stored_fields_format()
  }

  fn term_vectors_format(&self) -> Self::TermVectorsFormat {
    self.base.term_vectors_format()
  }

  fn field_infos_format(&self) -> Self::FieldInfosFormat {
    self.base.field_infos_format()
  }

  fn segment_info_format(&self) -> Self::SegmentInfoFormat {
    self.base.segment_info_format()
  }

  fn norms_format(&self) -> Self::NormsFormat {
    self.base.norms_format()
  }

  fn live_docs_format(&self) -> Self::LiveDocsFormat {
    self.base.live_docs_format()
  }

  fn compound_format(&self) -> Self::CompoundFormat {
    self.base.wrapped_codec.compound_format()
  }

  fn points_format(&self) -> Self::PointsFormat {
    self.base.points_format()
  }

  fn knn_vectors_format(&self) -> Result<Self::KnnVectorsFormat> {
    self.base.knn_vectors_format()
  }

  fn get_name(&self) -> &str {
    self.base.get_name()
  }
}
