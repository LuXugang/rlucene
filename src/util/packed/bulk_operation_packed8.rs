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
pub struct BulkOperationPacked8;
impl Decoder for BulkOperationPacked8 {
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
            for shift in (0..=56).rev().step_by(8) {
                values[values_offset] = ((block >> shift) & 255) as i64;
                values_offset += 1;
            }
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
            values[values_offset] = (blocks[blocks_offset] as u64 & 0xFF) as i64;
            blocks_offset += 1;
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
        iterations: u32,
    ) {
        for _ in 0..iterations {
            let block = blocks[blocks_offset];
            blocks_offset += 1;
            for shift in (0..=56).rev().step_by(8) {
                values[values_offset] = ((block >> shift) & 255) as i32;
                values_offset += 1;
            }
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
            values[values_offset] = blocks[blocks_offset] as i32;
            blocks_offset += 1;
            values_offset += 1;
        }
    }
}
impl Encoder for BulkOperationPacked8 {}
impl BulkOperation for BulkOperationPacked8 {}
