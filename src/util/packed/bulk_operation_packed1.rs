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
pub(crate) struct BulkOperationPacked1;
impl Decoder for BulkOperationPacked1 {
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

            for shift in (0..=63).rev() {
                values[values_offset] = ((block >> shift) & 1) as i64;
                values_offset += 1;
            }
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

            values[values_offset] = ((block >> 7) & 1) as i64;
            values[values_offset + 1] = ((block >> 6) & 1) as i64;
            values[values_offset + 2] = ((block >> 5) & 1) as i64;
            values[values_offset + 3] = ((block >> 4) & 1) as i64;
            values[values_offset + 4] = ((block >> 3) & 1) as i64;
            values[values_offset + 5] = ((block >> 2) & 1) as i64;
            values[values_offset + 6] = ((block >> 1) & 1) as i64;
            values[values_offset + 7] = (block & 1) as i64;

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
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            for shift in (0..=63).rev() {
                values[values_offset] = ((block >> shift) & 1) as i32;
                values_offset += 1;
            }
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
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block >> 7) & 1) as i32;
            values[values_offset + 1] = ((block >> 6) & 1) as i32;
            values[values_offset + 2] = ((block >> 5) & 1) as i32;
            values[values_offset + 3] = ((block >> 4) & 1) as i32;
            values[values_offset + 4] = ((block >> 3) & 1) as i32;
            values[values_offset + 5] = ((block >> 2) & 1) as i32;
            values[values_offset + 6] = ((block >> 1) & 1) as i32;
            values[values_offset + 7] = (block & 1) as i32;

            values_offset += 8;
        }
    }
}
impl Encoder for BulkOperationPacked1 {}
impl BulkOperation for BulkOperationPacked1 {}
