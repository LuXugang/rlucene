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
pub struct BulkOperationPacked15;
impl Decoder for BulkOperationPacked15 {
    fn decode_u64_to_i64(
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
            values[values_offset] = (block0 >> 49) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 34) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 19) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 4) & 32767) as i64;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 15) << 11) | (block1 >> 53)) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 38) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 23) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 8) & 32767) as i64;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 255) << 7) | (block2 >> 57)) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 42) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 27) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 12) & 32767) as i64;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 4095) << 3) | (block3 >> 61)) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 46) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 31) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 16) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 1) & 32767) as i64;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 1) << 14) | (block4 >> 50)) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 35) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 20) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 5) & 32767) as i64;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 31) << 10) | (block5 >> 54)) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 39) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 24) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 9) & 32767) as i64;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 511) << 6) | (block6 >> 58)) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 43) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 28) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 13) & 32767) as i64;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 8191) << 2) | (block7 >> 62)) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 47) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 32) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 17) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 2) & 32767) as i64;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 3) << 13) | (block8 >> 51)) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 36) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 21) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 6) & 32767) as i64;
            values_offset += 1;

            let block9 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block8 & 63) << 9) | (block9 >> 55)) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 40) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 25) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 10) & 32767) as i64;
            values_offset += 1;

            let block10 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block9 & 1023) << 5) | (block10 >> 59)) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 44) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 29) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 14) & 32767) as i64;
            values_offset += 1;

            let block11 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block10 & 16383) << 1) | (block11 >> 63)) as i64;
            values_offset += 1;
            values[values_offset] = ((block11 >> 48) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block11 >> 33) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block11 >> 18) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block11 >> 3) & 32767) as i64;
            values_offset += 1;

            let block12 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block11 & 7) << 12) | (block12 >> 52)) as i64;
            values_offset += 1;
            values[values_offset] = ((block12 >> 37) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block12 >> 22) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block12 >> 7) & 32767) as i64;
            values_offset += 1;

            let block13 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block12 & 127) << 8) | (block13 >> 56)) as i64;
            values_offset += 1;
            values[values_offset] = ((block13 >> 41) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block13 >> 26) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block13 >> 11) & 32767) as i64;
            values_offset += 1;

            let block14 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block13 & 2047) << 4) | (block14 >> 60)) as i64;
            values_offset += 1;
            values[values_offset] = ((block14 >> 45) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block14 >> 30) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = ((block14 >> 15) & 32767) as i64;
            values_offset += 1;
            values[values_offset] = (block14 & 32767) as i64;
            values_offset += 1;
        }
    }
    fn decode_u8_to_i64(
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
            values[values_offset] = (byte0 << 7) | (byte1 >> 1);
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte3 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte1 & 1) << 14) | (byte2 << 6) | (byte3 >> 2);
            values_offset += 1;

            let byte4 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte5 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte3 & 3) << 13) | (byte4 << 5) | (byte5 >> 3);
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte7 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte5 & 7) << 12) | (byte6 << 4) | (byte7 >> 4);
            values_offset += 1;

            let byte8 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte9 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte7 & 15) << 11) | (byte8 << 3) | (byte9 >> 5);
            values_offset += 1;

            let byte10 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte11 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte9 & 31) << 10) | (byte10 << 2) | (byte11 >> 6);
            values_offset += 1;

            let byte12 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte13 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte11 & 63) << 9) | (byte12 << 1) | (byte13 >> 7);
            values_offset += 1;

            let byte14 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte13 & 127) << 8) | byte14;
            values_offset += 1;
        }
    }
    fn decode_u64_to_i32(
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
            values[values_offset] = (block0 >> 49) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 34) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 19) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 4) & 32767) as i32;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 15) << 11) | (block1 >> 53)) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 38) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 23) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 8) & 32767) as i32;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 255) << 7) | (block2 >> 57)) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 42) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 27) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 12) & 32767) as i32;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 4095) << 3) | (block3 >> 61)) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 46) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 31) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 16) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 1) & 32767) as i32;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 1) << 14) | (block4 >> 50)) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 35) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 20) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 5) & 32767) as i32;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 31) << 10) | (block5 >> 54)) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 39) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 24) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 9) & 32767) as i32;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 511) << 6) | (block6 >> 58)) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 43) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 28) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 13) & 32767) as i32;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 8191) << 2) | (block7 >> 62)) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 47) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 32) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 17) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 2) & 32767) as i32;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 3) << 13) | (block8 >> 51)) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 36) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 21) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 6) & 32767) as i32;
            values_offset += 1;

            let block9 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block8 & 63) << 9) | (block9 >> 55)) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 40) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 25) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 10) & 32767) as i32;
            values_offset += 1;

            let block10 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block9 & 1023) << 5) | (block10 >> 59)) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 44) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 29) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 14) & 32767) as i32;
            values_offset += 1;

            let block11 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block10 & 16383) << 1) | (block11 >> 63)) as i32;
            values_offset += 1;
            values[values_offset] = ((block11 >> 48) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block11 >> 33) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block11 >> 18) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block11 >> 3) & 32767) as i32;
            values_offset += 1;

            let block12 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block11 & 7) << 12) | (block12 >> 52)) as i32;
            values_offset += 1;
            values[values_offset] = ((block12 >> 37) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block12 >> 22) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block12 >> 7) & 32767) as i32;
            values_offset += 1;

            let block13 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block12 & 127) << 8) | (block13 >> 56)) as i32;
            values_offset += 1;
            values[values_offset] = ((block13 >> 41) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block13 >> 26) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block13 >> 11) & 32767) as i32;
            values_offset += 1;

            let block14 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block13 & 2047) << 4) | (block14 >> 60)) as i32;
            values_offset += 1;
            values[values_offset] = ((block14 >> 45) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block14 >> 30) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = ((block14 >> 15) & 32767) as i32;
            values_offset += 1;
            values[values_offset] = (block14 & 32767) as i32;
            values_offset += 1;
        }
    }
    fn decode_u8_to_i32(
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
            values[values_offset] = (byte0 << 7) | (byte1 >> 1);
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte1 & 1) << 14) | (byte2 << 6) | (byte3 >> 2);
            values_offset += 1;

            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte5 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte3 & 3) << 13) | (byte4 << 5) | (byte5 >> 3);
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte7 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte5 & 7) << 12) | (byte6 << 4) | (byte7 >> 4);
            values_offset += 1;

            let byte8 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte9 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte7 & 15) << 11) | (byte8 << 3) | (byte9 >> 5);
            values_offset += 1;

            let byte10 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte11 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte9 & 31) << 10) | (byte10 << 2) | (byte11 >> 6);
            values_offset += 1;

            let byte12 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte13 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte11 & 63) << 9) | (byte12 << 1) | (byte13 >> 7);
            values_offset += 1;

            let byte14 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte13 & 127) << 8) | byte14;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked15 {}
impl BulkOperation for BulkOperationPacked15 {}
