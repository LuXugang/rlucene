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

pub(crate) struct BulkOperationPacked11;
impl Decoder for BulkOperationPacked11 {
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
            values[values_offset] = (block0 >> 53) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 42) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 31) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 20) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 9) & 2047) as i64;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 511) << 2) | (block1 >> 62)) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 51) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 40) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 29) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 18) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 7) & 2047) as i64;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 127) << 4) | (block2 >> 60)) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 49) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 38) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 27) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 16) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 5) & 2047) as i64;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 31) << 6) | (block3 >> 58)) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 47) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 36) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 25) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 14) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 3) & 2047) as i64;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 7) << 8) | (block4 >> 56)) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 45) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 34) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 23) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 12) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 1) & 2047) as i64;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 1) << 10) | (block5 >> 54)) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 43) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 32) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 21) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 10) & 2047) as i64;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 1023) << 1) | (block6 >> 63)) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 52) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 41) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 30) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 19) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 8) & 2047) as i64;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 255) << 3) | (block7 >> 61)) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 50) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 39) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 28) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 17) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block7 >> 6) & 2047) as i64;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 63) << 5) | (block8 >> 59)) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 48) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 37) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 26) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 15) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block8 >> 4) & 2047) as i64;
            values_offset += 1;

            let block9 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block8 & 15) << 7) | (block9 >> 57)) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 46) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 35) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 24) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 13) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block9 >> 2) & 2047) as i64;
            values_offset += 1;

            let block10 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block9 & 3) << 9) | (block10 >> 55)) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 44) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 33) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 22) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = ((block10 >> 11) & 2047) as i64;
            values_offset += 1;
            values[values_offset] = (block10 & 2047) as i64;
            values_offset += 1;
        }
    }
    /// Decodes blocks of type `u8` into `u64` values.
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
            values[values_offset] = ((byte0 << 3) | (byte1 >> 5)) as i64;
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte1 & 31) << 6) | (byte2 >> 2)) as i64;
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte2 & 3) << 9) | (byte3 << 1) | (byte4 >> 7)) as i64;
            values_offset += 1;

            let byte5 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte4 & 127) << 4) | (byte5 >> 4)) as i64;
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte5 & 15) << 7) | (byte6 >> 1)) as i64;
            values_offset += 1;

            let byte7 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            let byte8 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte6 & 1) << 10) | (byte7 << 2) | (byte8 >> 6)) as i64;
            values_offset += 1;

            let byte9 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte8 & 63) << 5) | (byte9 >> 3)) as i64;
            values_offset += 1;

            let byte10 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte9 & 7) << 8) | byte10) as i64;
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
            values[values_offset] = (block0 >> 53) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 42) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 31) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 20) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 9) & 2047) as i32;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 511) << 2) | (block1 >> 62)) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 51) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 40) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 29) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 18) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 7) & 2047) as i32;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 127) << 4) | (block2 >> 60)) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 49) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 38) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 27) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 16) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 5) & 2047) as i32;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 31) << 6) | (block3 >> 58)) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 47) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 36) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 25) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 14) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 3) & 2047) as i32;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 7) << 8) | (block4 >> 56)) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 45) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 34) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 23) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 12) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 1) & 2047) as i32;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 1) << 10) | (block5 >> 54)) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 43) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 32) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 21) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 10) & 2047) as i32;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 1023) << 1) | (block6 >> 63)) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 52) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 41) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 30) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 19) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 8) & 2047) as i32;
            values_offset += 1;

            let block7 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block6 & 255) << 3) | (block7 >> 61)) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 50) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 39) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 28) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 17) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block7 >> 6) & 2047) as i32;
            values_offset += 1;

            let block8 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block7 & 63) << 5) | (block8 >> 59)) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 48) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 37) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 26) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 15) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block8 >> 4) & 2047) as i32;
            values_offset += 1;

            let block9 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block8 & 15) << 7) | (block9 >> 57)) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 46) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 35) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 24) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 13) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block9 >> 2) & 2047) as i32;
            values_offset += 1;

            let block10 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block9 & 3) << 9) | (block10 >> 55)) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 44) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 33) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 22) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = ((block10 >> 11) & 2047) as i32;
            values_offset += 1;
            values[values_offset] = (block10 & 2047) as i32;
            values_offset += 1;
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
            values[values_offset] = (byte0 << 3) | (byte1 >> 5);
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte1 & 31) << 6) | (byte2 >> 2);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte2 & 3) << 9) | (byte3 << 1) | (byte4 >> 7);
            values_offset += 1;

            let byte5 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte4 & 127) << 4) | (byte5 >> 4);
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte5 & 15) << 7) | (byte6 >> 1);
            values_offset += 1;

            let byte7 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte8 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte6 & 1) << 10) | (byte7 << 2) | (byte8 >> 6);
            values_offset += 1;

            let byte9 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte8 & 63) << 5) | (byte9 >> 3);
            values_offset += 1;

            let byte10 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte9 & 7) << 8) | byte10;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked11 {}
