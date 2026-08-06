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
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;

use crate::core::codecs::compression::compressor::Compressor;
use crate::core::codecs::compression::decompressor::Decompressor;
use crate::core::codecs::lucene90::deflate_with_preset_dict_compression_mode::{
  DeflateWithPresetDictCompressionMode, DeflateWithPresetDictCompressor,
  DeflateWithPresetDictDecompressor,
};
use crate::core::codecs::lz4_with_preset_dict_compression_mode::{
  LZ4WithPresetDictCompressionMode, LZ4WithPresetDictCompressor, LZ4WithPresetDictDecompressor,
};
use crate::core::index::BytesRef;
use crate::core::index::sorting_stored_fields_consumer::{
  CompressorImpl, DecompressorImpl, NoCompression,
};
use crate::core::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::core::store::{DataInput, DataOutput};
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::close::Closeable;
use crate::core::util::compress::lz4::{
  FastCompressionHashTable, HashTableEnum, HighCompressionHashTable, LZ4,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};
#[cfg(test)]
use crate::test_framework::core::codecs::compressing::dummy::dummy_compressing_codec::{
  DummyCompressionMode, DummyCompressor, DummyDecompressor,
};

/// A compression mode. Tells how much effort should be spent on compression and
/// decompression of stored fields.
///
/// # Experimental
/// This feature is experimental. Its behavior might change in future versions.
pub struct CompressionMode;

impl CompressionMode {
  pub fn fast() -> CompressionModeEnum {
    CompressionModeEnum::Fast(LZ4FastCompressionMode)
  }
  pub fn fast_decompression() -> CompressionModeEnum {
    CompressionModeEnum::High(LZ4HighCompressionMode)
  }
  pub fn high_compression() -> CompressionModeEnum {
    CompressionModeEnum::Deflate(DeflateCompressionMode)
  }
}

pub(crate) trait CompressionModeBase: Display + Clone {
  /// Create a new `Compressor` instance.
  fn new_compressor(&self) -> CompressorEnum;
  /// Create a new `Decompressor` instance.
  fn new_decompressor(&self) -> DecompressorEnum;
}
/// A compression mode that trades compression ratio for speed. Although the
/// compression ratio might remain high, compression and decompression are very
/// fast. Use this mode with indices that have a high update rate but should be
/// able to load documents from disk quickly.
#[derive(Debug)]
pub struct LZ4FastCompressionMode;

impl Display for LZ4FastCompressionMode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "FAST")
  }
}

impl Clone for LZ4FastCompressionMode {
  fn clone(&self) -> Self {
    LZ4FastCompressionMode
  }
}

impl CompressionModeBase for LZ4FastCompressionMode {
  fn new_compressor(&self) -> CompressorEnum {
    CompressorEnum::LZ4Fast(LZ4FastCompressor::new())
  }

  fn new_decompressor(&self) -> DecompressorEnum {
    DecompressorEnum::LZ4(LZ4Decompressor)
  }
}
/// A compression mode that trades speed for compression ratio. Although
/// compression and decompression might be slow, this compression mode should
/// provide a good compression ratio. This mode might be interesting if/when
/// your index size is much bigger than your OS cache.
#[derive(Debug)]
pub struct DeflateCompressionMode;

impl Display for DeflateCompressionMode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "HIGH_COMPRESSION")
  }
}

impl Clone for DeflateCompressionMode {
  fn clone(&self) -> Self {
    DeflateCompressionMode
  }
}

impl CompressionModeBase for DeflateCompressionMode {
  fn new_compressor(&self) -> CompressorEnum {
    CompressorEnum::Deflate(DeflateCompressor::new(6))
  }

  fn new_decompressor(&self) -> DecompressorEnum {
    DecompressorEnum::Deflate(DeflateDecompressor::new())
  }
}

/// This compression mode is similar to `FAST` but it spends more time
/// compressing in order to improve the compression ratio. This compression mode
/// is best used with indices that have a low update rate but should be able to
/// load documents from disk quickly.
#[derive(Debug)]
pub struct LZ4HighCompressionMode;

impl Display for LZ4HighCompressionMode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "FAST_DECOMPRESSION")
  }
}

impl Clone for LZ4HighCompressionMode {
  fn clone(&self) -> Self {
    LZ4HighCompressionMode
  }
}

impl CompressionModeBase for LZ4HighCompressionMode {
  fn new_compressor(&self) -> CompressorEnum {
    CompressorEnum::LZ4High(LZ4HighCompressor::new(HighCompressionHashTable::new()))
  }

