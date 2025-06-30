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
pub(crate) struct BulkOperationPacked5;
impl Decoder for BulkOperationPacked5 {
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
            let block0 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (block0 >> 59) as i64;
            values[values_offset + 1] = ((block0 >> 54) & 31) as i64;
            values[values_offset + 2] = ((block0 >> 49) & 31) as i64;
            values[values_offset + 3] = ((block0 >> 44) & 31) as i64;
            values[values_offset + 4] = ((block0 >> 39) & 31) as i64;
            values[values_offset + 5] = ((block0 >> 34) & 31) as i64;
            values[values_offset + 6] = ((block0 >> 29) & 31) as i64;
            values[values_offset + 7] = ((block0 >> 24) & 31) as i64;
            values[values_offset + 8] = ((block0 >> 19) & 31) as i64;
            values[values_offset + 9] = ((block0 >> 14) & 31) as i64;
            values[values_offset + 10] = ((block0 >> 9) & 31) as i64;
            values[values_offset + 11] = ((block0 >> 4) & 31) as i64;
            values_offset += 12;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (((block0 & 15) << 1) | (block1 >> 63)) as i64;
            values[values_offset + 1] = ((block1 >> 58) & 31) as i64;
            values[values_offset + 2] = ((block1 >> 53) & 31) as i64;
            values[values_offset + 3] = ((block1 >> 48) & 31) as i64;
            values[values_offset + 4] = ((block1 >> 43) & 31) as i64;
            values[values_offset + 5] = ((block1 >> 38) & 31) as i64;
            values[values_offset + 6] = ((block1 >> 33) & 31) as i64;
            values[values_offset + 7] = ((block1 >> 28) & 31) as i64;
            values[values_offset + 8] = ((block1 >> 23) & 31) as i64;
            values[values_offset + 9] = ((block1 >> 18) & 31) as i64;
            values[values_offset + 10] = ((block1 >> 13) & 31) as i64;
            values[values_offset + 11] = ((block1 >> 8) & 31) as i64;
            values[values_offset + 12] = ((block1 >> 3) & 31) as i64;
            values_offset += 13;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (((block1 & 7) << 2) | (block2 >> 62)) as i64;
            values[values_offset + 1] = ((block2 >> 57) & 31) as i64;
            values[values_offset + 2] = ((block2 >> 52) & 31) as i64;
            values[values_offset + 3] = ((block2 >> 47) & 31) as i64;
            values[values_offset + 4] = ((block2 >> 42) & 31) as i64;
            values[values_offset + 5] = ((block2 >> 37) & 31) as i64;
            values[values_offset + 6] = ((block2 >> 32) & 31) as i64;
            values[values_offset + 7] = ((block2 >> 27) & 31) as i64;
            values[values_offset + 8] = ((block2 >> 22) & 31) as i64;
            values[values_offset + 9] = ((block2 >> 17) & 31) as i64;
            values[values_offset + 10] = ((block2 >> 12) & 31) as i64;
            values[values_offset + 11] = ((block2 >> 7) & 31) as i64;
            values[values_offset + 12] = ((block2 >> 2) & 31) as i64;
            values_offset += 13;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (((block2 & 3) << 3) | (block3 >> 61)) as i64;
            values[values_offset + 1] = ((block3 >> 56) & 31) as i64;
            values[values_offset + 2] = ((block3 >> 51) & 31) as i64;
            values[values_offset + 3] = ((block3 >> 46) & 31) as i64;
            values[values_offset + 4] = ((block3 >> 41) & 31) as i64;
            values[values_offset + 5] = ((block3 >> 36) & 31) as i64;
            values[values_offset + 6] = ((block3 >> 31) & 31) as i64;
            values[values_offset + 7] = ((block3 >> 26) & 31) as i64;
            values[values_offset + 8] = ((block3 >> 21) & 31) as i64;
            values[values_offset + 9] = ((block3 >> 16) & 31) as i64;
            values[values_offset + 10] = ((block3 >> 11) & 31) as i64;
            values[values_offset + 11] = ((block3 >> 6) & 31) as i64;
            values[values_offset + 12] = ((block3 >> 1) & 31) as i64;
            values_offset += 13;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (((block3 & 1) << 4) | (block4 >> 60)) as i64;
            values[values_offset + 1] = ((block4 >> 55) & 31) as i64;
            values[values_offset + 2] = ((block4 >> 50) & 31) as i64;
            values[values_offset + 3] = ((block4 >> 45) & 31) as i64;
            values[values_offset + 4] = ((block4 >> 40) & 31) as i64;
            values[values_offset + 5] = ((block4 >> 35) & 31) as i64;
            values[values_offset + 6] = ((block4 >> 30) & 31) as i64;
            values[values_offset + 7] = ((block4 >> 25) & 31) as i64;
            values[values_offset + 8] = ((block4 >> 20) & 31) as i64;
            values[values_offset + 9] = ((block4 >> 15) & 31) as i64;
            values[values_offset + 10] = ((block4 >> 10) & 31) as i64;
            values[values_offset + 11] = ((block4 >> 5) & 31) as i64;
            values[values_offset + 12] = (block4 & 31) as i64;
            values_offset += 13;
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
            let byte0 = blocks[blocks_offset] as u64;
            let byte1 = blocks[blocks_offset + 1] as u64;
            let byte2 = blocks[blocks_offset + 2] as u64;
            let byte3 = blocks[blocks_offset + 3] as u64;
            let byte4 = blocks[blocks_offset + 4] as u64;
            blocks_offset += 5;

