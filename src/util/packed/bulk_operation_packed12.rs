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
pub(crate) struct BulkOperationPacked12;
impl Decoder for BulkOperationPacked12 {
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
            values[values_offset] = (block0 >> 52) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 40) & 4095) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 28) & 4095) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 16) & 4095) as i64;
            values_offset += 1;
            values[values_offset] = ((block0 >> 4) & 4095) as i64;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 15) << 8) | (block1 >> 56)) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 44) & 4095) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 32) & 4095) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 20) & 4095) as i64;
            values_offset += 1;
            values[values_offset] = ((block1 >> 8) & 4095) as i64;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 255) << 4) | (block2 >> 60)) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 48) & 4095) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 36) & 4095) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 24) & 4095) as i64;
            values_offset += 1;
            values[values_offset] = ((block2 >> 12) & 4095) as i64;
            values_offset += 1;
            values[values_offset] = (block2 & 4095) as i64;
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
            values[values_offset] = (byte0 << 4) | (byte1 >> 4);
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as i64;
            blocks_offset += 1;
            values[values_offset] = ((byte1 & 15) << 8) | byte2;
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
            values[values_offset] = (block0 >> 52) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 40) & 4095) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 28) & 4095) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 16) & 4095) as i32;
            values_offset += 1;
            values[values_offset] = ((block0 >> 4) & 4095) as i32;
            values_offset += 1;

            let block1 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block0 & 15) << 8) | (block1 >> 56)) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 44) & 4095) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 32) & 4095) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 20) & 4095) as i32;
            values_offset += 1;
            values[values_offset] = ((block1 >> 8) & 4095) as i32;
            values_offset += 1;

            let block2 = blocks[blocks_offset];
            blocks_offset += 1;
            values[values_offset] = (((block1 & 255) << 4) | (block2 >> 60)) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 48) & 4095) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 36) & 4095) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 24) & 4095) as i32;
            values_offset += 1;
            values[values_offset] = ((block2 >> 12) & 4095) as i32;
            values_offset += 1;
            values[values_offset] = (block2 & 4095) as i32;
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
            values[values_offset] = (byte0 << 4) | (byte1 >> 4);
            values_offset += 1;

            let byte2 = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values[values_offset] = ((byte1 & 15) << 8) | byte2;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked12 {}
