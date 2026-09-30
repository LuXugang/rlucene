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

define_bulk_operation_packed_specialized!(BulkOperationPacked3, 3);
impl Decoder for BulkOperationPacked3 {
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

        $output[$vi] = ((block0 >> 61) & 7) as i64;
        $output[$vi + 1] = ((block0 >> 58) & 7) as i64;
        $output[$vi + 2] = ((block0 >> 55) & 7) as i64;
        $output[$vi + 3] = ((block0 >> 52) & 7) as i64;
        $output[$vi + 4] = ((block0 >> 49) & 7) as i64;
        $output[$vi + 5] = ((block0 >> 46) & 7) as i64;
        $output[$vi + 6] = ((block0 >> 43) & 7) as i64;
        $output[$vi + 7] = ((block0 >> 40) & 7) as i64;
        $output[$vi + 8] = ((block0 >> 37) & 7) as i64;
        $output[$vi + 9] = ((block0 >> 34) & 7) as i64;
        $output[$vi + 10] = ((block0 >> 31) & 7) as i64;
        $output[$vi + 11] = ((block0 >> 28) & 7) as i64;
        $output[$vi + 12] = ((block0 >> 25) & 7) as i64;
        $output[$vi + 13] = ((block0 >> 22) & 7) as i64;
        $output[$vi + 14] = ((block0 >> 19) & 7) as i64;
        $output[$vi + 15] = ((block0 >> 16) & 7) as i64;
        $output[$vi + 16] = ((block0 >> 13) & 7) as i64;
        $output[$vi + 17] = ((block0 >> 10) & 7) as i64;
        $output[$vi + 18] = ((block0 >> 7) & 7) as i64;
        $output[$vi + 19] = ((block0 >> 4) & 7) as i64;
        $output[$vi + 20] = ((block0 >> 1) & 7) as i64;

        let block1 = $input[$bi];
        $bi += 1;

        $output[$vi + 21] = (((block0 & 1) << 2) | (block1 >> 62)) as i64;
        $output[$vi + 22] = ((block1 >> 59) & 7) as i64;
        $output[$vi + 23] = ((block1 >> 56) & 7) as i64;
        $output[$vi + 24] = ((block1 >> 53) & 7) as i64;
        $output[$vi + 25] = ((block1 >> 50) & 7) as i64;
        $output[$vi + 26] = ((block1 >> 47) & 7) as i64;
        $output[$vi + 27] = ((block1 >> 44) & 7) as i64;
        $output[$vi + 28] = ((block1 >> 41) & 7) as i64;
        $output[$vi + 29] = ((block1 >> 38) & 7) as i64;
        $output[$vi + 30] = ((block1 >> 35) & 7) as i64;
        $output[$vi + 31] = ((block1 >> 32) & 7) as i64;
        $output[$vi + 32] = ((block1 >> 29) & 7) as i64;
        $output[$vi + 33] = ((block1 >> 26) & 7) as i64;
        $output[$vi + 34] = ((block1 >> 23) & 7) as i64;
        $output[$vi + 35] = ((block1 >> 20) & 7) as i64;
        $output[$vi + 36] = ((block1 >> 17) & 7) as i64;
        $output[$vi + 37] = ((block1 >> 14) & 7) as i64;
        $output[$vi + 38] = ((block1 >> 11) & 7) as i64;
        $output[$vi + 39] = ((block1 >> 8) & 7) as i64;
        $output[$vi + 40] = ((block1 >> 5) & 7) as i64;
        $output[$vi + 41] = ((block1 >> 2) & 7) as i64;

        let block2 = $input[$bi];
        $bi += 1;

