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
use crate::util::packed::Decoder;

struct BulkOperationPacked2;
impl Decoder for BulkOperationPacked2 {
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
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block >> 62) & 3) as i64;
            values[values_offset + 1] = ((block >> 60) & 3) as i64;
            values[values_offset + 2] = ((block >> 58) & 3) as i64;
            values[values_offset + 3] = ((block >> 56) & 3) as i64;
            values[values_offset + 4] = ((block >> 54) & 3) as i64;
            values[values_offset + 5] = ((block >> 52) & 3) as i64;
            values[values_offset + 6] = ((block >> 50) & 3) as i64;
            values[values_offset + 7] = ((block >> 48) & 3) as i64;
            values[values_offset + 8] = ((block >> 46) & 3) as i64;
            values[values_offset + 9] = ((block >> 44) & 3) as i64;
            values[values_offset + 10] = ((block >> 42) & 3) as i64;
            values[values_offset + 11] = ((block >> 40) & 3) as i64;
            values[values_offset + 12] = ((block >> 38) & 3) as i64;
            values[values_offset + 13] = ((block >> 36) & 3) as i64;
            values[values_offset + 14] = ((block >> 34) & 3) as i64;
            values[values_offset + 15] = ((block >> 32) & 3) as i64;
            values[values_offset + 16] = ((block >> 30) & 3) as i64;
            values[values_offset + 17] = ((block >> 28) & 3) as i64;
            values[values_offset + 18] = ((block >> 26) & 3) as i64;
            values[values_offset + 19] = ((block >> 24) & 3) as i64;
            values[values_offset + 20] = ((block >> 22) & 3) as i64;
            values[values_offset + 21] = ((block >> 20) & 3) as i64;
            values[values_offset + 22] = ((block >> 18) & 3) as i64;
            values[values_offset + 23] = ((block >> 16) & 3) as i64;
            values[values_offset + 24] = ((block >> 14) & 3) as i64;
            values[values_offset + 25] = ((block >> 12) & 3) as i64;
            values[values_offset + 26] = ((block >> 10) & 3) as i64;
            values[values_offset + 27] = ((block >> 8) & 3) as i64;
            values[values_offset + 28] = ((block >> 6) & 3) as i64;
            values[values_offset + 29] = ((block >> 4) & 3) as i64;
            values[values_offset + 30] = ((block >> 2) & 3) as i64;
            values[values_offset + 31] = (block & 3) as i64;

            values_offset += 32;
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
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block >> 6) & 3) as i64;
            values[values_offset + 1] = ((block >> 4) & 3) as i64;
            values[values_offset + 2] = ((block >> 2) & 3) as i64;
            values[values_offset + 3] = (block & 3) as i64;

            values_offset += 4;
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
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block >> 62) & 3) as i32;
            values[values_offset + 1] = ((block >> 60) & 3) as i32;
            values[values_offset + 2] = ((block >> 58) & 3) as i32;
            values[values_offset + 3] = ((block >> 56) & 3) as i32;
            values[values_offset + 4] = ((block >> 54) & 3) as i32;
            values[values_offset + 5] = ((block >> 52) & 3) as i32;
            values[values_offset + 6] = ((block >> 50) & 3) as i32;
            values[values_offset + 7] = ((block >> 48) & 3) as i32;
            values[values_offset + 8] = ((block >> 46) & 3) as i32;
            values[values_offset + 9] = ((block >> 44) & 3) as i32;
            values[values_offset + 10] = ((block >> 42) & 3) as i32;
            values[values_offset + 11] = ((block >> 40) & 3) as i32;
            values[values_offset + 12] = ((block >> 38) & 3) as i32;
            values[values_offset + 13] = ((block >> 36) & 3) as i32;
            values[values_offset + 14] = ((block >> 34) & 3) as i32;
            values[values_offset + 15] = ((block >> 32) & 3) as i32;
            values[values_offset + 16] = ((block >> 30) & 3) as i32;
            values[values_offset + 17] = ((block >> 28) & 3) as i32;
            values[values_offset + 18] = ((block >> 26) & 3) as i32;
            values[values_offset + 19] = ((block >> 24) & 3) as i32;
            values[values_offset + 20] = ((block >> 22) & 3) as i32;
            values[values_offset + 21] = ((block >> 20) & 3) as i32;
            values[values_offset + 22] = ((block >> 18) & 3) as i32;
            values[values_offset + 23] = ((block >> 16) & 3) as i32;
            values[values_offset + 24] = ((block >> 14) & 3) as i32;
            values[values_offset + 25] = ((block >> 12) & 3) as i32;
            values[values_offset + 26] = ((block >> 10) & 3) as i32;
            values[values_offset + 27] = ((block >> 8) & 3) as i32;
            values[values_offset + 28] = ((block >> 6) & 3) as i32;
            values[values_offset + 29] = ((block >> 4) & 3) as i32;
            values[values_offset + 30] = ((block >> 2) & 3) as i32;
            values[values_offset + 31] = (block & 3) as i32;

            values_offset += 32;
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
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block >> 6) & 3) as i32;
            values[values_offset + 1] = ((block >> 4) & 3) as i32;
            values[values_offset + 2] = ((block >> 2) & 3) as i32;
            values[values_offset + 3] = (block & 3) as i32;

            values_offset += 4;
        }
    }
}
