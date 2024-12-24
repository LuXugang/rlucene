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
pub(crate) struct BulkOperationPacked10;
impl Decoder for BulkOperationPacked10 {
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
            values[values_offset] = (block0 >> 54) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 44) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 34) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 24) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 14) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 4) & 1023) as i64;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 15) << 6) | (block1 >> 58)) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 48) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 38) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 28) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 18) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 8) & 1023) as i64;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 255) << 2) | (block2 >> 62)) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 52) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 42) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 32) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 22) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 12) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 2) & 1023) as i64;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 3) << 8) | (block3 >> 56)) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 46) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 36) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 26) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 16) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 6) & 1023) as i64;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 63) << 4) | (block4 >> 60)) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 50) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 40) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 30) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 20) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 10) & 1023) as i64;
            values_offset += 1;
            values[values_offset] = (block4 & 1023) as i64;
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
            values[values_offset] = ((byte0 << 2) | (byte1 >> 6)) as i64;
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte1 & 63) << 4) | (byte2 >> 4)) as i64;
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte2 & 15) << 6) | (byte3 >> 2)) as i64;
            values_offset += 1;

            let byte4 = blocks[blocks_offset] as u64;
            blocks_offset += 1;
            values[values_offset] = (((byte3 & 3) << 8) | byte4) as i64;
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
            values[values_offset] = (block0 >> 54) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 44) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 34) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 24) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 14) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 4) & 1023) as i32;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 15) << 6) | (block1 >> 58)) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 48) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 38) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 28) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 18) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 8) & 1023) as i32;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 255) << 2) | (block2 >> 62)) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 52) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 42) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 32) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 22) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 12) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 2) & 1023) as i32;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 3) << 8) | (block3 >> 56)) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 46) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 36) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 26) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 16) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 6) & 1023) as i32;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 63) << 4) | (block4 >> 60)) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 50) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 40) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 30) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 20) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 10) & 1023) as i32;
            values_offset += 1;
            values[values_offset] = (block4 & 1023) as i32;
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
            values[values_offset] = (byte0 << 2) | (byte1 >> 6);
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte1 & 63) << 4) | (byte2 >> 4);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte2 & 15) << 6) | (byte3 >> 2);
            values_offset += 1;

            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte3 & 3) << 8) | byte4;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked10 {}
