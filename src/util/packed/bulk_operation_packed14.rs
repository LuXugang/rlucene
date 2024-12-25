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

#[derive(Default)]
pub(crate) struct BulkOperationPacked14;
impl Decoder for BulkOperationPacked14 {
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
            values[values_offset] = (block0 >> 50) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 36) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 22) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 8) & 16383) as i64;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 255) << 6) | (block1 >> 58)) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 44) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 30) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 16) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 2) & 16383) as i64;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 3) << 12) | (block2 >> 52)) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 38) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 24) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 10) & 16383) as i64;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 1023) << 4) | (block3 >> 60)) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 46) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 32) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 18) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 4) & 16383) as i64;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 15) << 10) | (block4 >> 54)) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 40) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 26) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 12) & 16383) as i64;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 4095) << 2) | (block5 >> 62)) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 48) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 34) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 20) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block5 >> 6) & 16383) as i64;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 63) << 8) | (block6 >> 56)) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 42) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 28) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = ((block6 >> 14) & 16383) as i64;
            values_offset += 1;
            values[values_offset] = (block6 & 16383) as i64;
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
            values[values_offset] = (byte0 << 6) | (byte1 >> 2);
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte3 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte1 & 3) << 12) | (byte2 << 4) | (byte3 >> 4);
            values_offset += 1;

            let byte4 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte5 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte3 & 15) << 10) | (byte4 << 2) | (byte5 >> 6);
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte5 & 63) << 8) | byte6;
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
            values[values_offset] = (block0 >> 50) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 36) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 22) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 8) & 16383) as i32;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 255) << 6) | (block1 >> 58)) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 44) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 30) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 16) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 2) & 16383) as i32;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 3) << 12) | (block2 >> 52)) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 38) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 24) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 10) & 16383) as i32;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 1023) << 4) | (block3 >> 60)) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 46) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 32) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 18) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 4) & 16383) as i32;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 15) << 10) | (block4 >> 54)) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 40) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 26) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 12) & 16383) as i32;
            values_offset += 1;

            let block5 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block4 & 4095) << 2) | (block5 >> 62)) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 48) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 34) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 20) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block5 >> 6) & 16383) as i32;
            values_offset += 1;

            let block6 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block5 & 63) << 8) | (block6 >> 56)) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 42) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 28) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = ((block6 >> 14) & 16383) as i32;
            values_offset += 1;
            values[values_offset] = (block6 & 16383) as i32;
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
            values[values_offset] = (byte0 << 6) | (byte1 >> 2);
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte1 & 3) << 12) | (byte2 << 4) | (byte3 >> 4);
            values_offset += 1;

            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte5 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte3 & 15) << 10) | (byte4 << 2) | (byte5 >> 6);
            values_offset += 1;

            let byte6 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte5 & 63) << 8) | byte6;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked14 {}
