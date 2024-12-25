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
use crate::util::packed::bulk_operation::BulkOperation;
use crate::util::packed::{Decoder, Encoder};

#[derive(Default)]
pub(crate) struct BulkOperationPacked3;
impl Decoder for BulkOperationPacked3 {
    /// Decodes blocks of type `u64` into `u64` values.
    fn decode_long_to_long(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: u32,
    ) {
        for _ in 0..iterations {
            let block0 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block0 >> 61) & 7) as i64;
            values[values_offset + 1] = ((block0 >> 58) & 7) as i64;
            values[values_offset + 2] = ((block0 >> 55) & 7) as i64;
            values[values_offset + 3] = ((block0 >> 52) & 7) as i64;
            values[values_offset + 4] = ((block0 >> 49) & 7) as i64;
            values[values_offset + 5] = ((block0 >> 46) & 7) as i64;
            values[values_offset + 6] = ((block0 >> 43) & 7) as i64;
            values[values_offset + 7] = ((block0 >> 40) & 7) as i64;
            values[values_offset + 8] = ((block0 >> 37) & 7) as i64;
            values[values_offset + 9] = ((block0 >> 34) & 7) as i64;
            values[values_offset + 10] = ((block0 >> 31) & 7) as i64;
            values[values_offset + 11] = ((block0 >> 28) & 7) as i64;
            values[values_offset + 12] = ((block0 >> 25) & 7) as i64;
            values[values_offset + 13] = ((block0 >> 22) & 7) as i64;
            values[values_offset + 14] = ((block0 >> 19) & 7) as i64;
            values[values_offset + 15] = ((block0 >> 16) & 7) as i64;
            values[values_offset + 16] = ((block0 >> 13) & 7) as i64;
            values[values_offset + 17] = ((block0 >> 10) & 7) as i64;
            values[values_offset + 18] = ((block0 >> 7) & 7) as i64;
            values[values_offset + 19] = ((block0 >> 4) & 7) as i64;
            values[values_offset + 20] = ((block0 >> 1) & 7) as i64;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset + 21] = (((block0 & 1) << 2) | (block1 >> 62)) as i64;
            values[values_offset + 22] = ((block1 >> 59) & 7) as i64;
            values[values_offset + 23] = ((block1 >> 56) & 7) as i64;
            values[values_offset + 24] = ((block1 >> 53) & 7) as i64;
            values[values_offset + 25] = ((block1 >> 50) & 7) as i64;
            values[values_offset + 26] = ((block1 >> 47) & 7) as i64;
            values[values_offset + 27] = ((block1 >> 44) & 7) as i64;
            values[values_offset + 28] = ((block1 >> 41) & 7) as i64;
            values[values_offset + 29] = ((block1 >> 38) & 7) as i64;
            values[values_offset + 30] = ((block1 >> 35) & 7) as i64;
            values[values_offset + 31] = ((block1 >> 32) & 7) as i64;
            values[values_offset + 32] = ((block1 >> 29) & 7) as i64;
            values[values_offset + 33] = ((block1 >> 26) & 7) as i64;
            values[values_offset + 34] = ((block1 >> 23) & 7) as i64;
            values[values_offset + 35] = ((block1 >> 20) & 7) as i64;
            values[values_offset + 36] = ((block1 >> 17) & 7) as i64;
            values[values_offset + 37] = ((block1 >> 14) & 7) as i64;
            values[values_offset + 38] = ((block1 >> 11) & 7) as i64;
            values[values_offset + 39] = ((block1 >> 8) & 7) as i64;
            values[values_offset + 40] = ((block1 >> 5) & 7) as i64;
            values[values_offset + 41] = ((block1 >> 2) & 7) as i64;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset + 42] = (((block1 & 3) << 1) | (block2 >> 63)) as i64;
            values[values_offset + 43] = ((block2 >> 60) & 7) as i64;
            values[values_offset + 44] = ((block2 >> 57) & 7) as i64;
            values[values_offset + 45] = ((block2 >> 54) & 7) as i64;
            values[values_offset + 46] = ((block2 >> 51) & 7) as i64;
            values[values_offset + 47] = ((block2 >> 48) & 7) as i64;
            values[values_offset + 48] = ((block2 >> 45) & 7) as i64;
            values[values_offset + 49] = ((block2 >> 42) & 7) as i64;
            values[values_offset + 50] = ((block2 >> 39) & 7) as i64;
            values[values_offset + 51] = ((block2 >> 36) & 7) as i64;
            values[values_offset + 52] = ((block2 >> 33) & 7) as i64;
            values[values_offset + 53] = ((block2 >> 30) & 7) as i64;
            values[values_offset + 54] = ((block2 >> 27) & 7) as i64;
            values[values_offset + 55] = ((block2 >> 24) & 7) as i64;
            values[values_offset + 56] = ((block2 >> 21) & 7) as i64;
            values[values_offset + 57] = ((block2 >> 18) & 7) as i64;
            values[values_offset + 58] = ((block2 >> 15) & 7) as i64;
            values[values_offset + 59] = ((block2 >> 12) & 7) as i64;
            values[values_offset + 60] = ((block2 >> 9) & 7) as i64;
            values[values_offset + 61] = ((block2 >> 6) & 7) as i64;
            values[values_offset + 62] = ((block2 >> 3) & 7) as i64;
            values[values_offset + 63] = (block2 & 7) as i64;

            values_offset += 64;
        }
    }
    fn decode_byte_to_long(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: u32,
    ) {
        for _ in 0..iterations {
            let byte0 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (byte0 >> 5) as i64;
            values[values_offset + 1] = ((byte0 >> 2) & 7) as i64;
            let byte1 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset + 2] = (((byte0 & 3) << 1) | (byte1 >> 7)) as i64;
            values[values_offset + 3] = ((byte1 >> 4) & 7) as i64;
            values[values_offset + 4] = ((byte1 >> 1) & 7) as i64;
            let byte2 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset + 5] = (((byte1 & 1) << 2) | (byte2 >> 6)) as i64;
            values[values_offset + 6] = ((byte2 >> 3) & 7) as i64;
            values[values_offset + 7] = (byte2 & 7) as i64;
            values_offset += 8;
        }
    }
    fn decode_long_to_int(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: u32,
    ) {
        for _ in 0..iterations {
            let block0 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (block0 >> 61) as i32;
            values[values_offset + 1] = ((block0 >> 58) & 7) as i32;
            values[values_offset + 2] = ((block0 >> 55) & 7) as i32;
            values[values_offset + 3] = ((block0 >> 52) & 7) as i32;
            values[values_offset + 4] = ((block0 >> 49) & 7) as i32;
            values[values_offset + 5] = ((block0 >> 46) & 7) as i32;
            values[values_offset + 6] = ((block0 >> 43) & 7) as i32;
            values[values_offset + 7] = ((block0 >> 40) & 7) as i32;
            values[values_offset + 8] = ((block0 >> 37) & 7) as i32;
            values[values_offset + 9] = ((block0 >> 34) & 7) as i32;
            values[values_offset + 10] = ((block0 >> 31) & 7) as i32;
            values[values_offset + 11] = ((block0 >> 28) & 7) as i32;
            values[values_offset + 12] = ((block0 >> 25) & 7) as i32;
            values[values_offset + 13] = ((block0 >> 22) & 7) as i32;
            values[values_offset + 14] = ((block0 >> 19) & 7) as i32;
            values[values_offset + 15] = ((block0 >> 16) & 7) as i32;
            values[values_offset + 16] = ((block0 >> 13) & 7) as i32;
            values[values_offset + 17] = ((block0 >> 10) & 7) as i32;
            values[values_offset + 18] = ((block0 >> 7) & 7) as i32;
            values[values_offset + 19] = ((block0 >> 4) & 7) as i32;
            values[values_offset + 20] = ((block0 >> 1) & 7) as i32;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset + 21] = (((block0 & 1) << 2) | (block1 >> 62)) as i32;
            values[values_offset + 22] = ((block1 >> 59) & 7) as i32;
            values[values_offset + 23] = ((block1 >> 56) & 7) as i32;
            values[values_offset + 24] = ((block1 >> 53) & 7) as i32;
            values[values_offset + 25] = ((block1 >> 50) & 7) as i32;
            values[values_offset + 26] = ((block1 >> 47) & 7) as i32;
            values[values_offset + 27] = ((block1 >> 44) & 7) as i32;
            values[values_offset + 28] = ((block1 >> 41) & 7) as i32;
            values[values_offset + 29] = ((block1 >> 38) & 7) as i32;
            values[values_offset + 30] = ((block1 >> 35) & 7) as i32;
            values[values_offset + 31] = ((block1 >> 32) & 7) as i32;
            values[values_offset + 32] = ((block1 >> 29) & 7) as i32;
            values[values_offset + 33] = ((block1 >> 26) & 7) as i32;
            values[values_offset + 34] = ((block1 >> 23) & 7) as i32;
            values[values_offset + 35] = ((block1 >> 20) & 7) as i32;
            values[values_offset + 36] = ((block1 >> 17) & 7) as i32;
            values[values_offset + 37] = ((block1 >> 14) & 7) as i32;
            values[values_offset + 38] = ((block1 >> 11) & 7) as i32;
            values[values_offset + 39] = ((block1 >> 8) & 7) as i32;
            values[values_offset + 40] = ((block1 >> 5) & 7) as i32;
            values[values_offset + 41] = ((block1 >> 2) & 7) as i32;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset + 42] = (((block1 & 3) << 1) | (block2 >> 63)) as i32;
            values[values_offset + 43] = ((block2 >> 60) & 7) as i32;
            values[values_offset + 44] = ((block2 >> 57) & 7) as i32;
            values[values_offset + 45] = ((block2 >> 54) & 7) as i32;
            values[values_offset + 46] = ((block2 >> 51) & 7) as i32;
            values[values_offset + 47] = ((block2 >> 48) & 7) as i32;
            values[values_offset + 48] = ((block2 >> 45) & 7) as i32;
            values[values_offset + 49] = ((block2 >> 42) & 7) as i32;
            values[values_offset + 50] = ((block2 >> 39) & 7) as i32;
            values[values_offset + 51] = ((block2 >> 36) & 7) as i32;
            values[values_offset + 52] = ((block2 >> 33) & 7) as i32;
            values[values_offset + 53] = ((block2 >> 30) & 7) as i32;
            values[values_offset + 54] = ((block2 >> 27) & 7) as i32;
            values[values_offset + 55] = ((block2 >> 24) & 7) as i32;
            values[values_offset + 56] = ((block2 >> 21) & 7) as i32;
            values[values_offset + 57] = ((block2 >> 18) & 7) as i32;
            values[values_offset + 58] = ((block2 >> 15) & 7) as i32;
            values[values_offset + 59] = ((block2 >> 12) & 7) as i32;
            values[values_offset + 60] = ((block2 >> 9) & 7) as i32;
            values[values_offset + 61] = ((block2 >> 6) & 7) as i32;
            values[values_offset + 62] = ((block2 >> 3) & 7) as i32;
            values[values_offset + 63] = (block2 & 7) as i32;

            values_offset += 64;
        }
    }
    fn decode_byte_to_int(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: u32,
    ) {
        for _ in 0..iterations {
            let byte0 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = byte0 >> 5;
            values[values_offset + 1] = (byte0 >> 2) & 7;
            let byte1 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset + 2] = ((byte0 & 3) << 1) | (byte1 >> 7);
            values[values_offset + 3] = (byte1 >> 4) & 7;
            values[values_offset + 4] = (byte1 >> 1) & 7;
            let byte2 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset + 5] = ((byte1 & 1) << 2) | (byte2 >> 6);
            values[values_offset + 6] = (byte2 >> 3) & 7;
            values[values_offset + 7] = byte2 & 7;
            values_offset += 8;
        }
    }
}
impl Encoder for BulkOperationPacked3 {}
impl BulkOperation for BulkOperationPacked3 {}