  fn new_decompressor(&self) -> DecompressorEnum {
    DecompressorEnum::LZ4(LZ4Decompressor)
  }
}
#[derive(Debug)]
pub enum CompressionModeEnum {
  Fast(LZ4FastCompressionMode),
  High(LZ4HighCompressionMode),
  Deflate(DeflateCompressionMode),
  DeflateDict(DeflateWithPresetDictCompressionMode),
  LZ4Dict(LZ4WithPresetDictCompressionMode),
  Impl(NoCompression),
  #[cfg(test)]
  Dummy(DummyCompressionMode),
}

impl Display for CompressionModeEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      CompressionModeEnum::Fast(mode) => write!(f, "{mode}"),
      CompressionModeEnum::High(mode) => write!(f, "{mode}"),
      CompressionModeEnum::Deflate(mode) => write!(f, "{mode}"),
      CompressionModeEnum::DeflateDict(mode) => write!(f, "{mode}"),
      CompressionModeEnum::LZ4Dict(mode) => write!(f, "{mode}"),
      CompressionModeEnum::Impl(mode) => write!(f, "{mode}"),
      #[cfg(test)]
      CompressionModeEnum::Dummy(mode) => write!(f, "{mode}"),
    }
  }
}

impl CompressionModeBase for CompressionModeEnum {
  fn new_compressor(&self) -> CompressorEnum {
    match self {
      CompressionModeEnum::Fast(mode) => mode.new_compressor(),
      CompressionModeEnum::High(mode) => mode.new_compressor(),
      CompressionModeEnum::Deflate(mode) => mode.new_compressor(),
      CompressionModeEnum::DeflateDict(mode) => mode.new_compressor(),
      CompressionModeEnum::LZ4Dict(mode) => mode.new_compressor(),
      CompressionModeEnum::Impl(mode) => mode.new_compressor(),
      #[cfg(test)]
      CompressionModeEnum::Dummy(mode) => mode.new_compressor(),
    }
  }

  fn new_decompressor(&self) -> DecompressorEnum {
    match self {
      CompressionModeEnum::Fast(mode) => mode.new_decompressor(),
      CompressionModeEnum::High(mode) => mode.new_decompressor(),
      CompressionModeEnum::Deflate(mode) => mode.new_decompressor(),
      CompressionModeEnum::DeflateDict(mode) => mode.new_decompressor(),
      CompressionModeEnum::LZ4Dict(mode) => mode.new_decompressor(),
      CompressionModeEnum::Impl(mode) => mode.new_decompressor(),
      #[cfg(test)]
      CompressionModeEnum::Dummy(mode) => mode.new_decompressor(),
    }
  }
}
impl Clone for CompressionModeEnum {
  fn clone(&self) -> Self {
    match self {
      CompressionModeEnum::Fast(mode) => CompressionModeEnum::Fast(mode.clone()),
      CompressionModeEnum::High(mode) => CompressionModeEnum::High(mode.clone()),
      CompressionModeEnum::Deflate(mode) => CompressionModeEnum::Deflate(mode.clone()),
      CompressionModeEnum::DeflateDict(mode) => CompressionModeEnum::DeflateDict(mode.clone()),
      CompressionModeEnum::LZ4Dict(mode) => CompressionModeEnum::LZ4Dict(mode.clone()),
      CompressionModeEnum::Impl(mode) => CompressionModeEnum::Impl(mode.clone()),
      #[cfg(test)]
      CompressionModeEnum::Dummy(mode) => CompressionModeEnum::Dummy(*mode),
    }
  }
}
impl PartialEq for CompressionModeEnum {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Fast(_), Self::Fast(_))
      | (Self::High(_), Self::High(_))
      | (Self::Deflate(_), Self::Deflate(_))
      | (Self::DeflateDict(_), Self::DeflateDict(_))
      | (Self::LZ4Dict(_), Self::LZ4Dict(_))
      | (Self::Impl(_), Self::Impl(_)) => true,
      #[cfg(test)]
      (Self::Dummy(_), Self::Dummy(_)) => true,
      _ => false,
    }
  }
}

impl Eq for CompressionModeEnum {}

pub struct LZ4Decompressor;

impl Clone for LZ4Decompressor {
  fn clone(&self) -> Self {
    LZ4Decompressor
  }
}

