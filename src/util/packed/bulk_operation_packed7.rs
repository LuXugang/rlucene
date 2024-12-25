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
use crate::util::packed::{Decoder, Encoder};
use crate::util::packed::bulk_operation::BulkOperation;

#[derive(Default)]
pub(crate) struct BulkOperationPacked7;
impl Decoder for BulkOperationPacked7 {
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

            values[values_offset] = (block0 >> 57) as i64;
            values[values_offset + 1] = ((block0 >> 50) & 127) as i64;
            values[values_offset + 2] = ((block0 >> 43) & 127) as i64;
            values[values_offset + 3] = ((block0 >> 36) & 127) as i64;
            values[values_offset + 4] = ((block0 >> 29) & 127) as i64;
            values[values_offset + 5] = ((block0 >> 22) & 127) as i64;
            values[values_offset + 6] = ((block0 >> 15) & 127) as i64;
            values[values_offset + 7] = ((block0 >> 8) & 127) as i64;
            values[values_offset + 8] = ((block0 >> 1) & 127) as i64;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset + 9] = (((block0 & 1) << 6) | (block1 >> 58)) as i64;
            values[values_offset + 10] = ((block1 >> 51) & 127) as i64;
            values[values_offset + 11] = ((block1 >> 44) & 127) as i64;
            values[values_offset + 12] = ((block1 >> 37) & 127) as i64;
            values[values_offset + 13] = ((block1 >> 30) & 127) as i64;
            values[values_offset + 14] = ((block1 >> 23) & 127) as i64;
            values[values_offset + 15] = ((block1 >> 16) & 127) as i64;
            values[values_offset + 16] = ((block1 >> 9) & 127) as i64;
            values[values_offset + 17] = ((block1 >> 2) & 127) as i64;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset + 18] = (((block1 & 3) << 5) | (block2 >> 59)) as i64;
            values[values_offset + 19] = ((block2 >> 52) & 127) as i64;
            values[values_offset + 20] = ((block2 >> 45) & 127) as i64;
            values[values_offset + 21] = ((block2 >> 38) & 127) as i64;
            values[values_offset + 22] = ((block2 >> 31) & 127) as i64;
            values[values_offset + 23] = ((block2 >> 24) & 127) as i64;
            values[values_offset + 24] = ((block2 >> 17) & 127) as i64;
            values[values_offset + 25] = ((block2 >> 10) & 127) as i64;
            values[values_offset + 26] = ((block2 >> 3) & 127) as i64;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset + 27] = (((block2 & 7) << 4) | (block3 >> 60)) as i64;
            values[values_offset + 28] = ((block3 >> 53) & 127) as i64;
            values[values_offset + 29] = ((block3 >> 46) & 127) as i64;
            values[values_offset + 30] = ((block3 >> 39) & 127) as i64;
            values[values_offset + 31] = ((block3 >> 32) & 127) as i64;
            values[values_offset + 32] = ((block3 >> 25) & 127) as i64;
            values[values_offset + 33] = ((block3 >> 18) & 127) as i64;
            values[values_offset + 34] = ((block3 >> 11) & 127) as i64;
            values[values_offset + 35] = ((block3 >> 4) & 127) as i64;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset + 36] = (((block3 & 15) << 3) | (block4 >> 61)) as i64;
            values[values_offset + 37] = ((block4 >> 54) & 127) as i64;
            values[values_offset + 38] = ((block4 >> 47) & 127) as i64;
            values[values_offset + 39] = ((block4 >> 40) & 127) as i64;
            values[values_offset + 40] = ((block4 >> 33) & 127) as i64;
            values[values_offset + 41] = ((block4 >> 26) & 127) as i64;
            values[values_offset + 42] = ((block4 >> 19) & 127) as i64;
            values[values_offset + 43] = ((block4 >> 12) & 127) as i64;
            values[values_offset + 44] = ((block4 >> 5) & 127) as i64;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset + 45] = (((block4 & 31) << 2) | (block5 >> 62)) as i64;
            values[values_offset + 46] = ((block5 >> 55) & 127) as i64;
            values[values_offset + 47] = ((block5 >> 48) & 127) as i64;
            values[values_offset + 48] = ((block5 >> 41) & 127) as i64;
            values[values_offset + 49] = ((block5 >> 34) & 127) as i64;
            values[values_offset + 50] = ((block5 >> 27) & 127) as i64;
            values[values_offset + 51] = ((block5 >> 20) & 127) as i64;
            values[values_offset + 52] = ((block5 >> 13) & 127) as i64;
            values[values_offset + 53] = ((block5 >> 6) & 127) as i64;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset + 54] = (((block5 & 63) << 1) | (block6 >> 63)) as i64;
            values[values_offset + 55] = ((block6 >> 56) & 127) as i64;
            values[values_offset + 56] = ((block6 >> 49) & 127) as i64;
            values[values_offset + 57] = ((block6 >> 42) & 127) as i64;
            values[values_offset + 58] = ((block6 >> 35) & 127) as i64;
            values[values_offset + 59] = ((block6 >> 28) & 127) as i64;
            values[values_offset + 60] = ((block6 >> 21) & 127) as i64;
            values[values_offset + 61] = ((block6 >> 14) & 127) as i64;
            values[values_offset + 62] = ((block6 >> 7) & 127) as i64;
            values[values_offset + 63] = (block6 & 127) as i64;

            values_offset += 64;
        }
    }
    /// Decodes blocks of type `u8` into `u64` values.
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
            let byte1 = blocks[blocks_offset + 1] as u64;
            let byte2 = blocks[blocks_offset + 2] as u64;
            let byte3 = blocks[blocks_offset + 3] as u64;
            let byte4 = blocks[blocks_offset + 4] as u64;
            let byte5 = blocks[blocks_offset + 5] as u64;
            let byte6 = blocks[blocks_offset + 6] as u64;

            blocks_offset += 7;

            values[values_offset] = (byte0 >> 1) as i64;
            values[values_offset + 1] = (((byte0 & 1) << 6) | (byte1 >> 2)) as i64;
            values[values_offset + 2] = (((byte1 & 3) << 5) | (byte2 >> 3)) as i64;
            values[values_offset + 3] = (((byte2 & 7) << 4) | (byte3 >> 4)) as i64;
            values[values_offset + 4] = (((byte3 & 15) << 3) | (byte4 >> 5)) as i64;
            values[values_offset + 5] = (((byte4 & 31) << 2) | (byte5 >> 6)) as i64;
            values[values_offset + 6] = (((byte5 & 63) << 1) | (byte6 >> 7)) as i64;
            values[values_offset + 7] = (byte6 & 127) as i64;

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
            let block1 = blocks[blocks_offset + 1];
            let block2 = blocks[blocks_offset + 2];
            let block3 = blocks[blocks_offset + 3];
            let block4 = blocks[blocks_offset + 4];
            let block5 = blocks[blocks_offset + 5];
            let block6 = blocks[blocks_offset + 6];
            blocks_offset += 7;

            values[values_offset] = (block0 >> 57) as i32;
            values[values_offset + 1] = ((block0 >> 50) & 127) as i32;
            values[values_offset + 2] = ((block0 >> 43) & 127) as i32;
            values[values_offset + 3] = ((block0 >> 36) & 127) as i32;
            values[values_offset + 4] = ((block0 >> 29) & 127) as i32;
            values[values_offset + 5] = ((block0 >> 22) & 127) as i32;
            values[values_offset + 6] = ((block0 >> 15) & 127) as i32;
            values[values_offset + 7] = ((block0 >> 8) & 127) as i32;
            values[values_offset + 8] = ((block0 >> 1) & 127) as i32;
            values[values_offset + 9] = (((block0 & 1) << 6) | (block1 >> 58)) as i32;
            values[values_offset + 10] = ((block1 >> 51) & 127) as i32;
            values[values_offset + 11] = ((block1 >> 44) & 127) as i32;
            values[values_offset + 12] = ((block1 >> 37) & 127) as i32;
            values[values_offset + 13] = ((block1 >> 30) & 127) as i32;
            values[values_offset + 14] = ((block1 >> 23) & 127) as i32;
            values[values_offset + 15] = ((block1 >> 16) & 127) as i32;
            values[values_offset + 16] = ((block1 >> 9) & 127) as i32;
            values[values_offset + 17] = ((block1 >> 2) & 127) as i32;
            values[values_offset + 18] = (((block1 & 3) << 5) | (block2 >> 59)) as i32;
            values[values_offset + 19] = ((block2 >> 52) & 127) as i32;
            values[values_offset + 20] = ((block2 >> 45) & 127) as i32;
            values[values_offset + 21] = ((block2 >> 38) & 127) as i32;
            values[values_offset + 22] = ((block2 >> 31) & 127) as i32;
            values[values_offset + 23] = ((block2 >> 24) & 127) as i32;
            values[values_offset + 24] = ((block2 >> 17) & 127) as i32;
            values[values_offset + 25] = ((block2 >> 10) & 127) as i32;
            values[values_offset + 26] = ((block2 >> 3) & 127) as i32;
            values[values_offset + 27] = (((block2 & 7) << 4) | (block3 >> 60)) as i32;
            values[values_offset + 28] = ((block3 >> 53) & 127) as i32;
            values[values_offset + 29] = ((block3 >> 46) & 127) as i32;
            values[values_offset + 30] = ((block3 >> 39) & 127) as i32;
            values[values_offset + 31] = ((block3 >> 32) & 127) as i32;
            values[values_offset + 32] = ((block3 >> 25) & 127) as i32;
            values[values_offset + 33] = ((block3 >> 18) & 127) as i32;
            values[values_offset + 34] = ((block3 >> 11) & 127) as i32;
            values[values_offset + 35] = ((block3 >> 4) & 127) as i32;
            values[values_offset + 36] = (((block3 & 15) << 3) | (block4 >> 61)) as i32;
            values[values_offset + 37] = ((block4 >> 54) & 127) as i32;
            values[values_offset + 38] = ((block4 >> 47) & 127) as i32;
            values[values_offset + 39] = ((block4 >> 40) & 127) as i32;
            values[values_offset + 40] = ((block4 >> 33) & 127) as i32;
            values[values_offset + 41] = ((block4 >> 26) & 127) as i32;
            values[values_offset + 42] = ((block4 >> 19) & 127) as i32;
            values[values_offset + 43] = ((block4 >> 12) & 127) as i32;
            values[values_offset + 44] = ((block4 >> 5) & 127) as i32;
            values[values_offset + 45] = (((block4 & 31) << 2) | (block5 >> 62)) as i32;
            values[values_offset + 46] = ((block5 >> 55) & 127) as i32;
            values[values_offset + 47] = ((block5 >> 48) & 127) as i32;
            values[values_offset + 48] = ((block5 >> 41) & 127) as i32;
            values[values_offset + 49] = ((block5 >> 34) & 127) as i32;
            values[values_offset + 50] = ((block5 >> 27) & 127) as i32;
            values[values_offset + 51] = ((block5 >> 20) & 127) as i32;
            values[values_offset + 52] = ((block5 >> 13) & 127) as i32;
            values[values_offset + 53] = ((block5 >> 6) & 127) as i32;
            values[values_offset + 54] = (((block5 & 63) << 1) | (block6 >> 63)) as i32;
            values[values_offset + 55] = ((block6 >> 56) & 127) as i32;
            values[values_offset + 56] = ((block6 >> 49) & 127) as i32;
            values[values_offset + 57] = ((block6 >> 42) & 127) as i32;
            values[values_offset + 58] = ((block6 >> 35) & 127) as i32;
            values[values_offset + 59] = ((block6 >> 28) & 127) as i32;
            values[values_offset + 60] = ((block6 >> 21) & 127) as i32;
            values[values_offset + 61] = ((block6 >> 14) & 127) as i32;
            values[values_offset + 62] = ((block6 >> 7) & 127) as i32;
            values[values_offset + 63] = (block6 & 127) as i32;

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
            let byte1 = blocks[blocks_offset + 1] as i32;
            let byte2 = blocks[blocks_offset + 2] as i32;
            let byte3 = blocks[blocks_offset + 3] as i32;
            let byte4 = blocks[blocks_offset + 4] as i32;
            let byte5 = blocks[blocks_offset + 5] as i32;
            let byte6 = blocks[blocks_offset + 6] as i32;
            blocks_offset += 7;

            values[values_offset] = byte0 >> 1;
            values[values_offset + 1] = ((byte0 & 1) << 6) | (byte1 >> 2);
            values[values_offset + 2] = ((byte1 & 3) << 5) | (byte2 >> 3);
            values[values_offset + 3] = ((byte2 & 7) << 4) | (byte3 >> 4);
            values[values_offset + 4] = ((byte3 & 15) << 3) | (byte4 >> 5);
            values[values_offset + 5] = ((byte4 & 31) << 2) | (byte5 >> 6);
            values[values_offset + 6] = ((byte5 & 63) << 1) | (byte6 >> 7);
            values[values_offset + 7] = byte6 & 127;

            values_offset += 8;
        }
    }
}
impl Encoder for BulkOperationPacked7 {}
impl BulkOperation for BulkOperationPacked7{}
