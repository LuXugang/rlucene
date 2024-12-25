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
pub(crate) struct BulkOperationPacked17;
impl Decoder for BulkOperationPacked17 {
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
            values[values_offset] = (block0 >> 47) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 30) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 13) & 131_071) as i64;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 8_191) << 4) | (block1 >> 60)) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 43) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 26) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 9) & 131_071) as i64;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 511) << 8) | (block2 >> 56)) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 39) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 22) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 5) & 131_071) as i64;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 31) << 12) | (block3 >> 52)) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 35) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 18) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 1) & 131_071) as i64;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 1) << 16) | (block4 >> 48)) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 31) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 14) & 131_071) as i64;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 16_383) << 3) | (block5 >> 61)) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 44) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 27) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 10) & 131_071) as i64;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 1_023) << 7) | (block6 >> 57)) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 40) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 23) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 6) & 131_071) as i64;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 63) << 11) | (block7 >> 53)) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 36) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 19) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 2) & 131_071) as i64;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 3) << 15) | (block8 >> 49)) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 32) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 15) & 131_071) as i64;
            values_offset += 1;

            let block9 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block8 & 32_767) << 2) | (block9 >> 62)) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 45) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 28) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 11) & 131_071) as i64;
            values_offset += 1;

            let block10 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block9 & 2_047) << 6) | (block10 >> 58)) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 41) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 24) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 7) & 131_071) as i64;
            values_offset += 1;

            let block11 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block10 & 127) << 10) | (block11 >> 54)) as i64;
            values_offset += 1;
            values[values_offset] = ((block11 >> 37) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block11 >> 20) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block11 >> 3) & 131_071) as i64;
            values_offset += 1;

            let block12 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block11 & 7) << 14) | (block12 >> 50)) as i64;
            values_offset += 1;
            values[values_offset] = ((block12 >> 33) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block12 >> 16) & 131_071) as i64;
            values_offset += 1;

            let block13 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block12 & 65_535) << 1) | (block13 >> 63)) as i64;
            values_offset += 1;
            values[values_offset] = ((block13 >> 46) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block13 >> 29) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block13 >> 12) & 131_071) as i64;
            values_offset += 1;

            let block14 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block13 & 4_095) << 5) | (block14 >> 59)) as i64;
            values_offset += 1;
            values[values_offset] = ((block14 >> 42) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block14 >> 25) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block14 >> 8) & 131_071) as i64;
            values_offset += 1;

            let block15 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block14 & 255) << 9) | (block15 >> 55)) as i64;
            values_offset += 1;
            values[values_offset] = ((block15 >> 38) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block15 >> 21) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block15 >> 4) & 131_071) as i64;
            values_offset += 1;

            let block16 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block15 & 15) << 13) | (block16 >> 51)) as i64;
            values_offset += 1;
            values[values_offset] = ((block16 >> 34) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = ((block16 >> 17) & 131_071) as i64;
            values_offset += 1;
            values[values_offset] = (block16 & 131_071) as i64;
            values_offset += 1;
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
            let byte0 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte1 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte2 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = (byte0 << 9) | (byte1 << 1) | (byte2 >> 7);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte2 & 127) << 10) | (byte3 << 2) | (byte4 >> 6);
            values_offset += 1;

            let byte5 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte6 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte4 & 63) << 11) | (byte5 << 3) | (byte6 >> 5);
            values_offset += 1;

            let byte7 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte8 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte6 & 31) << 12) | (byte7 << 4) | (byte8 >> 4);
            values_offset += 1;

            let byte9 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte10 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte8 & 15) << 13) | (byte9 << 5) | (byte10 >> 3);
            values_offset += 1;

            let byte11 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte12 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte10 & 7) << 14) | (byte11 << 6) | (byte12 >> 2);
            values_offset += 1;

            let byte13 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte14 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte12 & 3) << 15) | (byte13 << 7) | (byte14 >> 1);
            values_offset += 1;

            let byte15 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte16 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte14 & 1) << 16) | (byte15 << 8) | byte16;
            values_offset += 1;
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
            values[values_offset] = (block0 >> 47) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 30) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 13) & 131071) as i32;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 8191) << 4) | (block1 >> 60)) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 43) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 26) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 9) & 131071) as i32;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 511) << 8) | (block2 >> 56)) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 39) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 22) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 5) & 131071) as i32;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 31) << 12) | (block3 >> 52)) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 35) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 18) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 1) & 131071) as i32;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 1) << 16) | (block4 >> 48)) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 31) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 14) & 131071) as i32;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 16383) << 3) | (block5 >> 61)) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 44) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 27) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 10) & 131071) as i32;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 1023) << 7) | (block6 >> 57)) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 40) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 23) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 6) & 131071) as i32;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 63) << 11) | (block7 >> 53)) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 36) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 19) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 2) & 131071) as i32;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 3) << 15) | (block8 >> 49)) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 32) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 15) & 131071) as i32;
            values_offset += 1;

            let block9 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block8 & 32767) << 2) | (block9 >> 62)) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 45) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 28) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 11) & 131071) as i32;
            values_offset += 1;

            let block10 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block9 & 2047) << 6) | (block10 >> 58)) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 41) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 24) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 7) & 131071) as i32;
            values_offset += 1;

            let block11 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block10 & 127) << 10) | (block11 >> 54)) as i32;
            values_offset += 1;
            values[values_offset] = ((block11 >> 37) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block11 >> 20) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block11 >> 3) & 131071) as i32;
            values_offset += 1;

            let block12 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block11 & 7) << 14) | (block12 >> 50)) as i32;
            values_offset += 1;
            values[values_offset] = ((block12 >> 33) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block12 >> 16) & 131071) as i32;
            values_offset += 1;

            let block13 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block12 & 65535) << 1) | (block13 >> 63)) as i32;
            values_offset += 1;
            values[values_offset] = ((block13 >> 46) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block13 >> 29) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block13 >> 12) & 131071) as i32;
            values_offset += 1;

            let block14 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block13 & 4095) << 5) | (block14 >> 59)) as i32;
            values_offset += 1;
            values[values_offset] = ((block14 >> 42) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block14 >> 25) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block14 >> 8) & 131071) as i32;
            values_offset += 1;

            let block15 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block14 & 255) << 9) | (block15 >> 55)) as i32;
            values_offset += 1;
            values[values_offset] = ((block15 >> 38) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block15 >> 21) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block15 >> 4) & 131071) as i32;
            values_offset += 1;

            let block16 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block15 & 15) << 13) | (block16 >> 51)) as i32;
            values_offset += 1;
            values[values_offset] = ((block16 >> 34) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = ((block16 >> 17) & 131071) as i32;
            values_offset += 1;
            values[values_offset] = (block16 & 131071) as i32;
            values_offset += 1;
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
            let byte1 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte2 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = (byte0 << 9) | (byte1 << 1) | (byte2 >> 7);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte2 & 127) << 10) | (byte3 << 2) | (byte4 >> 6);
            values_offset += 1;

            let byte5 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte6 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte4 & 63) << 11) | (byte5 << 3) | (byte6 >> 5);
            values_offset += 1;

            let byte7 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte8 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte6 & 31) << 12) | (byte7 << 4) | (byte8 >> 4);
            values_offset += 1;

            let byte9 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte10 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte8 & 15) << 13) | (byte9 << 5) | (byte10 >> 3);
            values_offset += 1;

            let byte11 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte12 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte10 & 7) << 14) | (byte11 << 6) | (byte12 >> 2);
            values_offset += 1;

            let byte13 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte14 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte12 & 3) << 15) | (byte13 << 7) | (byte14 >> 1);
            values_offset += 1;

            let byte15 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte16 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte14 & 1) << 16) | (byte15 << 8) | byte16;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked17 {}
impl BulkOperation for BulkOperationPacked17 {}
