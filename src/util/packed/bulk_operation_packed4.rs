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

#[derive(Default)]
pub(crate) struct BulkOperationPacked4;
impl Decoder for BulkOperationPacked4 {
    /// Decodes blocks of type `u64` into `u64` values.
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

            values[values_offset] = ((block >> 60) & 15) as i64;
            values[values_offset + 1] = ((block >> 56) & 15) as i64;
            values[values_offset + 2] = ((block >> 52) & 15) as i64;
            values[values_offset + 3] = ((block >> 48) & 15) as i64;
            values[values_offset + 4] = ((block >> 44) & 15) as i64;
            values[values_offset + 5] = ((block >> 40) & 15) as i64;
            values[values_offset + 6] = ((block >> 36) & 15) as i64;
            values[values_offset + 7] = ((block >> 32) & 15) as i64;
            values[values_offset + 8] = ((block >> 28) & 15) as i64;
            values[values_offset + 9] = ((block >> 24) & 15) as i64;
            values[values_offset + 10] = ((block >> 20) & 15) as i64;
            values[values_offset + 11] = ((block >> 16) & 15) as i64;
            values[values_offset + 12] = ((block >> 12) & 15) as i64;
            values[values_offset + 13] = ((block >> 8) & 15) as i64;
            values[values_offset + 14] = ((block >> 4) & 15) as i64;
            values[values_offset + 15] = (block & 15) as i64;

            values_offset += 16;
        }
    }

    /// Decodes blocks of type `u8` into `u64` values.
    fn decode_u8_to_i64(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block >> 4) & 15) as i64;
            values[values_offset + 1] = (block & 15) as i64;

            values_offset += 2;
        }
    }

    /// Decodes blocks of type `u64` into `i32` values.
    fn decode_u64_to_i32(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block >> 60) & 15) as i32;
            values[values_offset + 1] = ((block >> 56) & 15) as i32;
            values[values_offset + 2] = ((block >> 52) & 15) as i32;
            values[values_offset + 3] = ((block >> 48) & 15) as i32;
            values[values_offset + 4] = ((block >> 44) & 15) as i32;
            values[values_offset + 5] = ((block >> 40) & 15) as i32;
            values[values_offset + 6] = ((block >> 36) & 15) as i32;
            values[values_offset + 7] = ((block >> 32) & 15) as i32;
            values[values_offset + 8] = ((block >> 28) & 15) as i32;
            values[values_offset + 9] = ((block >> 24) & 15) as i32;
            values[values_offset + 10] = ((block >> 20) & 15) as i32;
            values[values_offset + 11] = ((block >> 16) & 15) as i32;
            values[values_offset + 12] = ((block >> 12) & 15) as i32;
            values[values_offset + 13] = ((block >> 8) & 15) as i32;
            values[values_offset + 14] = ((block >> 4) & 15) as i32;
            values[values_offset + 15] = (block & 15) as i32;

            values_offset += 16;
        }
    }

    //// Decodes blocks of type `u8` into `i32` values.
    fn decode_u8_to_i32(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block >> 4) & 15) as i32;
            values[values_offset + 1] = (block & 15) as i32;

            values_offset += 2;
        }
    }
}
impl Encoder for BulkOperationPacked4 {}
impl BulkOperation for BulkOperationPacked4 {}
