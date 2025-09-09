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
use crate::core::util::packed::bulk_operation::BulkOperation;
use crate::core::util::packed::{Decoder, Encoder};

#[derive(Default)]
pub(crate) struct BulkOperationPacked22;
impl Decoder for BulkOperationPacked22 {
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
            values[values_offset] = (block0 >> 42) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 20) & 4_194_303) as i64;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 1_048_575) << 2) | (block1 >> 62)) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 40) & 4_194_303) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 18) & 4_194_303) as i64;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 262_143) << 4) | (block2 >> 60)) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 38) & 4_194_303) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 16) & 4_194_303) as i64;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 65_535) << 6) | (block3 >> 58)) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 36) & 4_194_303) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 14) & 4_194_303) as i64;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 16_383) << 8) | (block4 >> 56)) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 34) & 4_194_303) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 12) & 4_194_303) as i64;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 4_095) << 10) | (block5 >> 54)) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 32) & 4_194_303) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 10) & 4_194_303) as i64;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 1_023) << 12) | (block6 >> 52)) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 30) & 4_194_303) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 8) & 4_194_303) as i64;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 255) << 14) | (block7 >> 50)) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 28) & 4_194_303) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 6) & 4_194_303) as i64;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 63) << 16) | (block8 >> 48)) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 26) & 4_194_303) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 4) & 4_194_303) as i64;
            values_offset += 1;

            let block9 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block8 & 15) << 18) | (block9 >> 46)) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 24) & 4_194_303) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 2) & 4_194_303) as i64;
            values_offset += 1;

            let block10 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block9 & 3) << 20) | (block10 >> 44)) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 22) & 4_194_303) as i64;
            values_offset += 1;
            values[values_offset] = (block10 & 4_194_303) as i64;
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
            values[values_offset] = (byte0 << 14) | (byte1 << 6) | (byte2 >> 2);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte5 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] =
                ((byte2 & 3) << 20) | (byte3 << 12) | (byte4 << 4) | (byte5 >> 4);
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte7 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte8 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] =
                ((byte5 & 15) << 18) | (byte6 << 10) | (byte7 << 2) | (byte8 >> 6);
            values_offset += 1;

            let byte9 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte10 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte8 & 63) << 16) | (byte9 << 8) | byte10;
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
            values[values_offset] = (block0 >> 42) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 20) & 0x3FFFFF) as i32;
            values_offset += 1;
            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 0xFFFFF) << 2) | (block1 >> 62)) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 40) & 0x3FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 18) & 0x3FFFFF) as i32;
            values_offset += 1;
            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 0x3FFFF) << 4) | (block2 >> 60)) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 38) & 0x3FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 16) & 0x3FFFFF) as i32;
            values_offset += 1;
            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 0xFFFF) << 6) | (block3 >> 58)) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 36) & 0x3FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 14) & 0x3FFFFF) as i32;
            values_offset += 1;
            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 0x3FFF) << 8) | (block4 >> 56)) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 34) & 0x3FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 12) & 0x3FFFFF) as i32;
            values_offset += 1;
            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 0xFFF) << 10) | (block5 >> 54)) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 32) & 0x3FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 10) & 0x3FFFFF) as i32;
            values_offset += 1;
            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 0x3FF) << 12) | (block6 >> 52)) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 30) & 0x3FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 8) & 0x3FFFFF) as i32;
            values_offset += 1;
            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 0xFF) << 14) | (block7 >> 50)) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 28) & 0x3FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 6) & 0x3FFFFF) as i32;
            values_offset += 1;
            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 0x3F) << 16) | (block8 >> 48)) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 26) & 0x3FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 4) & 0x3FFFFF) as i32;
            values_offset += 1;
            let block9 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block8 & 0xF) << 18) | (block9 >> 46)) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 24) & 0x3FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 2) & 0x3FFFFF) as i32;
            values_offset += 1;
            let block10 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block9 & 3) << 20) | (block10 >> 44)) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 22) & 0x3FFFFF) as i32;
            values_offset += 1;
            values[values_offset] = (block10 & 0x3FFFFF) as i32;
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
            values[values_offset] = (byte0 << 14) | (byte1 << 6) | (byte2 >> 2);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte5 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] =
                ((byte2 & 3) << 20) | (byte3 << 12) | (byte4 << 4) | (byte5 >> 4);
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte7 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte8 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] =
                ((byte5 & 15) << 18) | (byte6 << 10) | (byte7 << 2) | (byte8 >> 6);
            values_offset += 1;

            let byte9 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte10 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte8 & 63) << 16) | (byte9 << 8) | byte10;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked22 {}
impl BulkOperation for BulkOperationPacked22 {}
