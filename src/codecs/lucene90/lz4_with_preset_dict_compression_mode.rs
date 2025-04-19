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
use crate::codecs::compression::compression_mode::{
    CompressionModeBase, CompressorEnum, DecompressorEnum,
};
use crate::codecs::compression::compressor::Compressor;
use crate::codecs::compression::decompressor::Decompressor;
use crate::index::BytesRef;
use crate::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{ByteBuffersDataOutput, DataInput, DataOutput};
use crate::util::array_util::ArrayUtil;
use crate::util::clone::TryClone;
use crate::util::compress::lz4::{FastCompressionHashTable, HashTableEnum, LZ4};
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::SliceCopyOps;
use std::fmt::{Display, Formatter};

pub struct LZ4WithPresetDictCompressionMode;
impl LZ4WithPresetDictCompressionMode {
    // Shoot for 10 sub blocks
    const NUM_SUB_BLOCKS: i32 = 10;
    // And a dictionary whose size is about 2x smaller than sub blocks
    const DICT_SIZE_FACTOR: i32 = 2;
}

impl Display for LZ4WithPresetDictCompressionMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BEST_SPEED")
    }
}

impl Clone for LZ4WithPresetDictCompressionMode {
    fn clone(&self) -> Self {
        LZ4WithPresetDictCompressionMode
    }
}

impl CompressionModeBase for LZ4WithPresetDictCompressionMode {
    fn new_compressor(&self) -> CompressorEnum {
        CompressorEnum::LZ4Dict(LZ4WithPresetDictCompressor::new())
    }

    fn new_decompressor(&self) -> DecompressorEnum {
        DecompressorEnum::LZ4Dict(LZ4WithPresetDictDecompressor::new())
    }
}

#[derive(Clone)]
pub struct LZ4WithPresetDictDecompressor {
    compressed_lengths: Vec<i32>,
    buffer: Vec<u8>,
}

impl Default for LZ4WithPresetDictDecompressor {
    fn default() -> Self {
        Self::new()
    }
}

impl LZ4WithPresetDictDecompressor {
    pub fn new() -> Self {
        LZ4WithPresetDictDecompressor {
            compressed_lengths: Vec::new(),
            buffer: Vec::new(),
        }
    }

    fn read_compressed_lengths(
        &mut self,
        input: &mut impl DataInput,
        original_length: i32,
        dict_length: i32,
        block_length: i32,
    ) -> Result<i32> {
        input.read_vint()?; // Compressed length of the dictionary, unused
        let mut total_length = dict_length;
        let mut i = 0;
        if let Some(new_array) =
            ArrayUtil::grow_no_copy(&self.compressed_lengths, original_length / block_length + 1)?
        {
            self.compressed_lengths = new_array
        };
        while total_length < original_length {
            self.compressed_lengths[i as usize] = input.read_vint()?;
            total_length += block_length;
            i += 1;
        }
        Ok(i)
    }
}

impl TryClone for LZ4WithPresetDictDecompressor {
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(self.clone())
    }
}

