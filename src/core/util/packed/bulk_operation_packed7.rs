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

define_bulk_operation_packed_specialized!(BulkOperationPacked7, 7);
impl Decoder for BulkOperationPacked7 {
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

        $output[$vi] = (block0 >> 57) as i64;
        $output[$vi + 1] = ((block0 >> 50) & 127) as i64;
        $output[$vi + 2] = ((block0 >> 43) & 127) as i64;
        $output[$vi + 3] = ((block0 >> 36) & 127) as i64;
        $output[$vi + 4] = ((block0 >> 29) & 127) as i64;
        $output[$vi + 5] = ((block0 >> 22) & 127) as i64;
        $output[$vi + 6] = ((block0 >> 15) & 127) as i64;
        $output[$vi + 7] = ((block0 >> 8) & 127) as i64;
        $output[$vi + 8] = ((block0 >> 1) & 127) as i64;

        let block1 = $input[$bi];
        $bi += 1;

        $output[$vi + 9] = (((block0 & 1) << 6) | (block1 >> 58)) as i64;
        $output[$vi + 10] = ((block1 >> 51) & 127) as i64;
        $output[$vi + 11] = ((block1 >> 44) & 127) as i64;
        $output[$vi + 12] = ((block1 >> 37) & 127) as i64;
        $output[$vi + 13] = ((block1 >> 30) & 127) as i64;
        $output[$vi + 14] = ((block1 >> 23) & 127) as i64;
        $output[$vi + 15] = ((block1 >> 16) & 127) as i64;
        $output[$vi + 16] = ((block1 >> 9) & 127) as i64;
        $output[$vi + 17] = ((block1 >> 2) & 127) as i64;

        let block2 = $input[$bi];
        $bi += 1;

        $output[$vi + 18] = (((block1 & 3) << 5) | (block2 >> 59)) as i64;
        $output[$vi + 19] = ((block2 >> 52) & 127) as i64;
        $output[$vi + 20] = ((block2 >> 45) & 127) as i64;
        $output[$vi + 21] = ((block2 >> 38) & 127) as i64;
        $output[$vi + 22] = ((block2 >> 31) & 127) as i64;
        $output[$vi + 23] = ((block2 >> 24) & 127) as i64;
        $output[$vi + 24] = ((block2 >> 17) & 127) as i64;
        $output[$vi + 25] = ((block2 >> 10) & 127) as i64;
        $output[$vi + 26] = ((block2 >> 3) & 127) as i64;

        let block3 = $input[$bi];
        $bi += 1;

        $output[$vi + 27] = (((block2 & 7) << 4) | (block3 >> 60)) as i64;
        $output[$vi + 28] = ((block3 >> 53) & 127) as i64;
        $output[$vi + 29] = ((block3 >> 46) & 127) as i64;
        $output[$vi + 30] = ((block3 >> 39) & 127) as i64;
        $output[$vi + 31] = ((block3 >> 32) & 127) as i64;
        $output[$vi + 32] = ((block3 >> 25) & 127) as i64;
        $output[$vi + 33] = ((block3 >> 18) & 127) as i64;
        $output[$vi + 34] = ((block3 >> 11) & 127) as i64;
        $output[$vi + 35] = ((block3 >> 4) & 127) as i64;

        let block4 = $input[$bi];
        $bi += 1;

        $output[$vi + 36] = (((block3 & 15) << 3) | (block4 >> 61)) as i64;
        $output[$vi + 37] = ((block4 >> 54) & 127) as i64;
        $output[$vi + 38] = ((block4 >> 47) & 127) as i64;
        $output[$vi + 39] = ((block4 >> 40) & 127) as i64;
        $output[$vi + 40] = ((block4 >> 33) & 127) as i64;
        $output[$vi + 41] = ((block4 >> 26) & 127) as i64;
        $output[$vi + 42] = ((block4 >> 19) & 127) as i64;
        $output[$vi + 43] = ((block4 >> 12) & 127) as i64;
        $output[$vi + 44] = ((block4 >> 5) & 127) as i64;

        let block5 = $input[$bi];
        $bi += 1;

        $output[$vi + 45] = (((block4 & 31) << 2) | (block5 >> 62)) as i64;
        $output[$vi + 46] = ((block5 >> 55) & 127) as i64;
        $output[$vi + 47] = ((block5 >> 48) & 127) as i64;
        $output[$vi + 48] = ((block5 >> 41) & 127) as i64;
        $output[$vi + 49] = ((block5 >> 34) & 127) as i64;
        $output[$vi + 50] = ((block5 >> 27) & 127) as i64;
        $output[$vi + 51] = ((block5 >> 20) & 127) as i64;
        $output[$vi + 52] = ((block5 >> 13) & 127) as i64;
        $output[$vi + 53] = ((block5 >> 6) & 127) as i64;

        let block6 = $input[$bi];
        $bi += 1;

