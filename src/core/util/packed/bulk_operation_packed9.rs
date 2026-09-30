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

define_bulk_operation_packed_specialized!(BulkOperationPacked9, 9);
impl Decoder for BulkOperationPacked9 {
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

        $output[$vi] = (block0 >> 55) as i64;
        $output[$vi + 1] = ((block0 >> 46) & 511) as i64;
        $output[$vi + 2] = ((block0 >> 37) & 511) as i64;
        $output[$vi + 3] = ((block0 >> 28) & 511) as i64;
        $output[$vi + 4] = ((block0 >> 19) & 511) as i64;
        $output[$vi + 5] = ((block0 >> 10) & 511) as i64;
        $output[$vi + 6] = ((block0 >> 1) & 511) as i64;
        $vi += 7;

        let block1 = $input[$bi];
        $bi += 1;

        $output[$vi] = (((block0 & 1) << 8) | (block1 >> 56)) as i64;
        $output[$vi + 1] = ((block1 >> 47) & 511) as i64;
        $output[$vi + 2] = ((block1 >> 38) & 511) as i64;
        $output[$vi + 3] = ((block1 >> 29) & 511) as i64;
        $output[$vi + 4] = ((block1 >> 20) & 511) as i64;
        $output[$vi + 5] = ((block1 >> 11) & 511) as i64;
        $output[$vi + 6] = ((block1 >> 2) & 511) as i64;
        $vi += 7;

        let block2 = $input[$bi];
        $bi += 1;

        $output[$vi] = (((block1 & 3) << 7) | (block2 >> 57)) as i64;
        $output[$vi + 1] = ((block2 >> 48) & 511) as i64;
        $output[$vi + 2] = ((block2 >> 39) & 511) as i64;
        $output[$vi + 3] = ((block2 >> 30) & 511) as i64;
        $output[$vi + 4] = ((block2 >> 21) & 511) as i64;
        $output[$vi + 5] = ((block2 >> 12) & 511) as i64;
        $output[$vi + 6] = ((block2 >> 3) & 511) as i64;
        $vi += 7;

        let block3 = $input[$bi];
        $bi += 1;

        $output[$vi] = (((block2 & 7) << 6) | (block3 >> 58)) as i64;
        $output[$vi + 1] = ((block3 >> 49) & 511) as i64;
        $output[$vi + 2] = ((block3 >> 40) & 511) as i64;
        $output[$vi + 3] = ((block3 >> 31) & 511) as i64;
        $output[$vi + 4] = ((block3 >> 22) & 511) as i64;
        $output[$vi + 5] = ((block3 >> 13) & 511) as i64;
        $output[$vi + 6] = ((block3 >> 4) & 511) as i64;
        $vi += 7;

        let block4 = $input[$bi];
        $bi += 1;

        $output[$vi] = (((block3 & 15) << 5) | (block4 >> 59)) as i64;
        $output[$vi + 1] = ((block4 >> 50) & 511) as i64;
        $output[$vi + 2] = ((block4 >> 41) & 511) as i64;
        $output[$vi + 3] = ((block4 >> 32) & 511) as i64;
        $output[$vi + 4] = ((block4 >> 23) & 511) as i64;
        $output[$vi + 5] = ((block4 >> 14) & 511) as i64;
        $output[$vi + 6] = ((block4 >> 5) & 511) as i64;
        $vi += 7;

        let block5 = $input[$bi];
        $bi += 1;

        $output[$vi] = (((block4 & 31) << 4) | (block5 >> 60)) as i64;
        $output[$vi + 1] = ((block5 >> 51) & 511) as i64;
        $output[$vi + 2] = ((block5 >> 42) & 511) as i64;
        $output[$vi + 3] = ((block5 >> 33) & 511) as i64;
        $output[$vi + 4] = ((block5 >> 24) & 511) as i64;
        $output[$vi + 5] = ((block5 >> 15) & 511) as i64;
        $output[$vi + 6] = ((block5 >> 6) & 511) as i64;
        $vi += 7;

        let block6 = $input[$bi];
        $bi += 1;

        $output[$vi] = (((block5 & 63) << 3) | (block6 >> 61)) as i64;
        $output[$vi + 1] = ((block6 >> 52) & 511) as i64;
        $output[$vi + 2] = ((block6 >> 43) & 511) as i64;
        $output[$vi + 3] = ((block6 >> 34) & 511) as i64;
        $output[$vi + 4] = ((block6 >> 25) & 511) as i64;
        $output[$vi + 5] = ((block6 >> 16) & 511) as i64;
        $output[$vi + 6] = ((block6 >> 7) & 511) as i64;
        $vi += 7;

