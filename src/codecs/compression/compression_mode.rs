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
use crate::codecs::compression::compressor::Compressor;
use crate::codecs::compression::decompressor::Decompressor;
use crate::index::BytesRef;
use crate::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{DataInput, DataOutput};
use crate::util::compress::lz4::{
    FastCompressionHashTable, HashTableEnum, HighCompressionHashTable, LZ4,
};
use crate::util::error::lucene_error::LuceneError;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::fmt::{Display, Formatter};
use std::io::{Cursor, Write};
use std::sync::Arc;
/// A compression mode. Tells how much effort should be spent on compression and decompression of
/// stored fields.
///
/// # Experimental
/// This feature is experimental. Its behavior might change in future versions.
pub struct CompressionMode;

impl CompressionMode {
    pub fn fast() -> CompressionModeEnum {
        CompressionModeEnum::Fast(LZ4FastCompressionMode)
    }
}

trait CompressionModeBase: Display {
    /// Create a new `Compressor` instance.
    fn new_compressor(&self) -> CompressorEnum;
    /// Create a new `Decompressor` instance.
    fn new_decompressor(&self) -> DecompressorEnum;
}
enum CompressionModeEnum {
    Fast(LZ4FastCompressionMode),
}

/// A compression mode that trades compression ratio for speed. Although the compression ratio
/// might remain high, compression and decompression are very fast. Use this mode with indices that
/// have a high update rate but should be able to load documents from disk quickly.
struct LZ4FastCompressionMode;

impl Display for LZ4FastCompressionMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FAST")
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

struct LZ4Decompressor;
impl Clone for LZ4Decompressor {
    fn clone(&self) -> Self {
        LZ4Decompressor
    }
}
impl Decompressor for LZ4Decompressor {
    fn decompress<I>(
        &self,
        input: &mut I,
        original_length: i32,
        offset: i32,
        length: i32,
        bytes: &mut BytesRef,
    ) -> Result<(), LuceneError>
    where
        I: DataInput,
    {
        debug_assert!(offset + length <= original_length);

        // Add 7 padding bytes, not necessary but helps with decompression performance
        if bytes.bytes.len() < (original_length + 7) as usize {
            bytes.bytes = vec![0; (original_length + 7) as usize];
        }
        let decompressed_length = LZ4::decompress(input, offset + length, &mut bytes.bytes, 0)?;
        if decompressed_length > original_length {
            return Err(LuceneError::corrupt_index(format!(
                "Corrupted: lengths mismatch: {} > {} (resource={})",
                decompressed_length, original_length, input
            )));
        }
        bytes.offset = offset;
        bytes.length = length;
        Ok(())
    }
}

pub enum DecompressorEnum {
    LZ4(LZ4Decompressor),
}

impl Clone for DecompressorEnum {
    fn clone(&self) -> Self {
        match self {
            DecompressorEnum::LZ4(decompressor) => DecompressorEnum::LZ4(decompressor.clone()),
        }
    }
}

impl Decompressor for DecompressorEnum {
    fn decompress<I>(
        &self,
        input: &mut I,
        original_length: i32,
        offset: i32,
        length: i32,
        bytes: &mut BytesRef,
    ) -> Result<(), LuceneError>
    where
        I: DataInput,
    {
        match self {
            DecompressorEnum::LZ4(decompressor) => {
                decompressor.decompress(input, original_length, offset, length, bytes)
            }
        }
    }
}

struct LZ4FastCompressor {
    ht: HashTableEnum,
}
impl LZ4FastCompressor {
    pub fn new() -> Self {
        LZ4FastCompressor {
            ht: HashTableEnum::FastCompressionHashTable(FastCompressionHashTable::new()),
        }
    }
}

impl Compressor for LZ4FastCompressor {
    fn compress<D>(
        &mut self,
        buffers_input: &mut ByteBuffersDataInput,
        out: &mut D,
    ) -> Result<(), LuceneError>
    where
        D: DataOutput,
    {
        let len = buffers_input.length() as i32;
        let mut bytes = vec![0u8; len as usize];
        DataInput::read_bytes(buffers_input, bytes.as_mut_slice(), 0, len)?;
        LZ4::compress(Arc::new(bytes), 0, len, out, &mut self.ht)
    }
}

struct LZ4HighCompressor {
    ht: HashTableEnum,
}
impl LZ4HighCompressor {
    pub fn new(ht: HighCompressionHashTable) -> Self {
        LZ4HighCompressor {
            ht: HashTableEnum::HighCompressionHashTable(ht),
        }
    }
}
impl Compressor for LZ4HighCompressor {
    fn compress<D>(
        &mut self,
        buffers_input: &mut ByteBuffersDataInput,
        out: &mut D,
    ) -> Result<(), LuceneError>
    where
        D: DataOutput,
    {
        let len = buffers_input.length() as i32;
        let mut bytes = vec![0u8; len as usize];
        DataInput::read_bytes(buffers_input, bytes.as_mut_slice(), 0, len)?;
        LZ4::compress(Arc::new(bytes), 0, len, out, &mut self.ht)
    }
}

struct DeflateCompressor {
    compressor: DeflateEncoder<Cursor<Vec<u8>>>,
    compressed: Vec<u8>,
    closed: bool,
}

impl DeflateCompressor {
    pub fn new(level: u32) -> Self {
        let compressor = DeflateEncoder::new(Cursor::new(Vec::new()), Compression::new(level));
        DeflateCompressor {
            compressor,
            compressed: vec![0; 64],
            closed: false,
        }
    }
}
impl Compressor for DeflateCompressor {
    fn compress<D>(
        &mut self,
        buffers_input: &mut ByteBuffersDataInput,
        out: &mut D,
    ) -> Result<(), LuceneError>
    where
        D: DataOutput,
    {
        let len = buffers_input.length() as i32;
        let mut bytes = vec![0u8; len as usize];
        DataInput::read_bytes(buffers_input, bytes.as_mut_slice(), 0, len)?;
        self.compressor.get_mut().write_all(&bytes)?;
        let compressed = self.compressor.get_mut().get_ref();
        let compressed_len = compressed.len() as i32;
        out.write_bytes_with_len(compressed, compressed_len)?;
        Ok(())
    }
}

pub enum CompressorEnum {
    LZ4Fast(LZ4FastCompressor),
    LZ4High(LZ4HighCompressor),
    Deflate(DeflateCompressor),
}
impl Compressor for CompressorEnum {
    fn compress<D>(
        &mut self,
        buffers_input: &mut ByteBuffersDataInput,
        out: &mut D,
    ) -> Result<(), LuceneError>
    where
        D: DataOutput,
    {
        match self {
            CompressorEnum::LZ4Fast(compressor) => compressor.compress(buffers_input, out),
            CompressorEnum::LZ4High(compressor) => compressor.compress(buffers_input, out),
            CompressorEnum::Deflate(compressor) => compressor.compress(buffers_input, out),
        }
    }
}
