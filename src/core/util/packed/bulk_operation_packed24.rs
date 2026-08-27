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
use crate::core::util::packed::Decoder;
use crate::core::util::packed::bulk_operation::BulkOperation;
use crate::core::util::packed::bulk_operation_packed::{
  define_bulk_operation_packed_specialized, delegate_bulk_operation_packed_decoder_counts,
  impl_bulk_operation_packed_encoder,
};

define_bulk_operation_packed_specialized!(BulkOperationPacked24, 24);
impl Decoder for BulkOperationPacked24 {
  delegate_bulk_operation_packed_decoder_counts!();
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
      values[values_offset] = (block0 >> 40) as i32;
      values_offset += 1;
      values[values_offset] = ((block0 >> 16) & 16777215) as i32;
      values_offset += 1;

      let block1 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block0 & 65535) << 8) | (block1 >> 56)) as i32;
      values_offset += 1;
      values[values_offset] = ((block1 >> 32) & 16777215) as i32;
      values_offset += 1;
      values[values_offset] = ((block1 >> 8) & 16777215) as i32;
      values_offset += 1;

      let block2 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block1 & 255) << 16) | (block2 >> 48)) as i32;
      values_offset += 1;
      values[values_offset] = ((block2 >> 24) & 16777215) as i32;
      values_offset += 1;
      values[values_offset] = (block2 & 16777215) as i32;
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
      values[values_offset] = (byte0 << 16) | (byte1 << 8) | byte2;
      values_offset += 1;
    }
  }
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
      values[values_offset] = (block0 >> 40) as i64;
      values_offset += 1;
      values[values_offset] = ((block0 >> 16) & 0xFFFFFF) as i64;
      values_offset += 1;

      let block1 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block0 & 0xFFFF) << 8) | (block1 >> 56)) as i64;
      values_offset += 1;
      values[values_offset] = ((block1 >> 32) & 0xFFFFFF) as i64;
      values_offset += 1;
      values[values_offset] = ((block1 >> 8) & 0xFFFFFF) as i64;
      values_offset += 1;

      let block2 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block1 & 0xFF) << 16) | (block2 >> 48)) as i64;
      values_offset += 1;
      values[values_offset] = ((block2 >> 24) & 0xFFFFFF) as i64;
      values_offset += 1;
      values[values_offset] = (block2 & 0xFFFFFF) as i64;
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

      values[values_offset] = (byte0 << 16) | (byte1 << 8) | byte2;
      values_offset += 1;
    }
  }
}
impl_bulk_operation_packed_encoder!(BulkOperationPacked24);
impl BulkOperation for BulkOperationPacked24 {}
