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
pub struct BulkOperationPacked21;
impl Decoder for BulkOperationPacked21 {
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
            values[values_offset] = (block0 >> 43) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 22) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 1) & 2097151) as i64;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 1) << 20) | (block1 >> 44)) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 23) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 2) & 2097151) as i64;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 3) << 19) | (block2 >> 45)) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 24) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 3) & 2097151) as i64;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 7) << 18) | (block3 >> 46)) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 25) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 4) & 2097151) as i64;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 15) << 17) | (block4 >> 47)) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 26) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 5) & 2097151) as i64;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 31) << 16) | (block5 >> 48)) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 27) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 6) & 2097151) as i64;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 63) << 15) | (block6 >> 49)) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 28) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 7) & 2097151) as i64;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 127) << 14) | (block7 >> 50)) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 29) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 8) & 2097151) as i64;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 255) << 13) | (block8 >> 51)) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 30) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 9) & 2097151) as i64;
            values_offset += 1;

            let block9 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block8 & 511) << 12) | (block9 >> 52)) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 31) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 10) & 2097151) as i64;
            values_offset += 1;

            let block10 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block9 & 1023) << 11) | (block10 >> 53)) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 32) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 11) & 2097151) as i64;
            values_offset += 1;

            let block11 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block10 & 2047) << 10) | (block11 >> 54)) as i64;
            values_offset += 1;
            values[values_offset] = ((block11 >> 33) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block11 >> 12) & 2097151) as i64;
            values_offset += 1;

            let block12 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block11 & 4095) << 9) | (block12 >> 55)) as i64;
            values_offset += 1;
            values[values_offset] = ((block12 >> 34) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block12 >> 13) & 2097151) as i64;
            values_offset += 1;

            let block13 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block12 & 8191) << 8) | (block13 >> 56)) as i64;
            values_offset += 1;
            values[values_offset] = ((block13 >> 35) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block13 >> 14) & 2097151) as i64;
            values_offset += 1;

            let block14 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block13 & 16383) << 7) | (block14 >> 57)) as i64;
            values_offset += 1;
            values[values_offset] = ((block14 >> 36) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block14 >> 15) & 2097151) as i64;
            values_offset += 1;

            let block15 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block14 & 32767) << 6) | (block15 >> 58)) as i64;
            values_offset += 1;
            values[values_offset] = ((block15 >> 37) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block15 >> 16) & 2097151) as i64;
            values_offset += 1;

            let block16 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block15 & 65535) << 5) | (block16 >> 59)) as i64;
            values_offset += 1;
            values[values_offset] = ((block16 >> 38) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block16 >> 17) & 2097151) as i64;
            values_offset += 1;

            let block17 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block16 & 131071) << 4) | (block17 >> 60)) as i64;
            values_offset += 1;
            values[values_offset] = ((block17 >> 39) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block17 >> 18) & 2097151) as i64;
            values_offset += 1;

            let block18 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block17 & 262143) << 3) | (block18 >> 61)) as i64;
            values_offset += 1;
            values[values_offset] = ((block18 >> 40) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block18 >> 19) & 2097151) as i64;
            values_offset += 1;

            let block19 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block18 & 524287) << 2) | (block19 >> 62)) as i64;
            values_offset += 1;
            values[values_offset] = ((block19 >> 41) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block19 >> 20) & 2097151) as i64;
            values_offset += 1;

            let block20 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block19 & 1048575) << 1) | (block20 >> 63)) as i64;
            values_offset += 1;
            values[values_offset] = ((block20 >> 42) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = ((block20 >> 21) & 2097151) as i64;
            values_offset += 1;
            values[values_offset] = (block20 & 2097151) as i64;
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
            values[values_offset] = (byte0 << 13) | (byte1 << 5) | (byte2 >> 3);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte5 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] =
                ((byte2 & 7) << 18) | (byte3 << 10) | (byte4 << 2) | (byte5 >> 6);
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte7 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte5 & 63) << 15) | (byte6 << 7) | (byte7 >> 1);
            values_offset += 1;

            let byte8 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte9 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte10 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] =
                ((byte7 & 1) << 20) | (byte8 << 12) | (byte9 << 4) | (byte10 >> 4);
            values_offset += 1;

            let byte11 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte12 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte13 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] =
                ((byte10 & 15) << 17) | (byte11 << 9) | (byte12 << 1) | (byte13 >> 7);
            values_offset += 1;

            let byte14 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte15 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte13 & 127) << 14) | (byte14 << 6) | (byte15 >> 2);
            values_offset += 1;

            let byte16 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte17 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte18 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] =
                ((byte15 & 3) << 19) | (byte16 << 11) | (byte17 << 3) | (byte18 >> 5);
            values_offset += 1;

            let byte19 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte20 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte18 & 31) << 16) | (byte19 << 8) | byte20;
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
            values[values_offset] = (block0 >> 43) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 22) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 1) & 2_097_151) as i32;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 1) << 20) | (block1 >> 44)) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 23) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 2) & 2_097_151) as i32;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 3) << 19) | (block2 >> 45)) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 24) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 3) & 2_097_151) as i32;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 7) << 18) | (block3 >> 46)) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 25) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 4) & 2_097_151) as i32;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 15) << 17) | (block4 >> 47)) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 26) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 5) & 2_097_151) as i32;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 31) << 16) | (block5 >> 48)) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 27) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 6) & 2_097_151) as i32;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 63) << 15) | (block6 >> 49)) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 28) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 7) & 2_097_151) as i32;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 127) << 14) | (block7 >> 50)) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 29) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 8) & 2_097_151) as i32;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 255) << 13) | (block8 >> 51)) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 30) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 9) & 2_097_151) as i32;
            values_offset += 1;

            let block9 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block8 & 511) << 12) | (block9 >> 52)) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 31) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 10) & 2_097_151) as i32;
            values_offset += 1;

            let block10 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block9 & 1023) << 11) | (block10 >> 53)) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 32) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 11) & 2_097_151) as i32;
            values_offset += 1;

            let block11 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block10 & 2047) << 10) | (block11 >> 54)) as i32;
            values_offset += 1;
            values[values_offset] = ((block11 >> 33) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block11 >> 12) & 2_097_151) as i32;
            values_offset += 1;

            let block12 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block11 & 4095) << 9) | (block12 >> 55)) as i32;
            values_offset += 1;
            values[values_offset] = ((block12 >> 34) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block12 >> 13) & 2_097_151) as i32;
            values_offset += 1;

            let block13 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block12 & 8191) << 8) | (block13 >> 56)) as i32;
            values_offset += 1;
            values[values_offset] = ((block13 >> 35) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block13 >> 14) & 2_097_151) as i32;
            values_offset += 1;

            let block14 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block13 & 16_383) << 7) | (block14 >> 57)) as i32;
            values_offset += 1;
            values[values_offset] = ((block14 >> 36) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block14 >> 15) & 2_097_151) as i32;
            values_offset += 1;

            let block15 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block14 & 32_767) << 6) | (block15 >> 58)) as i32;
            values_offset += 1;
            values[values_offset] = ((block15 >> 37) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block15 >> 16) & 2_097_151) as i32;
            values_offset += 1;

            let block16 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block15 & 65_535) << 5) | (block16 >> 59)) as i32;
            values_offset += 1;
            values[values_offset] = ((block16 >> 38) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block16 >> 17) & 2_097_151) as i32;
            values_offset += 1;

            let block17 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block16 & 131_071) << 4) | (block17 >> 60)) as i32;
            values_offset += 1;
            values[values_offset] = ((block17 >> 39) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block17 >> 18) & 2_097_151) as i32;
            values_offset += 1;

            let block18 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block17 & 262_143) << 3) | (block18 >> 61)) as i32;
            values_offset += 1;
            values[values_offset] = ((block18 >> 40) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block18 >> 19) & 2_097_151) as i32;
            values_offset += 1;

            let block19 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block18 & 524_287) << 2) | (block19 >> 62)) as i32;
            values_offset += 1;
            values[values_offset] = ((block19 >> 41) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block19 >> 20) & 2_097_151) as i32;
            values_offset += 1;

            let block20 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block19 & 1_048_575) << 1) | (block20 >> 63)) as i32;
            values_offset += 1;
            values[values_offset] = ((block20 >> 42) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = ((block20 >> 21) & 2_097_151) as i32;
            values_offset += 1;
            values[values_offset] = (block20 & 2_097_151) as i32;
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
            values[values_offset] = (byte0 << 13) | (byte1 << 5) | (byte2 >> 3);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte5 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] =
                ((byte2 & 7) << 18) | (byte3 << 10) | (byte4 << 2) | (byte5 >> 6);
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte7 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte5 & 63) << 15) | (byte6 << 7) | (byte7 >> 1);
            values_offset += 1;

            let byte8 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte9 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte10 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] =
                ((byte7 & 1) << 20) | (byte8 << 12) | (byte9 << 4) | (byte10 >> 4);
            values_offset += 1;

            let byte11 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte12 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte13 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] =
                ((byte10 & 15) << 17) | (byte11 << 9) | (byte12 << 1) | (byte13 >> 7);
            values_offset += 1;

            let byte14 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte15 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte13 & 127) << 14) | (byte14 << 6) | (byte15 >> 2);
            values_offset += 1;

            let byte16 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte17 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte18 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] =
                ((byte15 & 3) << 19) | (byte16 << 11) | (byte17 << 3) | (byte18 >> 5);
            values_offset += 1;

            let byte19 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte20 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte18 & 31) << 16) | (byte19 << 8) | byte20;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked21 {}
impl BulkOperation for BulkOperationPacked21 {}
