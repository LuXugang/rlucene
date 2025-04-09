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
use crate::codecs::lz4_with_preset_dict_compression_mode::{
    LZ4WithPresetDictCompressionMode, LZ4WithPresetDictCompressor, LZ4WithPresetDictDecompressor,
};
use crate::index::BytesRef;
use crate::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{DataInput, DataOutput};
use crate::util::compress::lz4::{
    FastCompressionHashTable, HashTableEnum, HighCompressionHashTable, LZ4,
};
use crate::util::error::lucene_error::{LuceneError, Result};
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};

/// A compression mode. Tells how much effort should be spent on compression and decompression of
/// stored fields.
///
/// # Experimental
/// This feature is experimental. Its behavior might change in future versions.
#[allow(unused)]
pub struct CompressionMode;

#[allow(unused)]
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

#[allow(unused)]
pub(crate) trait CompressionModeBase: Display + Clone {
    /// Create a new `Compressor` instance.
    fn new_compressor(&self) -> CompressorEnum;
    /// Create a new `Decompressor` instance.
    fn new_decompressor(&self) -> DecompressorEnum;
}
/// A compression mode that trades compression ratio for speed. Although the compression ratio
/// might remain high, compression and decompression are very fast. Use this mode with indices that
/// have a high update rate but should be able to load documents from disk quickly.
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
/// A compression mode that trades speed for compression ratio. Although compression and
/// decompression might be slow, this compression mode should provide a good compression ratio.
/// This mode might be interesting if/when your index size is much bigger than your OS cache.
#[allow(unused)]
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
        DecompressorEnum::Deflate(DeflateDecompressor)
    }
}

/// This compression mode is similar to `FAST` but it spends more time compressing in order
/// to improve the compression ratio. This compression mode is best used with indices that have a
/// low update rate but should be able to load documents from disk quickly.
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

#[allow(unused)]
pub enum CompressionModeEnum {
    Fast(LZ4FastCompressionMode),
    High(LZ4HighCompressionMode),
    Deflate(DeflateCompressionMode),
    LZ4Dict(LZ4WithPresetDictCompressionMode),
}

impl Display for CompressionModeEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionModeEnum::Fast(mode) => write!(f, "{}", mode),
            CompressionModeEnum::High(mode) => write!(f, "{}", mode),
            CompressionModeEnum::Deflate(mode) => write!(f, "{}", mode),
            CompressionModeEnum::LZ4Dict(mode) => write!(f, "{}", mode),
        }
    }
}

impl CompressionModeBase for CompressionModeEnum {
    fn new_compressor(&self) -> CompressorEnum {
        match self {
            CompressionModeEnum::Fast(mode) => mode.new_compressor(),
            CompressionModeEnum::High(mode) => mode.new_compressor(),
            CompressionModeEnum::Deflate(mode) => mode.new_compressor(),
            CompressionModeEnum::LZ4Dict(mode) => mode.new_compressor(),
        }
    }

    fn new_decompressor(&self) -> DecompressorEnum {
        match self {
            CompressionModeEnum::Fast(mode) => mode.new_decompressor(),
            CompressionModeEnum::High(mode) => mode.new_decompressor(),
            CompressionModeEnum::Deflate(mode) => mode.new_decompressor(),
            CompressionModeEnum::LZ4Dict(mode) => mode.new_decompressor(),
        }
    }
}
impl Clone for CompressionModeEnum {
    fn clone(&self) -> Self {
        match self {
            CompressionModeEnum::Fast(mode) => CompressionModeEnum::Fast(mode.clone()),
            CompressionModeEnum::High(mode) => CompressionModeEnum::High(mode.clone()),
            CompressionModeEnum::Deflate(mode) => CompressionModeEnum::Deflate(mode.clone()),
            CompressionModeEnum::LZ4Dict(mode) => CompressionModeEnum::LZ4Dict(mode.clone()),
        }
    }
}