        $output[$vi + 54] = (((block5 & 63) << 1) | (block6 >> 63)) as i64;
        $output[$vi + 55] = ((block6 >> 56) & 127) as i64;
        $output[$vi + 56] = ((block6 >> 49) & 127) as i64;
        $output[$vi + 57] = ((block6 >> 42) & 127) as i64;
        $output[$vi + 58] = ((block6 >> 35) & 127) as i64;
        $output[$vi + 59] = ((block6 >> 28) & 127) as i64;
        $output[$vi + 60] = ((block6 >> 21) & 127) as i64;
        $output[$vi + 61] = ((block6 >> 14) & 127) as i64;
        $output[$vi + 62] = ((block6 >> 7) & 127) as i64;
        $output[$vi + 63] = (block6 & 127) as i64;

        $vi += 64;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(7)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(7).zip(values.chunks_exact_mut(64)) {
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
  #[allow(clippy::collapsible_if, clippy::chunks_exact_to_as_chunks)]
  fn decode_u8_to_i64(
    &self,
    blocks: &[u8],
    mut blocks_offset: usize,
    values: &mut [i64],
    mut values_offset: usize,
    iterations: usize,
  ) {
    // Java's generated per-group read/write sequence is shared by the
    // complete-range fast path and the exact short-buffer fallback.
    macro_rules! decode_group {
      ($input:ident, $bi:ident, $output:ident, $vi:ident) => {{
        let byte0 = $input[$bi] as u64;
        $bi += 1;

        $output[$vi] = (byte0 >> 1) as i64;

        let byte1 = $input[$bi] as u64;
        $bi += 1;

        $output[$vi + 1] = (((byte0 & 1) << 6) | (byte1 >> 2)) as i64;

        let byte2 = $input[$bi] as u64;
        $bi += 1;

        $output[$vi + 2] = (((byte1 & 3) << 5) | (byte2 >> 3)) as i64;

        let byte3 = $input[$bi] as u64;
        $bi += 1;

        $output[$vi + 3] = (((byte2 & 7) << 4) | (byte3 >> 4)) as i64;

        let byte4 = $input[$bi] as u64;
        $bi += 1;

        $output[$vi + 4] = (((byte3 & 15) << 3) | (byte4 >> 5)) as i64;

        let byte5 = $input[$bi] as u64;
        $bi += 1;

        $output[$vi + 5] = (((byte4 & 31) << 2) | (byte5 >> 6)) as i64;

        let byte6 = $input[$bi] as u64;
        $bi += 1;

        $output[$vi + 6] = (((byte5 & 63) << 1) | (byte6 >> 7)) as i64;
        $output[$vi + 7] = (byte6 & 127) as i64;

        $vi += 8;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(7)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(7).zip(values.chunks_exact_mut(8)) {
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

        $output[$vi] = (block0 >> 57) as i32;
        $output[$vi + 1] = ((block0 >> 50) & 127) as i32;
        $output[$vi + 2] = ((block0 >> 43) & 127) as i32;
        $output[$vi + 3] = ((block0 >> 36) & 127) as i32;
        $output[$vi + 4] = ((block0 >> 29) & 127) as i32;
        $output[$vi + 5] = ((block0 >> 22) & 127) as i32;
        $output[$vi + 6] = ((block0 >> 15) & 127) as i32;
        $output[$vi + 7] = ((block0 >> 8) & 127) as i32;
        $output[$vi + 8] = ((block0 >> 1) & 127) as i32;

        let block1 = $input[$bi];
        $bi += 1;

        $output[$vi + 9] = (((block0 & 1) << 6) | (block1 >> 58)) as i32;
        $output[$vi + 10] = ((block1 >> 51) & 127) as i32;
        $output[$vi + 11] = ((block1 >> 44) & 127) as i32;
        $output[$vi + 12] = ((block1 >> 37) & 127) as i32;
        $output[$vi + 13] = ((block1 >> 30) & 127) as i32;
        $output[$vi + 14] = ((block1 >> 23) & 127) as i32;
        $output[$vi + 15] = ((block1 >> 16) & 127) as i32;
        $output[$vi + 16] = ((block1 >> 9) & 127) as i32;
        $output[$vi + 17] = ((block1 >> 2) & 127) as i32;

        let block2 = $input[$bi];
        $bi += 1;

        $output[$vi + 18] = (((block1 & 3) << 5) | (block2 >> 59)) as i32;
        $output[$vi + 19] = ((block2 >> 52) & 127) as i32;
        $output[$vi + 20] = ((block2 >> 45) & 127) as i32;
        $output[$vi + 21] = ((block2 >> 38) & 127) as i32;
        $output[$vi + 22] = ((block2 >> 31) & 127) as i32;
        $output[$vi + 23] = ((block2 >> 24) & 127) as i32;
        $output[$vi + 24] = ((block2 >> 17) & 127) as i32;
        $output[$vi + 25] = ((block2 >> 10) & 127) as i32;
        $output[$vi + 26] = ((block2 >> 3) & 127) as i32;

        let block3 = $input[$bi];
        $bi += 1;

        $output[$vi + 27] = (((block2 & 7) << 4) | (block3 >> 60)) as i32;
        $output[$vi + 28] = ((block3 >> 53) & 127) as i32;
        $output[$vi + 29] = ((block3 >> 46) & 127) as i32;
        $output[$vi + 30] = ((block3 >> 39) & 127) as i32;
        $output[$vi + 31] = ((block3 >> 32) & 127) as i32;
        $output[$vi + 32] = ((block3 >> 25) & 127) as i32;
        $output[$vi + 33] = ((block3 >> 18) & 127) as i32;
        $output[$vi + 34] = ((block3 >> 11) & 127) as i32;
        $output[$vi + 35] = ((block3 >> 4) & 127) as i32;

        let block4 = $input[$bi];
        $bi += 1;

        $output[$vi + 36] = (((block3 & 15) << 3) | (block4 >> 61)) as i32;
        $output[$vi + 37] = ((block4 >> 54) & 127) as i32;
        $output[$vi + 38] = ((block4 >> 47) & 127) as i32;
        $output[$vi + 39] = ((block4 >> 40) & 127) as i32;
        $output[$vi + 40] = ((block4 >> 33) & 127) as i32;
        $output[$vi + 41] = ((block4 >> 26) & 127) as i32;
        $output[$vi + 42] = ((block4 >> 19) & 127) as i32;
        $output[$vi + 43] = ((block4 >> 12) & 127) as i32;
        $output[$vi + 44] = ((block4 >> 5) & 127) as i32;

        let block5 = $input[$bi];
        $bi += 1;

        $output[$vi + 45] = (((block4 & 31) << 2) | (block5 >> 62)) as i32;
        $output[$vi + 46] = ((block5 >> 55) & 127) as i32;
        $output[$vi + 47] = ((block5 >> 48) & 127) as i32;
        $output[$vi + 48] = ((block5 >> 41) & 127) as i32;
        $output[$vi + 49] = ((block5 >> 34) & 127) as i32;
        $output[$vi + 50] = ((block5 >> 27) & 127) as i32;
        $output[$vi + 51] = ((block5 >> 20) & 127) as i32;
        $output[$vi + 52] = ((block5 >> 13) & 127) as i32;
        $output[$vi + 53] = ((block5 >> 6) & 127) as i32;

        let block6 = $input[$bi];
        $bi += 1;

        $output[$vi + 54] = (((block5 & 63) << 1) | (block6 >> 63)) as i32;
        $output[$vi + 55] = ((block6 >> 56) & 127) as i32;
        $output[$vi + 56] = ((block6 >> 49) & 127) as i32;
        $output[$vi + 57] = ((block6 >> 42) & 127) as i32;
        $output[$vi + 58] = ((block6 >> 35) & 127) as i32;
        $output[$vi + 59] = ((block6 >> 28) & 127) as i32;
        $output[$vi + 60] = ((block6 >> 21) & 127) as i32;
        $output[$vi + 61] = ((block6 >> 14) & 127) as i32;
        $output[$vi + 62] = ((block6 >> 7) & 127) as i32;
        $output[$vi + 63] = (block6 & 127) as i32;

        $vi += 64;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(7)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(7).zip(values.chunks_exact_mut(64)) {
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
  #[allow(clippy::collapsible_if, clippy::chunks_exact_to_as_chunks)]
  fn decode_u8_to_i32(
    &self,
    blocks: &[u8],
    mut blocks_offset: usize,
    values: &mut [i32],
    mut values_offset: usize,
    iterations: usize,
  ) -> Result<()> {
    // Java's generated per-group read/write sequence is shared by the
    // complete-range fast path and the exact short-buffer fallback.
    macro_rules! decode_group {
      ($input:ident, $bi:ident, $output:ident, $vi:ident) => {{
        let byte0 = $input[$bi] as i32;
        $bi += 1;

        $output[$vi] = byte0 >> 1;

        let byte1 = $input[$bi] as i32;
        $bi += 1;

        $output[$vi + 1] = ((byte0 & 1) << 6) | (byte1 >> 2);

        let byte2 = $input[$bi] as i32;
        $bi += 1;

        $output[$vi + 2] = ((byte1 & 3) << 5) | (byte2 >> 3);

        let byte3 = $input[$bi] as i32;
        $bi += 1;

        $output[$vi + 3] = ((byte2 & 7) << 4) | (byte3 >> 4);

        let byte4 = $input[$bi] as i32;
        $bi += 1;

        $output[$vi + 4] = ((byte3 & 15) << 3) | (byte4 >> 5);

        let byte5 = $input[$bi] as i32;
        $bi += 1;

        $output[$vi + 5] = ((byte4 & 31) << 2) | (byte5 >> 6);

        let byte6 = $input[$bi] as i32;
        $bi += 1;

        $output[$vi + 6] = ((byte5 & 63) << 1) | (byte6 >> 7);
        $output[$vi + 7] = byte6 & 127;

        $vi += 8;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(7)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(7).zip(values.chunks_exact_mut(8)) {
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
}
impl_bulk_operation_packed_encoder!(BulkOperationPacked7);
impl BulkOperation for BulkOperationPacked7 {}
