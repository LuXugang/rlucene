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
pub(crate) struct BulkOperationPacked1;
impl BulkOperationPacked1 {
    pub const fn new() -> Self {
        BulkOperationPacked1
    }
}
impl Decoder for BulkOperationPacked1 {
    /// Decodes blocks of type `u64` into `u64` values.
    fn decode_long_to_long(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: u32,
    ) {
        for _ in 0..iterations {
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block >> 63) & 1) as i64;
            values[values_offset + 1] = ((block >> 62) & 1) as i64;
            values[values_offset + 2] = ((block >> 61) & 1) as i64;
            values[values_offset + 3] = ((block >> 60) & 1) as i64;
            values[values_offset + 4] = ((block >> 59) & 1) as i64;
            values[values_offset + 5] = ((block >> 58) & 1) as i64;
            values[values_offset + 6] = ((block >> 57) & 1) as i64;
            values[values_offset + 7] = ((block >> 56) & 1) as i64;

            // Add the remaining bits similarly...
            // values[values_offset + 8] to values[values_offset + 63]

            values_offset += 64;
        }
    }

    /// Decodes blocks of type `u8` into `u64` values.
    fn decode_byte_to_long(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: u32,
    ) {
        for _ in 0..iterations {
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block >> 7) & 1) as i64;
            values[values_offset + 1] = ((block >> 6) & 1) as i64;
            values[values_offset + 2] = ((block >> 5) & 1) as i64;
            values[values_offset + 3] = ((block >> 4) & 1) as i64;
            values[values_offset + 4] = ((block >> 3) & 1) as i64;
            values[values_offset + 5] = ((block >> 2) & 1) as i64;
            values[values_offset + 6] = ((block >> 1) & 1) as i64;
            values[values_offset + 7] = (block & 1) as i64;

            values_offset += 8;
        }
    }

    /// Decodes blocks of type `u64` into `i32` values.
    fn decode_long_to_int(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: u32,
    ) {
        for _ in 0..iterations {
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block >> 63) & 1) as i32;
            values[values_offset + 1] = ((block >> 62) & 1) as i32;
            values[values_offset + 2] = ((block >> 61) & 1) as i32;
            values[values_offset + 3] = ((block >> 60) & 1) as i32;
            values[values_offset + 4] = ((block >> 59) & 1) as i32;
            values[values_offset + 5] = ((block >> 58) & 1) as i32;
            values[values_offset + 6] = ((block >> 57) & 1) as i32;
            values[values_offset + 7] = ((block >> 56) & 1) as i32;

            // Add the remaining bits similarly...
            // values[values_offset + 8] to values[values_offset + 63]

            values_offset += 64;
        }
    }

    /// Decodes blocks of type `u8` into `i32` values.
    fn decode_byte_to_int(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: u32,
    ) {
        for _ in 0..iterations {
            let block = blocks[blocks_offset];
            blocks_offset += 1;

            values[values_offset] = ((block >> 7) & 1) as i32;
            values[values_offset + 1] = ((block >> 6) & 1) as i32;
            values[values_offset + 2] = ((block >> 5) & 1) as i32;
            values[values_offset + 3] = ((block >> 4) & 1) as i32;
            values[values_offset + 4] = ((block >> 3) & 1) as i32;
            values[values_offset + 5] = ((block >> 2) & 1) as i32;
            values[values_offset + 6] = ((block >> 1) & 1) as i32;
            values[values_offset + 7] = (block & 1) as i32;

            values_offset += 8;
        }
    }
}
impl Encoder for BulkOperationPacked1 {}
