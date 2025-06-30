/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::util::packed::bulk_operation::BulkOperation;
use crate::util::packed::{Decoder, Encoder};

const BLOCK_COUNT: i32 = 1;
/// Non-specialized `BulkOperation` for
/// `PackedInts.Format::PACKED_SINGLE_BLOCK`.
#[derive(Default)]
pub(crate) struct BulkOperationPackedSingleBlock {
    bits_per_value: i32,
    value_count: i32,
    mask: u64,
}
impl BulkOperationPackedSingleBlock {
    pub const fn new(bits_per_value: i32) -> Self {
        Self {
            bits_per_value,
            value_count: 64 / bits_per_value,
            mask: (1u64 << bits_per_value) - 1,
        }
    }
    /// Decodes a block into a slice of `i64` values.
    pub fn decode_to_i64(
        &self,
        mut block: u64,
        values: &mut [i64],
        mut values_offset: usize,
    ) -> usize {
        values[values_offset] = (block & self.mask) as i64;
        values_offset += 1;
        for _ in 1..self.value_count {
            block >>= self.bits_per_value;
            values[values_offset] = (block & self.mask) as i64;
            values_offset += 1;
        }
        values_offset
    }

    /// Decodes a block into a slice of `i32` values.
    pub fn decode_to_i32(
        &self,
        mut block: u64,
        values: &mut [i32],
        mut values_offset: usize,
    ) -> usize {
        values[values_offset] = (block & self.mask) as i32;
        values_offset += 1;
        for _ in 1..self.value_count {
            block >>= self.bits_per_value;
            values[values_offset] = (block & self.mask) as i32;
            values_offset += 1;
        }
        values_offset
    }

    /// Encodes a slice of `i64` values into a block.
    pub fn encode_from_i64(&self, values: &[i64], mut values_offset: usize) -> u64 {
        let mut block = values[values_offset] as u64;
        values_offset += 1;
        for j in 1..self.value_count {
            block |= (values[values_offset] as u64) << (j * self.bits_per_value);
            values_offset += 1;
        }
        block
    }

    /// Encodes a slice of `i32` values into a block.
    pub fn encode_from_i32(&self, values: &[i32], mut values_offset: usize) -> u64 {
        let mut block = (values[values_offset] as u64) & 0xFFFFFFFF;
        values_offset += 1;
        for j in 1..self.value_count {
            block |= ((values[values_offset] as u64) & 0xFFFFFFFF) << (j * self.bits_per_value);
            values_offset += 1;
        }
        block
    }
    fn read_long(blocks: &[u8], blocks_offset: usize) -> u64 {
        ((blocks[blocks_offset] as u64) << 56)
            | ((blocks[blocks_offset + 1] as u64) << 48)
            | ((blocks[blocks_offset + 2] as u64) << 40)
            | ((blocks[blocks_offset + 3] as u64) << 32)
            | ((blocks[blocks_offset + 4] as u64) << 24)
            | ((blocks[blocks_offset + 5] as u64) << 16)
            | ((blocks[blocks_offset + 6] as u64) << 8)
            | (blocks[blocks_offset + 7] as u64)
    }
}
impl Decoder for BulkOperationPackedSingleBlock {
    fn long_block_count(&self) -> i32 {
        BLOCK_COUNT
    }

    fn long_value_count(&self) -> i32 {
        self.value_count
    }

    fn byte_block_count(&self) -> i32 {
        BLOCK_COUNT * 8
    }

    fn byte_value_count(&self) -> i32 {
        self.value_count
    }

    fn decode_u64_to_i64(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            let block = blocks[blocks_offset];
            blocks_offset += 1;
            values_offset = self.decode_to_i64(block, values, values_offset);
        }
    }

    fn decode_u8_to_i64(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            let block = Self::read_long(blocks, blocks_offset);
            blocks_offset += 8;
            values_offset = self.decode_to_i64(block, values, values_offset);
        }
    }

    fn decode_u64_to_i32(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: i32,
    ) {
        debug_assert!(
            self.bits_per_value <= 32,
            "Cannot decode {}-bits values into an i32 array",
            self.bits_per_value
        );
        for _ in 0..iterations {
            let block = blocks[blocks_offset];
            blocks_offset += 1;
            values_offset = self.decode_to_i32(block, values, values_offset);
        }
    }

    fn decode_u8_to_i32(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: i32,
    ) {
        debug_assert!(
            self.bits_per_value <= 32,
            "Cannot decode {}-bits values into an i32 array",
            self.bits_per_value
        );

        for _ in 0..iterations {
            let block = Self::read_long(blocks, blocks_offset);
            blocks_offset += 8;
            values_offset = self.decode_to_i32(block, values, values_offset);
        }
    }
}
impl Encoder for BulkOperationPackedSingleBlock {
    fn long_block_count(&self) -> i32 {
        Decoder::long_block_count(self)
    }

    fn long_value_count(&self) -> i32 {
        Decoder::long_value_count(self)
    }

    fn byte_block_count(&self) -> i32 {
        Decoder::byte_block_count(self)
    }

    fn byte_value_count(&self) -> i32 {
        Decoder::byte_value_count(self)
    }

    fn encode_i64_to_u64(
        &self,
        values: &[i64],
        mut values_offset: usize,
        blocks: &mut [u64],
        mut blocks_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            blocks[blocks_offset] = self.encode_from_i64(values, values_offset);
            blocks_offset += 1;
            values_offset += self.value_count as usize;
        }
    }

    fn encode_i64_to_u8(
        &self,
        values: &[i64],
        mut values_offset: usize,
        blocks: &mut [u8],
        mut blocks_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            let block = self.encode_from_i64(values, values_offset);
            values_offset += self.value_count as usize;
            blocks_offset = self.write_long(block, blocks, blocks_offset);
        }
    }

    fn encode_i32_to_u64(
        &self,
        values: &[i32],
        mut values_offset: usize,
        blocks: &mut [u64],
        mut blocks_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            blocks[blocks_offset] = self.encode_from_i32(values, values_offset);
            blocks_offset += 1;
            values_offset += self.value_count as usize;
        }
    }

    fn encode_i32_to_u8(
        &self,
        values: &[i32],
        mut values_offset: usize,
        blocks: &mut [u8],
        mut blocks_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            let block = self.encode_from_i32(values, values_offset);
            values_offset += self.value_count as usize;
            blocks_offset = self.write_long(block, blocks, blocks_offset);
        }
    }
}
impl BulkOperation for BulkOperationPackedSingleBlock {}