        $output[$vi + 42] = (((block1 & 3) << 1) | (block2 >> 63)) as i64;
        $output[$vi + 43] = ((block2 >> 60) & 7) as i64;
        $output[$vi + 44] = ((block2 >> 57) & 7) as i64;
        $output[$vi + 45] = ((block2 >> 54) & 7) as i64;
        $output[$vi + 46] = ((block2 >> 51) & 7) as i64;
        $output[$vi + 47] = ((block2 >> 48) & 7) as i64;
        $output[$vi + 48] = ((block2 >> 45) & 7) as i64;
        $output[$vi + 49] = ((block2 >> 42) & 7) as i64;
        $output[$vi + 50] = ((block2 >> 39) & 7) as i64;
        $output[$vi + 51] = ((block2 >> 36) & 7) as i64;
        $output[$vi + 52] = ((block2 >> 33) & 7) as i64;
        $output[$vi + 53] = ((block2 >> 30) & 7) as i64;
        $output[$vi + 54] = ((block2 >> 27) & 7) as i64;
        $output[$vi + 55] = ((block2 >> 24) & 7) as i64;
        $output[$vi + 56] = ((block2 >> 21) & 7) as i64;
        $output[$vi + 57] = ((block2 >> 18) & 7) as i64;
        $output[$vi + 58] = ((block2 >> 15) & 7) as i64;
        $output[$vi + 59] = ((block2 >> 12) & 7) as i64;
        $output[$vi + 60] = ((block2 >> 9) & 7) as i64;
        $output[$vi + 61] = ((block2 >> 6) & 7) as i64;
        $output[$vi + 62] = ((block2 >> 3) & 7) as i64;
        $output[$vi + 63] = (block2 & 7) as i64;