impl Decompressor for LZ4Decompressor {
  fn decompress(
    &mut self,
    input: &mut impl DataInput,
    original_length: i32,
    offset: i32,
    length: i32,
    bytes: &mut BytesRef<Vec<u8>>,
  ) -> Result<()> {
    debug_assert!(offset + length <= original_length);

    // Add 7 padding bytes, not necessary but helps with decompression
    // performance
    if bytes.bytes.len() < (original_length + 7) as usize {
      ArrayUtil::grow_no_copy(&mut bytes.bytes, (original_length + 7) as usize)?;
    }
    let decompressed_length = LZ4::decompress(input, offset + length, &mut bytes.bytes, 0)?;
    if decompressed_length > original_length {
      return Err(LuceneError::corrupt_index(format!(
        "Corrupted: lengths mismatch: {decompressed_length} > {original_length} (resource={input})"
      )));
    }
    bytes.offset = offset as usize;
    bytes.length = length as usize;
    Ok(())
  }
}

pub enum DecompressorEnum {
  LZ4(LZ4Decompressor),
  Deflate(DeflateDecompressor),
  DeflateDict(DeflateWithPresetDictDecompressor),
  LZ4Dict(LZ4WithPresetDictDecompressor),
  Impl1(DecompressorImpl),
  #[cfg(test)]
  Dummy(DummyDecompressor),
}

impl Clone for DecompressorEnum {
  fn clone(&self) -> Self {
    match self {
      DecompressorEnum::LZ4(decompressor) => DecompressorEnum::LZ4(decompressor.clone()),
      DecompressorEnum::Deflate(decompressor) => DecompressorEnum::Deflate(decompressor.clone()),
      DecompressorEnum::DeflateDict(decompressor) => {
        DecompressorEnum::DeflateDict(decompressor.clone())
      },
      DecompressorEnum::LZ4Dict(decompressor) => DecompressorEnum::LZ4Dict(decompressor.clone()),
      DecompressorEnum::Impl1(decompressor) => DecompressorEnum::Impl1(decompressor.clone()),
      #[cfg(test)]
      DecompressorEnum::Dummy(decompressor) => DecompressorEnum::Dummy(*decompressor),
    }
  }
}

impl Decompressor for DecompressorEnum {
  fn decompress(
    &mut self,
    input: &mut impl DataInput,
    original_length: i32,
    offset: i32,
    length: i32,
    bytes: &mut BytesRef<Vec<u8>>,
  ) -> Result<()> {
    match self {
      DecompressorEnum::LZ4(decompressor) => {
        decompressor.decompress(input, original_length, offset, length, bytes)
      },
      DecompressorEnum::Deflate(decompressor) => {
        decompressor.decompress(input, original_length, offset, length, bytes)
      },
      DecompressorEnum::DeflateDict(decompressor) => {
        decompressor.decompress(input, original_length, offset, length, bytes)
      },
      DecompressorEnum::LZ4Dict(decompressor) => {
        decompressor.decompress(input, original_length, offset, length, bytes)
      },
      DecompressorEnum::Impl1(decompressor) => {
        decompressor.decompress(input, original_length, offset, length, bytes)
      },
      #[cfg(test)]
      DecompressorEnum::Dummy(decompressor) => {
        decompressor.decompress(input, original_length, offset, length, bytes)
      },
    }
  }
}

pub struct LZ4FastCompressor {
  ht: HashTableEnum,
}
impl LZ4FastCompressor {
  fn new() -> Self {
    LZ4FastCompressor {
      ht: HashTableEnum::Fast(FastCompressionHashTable::new()),
    }
  }
}

impl Compressor for LZ4FastCompressor {
  fn compress(
    &mut self,
    buffers_input: &mut ByteBuffersDataInput<&[u8]>,
    out: &mut impl DataOutput,
  ) -> Result<()> {
    let len = buffers_input.length();
    let mut bytes = vec![0u8; len];
    DataInput::read_bytes(buffers_input, bytes.as_mut_slice(), 0, len)?;
    LZ4::compress(bytes.as_slice(), 0, len as i32, out, &mut self.ht)?;
    Ok(())
  }
}

impl Closeable for LZ4FastCompressor {}

pub struct LZ4HighCompressor {
  ht: HashTableEnum,
}
impl LZ4HighCompressor {
  fn new(ht: HighCompressionHashTable) -> Self {
    LZ4HighCompressor {
      ht: HashTableEnum::High(ht),
    }
  }
}

impl Compressor for LZ4HighCompressor {
  fn compress(
    &mut self,
    buffers_input: &mut ByteBuffersDataInput<&[u8]>,
    out: &mut impl DataOutput,
  ) -> Result<()> {
    let len = buffers_input.length();
    let mut bytes = vec![0u8; len];
    DataInput::read_bytes(buffers_input, bytes.as_mut_slice(), 0, len)?;
    LZ4::compress(bytes.as_slice(), 0, len as i32, out, &mut self.ht)?;
    Ok(())
  }
}

impl Closeable for LZ4HighCompressor {}

