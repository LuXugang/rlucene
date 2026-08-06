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
use crate::core::codecs::compressing::lucene90_compressing_stored_fields_format::Lucene90CompressingStoredFieldsFormat;
use crate::core::codecs::compressing::lucene90_compressing_term_vectors_format::Lucene90CompressingTermVectorsFormat;
use crate::core::codecs::compression::compression_mode::{CompressionMode, CompressionModeEnum};
use crate::core::codecs::lucene90::deflate_with_preset_dict_compression_mode::DeflateWithPresetDictCompressionMode;
use crate::core::codecs::lucene90::lz4_with_preset_dict_compression_mode::LZ4WithPresetDictCompressionMode;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::packed::direct_monotonic_writer::{MAX_BLOCK_SHIFT, MIN_BLOCK_SHIFT};
use crate::test_framework::core::codecs::compressing::dummy::dummy_compressing_codec::DummyCompressingCodec;
use crate::test_framework::core::util::test_util::{DefaultCodec, TestUtil};
use rand::{Rng, RngExt};
use std::fmt::{Display, Formatter};

/// A codec that uses [`Lucene90CompressingStoredFieldsFormat`] for its stored
/// fields and delegates to the default codec for everything else.
#[derive(Clone)]
pub struct CompressingCodec {
  name: &'static str,
  delegate: DefaultCodec,
  stored_fields_format: Lucene90CompressingStoredFieldsFormat,
  term_vectors_format: Lucene90CompressingTermVectorsFormat,
}

impl CompressingCodec {
  /// Create a random instance.
  pub fn random_instance_with_parameters<R>(
    random: &mut R,
    chunk_size: i32,
    max_docs_per_chunk: i32,
    with_segment_suffix: bool,
    block_shift: i32,
  ) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    match random.random_range(0..6) {
      0 => Self::new(
        "FastCompressingStoredFieldsData",
        "FastCompressingStoredFields",
        CompressionMode::fast(),
        chunk_size,
        max_docs_per_chunk,
        with_segment_suffix,
        block_shift,
      ),
      1 => Self::new(
        "FastDecompressionCompressingStoredFieldsData",
        "FastDecompressionCompressingStoredFields",
        CompressionMode::fast_decompression(),
        chunk_size,
        max_docs_per_chunk,
        with_segment_suffix,
        block_shift,
      ),
      2 => Self::new(
        "HighCompressionCompressingStoredFieldsData",
        "HighCompressionCompressingStoredFields",
        CompressionMode::high_compression(),
        chunk_size,
        max_docs_per_chunk,
        with_segment_suffix,
        block_shift,
      ),
      3 => DummyCompressingCodec::new(
        chunk_size,
        max_docs_per_chunk,
        with_segment_suffix,
        block_shift,
      )
      .map(Into::into),
      4 => Self::new(
        "DeflateWithPresetCompressingStoredFieldsData",
        "DeflateWithPresetCompressingStoredFields",
        CompressionModeEnum::DeflateDict(DeflateWithPresetDictCompressionMode),
        chunk_size,
        max_docs_per_chunk,
        with_segment_suffix,
        block_shift,
      ),
      5 => Self::new(
        "LZ4WithPresetCompressingStoredFieldsData",
        "DeflateWithPresetCompressingStoredFields",
        CompressionModeEnum::LZ4Dict(LZ4WithPresetDictCompressionMode),
        chunk_size,
        max_docs_per_chunk,
        with_segment_suffix,
        block_shift,
      ),
      _ => unreachable!(),
    }
  }

  /// Creates a random [`CompressingCodec`] that is using an empty segment suffix.
  pub fn random_instance<R>(random: &mut R) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    let chunk_size = if random.random_bool(0.5) {
      TestUtil::next_int(random, 10, 100)
    } else {
      TestUtil::next_int(random, 10, 1 << 15)
    };
    let chunk_docs = if random.random_bool(0.5) {
      TestUtil::next_int(random, 1, 10)
    } else {
      TestUtil::next_int(random, 64, 1024)
    };
    let block_shift = if random.random_bool(0.5) {
      TestUtil::next_int(random, MIN_BLOCK_SHIFT, 10)
    } else {
      TestUtil::next_int(random, MIN_BLOCK_SHIFT, MAX_BLOCK_SHIFT)
    };
    Self::random_instance_with_parameters(random, chunk_size, chunk_docs, false, block_shift)
  }

  /// Creates a random [`CompressingCodec`] with more reasonable parameters for big tests.
  pub fn reasonable_instance<R>(random: &mut R) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    // e.g. defaults use 2^14 for FAST and ~ 2^16 for HIGH
    let chunk_size = TestUtil::next_int(random, 1 << 13, 1 << 17);
    // e.g. defaults use 128 for FAST and 512 for HIGH
    let chunk_docs = TestUtil::next_int(random, 1 << 6, 1 << 10);
    // e.g. defaults use 1024 for both cases
    let block_shift = TestUtil::next_int(random, 8, 12);
    Self::random_instance_with_parameters(random, chunk_size, chunk_docs, false, block_shift)
  }

  pub(super) fn new(
    name: &'static str,
    segment_suffix: &'static str,
    compression_mode: CompressionModeEnum,
    chunk_size: i32,
    max_docs_per_chunk: i32,
    with_segment_suffix: bool,
    block_shift: i32,
  ) -> Result<Self> {
    let segment_suffix = if with_segment_suffix {
      segment_suffix
    } else {
      ""
    };
    let stored_fields_format = Lucene90CompressingStoredFieldsFormat::with_suffix(
      name,
      segment_suffix,
      compression_mode.clone(),
      chunk_size,
      max_docs_per_chunk,
      block_shift,
    )?;
    let term_vectors_format = Lucene90CompressingTermVectorsFormat::new(
      name,
      segment_suffix,
      compression_mode,
      chunk_size,
      max_docs_per_chunk,
      block_shift,
    )?;
    Ok(Self {
      name,
      delegate: TestUtil::get_default_codec(),
      stored_fields_format,
      term_vectors_format,
    })
  }

  pub(crate) fn for_name(name: &str) -> Result<Self> {
    match name {
      "FastCompressingStoredFieldsData" => Self::new(
        "FastCompressingStoredFieldsData",
        "FastCompressingStoredFields",
        CompressionMode::fast(),
        1 << 14,
        128,
        false,
        10,
      ),
      "FastDecompressionCompressingStoredFieldsData" => Self::new(
        "FastDecompressionCompressingStoredFieldsData",
        "FastDecompressionCompressingStoredFields",
        CompressionMode::fast_decompression(),
        1 << 14,
        256,
        false,
        10,
      ),
      "HighCompressionCompressingStoredFieldsData" => Self::new(
        "HighCompressionCompressingStoredFieldsData",
        "HighCompressionCompressingStoredFields",
        CompressionMode::high_compression(),
        61_440,
        512,
        false,
        10,
      ),
      "DummyCompressingStoredFieldsData" => {
        DummyCompressingCodec::default_instance().map(Into::into)
      },
      "DeflateWithPresetCompressingStoredFieldsData" => Self::new(
        "DeflateWithPresetCompressingStoredFieldsData",
        "DeflateWithPresetCompressingStoredFields",
        CompressionModeEnum::DeflateDict(DeflateWithPresetDictCompressionMode),
        1 << 18,
        512,
        false,
        10,
      ),
      "LZ4WithPresetCompressingStoredFieldsData" => Self::new(
        "LZ4WithPresetCompressingStoredFieldsData",
        "DeflateWithPresetCompressingStoredFields",
        CompressionModeEnum::LZ4Dict(LZ4WithPresetDictCompressionMode),
        1 << 18,
        512,
        false,
        10,
      ),
      _ => Err(LuceneError::illegal_argument(format!(
        "unknown compressing codec name: {name}"
      ))),
    }
  }
}