        $vi += 64;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(3)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(3).zip(values.chunks_exact_mut(64)) {
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
      values[values_offset] = (byte0 >> 5) as i64;
      values[values_offset + 1] = ((byte0 >> 2) & 7) as i64;
      let byte1 = blocks[blocks_offset] as u64;
      blocks_offset += 1;
      values[values_offset + 2] = (((byte0 & 3) << 1) | (byte1 >> 7)) as i64;
      values[values_offset + 3] = ((byte1 >> 4) & 7) as i64;
      values[values_offset + 4] = ((byte1 >> 1) & 7) as i64;
      let byte2 = blocks[blocks_offset] as u64;
      blocks_offset += 1;
      values[values_offset + 5] = (((byte1 & 1) << 2) | (byte2 >> 6)) as i64;
      values[values_offset + 6] = ((byte2 >> 3) & 7) as i64;
      values[values_offset + 7] = (byte2 & 7) as i64;
      values_offset += 8;
    }
  }
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

        $output[$vi] = (block0 >> 61) as i32;
        $output[$vi + 1] = ((block0 >> 58) & 7) as i32;
        $output[$vi + 2] = ((block0 >> 55) & 7) as i32;
        $output[$vi + 3] = ((block0 >> 52) & 7) as i32;
        $output[$vi + 4] = ((block0 >> 49) & 7) as i32;
        $output[$vi + 5] = ((block0 >> 46) & 7) as i32;
        $output[$vi + 6] = ((block0 >> 43) & 7) as i32;
        $output[$vi + 7] = ((block0 >> 40) & 7) as i32;
        $output[$vi + 8] = ((block0 >> 37) & 7) as i32;
        $output[$vi + 9] = ((block0 >> 34) & 7) as i32;
        $output[$vi + 10] = ((block0 >> 31) & 7) as i32;
        $output[$vi + 11] = ((block0 >> 28) & 7) as i32;
        $output[$vi + 12] = ((block0 >> 25) & 7) as i32;
        $output[$vi + 13] = ((block0 >> 22) & 7) as i32;
        $output[$vi + 14] = ((block0 >> 19) & 7) as i32;
        $output[$vi + 15] = ((block0 >> 16) & 7) as i32;
        $output[$vi + 16] = ((block0 >> 13) & 7) as i32;
        $output[$vi + 17] = ((block0 >> 10) & 7) as i32;
        $output[$vi + 18] = ((block0 >> 7) & 7) as i32;
        $output[$vi + 19] = ((block0 >> 4) & 7) as i32;
        $output[$vi + 20] = ((block0 >> 1) & 7) as i32;

        let block1 = $input[$bi];
        $bi += 1;

        $output[$vi + 21] = (((block0 & 1) << 2) | (block1 >> 62)) as i32;
        $output[$vi + 22] = ((block1 >> 59) & 7) as i32;
        $output[$vi + 23] = ((block1 >> 56) & 7) as i32;
        $output[$vi + 24] = ((block1 >> 53) & 7) as i32;
        $output[$vi + 25] = ((block1 >> 50) & 7) as i32;
        $output[$vi + 26] = ((block1 >> 47) & 7) as i32;
        $output[$vi + 27] = ((block1 >> 44) & 7) as i32;
        $output[$vi + 28] = ((block1 >> 41) & 7) as i32;
        $output[$vi + 29] = ((block1 >> 38) & 7) as i32;
        $output[$vi + 30] = ((block1 >> 35) & 7) as i32;
        $output[$vi + 31] = ((block1 >> 32) & 7) as i32;
        $output[$vi + 32] = ((block1 >> 29) & 7) as i32;
        $output[$vi + 33] = ((block1 >> 26) & 7) as i32;
        $output[$vi + 34] = ((block1 >> 23) & 7) as i32;
        $output[$vi + 35] = ((block1 >> 20) & 7) as i32;
        $output[$vi + 36] = ((block1 >> 17) & 7) as i32;
        $output[$vi + 37] = ((block1 >> 14) & 7) as i32;
        $output[$vi + 38] = ((block1 >> 11) & 7) as i32;
        $output[$vi + 39] = ((block1 >> 8) & 7) as i32;
        $output[$vi + 40] = ((block1 >> 5) & 7) as i32;
        $output[$vi + 41] = ((block1 >> 2) & 7) as i32;

        let block2 = $input[$bi];
        $bi += 1;

        $output[$vi + 42] = (((block1 & 3) << 1) | (block2 >> 63)) as i32;
        $output[$vi + 43] = ((block2 >> 60) & 7) as i32;
        $output[$vi + 44] = ((block2 >> 57) & 7) as i32;
        $output[$vi + 45] = ((block2 >> 54) & 7) as i32;
        $output[$vi + 46] = ((block2 >> 51) & 7) as i32;
        $output[$vi + 47] = ((block2 >> 48) & 7) as i32;
        $output[$vi + 48] = ((block2 >> 45) & 7) as i32;
        $output[$vi + 49] = ((block2 >> 42) & 7) as i32;
        $output[$vi + 50] = ((block2 >> 39) & 7) as i32;
        $output[$vi + 51] = ((block2 >> 36) & 7) as i32;
        $output[$vi + 52] = ((block2 >> 33) & 7) as i32;
        $output[$vi + 53] = ((block2 >> 30) & 7) as i32;
        $output[$vi + 54] = ((block2 >> 27) & 7) as i32;
        $output[$vi + 55] = ((block2 >> 24) & 7) as i32;
        $output[$vi + 56] = ((block2 >> 21) & 7) as i32;
        $output[$vi + 57] = ((block2 >> 18) & 7) as i32;
        $output[$vi + 58] = ((block2 >> 15) & 7) as i32;
        $output[$vi + 59] = ((block2 >> 12) & 7) as i32;
        $output[$vi + 60] = ((block2 >> 9) & 7) as i32;
        $output[$vi + 61] = ((block2 >> 6) & 7) as i32;
        $output[$vi + 62] = ((block2 >> 3) & 7) as i32;
        $output[$vi + 63] = (block2 & 7) as i32;

        $vi += 64;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(3)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(3).zip(values.chunks_exact_mut(64)) {
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
      values[values_offset] = byte0 >> 5;
      values[values_offset + 1] = (byte0 >> 2) & 7;
      let byte1 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset + 2] = ((byte0 & 3) << 1) | (byte1 >> 7);
      values[values_offset + 3] = (byte1 >> 4) & 7;
      values[values_offset + 4] = (byte1 >> 1) & 7;
      let byte2 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset + 5] = ((byte1 & 1) << 2) | (byte2 >> 6);
      values[values_offset + 6] = (byte2 >> 3) & 7;
      values[values_offset + 7] = byte2 & 7;
      values_offset += 8;
    }
    Ok(())
  }
}
impl_bulk_operation_packed_encoder!(BulkOperationPacked3);
impl BulkOperation for BulkOperationPacked3 {}
