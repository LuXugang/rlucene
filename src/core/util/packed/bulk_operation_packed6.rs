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

define_bulk_operation_packed_specialized!(BulkOperationPacked6, 6);
impl Decoder for BulkOperationPacked6 {
  delegate_bulk_operation_packed_decoder_counts!();
  /// Decodes blocks of type `u64` into `u64` values.
  #[allow(clippy::collapsible_if, clippy::chunks_exact_to_as_chunks)]
  fn decode_u64_to_i64(
    &self,
    blocks: &[u64],
    mut blocks_offset: usize,
    values: &mut [i64],
    mut values_offset: usize,
    iterations: usize,
  ) {
    // Java's generated per-group read/write sequence is shared by the
    // complete-range fast path and the exact short-buffer fallback.
    macro_rules! decode_group {
      ($input:ident, $bi:ident, $output:ident, $vi:ident) => {{
        let block0 = $input[$bi];
        $bi += 1;

        $output[$vi] = (block0 >> 58) as i64;
        $output[$vi + 1] = ((block0 >> 52) & 63) as i64;
        $output[$vi + 2] = ((block0 >> 46) & 63) as i64;
        $output[$vi + 3] = ((block0 >> 40) & 63) as i64;
        $output[$vi + 4] = ((block0 >> 34) & 63) as i64;
        $output[$vi + 5] = ((block0 >> 28) & 63) as i64;
        $output[$vi + 6] = ((block0 >> 22) & 63) as i64;
        $output[$vi + 7] = ((block0 >> 16) & 63) as i64;
        $output[$vi + 8] = ((block0 >> 10) & 63) as i64;
        $output[$vi + 9] = ((block0 >> 4) & 63) as i64;

        let block1 = $input[$bi];
        $bi += 1;

        $output[$vi + 10] = (((block0 & 15) << 2) | (block1 >> 62)) as i64;
        $output[$vi + 11] = ((block1 >> 56) & 63) as i64;
        $output[$vi + 12] = ((block1 >> 50) & 63) as i64;
        $output[$vi + 13] = ((block1 >> 44) & 63) as i64;
        $output[$vi + 14] = ((block1 >> 38) & 63) as i64;
        $output[$vi + 15] = ((block1 >> 32) & 63) as i64;
        $output[$vi + 16] = ((block1 >> 26) & 63) as i64;
        $output[$vi + 17] = ((block1 >> 20) & 63) as i64;
        $output[$vi + 18] = ((block1 >> 14) & 63) as i64;
        $output[$vi + 19] = ((block1 >> 8) & 63) as i64;
        $output[$vi + 20] = ((block1 >> 2) & 63) as i64;

        let block2 = $input[$bi];
        $bi += 1;

        $output[$vi + 21] = (((block1 & 3) << 4) | (block2 >> 60)) as i64;
        $output[$vi + 22] = ((block2 >> 54) & 63) as i64;
        $output[$vi + 23] = ((block2 >> 48) & 63) as i64;
        $output[$vi + 24] = ((block2 >> 42) & 63) as i64;
        $output[$vi + 25] = ((block2 >> 36) & 63) as i64;
        $output[$vi + 26] = ((block2 >> 30) & 63) as i64;
        $output[$vi + 27] = ((block2 >> 24) & 63) as i64;
        $output[$vi + 28] = ((block2 >> 18) & 63) as i64;
        $output[$vi + 29] = ((block2 >> 12) & 63) as i64;
        $output[$vi + 30] = ((block2 >> 6) & 63) as i64;
        $output[$vi + 31] = (block2 & 63) as i64;

        $vi += 32;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(3)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(32)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(3).zip(values.chunks_exact_mut(32)) {
          let mut blocks_offset = 0;
          let mut values_offset = 0;
          decode_group!(blocks, blocks_offset, values, values_offset);
          let _ = (blocks_offset, values_offset);
        }
        return;
      }
    }

    for _ in 0..iterations {
      decode_group!(blocks, blocks_offset, values, values_offset);
    }
  }

  /// Decodes blocks of type `u8` into `u64` values.
  fn decode_u8_to_i64(
    &self,
    blocks: &[u8],
    mut blocks_offset: usize,
    values: &mut [i64],
    mut values_offset: usize,
    iterations: usize,
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
  #[allow(clippy::collapsible_if, clippy::chunks_exact_to_as_chunks)]
  fn decode_u64_to_i32(
    &self,
    blocks: &[u64],
    mut blocks_offset: usize,
    values: &mut [i32],
    mut values_offset: usize,
    iterations: usize,
  ) -> Result<()> {
    // Java's generated per-group read/write sequence is shared by the
    // complete-range fast path and the exact short-buffer fallback.
    macro_rules! decode_group {
      ($input:ident, $bi:ident, $output:ident, $vi:ident) => {{
        let block0 = $input[$bi];
        $bi += 1;

        $output[$vi] = (block0 >> 58) as i32;
        $output[$vi + 1] = ((block0 >> 52) & 63) as i32;
        $output[$vi + 2] = ((block0 >> 46) & 63) as i32;
        $output[$vi + 3] = ((block0 >> 40) & 63) as i32;
        $output[$vi + 4] = ((block0 >> 34) & 63) as i32;
        $output[$vi + 5] = ((block0 >> 28) & 63) as i32;
        $output[$vi + 6] = ((block0 >> 22) & 63) as i32;
        $output[$vi + 7] = ((block0 >> 16) & 63) as i32;
        $output[$vi + 8] = ((block0 >> 10) & 63) as i32;
        $output[$vi + 9] = ((block0 >> 4) & 63) as i32;

        let block1 = $input[$bi];
        $bi += 1;

        $output[$vi + 10] = (((block0 & 15) << 2) | (block1 >> 62)) as i32;
        $output[$vi + 11] = ((block1 >> 56) & 63) as i32;
        $output[$vi + 12] = ((block1 >> 50) & 63) as i32;
        $output[$vi + 13] = ((block1 >> 44) & 63) as i32;
        $output[$vi + 14] = ((block1 >> 38) & 63) as i32;
        $output[$vi + 15] = ((block1 >> 32) & 63) as i32;
        $output[$vi + 16] = ((block1 >> 26) & 63) as i32;
        $output[$vi + 17] = ((block1 >> 20) & 63) as i32;
        $output[$vi + 18] = ((block1 >> 14) & 63) as i32;
        $output[$vi + 19] = ((block1 >> 8) & 63) as i32;
        $output[$vi + 20] = ((block1 >> 2) & 63) as i32;

        let block2 = $input[$bi];
        $bi += 1;

        $output[$vi + 21] = (((block1 & 3) << 4) | (block2 >> 60)) as i32;
        $output[$vi + 22] = ((block2 >> 54) & 63) as i32;
        $output[$vi + 23] = ((block2 >> 48) & 63) as i32;
        $output[$vi + 24] = ((block2 >> 42) & 63) as i32;
        $output[$vi + 25] = ((block2 >> 36) & 63) as i32;
        $output[$vi + 26] = ((block2 >> 30) & 63) as i32;
        $output[$vi + 27] = ((block2 >> 24) & 63) as i32;
        $output[$vi + 28] = ((block2 >> 18) & 63) as i32;
        $output[$vi + 29] = ((block2 >> 12) & 63) as i32;
        $output[$vi + 30] = ((block2 >> 6) & 63) as i32;
        $output[$vi + 31] = (block2 & 63) as i32;

        $vi += 32;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(3)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(32)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(3).zip(values.chunks_exact_mut(32)) {
          let mut blocks_offset = 0;
          let mut values_offset = 0;
          decode_group!(blocks, blocks_offset, values, values_offset);
          let _ = (blocks_offset, values_offset);
        }
        return Ok(());
      }
    }

    for _ in 0..iterations {
      decode_group!(blocks, blocks_offset, values, values_offset);
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
    iterations: usize,
  ) -> Result<()> {
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
    Ok(())
  }
}
impl_bulk_operation_packed_encoder!(BulkOperationPacked6);
impl BulkOperation for BulkOperationPacked6 {}
