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

define_bulk_operation_packed_specialized!(BulkOperationPacked6, 6);
impl Decoder for BulkOperationPacked6 {
  delegate_bulk_operation_packed_decoder_counts!();
  /// Decodes blocks of type `u64` into `u64` values.
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

      values[values_offset] = (block0 >> 58) as i64;
      values[values_offset + 1] = ((block0 >> 52) & 63) as i64;
      values[values_offset + 2] = ((block0 >> 46) & 63) as i64;
      values[values_offset + 3] = ((block0 >> 40) & 63) as i64;
      values[values_offset + 4] = ((block0 >> 34) & 63) as i64;
      values[values_offset + 5] = ((block0 >> 28) & 63) as i64;
      values[values_offset + 6] = ((block0 >> 22) & 63) as i64;
      values[values_offset + 7] = ((block0 >> 16) & 63) as i64;
      values[values_offset + 8] = ((block0 >> 10) & 63) as i64;
      values[values_offset + 9] = ((block0 >> 4) & 63) as i64;

      let block1 = blocks[blocks_offset];
      blocks_offset += 1;

      values[values_offset + 10] = (((block0 & 15) << 2) | (block1 >> 62)) as i64;
      values[values_offset + 11] = ((block1 >> 56) & 63) as i64;
      values[values_offset + 12] = ((block1 >> 50) & 63) as i64;
      values[values_offset + 13] = ((block1 >> 44) & 63) as i64;
      values[values_offset + 14] = ((block1 >> 38) & 63) as i64;
      values[values_offset + 15] = ((block1 >> 32) & 63) as i64;
      values[values_offset + 16] = ((block1 >> 26) & 63) as i64;
      values[values_offset + 17] = ((block1 >> 20) & 63) as i64;
      values[values_offset + 18] = ((block1 >> 14) & 63) as i64;
      values[values_offset + 19] = ((block1 >> 8) & 63) as i64;
      values[values_offset + 20] = ((block1 >> 2) & 63) as i64;

      let block2 = blocks[blocks_offset];
      blocks_offset += 1;

      values[values_offset + 21] = (((block1 & 3) << 4) | (block2 >> 60)) as i64;
      values[values_offset + 22] = ((block2 >> 54) & 63) as i64;
      values[values_offset + 23] = ((block2 >> 48) & 63) as i64;
      values[values_offset + 24] = ((block2 >> 42) & 63) as i64;
      values[values_offset + 25] = ((block2 >> 36) & 63) as i64;
      values[values_offset + 26] = ((block2 >> 30) & 63) as i64;
      values[values_offset + 27] = ((block2 >> 24) & 63) as i64;
      values[values_offset + 28] = ((block2 >> 18) & 63) as i64;
      values[values_offset + 29] = ((block2 >> 12) & 63) as i64;
      values[values_offset + 30] = ((block2 >> 6) & 63) as i64;
      values[values_offset + 31] = (block2 & 63) as i64;

      values_offset += 32;
    }
  }

  /// Decodes blocks of type `u8` into `u64` values.
  fn decode_u8_to_i64(
    &self,
    blocks: &[u8],
    mut blocks_offset: usize,
    values: &mut [i64],
    mut values_offset: usize,
    iterations: i32,
  ) {
    for _ in 0..iterations {
      let byte0 = blocks[blocks_offset] as u64;
      blocks_offset += 1;
      values[values_offset] = (byte0 >> 2) as i64;

      let byte1 = blocks[blocks_offset] as u64;
      blocks_offset += 1;
      values[values_offset + 1] = (((byte0 & 3) << 4) | (byte1 >> 4)) as i64;

      let byte2 = blocks[blocks_offset] as u64;
      blocks_offset += 1;
      values[values_offset + 2] = (((byte1 & 15) << 2) | (byte2 >> 6)) as i64;

      values[values_offset + 3] = (byte2 & 63) as i64;

      values_offset += 4;
    }
  }
  /// Decodes blocks of type `u64` into `i32` values.
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

      values[values_offset] = (block0 >> 58) as i32;
      values[values_offset + 1] = ((block0 >> 52) & 63) as i32;
      values[values_offset + 2] = ((block0 >> 46) & 63) as i32;
      values[values_offset + 3] = ((block0 >> 40) & 63) as i32;
      values[values_offset + 4] = ((block0 >> 34) & 63) as i32;
      values[values_offset + 5] = ((block0 >> 28) & 63) as i32;
      values[values_offset + 6] = ((block0 >> 22) & 63) as i32;
      values[values_offset + 7] = ((block0 >> 16) & 63) as i32;
      values[values_offset + 8] = ((block0 >> 10) & 63) as i32;
      values[values_offset + 9] = ((block0 >> 4) & 63) as i32;

      let block1 = blocks[blocks_offset];
      blocks_offset += 1;

      values[values_offset + 10] = (((block0 & 15) << 2) | (block1 >> 62)) as i32;
      values[values_offset + 11] = ((block1 >> 56) & 63) as i32;
      values[values_offset + 12] = ((block1 >> 50) & 63) as i32;
      values[values_offset + 13] = ((block1 >> 44) & 63) as i32;
      values[values_offset + 14] = ((block1 >> 38) & 63) as i32;
      values[values_offset + 15] = ((block1 >> 32) & 63) as i32;
      values[values_offset + 16] = ((block1 >> 26) & 63) as i32;
      values[values_offset + 17] = ((block1 >> 20) & 63) as i32;
      values[values_offset + 18] = ((block1 >> 14) & 63) as i32;
      values[values_offset + 19] = ((block1 >> 8) & 63) as i32;
      values[values_offset + 20] = ((block1 >> 2) & 63) as i32;

      let block2 = blocks[blocks_offset];
      blocks_offset += 1;

      values[values_offset + 21] = (((block1 & 3) << 4) | (block2 >> 60)) as i32;
      values[values_offset + 22] = ((block2 >> 54) & 63) as i32;
      values[values_offset + 23] = ((block2 >> 48) & 63) as i32;
      values[values_offset + 24] = ((block2 >> 42) & 63) as i32;
      values[values_offset + 25] = ((block2 >> 36) & 63) as i32;
      values[values_offset + 26] = ((block2 >> 30) & 63) as i32;
      values[values_offset + 27] = ((block2 >> 24) & 63) as i32;
      values[values_offset + 28] = ((block2 >> 18) & 63) as i32;
      values[values_offset + 29] = ((block2 >> 12) & 63) as i32;
      values[values_offset + 30] = ((block2 >> 6) & 63) as i32;
      values[values_offset + 31] = (block2 & 63) as i32;

      values_offset += 32;
    }
  }

  /// Decodes blocks of type `u8` into `i32` values.
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

      values[values_offset] = byte0 >> 2;

      let byte1 = blocks[blocks_offset] as i32;
      blocks_offset += 1;

      values[values_offset + 1] = ((byte0 & 3) << 4) | (byte1 >> 4);

      let byte2 = blocks[blocks_offset] as i32;
      blocks_offset += 1;

      values[values_offset + 2] = ((byte1 & 15) << 2) | (byte2 >> 6);
      values[values_offset + 3] = byte2 & 63;

      values_offset += 4;
    }
  }
}
impl_bulk_operation_packed_encoder!(BulkOperationPacked6);
impl BulkOperation for BulkOperationPacked6 {}