impl Decompressor for LZ4WithPresetDictDecompressor {
    fn decompress(
        &mut self,
        input: &mut impl DataInput,
        original_length: i32,
        offset: i32,
        length: i32,
        bytes: &mut BytesRef,
    ) -> Result<()> {
        debug_assert!(offset + length <= original_length);

        if length == 0 {
            bytes.length = 0;
            return Ok(());
        }
        let dict_length = input.read_vint()?; // Read dictionary length
        let block_length = input.read_vint()?; // Read block length

        let num_blocks =
            self.read_compressed_lengths(input, original_length, dict_length, block_length)?;

        // Grow the buffer to fit the dictionary and block length
        if let Some(new_array) = ArrayUtil::grow_no_copy(&self.buffer, dict_length + block_length)?
        {
            self.buffer = new_array
        }
        bytes.length = 0;

        // Read the dictionary
        if LZ4::decompress(input, dict_length, &mut self.buffer, 0)? != dict_length {
            return Err(LuceneError::corrupt_index(format!(
                "Illegal dict length  (resource={})",
                input
            )));
        }

        let mut offset_in_block = dict_length;
        let mut offset_in_bytes_ref = offset;

        if offset >= dict_length {
            offset_in_bytes_ref -= dict_length;

            // Skip unneeded blocks
            let mut num_bytes_to_skip = 0;
            for i in 0..num_blocks {
                if offset_in_block + block_length < offset {
                    let compressed_block_length = self.compressed_lengths[i as usize];
                    num_bytes_to_skip += compressed_block_length;
                    offset_in_block += block_length;
                    offset_in_bytes_ref -= block_length;
                } else {
                    break;
                }
            }
            input.skip_bytes(num_bytes_to_skip as i64)?;
        } else {
            // The dictionary contains some bytes we need, copy its content to the BytesRef
            if let Some(new_array) = ArrayUtil::grow_no_copy(&bytes.bytes, dict_length)? {
                bytes.bytes = new_array
            }
            bytes
                .bytes
                .copy_from(&self.buffer[0..dict_length as usize], 0);
            bytes.length = dict_length;
        }

        // Read blocks that intersect with the interval we need
        if offset_in_block < offset + length {
            ArrayUtil::grow_with_len(
                &mut bytes.bytes,
                bytes.length + offset + length - offset_in_block,
            )?;
        }

        while offset_in_block < offset + length {
            let bytes_to_decompress = (offset + length - offset_in_block).min(block_length);
            LZ4::decompress(input, bytes_to_decompress, &mut self.buffer, dict_length)?;
            bytes.bytes.copy_from(
                &self.buffer[dict_length as usize..(dict_length + bytes_to_decompress) as usize],
                bytes.length as usize,
            );
            bytes.length += bytes_to_decompress;
            offset_in_block += block_length;
        }
        bytes.offset = offset_in_bytes_ref;
        bytes.length = length;
        debug_assert!(bytes.is_valid()?);
        Ok(())
    }
}

pub struct LZ4WithPresetDictCompressor {
    compressed: ByteBuffersDataOutput,
    hash_table: HashTableEnum,
    buffer: Vec<u8>,
}

impl LZ4WithPresetDictCompressor {
    #[allow(unused)]
    fn new() -> Self {
        LZ4WithPresetDictCompressor {
            compressed: ByteBuffersDataOutput::with_resettable_instance(),
            hash_table: HashTableEnum::Fast(FastCompressionHashTable::new()),
            buffer: Vec::new(),
        }
    }
    fn do_compress(
        &mut self,
        bytes: Vec<u8>,
        dict_len: i32,
        len: i32,
        out: &mut impl DataOutput,
    ) -> Result<()> {
        let prev_compressed_size = self.compressed.size();
        LZ4::compress_with_dictionary(
            bytes,
            0,
            dict_len,
            len,
            &mut self.compressed,
            &mut self.hash_table,
        )?;
        // Write the number of compressed bytes
        out.write_vint((self.compressed.size() - prev_compressed_size).try_into()?)
    }
}
impl Compressor for LZ4WithPresetDictCompressor {
    fn compress(
        &mut self,
        buffers_input: &mut ByteBuffersDataInput,
        out: &mut impl DataOutput,
    ) -> Result<()> {
        let len = (buffers_input.length() - buffers_input.position()) as i32;
        let dict_length = (len
            / (LZ4WithPresetDictCompressionMode::NUM_SUB_BLOCKS
                * LZ4WithPresetDictCompressionMode::DICT_SIZE_FACTOR))
            .min(LZ4::MAX_DISTANCE);
        let block_length = (len - dict_length + LZ4WithPresetDictCompressionMode::NUM_SUB_BLOCKS
            - 1)
            / LZ4WithPresetDictCompressionMode::NUM_SUB_BLOCKS;

        if let Some(new_array) = ArrayUtil::grow_no_copy(&self.buffer, dict_length + block_length)?
        {
            self.buffer = new_array
        }

        out.write_vint(dict_length)?;
        out.write_vint(block_length)?;

        self.compressed.reset();
        // Compress the dictionary first
        DataInput::read_bytes(buffers_input, &mut self.buffer, 0, dict_length)?;
        let moved_data = std::mem::take(&mut self.buffer);
        self.do_compress(moved_data, 0, dict_length, out)?;

        // And then sub blocks
        let mut start = dict_length;
        while start < len {
            let l = (len - start).min(block_length);
            debug_assert!(self.buffer.is_empty());
            self.buffer = vec![0; (dict_length + block_length) as usize];
            DataInput::read_bytes(buffers_input, &mut self.buffer, dict_length, l)?;
            let moved_data = std::mem::take(&mut self.buffer);
            self.do_compress(moved_data, dict_length, l, out)?;
            start += block_length;
        }
        // We only wrote lengths so far, now write compressed data
        self.compressed.copy_to(out)?;
        Ok(())
    }
}
