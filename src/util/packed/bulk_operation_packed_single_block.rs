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

const BLOCK_COUNT: usize = 1;
/// Non-specialized `BulkOperation` for `PackedInts.Format::PACKED_SINGLE_BLOCK`.
#[derive(Default)]
pub(crate) struct BulkOperationPackedSingleBlock {
    bits_per_value: usize,
    value_count: usize,
    mask: u64,
}
impl BulkOperationPackedSingleBlock {
    pub const fn new(bits_per_value: usize) -> Self {
        Self {
            bits_per_value,
            value_count: 64 / bits_per_value,
            mask: (1u64 << bits_per_value) - 1,
        }
    }
}
impl Decoder for BulkOperationPackedSingleBlock {
    fn long_block_count(&self) -> u32 {
        todo!()
    }

    fn long_value_count(&self) -> u32 {
        todo!()
    }

    fn byte_block_count(&self) -> u32 {
        todo!()
    }

    fn byte_value_count(&self) -> u32 {
        todo!()
    }

    fn decode_long_to_long(
        &self,
        blocks: &[u64],
        blocks_offset: usize,
        values: &mut [i64],
        values_offset: usize,
        iterations: u32,
    ) {
        todo!()
    }

    fn decode_byte_to_long(
        &self,
        blocks: &[u8],
        blocks_offset: usize,
        values: &mut [i64],
        values_offset: usize,
        iterations: u32,
    ) {
        todo!()
    }

    fn decode_long_to_int(
        &self,
        blocks: &[u64],
        blocks_offset: usize,
        values: &mut [i32],
        values_offset: usize,
        iterations: u32,
    ) {
        todo!()
    }

    fn decode_byte_to_int(
        &self,
        blocks: &[u8],
        blocks_offset: usize,
        values: &mut [i32],
        values_offset: usize,
        iterations: u32,
    ) {
        todo!()
    }
}
impl Encoder for BulkOperationPackedSingleBlock {}
