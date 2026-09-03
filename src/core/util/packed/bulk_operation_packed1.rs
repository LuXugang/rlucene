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
use crate::core::util::error::lucene_error::Result;
use crate::core::util::packed::Decoder;
use crate::core::util::packed::bulk_operation::BulkOperation;
use crate::core::util::packed::bulk_operation_packed::{
  define_bulk_operation_packed_specialized, delegate_bulk_operation_packed_decoder_counts,
  impl_bulk_operation_packed_encoder,
};

define_bulk_operation_packed_specialized!(BulkOperationPacked1, 1);
impl Decoder for BulkOperationPacked1 {
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
      let block = blocks[blocks_offset];
      blocks_offset += 1;

      for shift in (0..=63).rev() {
        values[values_offset] = ((block >> shift) & 1) as i64;
        values_offset += 1;
      }
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
  fn decode_u64_to_i32(
    &self,
    blocks: &[u64],
    mut blocks_offset: usize,
    values: &mut [i32],
    mut values_offset: usize,
    iterations: i32,
  ) -> Result<()> {
    for _ in 0..iterations {
      let block = blocks[blocks_offset];
      blocks_offset += 1;

      for shift in (0..=63).rev() {
        values[values_offset] = ((block >> shift) & 1) as i32;
        values_offset += 1;
      }
    }
    Ok(())
  }

  /// Decodes blocks of type `u8` into `i32` values.
  fn decode_u8_to_i32(
    &self,
    blocks: &[u8],
    mut blocks_offset: usize,
    values: &mut [i32],
    mut values_offset: usize,
    iterations: i32,
  ) -> Result<()> {
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
    Ok(())
  }
}
impl_bulk_operation_packed_encoder!(BulkOperationPacked1);
impl BulkOperation for BulkOperationPacked1 {}