pub struct DeflateDecompressor {
  compressed: Vec<u8>,
}

impl DeflateDecompressor {
  fn new() -> Self {
    Self {
      compressed: Vec::new(),
    }
  }
}

impl Clone for DeflateDecompressor {
  fn clone(&self) -> Self {
    Self::new()
  }
}

impl Decompressor for DeflateDecompressor {
  fn decompress(
    &mut self,
    input: &mut impl DataInput,
    original_length: i32,
    offset: i32,
    length: i32,
    bytes: &mut BytesRef<Vec<u8>>,
  ) -> Result<()> {
    if length == 0 {
      bytes.length = 0;
      return Ok(());
    }
    debug_assert!(offset + length <= original_length);

    let compressed_length = input.read_vint()?;
    let compressed_length = compressed_length as usize;
    ArrayUtil::grow_no_copy(&mut self.compressed, compressed_length)?;
    input.read_bytes(&mut self.compressed, 0, compressed_length)?;

    let mut decoder = DeflateDecoder::new(&self.compressed[..compressed_length]);
    bytes.bytes.clear();
    decoder.read_to_end(&mut bytes.bytes)?;
    if bytes.bytes.len() > original_length as usize {
      return Err(LuceneError::corrupt_index(format!(
        "Lengths mismatch: {} != {} (resource={})",
        bytes.bytes.len(),
        original_length,
        input
      )));
    }
    bytes.offset = offset as usize;
    bytes.length = length as usize;
    Ok(())
  }
}

pub struct DeflateCompressor {
  level: u32,
  compressed: Vec<u8>,
}

impl DeflateCompressor {
  fn new(level: u32) -> Self {
    DeflateCompressor {
      level,
      compressed: Vec::with_capacity(64),
    }
  }
}

impl Compressor for DeflateCompressor {
  fn compress(
    &mut self,
    buffers_input: &mut ByteBuffersDataInput<&[u8]>,
    out: &mut impl DataOutput,
  ) -> Result<()> {
    let len = buffers_input.length();
    let mut bytes = vec![0; len];
    DataInput::read_bytes(buffers_input, bytes.as_mut_slice(), 0, len)?;
    self.compressed.clear();
    let compressed = std::mem::take(&mut self.compressed);
    let mut compressor = DeflateEncoder::new(compressed, Compression::new(self.level));
    compressor.write_all(&bytes)?;
    self.compressed = compressor.finish()?;
    debug_assert!(self.compressed.len() <= i32::MAX as usize);
    out.write_vint(self.compressed.len() as i32)?;
    out.write_bytes_with_len(&self.compressed, self.compressed.len())?;
    Ok(())
  }
}

impl Closeable for DeflateCompressor {}

pub enum CompressorEnum {
  LZ4Fast(LZ4FastCompressor),
  LZ4High(LZ4HighCompressor),
  Deflate(DeflateCompressor),
  DeflateDict(DeflateWithPresetDictCompressor),
  LZ4Dict(LZ4WithPresetDictCompressor),
  Impl1(CompressorImpl),
  #[cfg(test)]
  Dummy(DummyCompressor),
}

impl Closeable for CompressorEnum {
  fn close(&mut self) -> Result<()> {
    match self {
      CompressorEnum::LZ4Fast(compressor) => compressor.close(),
      CompressorEnum::LZ4High(compressor) => compressor.close(),
      CompressorEnum::Deflate(compressor) => compressor.close(),
      CompressorEnum::DeflateDict(compressor) => compressor.close(),
      CompressorEnum::LZ4Dict(compressor) => compressor.close(),
      CompressorEnum::Impl1(compressor) => compressor.close(),
      #[cfg(test)]
      CompressorEnum::Dummy(compressor) => compressor.close(),
    }
  }
}

impl Compressor for CompressorEnum {
  fn compress(
    &mut self,
    buffers_input: &mut ByteBuffersDataInput<&[u8]>,
    out: &mut impl DataOutput,
  ) -> Result<()> {
    match self {
      CompressorEnum::LZ4Fast(compressor) => compressor.compress(buffers_input, out),
      CompressorEnum::LZ4High(compressor) => compressor.compress(buffers_input, out),
      CompressorEnum::Deflate(compressor) => compressor.compress(buffers_input, out),
      CompressorEnum::DeflateDict(compressor) => compressor.compress(buffers_input, out),
      CompressorEnum::LZ4Dict(compressor) => compressor.compress(buffers_input, out),
      CompressorEnum::Impl1(compressor) => compressor.compress(buffers_input, out),
      #[cfg(test)]
      CompressorEnum::Dummy(compressor) => compressor.compress(buffers_input, out),
    }
  }
}