        let block7 = $input[$bi];
        $bi += 1;

        $output[$vi] = (((block6 & 127) << 2) | (block7 >> 62)) as i64;
        $output[$vi + 1] = ((block7 >> 53) & 511) as i64;
        $output[$vi + 2] = ((block7 >> 44) & 511) as i64;
        $output[$vi + 3] = ((block7 >> 35) & 511) as i64;
        $output[$vi + 4] = ((block7 >> 26) & 511) as i64;
        $output[$vi + 5] = ((block7 >> 17) & 511) as i64;
        $output[$vi + 6] = ((block7 >> 8) & 511) as i64;
        $vi += 7;

        let block8 = $input[$bi];
        $bi += 1;

        $output[$vi] = (((block7 & 255) << 1) | (block8 >> 63)) as i64;
        $output[$vi + 1] = ((block8 >> 54) & 511) as i64;
        $output[$vi + 2] = ((block8 >> 45) & 511) as i64;
        $output[$vi + 3] = ((block8 >> 36) & 511) as i64;
        $output[$vi + 4] = ((block8 >> 27) & 511) as i64;
        $output[$vi + 5] = ((block8 >> 18) & 511) as i64;
        $output[$vi + 6] = ((block8 >> 9) & 511) as i64;
        $output[$vi + 7] = (block8 & 511) as i64;
        $vi += 8;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(9)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(9).zip(values.chunks_exact_mut(64)) {
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
        let byte1 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = ((byte0 << 1) | (byte1 >> 7)) as i64;
        $vi += 1;

        let byte2 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte1 & 127) << 2) | (byte2 >> 6)) as i64;
        $vi += 1;

        let byte3 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte2 & 63) << 3) | (byte3 >> 5)) as i64;
        $vi += 1;

        let byte4 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte3 & 31) << 4) | (byte4 >> 4)) as i64;
        $vi += 1;

        let byte5 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte4 & 15) << 5) | (byte5 >> 3)) as i64;
        $vi += 1;

        let byte6 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte5 & 7) << 6) | (byte6 >> 2)) as i64;
        $vi += 1;

        let byte7 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte6 & 3) << 7) | (byte7 >> 1)) as i64;
        $vi += 1;

        let byte8 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte7 & 1) << 8) | byte8) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(9)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(9).zip(values.chunks_exact_mut(8)) {
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
        $output[$vi] = (block0 >> 55) as i32;
        $output[$vi + 1] = ((block0 >> 46) & 511) as i32;
        $output[$vi + 2] = ((block0 >> 37) & 511) as i32;
        $output[$vi + 3] = ((block0 >> 28) & 511) as i32;
        $output[$vi + 4] = ((block0 >> 19) & 511) as i32;
        $output[$vi + 5] = ((block0 >> 10) & 511) as i32;
        $output[$vi + 6] = ((block0 >> 1) & 511) as i32;
        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi + 7] = (((block0 & 1) << 8) | (block1 >> 56)) as i32;
        $output[$vi + 8] = ((block1 >> 47) & 511) as i32;
        $output[$vi + 9] = ((block1 >> 38) & 511) as i32;
        $output[$vi + 10] = ((block1 >> 29) & 511) as i32;
        $output[$vi + 11] = ((block1 >> 20) & 511) as i32;
        $output[$vi + 12] = ((block1 >> 11) & 511) as i32;
        $output[$vi + 13] = ((block1 >> 2) & 511) as i32;
        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi + 14] = (((block1 & 3) << 7) | (block2 >> 57)) as i32;
        $output[$vi + 15] = ((block2 >> 48) & 511) as i32;
        $output[$vi + 16] = ((block2 >> 39) & 511) as i32;
        $output[$vi + 17] = ((block2 >> 30) & 511) as i32;
        $output[$vi + 18] = ((block2 >> 21) & 511) as i32;
        $output[$vi + 19] = ((block2 >> 12) & 511) as i32;
        $output[$vi + 20] = ((block2 >> 3) & 511) as i32;
        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi + 21] = (((block2 & 7) << 6) | (block3 >> 58)) as i32;
        $output[$vi + 22] = ((block3 >> 49) & 511) as i32;
        $output[$vi + 23] = ((block3 >> 40) & 511) as i32;
        $output[$vi + 24] = ((block3 >> 31) & 511) as i32;
        $output[$vi + 25] = ((block3 >> 22) & 511) as i32;
        $output[$vi + 26] = ((block3 >> 13) & 511) as i32;
        $output[$vi + 27] = ((block3 >> 4) & 511) as i32;
        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi + 28] = (((block3 & 15) << 5) | (block4 >> 59)) as i32;
        $output[$vi + 29] = ((block4 >> 50) & 511) as i32;
        $output[$vi + 30] = ((block4 >> 41) & 511) as i32;
        $output[$vi + 31] = ((block4 >> 32) & 511) as i32;
        $output[$vi + 32] = ((block4 >> 23) & 511) as i32;
        $output[$vi + 33] = ((block4 >> 14) & 511) as i32;
        $output[$vi + 34] = ((block4 >> 5) & 511) as i32;
        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi + 35] = (((block4 & 31) << 4) | (block5 >> 60)) as i32;
        $output[$vi + 36] = ((block5 >> 51) & 511) as i32;
        $output[$vi + 37] = ((block5 >> 42) & 511) as i32;
        $output[$vi + 38] = ((block5 >> 33) & 511) as i32;
        $output[$vi + 39] = ((block5 >> 24) & 511) as i32;
        $output[$vi + 40] = ((block5 >> 15) & 511) as i32;
        $output[$vi + 41] = ((block5 >> 6) & 511) as i32;
        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi + 42] = (((block5 & 63) << 3) | (block6 >> 61)) as i32;
        $output[$vi + 43] = ((block6 >> 52) & 511) as i32;
        $output[$vi + 44] = ((block6 >> 43) & 511) as i32;
        $output[$vi + 45] = ((block6 >> 34) & 511) as i32;
        $output[$vi + 46] = ((block6 >> 25) & 511) as i32;
        $output[$vi + 47] = ((block6 >> 16) & 511) as i32;
        $output[$vi + 48] = ((block6 >> 7) & 511) as i32;
        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi + 49] = (((block6 & 127) << 2) | (block7 >> 62)) as i32;
        $output[$vi + 50] = ((block7 >> 53) & 511) as i32;
        $output[$vi + 51] = ((block7 >> 44) & 511) as i32;
        $output[$vi + 52] = ((block7 >> 35) & 511) as i32;
        $output[$vi + 53] = ((block7 >> 26) & 511) as i32;
        $output[$vi + 54] = ((block7 >> 17) & 511) as i32;
        $output[$vi + 55] = ((block7 >> 8) & 511) as i32;
        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi + 56] = (((block7 & 255) << 1) | (block8 >> 63)) as i32;
        $output[$vi + 57] = ((block8 >> 54) & 511) as i32;
        $output[$vi + 58] = ((block8 >> 45) & 511) as i32;
        $output[$vi + 59] = ((block8 >> 36) & 511) as i32;
        $output[$vi + 60] = ((block8 >> 27) & 511) as i32;
        $output[$vi + 61] = ((block8 >> 18) & 511) as i32;
        $output[$vi + 62] = ((block8 >> 9) & 511) as i32;
        $output[$vi + 63] = (block8 & 511) as i32;
        $vi += 64;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(9)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(9).zip(values.chunks_exact_mut(64)) {
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
        let byte1 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = (byte0 << 1) | (byte1 >> 7);
        $vi += 1;

        let byte2 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte1 & 127) << 2) | (byte2 >> 6);
        $vi += 1;

        let byte3 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte2 & 63) << 3) | (byte3 >> 5);
        $vi += 1;

        let byte4 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte3 & 31) << 4) | (byte4 >> 4);
        $vi += 1;

        let byte5 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte4 & 15) << 5) | (byte5 >> 3);
        $vi += 1;

        let byte6 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte5 & 7) << 6) | (byte6 >> 2);
        $vi += 1;

        let byte7 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte6 & 3) << 7) | (byte7 >> 1);
        $vi += 1;

        let byte8 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte7 & 1) << 8) | byte8;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(9)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(9).zip(values.chunks_exact_mut(8)) {
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
impl_bulk_operation_packed_encoder!(BulkOperationPacked9);
impl BulkOperation for BulkOperationPacked9 {}