impl Codec for CompressingCodec {
  type PostingsFormat = <DefaultCodec as Codec>::PostingsFormat;
  type DocValuesFormat = <DefaultCodec as Codec>::DocValuesFormat;
  type StoredFieldsFormat = Lucene90CompressingStoredFieldsFormat;
  type TermVectorsFormat = Lucene90CompressingTermVectorsFormat;
  type FieldInfosFormat = <DefaultCodec as Codec>::FieldInfosFormat;
  type SegmentInfoFormat = <DefaultCodec as Codec>::SegmentInfoFormat;
  type NormsFormat = <DefaultCodec as Codec>::NormsFormat;
  type LiveDocsFormat = <DefaultCodec as Codec>::LiveDocsFormat;
  type CompoundFormat = <DefaultCodec as Codec>::CompoundFormat;
  type PointsFormat = <DefaultCodec as Codec>::PointsFormat;
  type KnnVectorsFormat = <DefaultCodec as Codec>::KnnVectorsFormat;

  fn postings_format(&self) -> Self::PostingsFormat {
    self.delegate.postings_format()
  }

  fn doc_values_format(&self) -> Self::DocValuesFormat {
    self.delegate.doc_values_format()
  }

  fn stored_fields_format(&self) -> Self::StoredFieldsFormat {
    self.stored_fields_format.clone()
  }

  fn term_vectors_format(&self) -> Self::TermVectorsFormat {
    self.term_vectors_format.clone()
  }

  fn field_infos_format(&self) -> Self::FieldInfosFormat {
    self.delegate.field_infos_format()
  }

  fn segment_info_format(&self) -> Self::SegmentInfoFormat {
    self.delegate.segment_info_format()
  }

  fn norms_format(&self) -> Self::NormsFormat {
    self.delegate.norms_format()
  }

  fn live_docs_format(&self) -> Self::LiveDocsFormat {
    self.delegate.live_docs_format()
  }

  fn compound_format(&self) -> Self::CompoundFormat {
    self.delegate.compound_format()
  }

  fn points_format(&self) -> Self::PointsFormat {
    self.delegate.points_format()
  }

  fn knn_vectors_format(&self) -> Result<Self::KnnVectorsFormat> {
    self.delegate.knn_vectors_format()
  }

  fn get_name(&self) -> &str {
    self.name
  }
}

impl Display for CompressingCodec {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}(storedFieldsFormat={}, termVectorsFormat={})",
      self.name, self.stored_fields_format, self.term_vectors_format
    )
  }
}
