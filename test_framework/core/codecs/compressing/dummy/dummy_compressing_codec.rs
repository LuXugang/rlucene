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
use crate::core::codecs::compression::compression_mode::{
  CompressionModeBase, CompressionModeEnum, CompressorEnum, DecompressorEnum,
};
use crate::core::codecs::compression::compressor::Compressor;
use crate::core::codecs::compression::decompressor::Decompressor;
use crate::core::index::BytesRef;
use crate::core::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::core::store::{DataInput, DataOutput};
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::codecs::compressing::compressing_codec::CompressingCodec;
use std::fmt::{Display, Formatter};

pub static DUMMY: DummyCompressionMode = DummyCompressionMode;

static DUMMY_DECOMPRESSOR: DummyDecompressor = DummyDecompressor;

static DUMMY_COMPRESSOR: DummyCompressor = DummyCompressor;

#[derive(Clone, Copy, Debug)]
pub struct DummyCompressionMode;

impl Display for DummyCompressionMode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "DUMMY")
  }
}

impl CompressionModeBase for DummyCompressionMode {
  fn new_compressor(&self) -> CompressorEnum {
    CompressorEnum::Dummy(DUMMY_COMPRESSOR)
  }

  fn new_decompressor(&self) -> DecompressorEnum {
    DecompressorEnum::Dummy(DUMMY_DECOMPRESSOR)
  }
}

#[derive(Clone, Copy)]
pub struct DummyDecompressor;

impl Decompressor for DummyDecompressor {
  fn decompress(
    &mut self,
    input: &mut impl DataInput,
    original_length: i32,
    offset: i32,
    length: i32,
    bytes: &mut BytesRef<Vec<u8>>,
  ) -> Result<()> {
    debug_assert!(offset + length <= original_length);
    let original_length = original_length as usize;
    if bytes.bytes.len() < original_length {
      bytes.bytes = vec![0; ArrayUtil::oversize(original_length, 1)?];
    }
    input.read_bytes(&mut bytes.bytes, 0, (offset + length) as usize)?;
    bytes.offset = offset as usize;
    bytes.length = length as usize;
    Ok(())
  }
}

#[derive(Clone, Copy)]
pub struct DummyCompressor;

impl Compressor for DummyCompressor {
  fn compress(
    &mut self,
    buffers_input: &mut ByteBuffersDataInput<&[u8]>,
    out: &mut impl DataOutput,
  ) -> Result<()> {
    let length = buffers_input.length();
    out.copy_bytes(buffers_input, length)
  }
}

impl Closeable for DummyCompressor {}

/// Compression codec that does not compress data, useful for testing.
// In its own module to make sure the compressing codec types are visible
// enough to let people write their own compression mode.
#[derive(Clone)]
pub struct DummyCompressingCodec {
  base: CompressingCodec,
}

impl DummyCompressingCodec {
  /// Constructor that allows configuring the chunk size.
  pub fn new(
    chunk_size: i32,
    max_docs_per_chunk: i32,
    with_segment_suffix: bool,
    block_shift: i32,
  ) -> Result<Self> {
    Ok(Self {
      base: CompressingCodec::new(
        "DummyCompressingStoredFieldsData",
        "DummyCompressingStoredFields",
        CompressionModeEnum::Dummy(DUMMY),
        chunk_size,
        max_docs_per_chunk,
        with_segment_suffix,
        block_shift,
      )?,
    })
  }

  /// Default constructor.
  pub fn default_instance() -> Result<Self> {
    Self::new(1 << 14, 128, false, 10)
  }
}

impl From<DummyCompressingCodec> for CompressingCodec {
  fn from(codec: DummyCompressingCodec) -> Self {
    codec.base
  }
}

impl Codec for DummyCompressingCodec {
  type PostingsFormat = <CompressingCodec as Codec>::PostingsFormat;
  type DocValuesFormat = <CompressingCodec as Codec>::DocValuesFormat;
  type StoredFieldsFormat = <CompressingCodec as Codec>::StoredFieldsFormat;
  type TermVectorsFormat = <CompressingCodec as Codec>::TermVectorsFormat;
  type FieldInfosFormat = <CompressingCodec as Codec>::FieldInfosFormat;
  type SegmentInfoFormat = <CompressingCodec as Codec>::SegmentInfoFormat;
  type NormsFormat = <CompressingCodec as Codec>::NormsFormat;
  type LiveDocsFormat = <CompressingCodec as Codec>::LiveDocsFormat;
  type CompoundFormat = <CompressingCodec as Codec>::CompoundFormat;
  type PointsFormat = <CompressingCodec as Codec>::PointsFormat;
  type KnnVectorsFormat = <CompressingCodec as Codec>::KnnVectorsFormat;

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
    self.base.compound_format()
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

impl Display for DummyCompressingCodec {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.base, f)
  }
}
