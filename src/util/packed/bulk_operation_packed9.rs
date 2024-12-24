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
use crate::util::packed::bulk_operation_packed1::BulkOperationPacked1;
use crate::util::packed::{Decoder, Encoder};

#[derive(Default)]
pub(crate) struct BulkOperationPacked9;
impl Decoder for BulkOperationPacked9 {
    /// Decodes blocks of type `u64` into `u64` values.
    fn decode_long_to_long(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: usize,
    ) {
        for _ in 0..iterations {
            let block0 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (block0 >> 55) as i64;
            values[values_offset + 1] = ((block0 >> 46) & 511) as i64;
            values[values_offset + 2] = ((block0 >> 37) & 511) as i64;
            values[values_offset + 3] = ((block0 >> 28) & 511) as i64;
            values[values_offset + 4] = ((block0 >> 19) & 511) as i64;
            values[values_offset + 5] = ((block0 >> 10) & 511) as i64;
            values[values_offset + 6] = ((block0 >> 1) & 511) as i64;
            values_offset += 7;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (((block0 & 1) << 8) | (block1 >> 56)) as i64;
            values[values_offset + 1] = ((block1 >> 47) & 511) as i64;
            values[values_offset + 2] = ((block1 >> 38) & 511) as i64;
            values[values_offset + 3] = ((block1 >> 29) & 511) as i64;
            values[values_offset + 4] = ((block1 >> 20) & 511) as i64;
            values[values_offset + 5] = ((block1 >> 11) & 511) as i64;
            values[values_offset + 6] = ((block1 >> 2) & 511) as i64;
            values_offset += 7;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (((block1 & 3) << 7) | (block2 >> 57)) as i64;
            values[values_offset + 1] = ((block2 >> 48) & 511) as i64;
            values[values_offset + 2] = ((block2 >> 39) & 511) as i64;
            values[values_offset + 3] = ((block2 >> 30) & 511) as i64;
            values[values_offset + 4] = ((block2 >> 21) & 511) as i64;
            values[values_offset + 5] = ((block2 >> 12) & 511) as i64;
            values[values_offset + 6] = ((block2 >> 3) & 511) as i64;
            values_offset += 7;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (((block2 & 7) << 6) | (block3 >> 58)) as i64;
            values[values_offset + 1] = ((block3 >> 49) & 511) as i64;
            values[values_offset + 2] = ((block3 >> 40) & 511) as i64;
            values[values_offset + 3] = ((block3 >> 31) & 511) as i64;
            values[values_offset + 4] = ((block3 >> 22) & 511) as i64;
            values[values_offset + 5] = ((block3 >> 13) & 511) as i64;
            values[values_offset + 6] = ((block3 >> 4) & 511) as i64;
            values_offset += 7;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (((block3 & 15) << 5) | (block4 >> 59)) as i64;
            values[values_offset + 1] = ((block4 >> 50) & 511) as i64;
            values[values_offset + 2] = ((block4 >> 41) & 511) as i64;
            values[values_offset + 3] = ((block4 >> 32) & 511) as i64;
            values[values_offset + 4] = ((block4 >> 23) & 511) as i64;
            values[values_offset + 5] = ((block4 >> 14) & 511) as i64;
            values[values_offset + 6] = ((block4 >> 5) & 511) as i64;
            values_offset += 7;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (((block4 & 31) << 4) | (block5 >> 60)) as i64;
            values[values_offset + 1] = ((block5 >> 51) & 511) as i64;
            values[values_offset + 2] = ((block5 >> 42) & 511) as i64;
            values[values_offset + 3] = ((block5 >> 33) & 511) as i64;
            values[values_offset + 4] = ((block5 >> 24) & 511) as i64;
            values[values_offset + 5] = ((block5 >> 15) & 511) as i64;
            values[values_offset + 6] = ((block5 >> 6) & 511) as i64;
            values_offset += 7;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (((block5 & 63) << 3) | (block6 >> 61)) as i64;
            values[values_offset + 1] = ((block6 >> 52) & 511) as i64;
            values[values_offset + 2] = ((block6 >> 43) & 511) as i64;
            values[values_offset + 3] = ((block6 >> 34) & 511) as i64;
            values[values_offset + 4] = ((block6 >> 25) & 511) as i64;
            values[values_offset + 5] = ((block6 >> 16) & 511) as i64;
            values[values_offset + 6] = ((block6 >> 7) & 511) as i64;
            values_offset += 7;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (((block6 & 127) << 2) | (block7 >> 62)) as i64;
            values[values_offset + 1] = ((block7 >> 53) & 511) as i64;
            values[values_offset + 2] = ((block7 >> 44) & 511) as i64;
            values[values_offset + 3] = ((block7 >> 35) & 511) as i64;
            values[values_offset + 4] = ((block7 >> 26) & 511) as i64;
            values[values_offset + 5] = ((block7 >> 17) & 511) as i64;
            values[values_offset + 6] = ((block7 >> 8) & 511) as i64;
            values_offset += 7;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = (((block7 & 255) << 1) | (block8 >> 63)) as i64;
            values[values_offset + 1] = ((block8 >> 54) & 511) as i64;
            values[values_offset + 2] = ((block8 >> 45) & 511) as i64;
            values[values_offset + 3] = ((block8 >> 36) & 511) as i64;
            values[values_offset + 4] = ((block8 >> 27) & 511) as i64;
            values[values_offset + 5] = ((block8 >> 18) & 511) as i64;
            values[values_offset + 6] = ((block8 >> 9) & 511) as i64;
            values[values_offset + 7] = (block8 & 511) as i64;
            values_offset += 8;
        }
    }
    fn decode_byte_to_long(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: usize,
    ) {
        for _ in 0..iterations {
            let byte0 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            let byte1 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = ((byte0 << 1) | (byte1 >> 7)) as i64;
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte1 & 127) << 2) | (byte2 >> 6)) as i64;
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte2 & 63) << 3) | (byte3 >> 5)) as i64;
            values_offset += 1;

            let byte4 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte3 & 31) << 4) | (byte4 >> 4)) as i64;
            values_offset += 1;

            let byte5 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte4 & 15) << 5) | (byte5 >> 3)) as i64;
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte5 & 7) << 6) | (byte6 >> 2)) as i64;
            values_offset += 1;

            let byte7 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte6 & 3) << 7) | (byte7 >> 1)) as i64;
            values_offset += 1;

            let byte8 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte7 & 1) << 8) | byte8) as i64;
            values_offset += 1;
        }
    }
    /// Decodes blocks of type `u64` into `i32` values.
    fn decode_long_to_int(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: usize,
    ) {
        for _ in 0..iterations {
            let block0 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (block0 >> 55) as i32;
            values[values_offset + 1] = ((block0 >> 46) & 511) as i32;
            values[values_offset + 2] = ((block0 >> 37) & 511) as i32;
            values[values_offset + 3] = ((block0 >> 28) & 511) as i32;
            values[values_offset + 4] = ((block0 >> 19) & 511) as i32;
            values[values_offset + 5] = ((block0 >> 10) & 511) as i32;
            values[values_offset + 6] = ((block0 >> 1) & 511) as i32;
            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset + 7] = (((block0 & 1) << 8) | (block1 >> 56)) as i32;
            values[values_offset + 8] = ((block1 >> 47) & 511) as i32;
            values[values_offset + 9] = ((block1 >> 38) & 511) as i32;
            values[values_offset + 10] = ((block1 >> 29) & 511) as i32;
            values[values_offset + 11] = ((block1 >> 20) & 511) as i32;
            values[values_offset + 12] = ((block1 >> 11) & 511) as i32;
            values[values_offset + 13] = ((block1 >> 2) & 511) as i32;
            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset + 14] = (((block1 & 3) << 7) | (block2 >> 57)) as i32;
            values[values_offset + 15] = ((block2 >> 48) & 511) as i32;
            values[values_offset + 16] = ((block2 >> 39) & 511) as i32;
            values[values_offset + 17] = ((block2 >> 30) & 511) as i32;
            values[values_offset + 18] = ((block2 >> 21) & 511) as i32;
            values[values_offset + 19] = ((block2 >> 12) & 511) as i32;
            values[values_offset + 20] = ((block2 >> 3) & 511) as i32;
            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset + 21] = (((block2 & 7) << 6) | (block3 >> 58)) as i32;
            values[values_offset + 22] = ((block3 >> 49) & 511) as i32;
            values[values_offset + 23] = ((block3 >> 40) & 511) as i32;
            values[values_offset + 24] = ((block3 >> 31) & 511) as i32;
            values[values_offset + 25] = ((block3 >> 22) & 511) as i32;
            values[values_offset + 26] = ((block3 >> 13) & 511) as i32;
            values[values_offset + 27] = ((block3 >> 4) & 511) as i32;
            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset + 28] = (((block3 & 15) << 5) | (block4 >> 59)) as i32;
            values[values_offset + 29] = ((block4 >> 50) & 511) as i32;
            values[values_offset + 30] = ((block4 >> 41) & 511) as i32;
            values[values_offset + 31] = ((block4 >> 32) & 511) as i32;
            values[values_offset + 32] = ((block4 >> 23) & 511) as i32;
            values[values_offset + 33] = ((block4 >> 14) & 511) as i32;
            values[values_offset + 34] = ((block4 >> 5) & 511) as i32;
            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset + 35] = (((block4 & 31) << 4) | (block5 >> 60)) as i32;
            values[values_offset + 36] = ((block5 >> 51) & 511) as i32;
            values[values_offset + 37] = ((block5 >> 42) & 511) as i32;
            values[values_offset + 38] = ((block5 >> 33) & 511) as i32;
            values[values_offset + 39] = ((block5 >> 24) & 511) as i32;
            values[values_offset + 40] = ((block5 >> 15) & 511) as i32;
            values[values_offset + 41] = ((block5 >> 6) & 511) as i32;
            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset + 42] = (((block5 & 63) << 3) | (block6 >> 61)) as i32;
            values[values_offset + 43] = ((block6 >> 52) & 511) as i32;
            values[values_offset + 44] = ((block6 >> 43) & 511) as i32;
            values[values_offset + 45] = ((block6 >> 34) & 511) as i32;
            values[values_offset + 46] = ((block6 >> 25) & 511) as i32;
            values[values_offset + 47] = ((block6 >> 16) & 511) as i32;
            values[values_offset + 48] = ((block6 >> 7) & 511) as i32;
            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset + 49] = (((block6 & 127) << 2) | (block7 >> 62)) as i32;
            values[values_offset + 50] = ((block7 >> 53) & 511) as i32;
            values[values_offset + 51] = ((block7 >> 44) & 511) as i32;
            values[values_offset + 52] = ((block7 >> 35) & 511) as i32;
            values[values_offset + 53] = ((block7 >> 26) & 511) as i32;
            values[values_offset + 54] = ((block7 >> 17) & 511) as i32;
            values[values_offset + 55] = ((block7 >> 8) & 511) as i32;
            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset + 56] = (((block7 & 255) << 1) | (block8 >> 63)) as i32;
            values[values_offset + 57] = ((block8 >> 54) & 511) as i32;
            values[values_offset + 58] = ((block8 >> 45) & 511) as i32;
            values[values_offset + 59] = ((block8 >> 36) & 511) as i32;
            values[values_offset + 60] = ((block8 >> 27) & 511) as i32;
            values[values_offset + 61] = ((block8 >> 18) & 511) as i32;
            values[values_offset + 62] = ((block8 >> 9) & 511) as i32;
            values[values_offset + 63] = (block8 & 511) as i32;
            values_offset += 64;
        }
    }
    /// Decodes blocks of type `u8` into `i32` values.
    fn decode_byte_to_int(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: usize,
    ) {
        for _ in 0..iterations {
            let byte0 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte1 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = (byte0 << 1) | (byte1 >> 7);
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte1 & 127) << 2) | (byte2 >> 6);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte2 & 63) << 3) | (byte3 >> 5);
            values_offset += 1;

            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte3 & 31) << 4) | (byte4 >> 4);
            values_offset += 1;

            let byte5 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte4 & 15) << 5) | (byte5 >> 3);
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte5 & 7) << 6) | (byte6 >> 2);
            values_offset += 1;

            let byte7 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte6 & 3) << 7) | (byte7 >> 1);
            values_offset += 1;

            let byte8 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte7 & 1) << 8) | byte8;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked9 {}