            values[values_offset] = (byte0 >> 3) as i64;
            values[values_offset + 1] = (((byte0 & 7) << 2) | (byte1 >> 6)) as i64;
            values[values_offset + 2] = ((byte1 >> 1) & 31) as i64;
            values[values_offset + 3] = (((byte1 & 1) << 4) | (byte2 >> 4)) as i64;
            values[values_offset + 4] = (((byte2 & 15) << 1) | (byte3 >> 7)) as i64;
            values[values_offset + 5] = ((byte3 >> 2) & 31) as i64;
            values[values_offset + 6] = (((byte3 & 3) << 3) | (byte4 >> 5)) as i64;
            values[values_offset + 7] = (byte4 & 31) as i64;
            values_offset += 8;
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
            let block0 = blocks[blocks_offset];
            let block1 = blocks[blocks_offset + 1];
            let block2 = blocks[blocks_offset + 2];
            let block3 = blocks[blocks_offset + 3];
            let block4 = blocks[blocks_offset + 4];
            blocks_offset += 5;

            values[values_offset] = (block0 >> 59) as i32;
            values[values_offset + 1] = ((block0 >> 54) & 31) as i32;
            values[values_offset + 2] = ((block0 >> 49) & 31) as i32;
            values[values_offset + 3] = ((block0 >> 44) & 31) as i32;
            values[values_offset + 4] = ((block0 >> 39) & 31) as i32;
            values[values_offset + 5] = ((block0 >> 34) & 31) as i32;
            values[values_offset + 6] = ((block0 >> 29) & 31) as i32;
            values[values_offset + 7] = ((block0 >> 24) & 31) as i32;
            values[values_offset + 8] = ((block0 >> 19) & 31) as i32;
            values[values_offset + 9] = ((block0 >> 14) & 31) as i32;
            values[values_offset + 10] = ((block0 >> 9) & 31) as i32;
            values[values_offset + 11] = ((block0 >> 4) & 31) as i32;
            values[values_offset + 12] = (((block0 & 15) << 1) | (block1 >> 63)) as i32;

