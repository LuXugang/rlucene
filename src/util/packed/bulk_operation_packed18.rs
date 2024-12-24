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

pub(crate) struct BulkOperationPacked18;

impl Decoder for BulkOperationPacked18 {
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
            values[values_offset] = (block0 >> 46) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 28) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 10) & 262_143) as i64;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 1_023) << 8) | (block1 >> 56)) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 38) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 20) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 2) & 262_143) as i64;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 3) << 16) | (block2 >> 48)) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 30) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 12) & 262_143) as i64;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 4_095) << 6) | (block3 >> 58)) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 40) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 22) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 4) & 262_143) as i64;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 15) << 14) | (block4 >> 50)) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 32) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 14) & 262_143) as i64;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 16_383) << 4) | (block5 >> 60)) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 42) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 24) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 6) & 262_143) as i64;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 63) << 12) | (block6 >> 52)) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 34) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 16) & 262_143) as i64;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 65_535) << 2) | (block7 >> 62)) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 44) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 26) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 8) & 262_143) as i64;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 255) << 10) | (block8 >> 54)) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 36) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 18) & 262_143) as i64;
            values_offset += 1;
            values[values_offset] = (block8 & 262_143) as i64;
            values_offset += 1;
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
            let byte0 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte1 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte2 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = (byte0 << 10) | (byte1 << 2) | (byte2 >> 6);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte2 & 63) << 12) | (byte3 << 4) | (byte4 >> 4);
            values_offset += 1;

            let byte5 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte6 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte4 & 15) << 14) | (byte5 << 6) | (byte6 >> 2);
            values_offset += 1;

            let byte7 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte8 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte6 & 3) << 16) | (byte7 << 8) | byte8;
            values_offset += 1;
        }
    }
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
            values[values_offset] = (block0 >> 46) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 28) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 10) & 262_143) as i32;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 1_023) << 8) | (block1 >> 56)) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 38) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 20) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 2) & 262_143) as i32;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 3) << 16) | (block2 >> 48)) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 30) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 12) & 262_143) as i32;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 4_095) << 6) | (block3 >> 58)) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 40) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 22) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 4) & 262_143) as i32;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 15) << 14) | (block4 >> 50)) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 32) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 14) & 262_143) as i32;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 16_383) << 4) | (block5 >> 60)) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 42) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 24) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 6) & 262_143) as i32;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 63) << 12) | (block6 >> 52)) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 34) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 16) & 262_143) as i32;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 65_535) << 2) | (block7 >> 62)) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 44) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 26) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 8) & 262_143) as i32;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 255) << 10) | (block8 >> 54)) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 36) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 18) & 262_143) as i32;
            values_offset += 1;
            values[values_offset] = (block8 & 262_143) as i32;
            values_offset += 1;
        }
    }
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
            let byte2 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = (byte0 << 10) | (byte1 << 2) | (byte2 >> 6);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte2 & 63) << 12) | (byte3 << 4) | (byte4 >> 4);
            values_offset += 1;

            let byte5 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte6 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte4 & 15) << 14) | (byte5 << 6) | (byte6 >> 2);
            values_offset += 1;

            let byte7 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte8 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte6 & 3) << 16) | (byte7 << 8) | byte8;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked18 {}