pub struct LZ4Decompressor;
impl Clone for LZ4Decompressor {
    fn clone(&self) -> Self {
        LZ4Decompressor
    }
}
impl Decompressor for LZ4Decompressor {
    fn decompress<I>(
        &mut self,
        input: &mut I,
        original_length: i32,
        offset: i32,
        length: i32,
        bytes: &mut BytesRef,
    ) -> Result<()>
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

#[allow(unused)]
pub enum DecompressorEnum {
    LZ4(LZ4Decompressor),
    Deflate(DeflateDecompressor),
    LZ4Dict(LZ4WithPresetDictDecompressor),
}

impl Clone for DecompressorEnum {
    fn clone(&self) -> Self {
        match self {
            DecompressorEnum::LZ4(decompressor) => DecompressorEnum::LZ4(decompressor.clone()),
            DecompressorEnum::Deflate(decompressor) => {
                DecompressorEnum::Deflate(decompressor.clone())
            }
            DecompressorEnum::LZ4Dict(decompressor) => {
                DecompressorEnum::LZ4Dict(decompressor.clone())
            }
        }
    }
}

impl Decompressor for DecompressorEnum {
    fn decompress<I>(
        &mut self,
        input: &mut I,
        original_length: i32,
        offset: i32,
        length: i32,
        bytes: &mut BytesRef,
    ) -> Result<()>
    where
        I: DataInput,
    {
        match self {
            DecompressorEnum::LZ4(decompressor) => {
                decompressor.decompress(input, original_length, offset, length, bytes)
            }
            DecompressorEnum::Deflate(decompressor) => {
                decompressor.decompress(input, original_length, offset, length, bytes)
            }
            DecompressorEnum::LZ4Dict(decompressor) => {
                decompressor.decompress(input, original_length, offset, length, bytes)
            }
        }
    }
}

pub(crate) struct LZ4FastCompressor {
    ht: HashTableEnum,
}
impl LZ4FastCompressor {
    #[allow(unused)]
    pub fn new() -> Self {
        LZ4FastCompressor {
            ht: HashTableEnum::Fast(FastCompressionHashTable::new()),
        }
    }
}

impl Compressor for LZ4FastCompressor {
    fn compress<D>(&mut self, buffers_input: &mut ByteBuffersDataInput, out: &mut D) -> Result<()>
    where
        D: DataOutput,
    {
        let len = buffers_input.length() as i32;
        let mut bytes = vec![0u8; len as usize];
        DataInput::read_bytes(buffers_input, bytes.as_mut_slice(), 0, len)?;
        LZ4::compress(bytes, 0, len, out, &mut self.ht)
    }
}

pub(crate) struct LZ4HighCompressor {
    ht: HashTableEnum,
}
impl LZ4HighCompressor {
    #[allow(unused)]
    pub fn new(ht: HighCompressionHashTable) -> Self {
        LZ4HighCompressor {
            ht: HashTableEnum::High(ht),
        }
    }
}
impl Compressor for LZ4HighCompressor {
    fn compress<D>(&mut self, buffers_input: &mut ByteBuffersDataInput, out: &mut D) -> Result<()>
    where
        D: DataOutput,
    {
        let len = buffers_input.length() as i32;
        let mut bytes = vec![0u8; len as usize];
        DataInput::read_bytes(buffers_input, bytes.as_mut_slice(), 0, len)?;
        LZ4::compress(bytes, 0, len, out, &mut self.ht)
    }
}

pub struct DeflateDecompressor;

impl Clone for DeflateDecompressor {
    fn clone(&self) -> Self {
        DeflateDecompressor
    }
}

impl Decompressor for DeflateDecompressor {
    fn decompress<I>(
        &mut self,
        input: &mut I,
        original_length: i32,
        offset: i32,
        length: i32,
        bytes: &mut BytesRef,
    ) -> Result<()>
    where
        I: DataInput,
    {
        if length == 0 {
            bytes.length = 0;
            return Ok(());
        }
        debug_assert!(offset + length <= original_length);

        let compressed_length = input.read_vint()?;
        let mut compressed = vec![0; compressed_length as usize];
        input.read_bytes(compressed.as_mut_slice(), 0, compressed_length)?;

        let mut decoder = DeflateDecoder::new(compressed.as_slice());
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        if decompressed.len() > original_length as usize {
            return Err(LuceneError::corrupt_index(format!(
                "Lengths mismatch: {} != {} (resource={})",
                decompressed.len(),
                original_length,
                input
            )));
        }
        bytes.bytes = decompressed;
        bytes.offset = offset;
        bytes.length = length;
        Ok(())
    }
}

pub(crate) struct DeflateCompressor {
    level: u32,
}

#[allow(unused)]
impl DeflateCompressor {
    pub fn new(level: u32) -> Self {
        DeflateCompressor { level }
    }
}
impl Compressor for DeflateCompressor {
    fn compress<D>(&mut self, buffers_input: &mut ByteBuffersDataInput, out: &mut D) -> Result<()>
    where
        D: DataOutput,
    {
        let len = buffers_input.length() as i32;
        let mut bytes = vec![0; len as usize];
        DataInput::read_bytes(buffers_input, bytes.as_mut_slice(), 0, len)?;
        let mut compressor = DeflateEncoder::new(Vec::new(), Compression::new(self.level));
        compressor.write_all(&bytes)?;
        let compressed = compressor.finish()?;
        debug_assert!(compressed.len() <= i32::MAX as usize);
        out.write_vint(compressed.len() as i32)?;
        out.write_bytes_with_len(&compressed, compressed.len() as i32)?;
        Ok(())
    }
}

#[allow(unused)]
pub enum CompressorEnum {
    LZ4Fast(LZ4FastCompressor),
    LZ4High(LZ4HighCompressor),
    Deflate(DeflateCompressor),
    LZ4Dict(LZ4WithPresetDictCompressor),
}
impl Compressor for CompressorEnum {
    fn compress<D>(&mut self, buffers_input: &mut ByteBuffersDataInput, out: &mut D) -> Result<()>
    where
        D: DataOutput,
    {
        match self {
            CompressorEnum::LZ4Fast(compressor) => compressor.compress(buffers_input, out),
            CompressorEnum::LZ4High(compressor) => compressor.compress(buffers_input, out),
            CompressorEnum::Deflate(compressor) => compressor.compress(buffers_input, out),
            CompressorEnum::LZ4Dict(compressor) => compressor.compress(buffers_input, out),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::codecs::compression::compression_mode::{
        CompressionMode, CompressionModeBase, CompressionModeEnum, CompressorEnum, DecompressorEnum,
    };
    use crate::codecs::compression::compressor::Compressor;
    use crate::codecs::compression::decompressor::Decompressor;
    use crate::index::BytesRef;
    use crate::store::byte_buffers_data_input::ByteBuffersDataInput;
    use crate::store::{ByteArrayDataInput, ByteArrayDataOutput};
    use crate::test::util::lucene_test_case::{at_least, is_night_mode, random};

    use crate::test::util::test_util::TestUtil;
    use crate::util::array_util::ArrayUtil;

    use crate::codecs::lz4_with_preset_dict_compression_mode::LZ4WithPresetDictCompressionMode;
    use crate::util::error::lucene_error::Result;
    use rand::rngs::StdRng;
    use rand::Rng;

    use std::io::Cursor;

    trait AbstractTestCompressionMode {
        fn get_mode(&self) -> CompressionModeEnum;
        fn random_array(random: &mut StdRng) -> (Vec<u8>, i32) {
            let bigsize = if is_night_mode() {
                192 * 1024
            } else {
                33 * 1024
            };
            let max = if random.random_bool(0.5) {
                random.random_range(0..4)
            } else {
                random.random_range(0..255)
            };
            let length = if random.random_bool(0.5) {
                random.random_range(0..20)
            } else {
                random.random_range(0..bigsize)
            };
            (Self::random_array_impl(random, length, max), length)
        }
        fn random_array_impl(random: &mut StdRng, length: i32, _max: i32) -> Vec<u8> {
            let remainder = length % 1024;
            let new_length = if remainder == 0 {
                length
            } else {
                length + (1024 - remainder)
            };
            if length == 0 {
                vec![0u8; 1024]
            } else {
                let mut arr = vec![0u8; new_length as usize];
                for i in 0..length {
                    // TODO: 为什么这里使用0..max就报错呢？
                    // arr[i as usize] = random.random_range(0..=max) as u8;
                    arr[i as usize] = random.random();
                }
                arr
            }
        }

        fn compress(&self, decompressed: &[u8], off: i32, len: i32, limit: i32) -> Result<Vec<u8>> {
            let mut compressor = self.get_mode().new_compressor();
            Self::compress_with_compressor(&mut compressor, decompressed, off, len, limit)
        }

        fn compress_with_compressor(
            compressor: &mut CompressorEnum,
            decompressed: &[u8],
            off: i32,
            len: i32,
            limit: i32,
        ) -> Result<Vec<u8>> {
            let compressed_len = len * 3 + 16;
            let compressed = vec![0; compressed_len as usize]; // should be enough
            let mut cursor_vec = Vec::new();
            let chunk_size = 1024;
            let decompressed_len = decompressed.len() as i64;
            let vec = vec![0u8; chunk_size];
            let empty = vec.as_slice();
            if decompressed_len == 0 {
                cursor_vec.push(Cursor::new(empty));
            } else {
                for chunk in decompressed.chunks(chunk_size) {
                    cursor_vec.push(Cursor::new(chunk));
                }
            }

            let mut input = ByteBuffersDataInput::new(cursor_vec, limit as i64)
                .slice(off as i64, len as i64)?;
            let mut out = ByteArrayDataOutput::with_bytes(compressed);

            compressor.compress(&mut input, &mut out)?;
            let compressed_len = out.get_position();
            let result = ArrayUtil::copy_of_sub_array(&out.bytes, 0, compressed_len);
            Ok(result)
        }

        fn decompress(&self, compressed: Vec<u8>, original_length: i32) -> Result<Vec<u8>> {
            let mut decompressor = self.get_mode().new_decompressor();
            Self::decompress_with_decompressor(&mut decompressor, compressed, original_length)
        }

        fn decompress_with_decompressor(
            decompressor: &mut DecompressorEnum,
            compressed: Vec<u8>,
            original_length: i32,
        ) -> Result<Vec<u8>> {
            let mut bytes = BytesRef::default();
            let mut input = ByteArrayDataInput::with_bytes(compressed);
            decompressor.decompress(&mut input, original_length, 0, original_length, &mut bytes)?;
            Ok(BytesRef::deep_copy_of(&bytes).bytes)
        }
        fn decompress_with_range(
            &self,
            compressed: Vec<u8>,
            original_length: i32,
            offset: i32,
            length: i32,
        ) -> Result<Vec<u8>> {
            let mut decompressor = self.get_mode().new_decompressor();
            let mut bytes = BytesRef::default();
            let mut input = ByteArrayDataInput::with_bytes(compressed);
            decompressor.decompress(&mut input, original_length, offset, length, &mut bytes)?;
            Ok(BytesRef::deep_copy_of(&bytes).bytes)
        }

        fn test_decompress(&self, random: &mut StdRng) -> Result<()> {
            let iterations = at_least(random, 3);
            for _ in 0..iterations {
                let (decompressed, limit) = Self::random_array(random);
                let decompressed_len = decompressed.len();
                assert!(decompressed_len <= i32::MAX as usize);
                assert!(limit as usize <= decompressed_len);
                let off = if random.random_bool(0.5) {
                    0
                } else {
                    TestUtil::next_int(random, 0, limit)
                };
                let len = if random.random_bool(0.5) {
                    limit - off
                } else {
                    TestUtil::next_int(random, 0, limit - off)
                };
                let compressed = self.compress(decompressed.as_slice(), off, len, limit)?;
                let restored = self.decompress(compressed, len)?;
                assert_eq!(
                    ArrayUtil::copy_of_sub_array(&decompressed, off, off + len),
                    restored
                );
            }
            Ok(())
        }

        fn test_partial_decompress(&self, random: &mut StdRng) -> Result<()> {
            let iterations = at_least(random, 3);
            for _ in 0..iterations {
                let (decompressed, limit) = Self::random_array(random);
                let compressed = self.compress(
                    &decompressed,
                    0,
                    std::cmp::min(decompressed.len(), limit as usize) as i32,
                    limit,
                )?;
                assert!(decompressed.len() <= i32::MAX as usize);
                let valid_len = std::cmp::min(decompressed.len(), limit as usize) as i32;
                let (offset, length) = if valid_len == 0 {
                    (0, 0)
                } else {
                    let offset_inner = random.random_range(0..valid_len);
                    (
                        offset_inner,
                        random.random_range(0..valid_len - offset_inner),
                    )
                };
                let restored = self.decompress_with_range(compressed, valid_len, offset, length)?;
                assert_eq!(
                    ArrayUtil::copy_of_sub_array(&decompressed, offset, offset + length),
                    restored
                );
            }
            Ok(())
        }

        fn test(&self, decompressed: &[u8], limit: i32) -> Result<Vec<u8>> {
            self.test_with_range(decompressed, 0, decompressed.len() as i32, limit)
        }

        fn test_with_range(
            &self,
            decompressed: &[u8],
            off: i32,
            len: i32,
            limit: i32,
        ) -> Result<Vec<u8>> {
            assert!(off <= limit);
            assert!(limit <= len);
            let compressed = self.compress(decompressed, off, std::cmp::min(len, limit), limit)?;
            let compressed_copy = compressed.clone();
            let restored = self.decompress(compressed, limit)?;
            assert_eq!(limit as usize, restored.len());
            assert_eq!(
                ArrayUtil::copy_of_sub_array(decompressed, off, off + std::cmp::min(len, limit)),
                restored
            );
            Ok(compressed_copy)
        }

        fn test_empty_sequence(&self) -> Result<()> {
            self.test(&[], 0)?;
            Ok(())
        }

        fn test_short_sequence(&self, random: &mut StdRng) -> Result<()> {
            let limit = random.random_range(0..256);
            let mut bytes = vec![0u8; 1024];
            for byte in bytes.iter_mut().take(limit) {
                *byte = random.random();
            }
            self.test(&bytes, limit as i32)?;
            Ok(())
        }

        fn test_incompressible(&self, random: &mut StdRng) -> Result<()> {
            let limit = random.random_range(20..=256);
            let mut decompressed = vec![0; 1024];
            for byte in decompressed.iter_mut().take(limit) {
                *byte = random.random();
            }
            self.test(&decompressed, limit as i32)?;
            Ok(())
        }

        fn test_constant(&self, random: &mut StdRng) -> Result<()> {
            let limit = TestUtil::next_int(random, 1, 10000);
            let mut decompressed = vec![0; 10240];
            for byte in decompressed.iter_mut().take(limit as usize) {
                *byte = random.random();
            }
            self.test(&decompressed, limit)?;
            Ok(())
        }

        fn test_extremely_large_input(&self) -> Result<()> {
            let limit = 1 << 24; // 16MB
            let mut decompressed = vec![0u8; limit as usize];
            for (i, byte) in decompressed.iter_mut().enumerate() {
                *byte = (i & 0x0F) as u8
            }
            self.test(&decompressed, limit)?;
            Ok(())
        }
    }

    // TestFastCompressionMode
    struct TestFastCompressionMode;
    impl AbstractTestCompressionMode for TestFastCompressionMode {
        fn get_mode(&self) -> CompressionModeEnum {
            CompressionMode::fast()
        }
    }
    // TestFastDecompressionMode
    struct TestFastDecompressionMode;
    impl AbstractTestCompressionMode for TestFastDecompressionMode {
        fn get_mode(&self) -> CompressionModeEnum {
            CompressionMode::fast_decompression()
        }
    }
    // TestHighCompressionMode
    struct TestHighCompressionMode;
    impl AbstractTestCompressionMode for TestHighCompressionMode {
        fn get_mode(&self) -> CompressionModeEnum {
            CompressionMode::high_compression()
        }
    }

    // TestLZ4WithPresetDictCompressionMode
    struct TestLZ4WithPresetDictCompressionMode;
    impl AbstractTestCompressionMode for TestLZ4WithPresetDictCompressionMode {
        fn get_mode(&self) -> CompressionModeEnum {
            CompressionModeEnum::LZ4Dict(LZ4WithPresetDictCompressionMode)
        }
    }
    #[test]
    fn test_decompress() -> Result<()> {
        let mut random = random();
        TestFastCompressionMode.test_decompress(&mut random)?;
        TestFastDecompressionMode.test_decompress(&mut random)?;
        TestHighCompressionMode.test_decompress(&mut random)?;
        TestLZ4WithPresetDictCompressionMode.test_decompress(&mut random)
    }
    #[test]
    fn test_partial_decompress() -> Result<()> {
        let mut random = random();
        TestFastCompressionMode.test_partial_decompress(&mut random)?;
        TestFastDecompressionMode.test_partial_decompress(&mut random)?;
        TestHighCompressionMode.test_partial_decompress(&mut random)?;
        TestLZ4WithPresetDictCompressionMode.test_partial_decompress(&mut random)
    }
    #[test]
    fn test_empty_sequence() -> Result<()> {
        TestFastCompressionMode.test_empty_sequence()?;
        TestFastDecompressionMode.test_empty_sequence()?;
        TestHighCompressionMode.test_empty_sequence()?;
        TestLZ4WithPresetDictCompressionMode.test_empty_sequence()
    }
    #[test]
    fn test_short_sequence() -> Result<()> {
        let mut random = random();
        TestFastCompressionMode.test_short_sequence(&mut random)?;
        TestFastDecompressionMode.test_short_sequence(&mut random)?;
        TestHighCompressionMode.test_short_sequence(&mut random)?;
        TestLZ4WithPresetDictCompressionMode.test_short_sequence(&mut random)
    }
    #[test]
    fn test_incompressible() -> Result<()> {
        let mut random = random();
        TestFastCompressionMode.test_incompressible(&mut random)?;
        TestFastDecompressionMode.test_incompressible(&mut random)?;
        TestHighCompressionMode.test_incompressible(&mut random)?;
        TestLZ4WithPresetDictCompressionMode.test_incompressible(&mut random)
    }
    #[test]
    fn test_constant() -> Result<()> {
        let mut random = random();
        TestFastCompressionMode.test_constant(&mut random)?;
        TestFastDecompressionMode.test_constant(&mut random)?;
        TestHighCompressionMode.test_constant(&mut random)?;
        TestLZ4WithPresetDictCompressionMode.test_constant(&mut random)
    }
    #[test]
    fn test_extremely_large_input() -> Result<()> {
        TestFastCompressionMode.test_extremely_large_input()?;
        TestFastDecompressionMode.test_extremely_large_input()?;
        TestHighCompressionMode.test_extremely_large_input()?;
        TestLZ4WithPresetDictCompressionMode.test_extremely_large_input()
    }
}
