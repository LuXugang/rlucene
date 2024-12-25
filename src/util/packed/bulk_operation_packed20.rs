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
pub struct BulkOperationPacked20;
impl Decoder for BulkOperationPacked20 {
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
            values[values_offset] = (block0 >> 44) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 24) & 1_048_575) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 4) & 1_048_575) as i64;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 15) << 16) | (block1 >> 48)) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 28) & 1_048_575) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 8) & 1_048_575) as i64;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 255) << 12) | (block2 >> 52)) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 32) & 1_048_575) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 12) & 1_048_575) as i64;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 4095) << 8) | (block3 >> 56)) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 36) & 1_048_575) as i64;
            values_offset += 1;
            values[values_offset] = ((block3 >> 16) & 1_048_575) as i64;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 65_535) << 4) | (block4 >> 60)) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 40) & 1_048_575) as i64;
            values_offset += 1;
            values[values_offset] = ((block4 >> 20) & 1_048_575) as i64;
            values_offset += 1;
            values[values_offset] = (block4 & 1_048_575) as i64;
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
            values[values_offset] = (byte0 << 12) | (byte1 << 4) | (byte2 >> 4);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte2 & 15) << 16) | (byte3 << 8) | byte4;
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
            values[values_offset] = (block0 >> 44) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 24) & 1_048_575) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 4) & 1_048_575) as i32;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 15) << 16) | (block1 >> 48)) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 28) & 1_048_575) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 8) & 1_048_575) as i32;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 255) << 12) | (block2 >> 52)) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 32) & 1_048_575) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 12) & 1_048_575) as i32;
            values_offset += 1;

            let block3 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block2 & 4095) << 8) | (block3 >> 56)) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 36) & 1_048_575) as i32;
            values_offset += 1;
            values[values_offset] = ((block3 >> 16) & 1_048_575) as i32;
            values_offset += 1;

            let block4 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block3 & 65_535) << 4) | (block4 >> 60)) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 40) & 1_048_575) as i32;
            values_offset += 1;
            values[values_offset] = ((block4 >> 20) & 1_048_575) as i32;
            values_offset += 1;
            values[values_offset] = (block4 & 1_048_575) as i32;
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
            values[values_offset] = (byte0 << 12) | (byte1 << 4) | (byte2 >> 4);
            values_offset += 1;

            let byte3 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            let byte4 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte2 & 15) << 16) | (byte3 << 8) | byte4;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked20 {}
impl BulkOperation for BulkOperationPacked20 {}