            values[values_offset + 13] = ((block1 >> 58) & 31) as i32;
            values[values_offset + 14] = ((block1 >> 53) & 31) as i32;
            values[values_offset + 15] = ((block1 >> 48) & 31) as i32;
            values[values_offset + 16] = ((block1 >> 43) & 31) as i32;
            values[values_offset + 17] = ((block1 >> 38) & 31) as i32;
            values[values_offset + 18] = ((block1 >> 33) & 31) as i32;
            values[values_offset + 19] = ((block1 >> 28) & 31) as i32;
            values[values_offset + 20] = ((block1 >> 23) & 31) as i32;
            values[values_offset + 21] = ((block1 >> 18) & 31) as i32;
            values[values_offset + 22] = ((block1 >> 13) & 31) as i32;
            values[values_offset + 23] = ((block1 >> 8) & 31) as i32;
            values[values_offset + 24] = ((block1 >> 3) & 31) as i32;
            values[values_offset + 25] = (((block1 & 7) << 2) | (block2 >> 62)) as i32;

            values[values_offset + 26] = ((block2 >> 57) & 31) as i32;
            values[values_offset + 27] = ((block2 >> 52) & 31) as i32;
            values[values_offset + 28] = ((block2 >> 47) & 31) as i32;
            values[values_offset + 29] = ((block2 >> 42) & 31) as i32;
            values[values_offset + 30] = ((block2 >> 37) & 31) as i32;
            values[values_offset + 31] = ((block2 >> 32) & 31) as i32;
            values[values_offset + 32] = ((block2 >> 27) & 31) as i32;
            values[values_offset + 33] = ((block2 >> 22) & 31) as i32;
            values[values_offset + 34] = ((block2 >> 17) & 31) as i32;
            values[values_offset + 35] = ((block2 >> 12) & 31) as i32;
            values[values_offset + 36] = ((block2 >> 7) & 31) as i32;
            values[values_offset + 37] = ((block2 >> 2) & 31) as i32;
            values[values_offset + 38] = (((block2 & 3) << 3) | (block3 >> 61)) as i32;

            values[values_offset + 39] = ((block3 >> 56) & 31) as i32;
            values[values_offset + 40] = ((block3 >> 51) & 31) as i32;
            values[values_offset + 41] = ((block3 >> 46) & 31) as i32;
            values[values_offset + 42] = ((block3 >> 41) & 31) as i32;
            values[values_offset + 43] = ((block3 >> 36) & 31) as i32;
            values[values_offset + 44] = ((block3 >> 31) & 31) as i32;
            values[values_offset + 45] = ((block3 >> 26) & 31) as i32;
            values[values_offset + 46] = ((block3 >> 21) & 31) as i32;
            values[values_offset + 47] = ((block3 >> 16) & 31) as i32;
            values[values_offset + 48] = ((block3 >> 11) & 31) as i32;
            values[values_offset + 49] = ((block3 >> 6) & 31) as i32;
            values[values_offset + 50] = ((block3 >> 1) & 31) as i32;
            values[values_offset + 51] = (((block3 & 1) << 4) | (block4 >> 60)) as i32;

            for shift in (0..=55).rev().step_by(5) {
                values[values_offset + 52 + (55 - shift) / 5] = ((block4 >> shift) & 31) as i32;
            }
            values_offset += 64;
        }
    }
    /// Decodes blocks of type `u8` into `i32` values.
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
            values[values_offset] = byte0 >> 3;
            values_offset += 1;

            let byte1 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte0 & 7) << 2) | (byte1 >> 6);
            values_offset += 1;
            values[values_offset] = (byte1 >> 1) & 31;
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte1 & 1) << 4) | (byte2 >> 4);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte2 & 15) << 1) | (byte3 >> 7);
            values_offset += 1;
            values[values_offset] = (byte3 >> 2) & 31;
            values_offset += 1;

            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte3 & 3) << 3) | (byte4 >> 5);
            values_offset += 1;
            values[values_offset] = byte4 & 31;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked5 {}
impl BulkOperation for BulkOperationPacked5 {}
