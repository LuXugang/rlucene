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
pub(crate) struct BulkOperationPacked20;
impl Decoder for BulkOperationPacked20 {
    fn decode_u64_to_i64(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: i32,
    ) {
        for _ in 0..iterations {
            let block0 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (block0 >> 44) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 24) & 1_048_575) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 4) & 1_048_575) as i64;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 15) << 16) | (block1 >> 48)) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 28) & 1_048_575) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 8) & 1_048_575) as i64;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 255) << 12) | (block2 >> 52)) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 32) & 1_048_575) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 12) & 1_048_575) as i64;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 4095) << 8) | (block3 >> 56)) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 36) & 1_048_575) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 16) & 1_048_575) as i64;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 65_535) << 4) | (block4 >> 60)) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 40) & 1_048_575) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 20) & 1_048_575) as i64;
            values_offset += 1;
            values[values_offset] = (block4 & 1_048_575) as i64;
            values_offset += 1;
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
            let byte0 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte1 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte2 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = (byte0 << 12) | (byte1 << 4) | (byte2 >> 4);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte2 & 15) << 16) | (byte3 << 8) | byte4;
            values_offset += 1;
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
        for _ in 0..iterations {
            let block0 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (block0 >> 44) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 24) & 1_048_575) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 4) & 1_048_575) as i32;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 15) << 16) | (block1 >> 48)) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 28) & 1_048_575) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 8) & 1_048_575) as i32;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 255) << 12) | (block2 >> 52)) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 32) & 1_048_575) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 12) & 1_048_575) as i32;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 4095) << 8) | (block3 >> 56)) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 36) & 1_048_575) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 16) & 1_048_575) as i32;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 65_535) << 4) | (block4 >> 60)) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 40) & 1_048_575) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 20) & 1_048_575) as i32;
            values_offset += 1;
            values[values_offset] = (block4 & 1_048_575) as i32;
            values_offset += 1;
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
        for _ in 0..iterations {
            let byte0 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte1 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte2 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = (byte0 << 12) | (byte1 << 4) | (byte2 >> 4);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte2 & 15) << 16) | (byte3 << 8) | byte4;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked20 {}
impl BulkOperation for BulkOperationPacked20 {}
