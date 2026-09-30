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

define_bulk_operation_packed_specialized!(BulkOperationPacked5, 5);
impl Decoder for BulkOperationPacked5 {
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

        $output[$vi] = (block0 >> 59) as i64;
        $output[$vi + 1] = ((block0 >> 54) & 31) as i64;
        $output[$vi + 2] = ((block0 >> 49) & 31) as i64;
        $output[$vi + 3] = ((block0 >> 44) & 31) as i64;
        $output[$vi + 4] = ((block0 >> 39) & 31) as i64;
        $output[$vi + 5] = ((block0 >> 34) & 31) as i64;
        $output[$vi + 6] = ((block0 >> 29) & 31) as i64;
        $output[$vi + 7] = ((block0 >> 24) & 31) as i64;
        $output[$vi + 8] = ((block0 >> 19) & 31) as i64;
        $output[$vi + 9] = ((block0 >> 14) & 31) as i64;
        $output[$vi + 10] = ((block0 >> 9) & 31) as i64;
        $output[$vi + 11] = ((block0 >> 4) & 31) as i64;
        $vi += 12;

        let block1 = $input[$bi];
        $bi += 1;

        $output[$vi] = (((block0 & 15) << 1) | (block1 >> 63)) as i64;
        $output[$vi + 1] = ((block1 >> 58) & 31) as i64;
        $output[$vi + 2] = ((block1 >> 53) & 31) as i64;
        $output[$vi + 3] = ((block1 >> 48) & 31) as i64;
        $output[$vi + 4] = ((block1 >> 43) & 31) as i64;
        $output[$vi + 5] = ((block1 >> 38) & 31) as i64;
        $output[$vi + 6] = ((block1 >> 33) & 31) as i64;
        $output[$vi + 7] = ((block1 >> 28) & 31) as i64;
        $output[$vi + 8] = ((block1 >> 23) & 31) as i64;
        $output[$vi + 9] = ((block1 >> 18) & 31) as i64;
        $output[$vi + 10] = ((block1 >> 13) & 31) as i64;
        $output[$vi + 11] = ((block1 >> 8) & 31) as i64;
        $output[$vi + 12] = ((block1 >> 3) & 31) as i64;
        $vi += 13;

        let block2 = $input[$bi];
        $bi += 1;

        $output[$vi] = (((block1 & 7) << 2) | (block2 >> 62)) as i64;
        $output[$vi + 1] = ((block2 >> 57) & 31) as i64;
        $output[$vi + 2] = ((block2 >> 52) & 31) as i64;
        $output[$vi + 3] = ((block2 >> 47) & 31) as i64;
        $output[$vi + 4] = ((block2 >> 42) & 31) as i64;
        $output[$vi + 5] = ((block2 >> 37) & 31) as i64;
        $output[$vi + 6] = ((block2 >> 32) & 31) as i64;
        $output[$vi + 7] = ((block2 >> 27) & 31) as i64;
        $output[$vi + 8] = ((block2 >> 22) & 31) as i64;
        $output[$vi + 9] = ((block2 >> 17) & 31) as i64;
        $output[$vi + 10] = ((block2 >> 12) & 31) as i64;
        $output[$vi + 11] = ((block2 >> 7) & 31) as i64;
        $output[$vi + 12] = ((block2 >> 2) & 31) as i64;
        $vi += 13;

        let block3 = $input[$bi];
        $bi += 1;

        $output[$vi] = (((block2 & 3) << 3) | (block3 >> 61)) as i64;
        $output[$vi + 1] = ((block3 >> 56) & 31) as i64;
        $output[$vi + 2] = ((block3 >> 51) & 31) as i64;
        $output[$vi + 3] = ((block3 >> 46) & 31) as i64;
        $output[$vi + 4] = ((block3 >> 41) & 31) as i64;
        $output[$vi + 5] = ((block3 >> 36) & 31) as i64;
        $output[$vi + 6] = ((block3 >> 31) & 31) as i64;
        $output[$vi + 7] = ((block3 >> 26) & 31) as i64;
        $output[$vi + 8] = ((block3 >> 21) & 31) as i64;
        $output[$vi + 9] = ((block3 >> 16) & 31) as i64;
        $output[$vi + 10] = ((block3 >> 11) & 31) as i64;
        $output[$vi + 11] = ((block3 >> 6) & 31) as i64;
        $output[$vi + 12] = ((block3 >> 1) & 31) as i64;
        $vi += 13;

        let block4 = $input[$bi];
        $bi += 1;

        $output[$vi] = (((block3 & 1) << 4) | (block4 >> 60)) as i64;
        $output[$vi + 1] = ((block4 >> 55) & 31) as i64;
        $output[$vi + 2] = ((block4 >> 50) & 31) as i64;
        $output[$vi + 3] = ((block4 >> 45) & 31) as i64;
        $output[$vi + 4] = ((block4 >> 40) & 31) as i64;
        $output[$vi + 5] = ((block4 >> 35) & 31) as i64;
        $output[$vi + 6] = ((block4 >> 30) & 31) as i64;
        $output[$vi + 7] = ((block4 >> 25) & 31) as i64;
        $output[$vi + 8] = ((block4 >> 20) & 31) as i64;
        $output[$vi + 9] = ((block4 >> 15) & 31) as i64;
        $output[$vi + 10] = ((block4 >> 10) & 31) as i64;
        $output[$vi + 11] = ((block4 >> 5) & 31) as i64;
        $output[$vi + 12] = (block4 & 31) as i64;
        $vi += 13;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(5)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(5).zip(values.chunks_exact_mut(64)) {
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

        $output[$vi] = (byte0 >> 3) as i64;

        let byte1 = $input[$bi] as u64;
        $bi += 1;

        $output[$vi + 1] = (((byte0 & 7) << 2) | (byte1 >> 6)) as i64;
        $output[$vi + 2] = ((byte1 >> 1) & 31) as i64;

        let byte2 = $input[$bi] as u64;
        $bi += 1;

        $output[$vi + 3] = (((byte1 & 1) << 4) | (byte2 >> 4)) as i64;

        let byte3 = $input[$bi] as u64;
        $bi += 1;

        $output[$vi + 4] = (((byte2 & 15) << 1) | (byte3 >> 7)) as i64;
        $output[$vi + 5] = ((byte3 >> 2) & 31) as i64;

        let byte4 = $input[$bi] as u64;
        $bi += 1;

        $output[$vi + 6] = (((byte3 & 3) << 3) | (byte4 >> 5)) as i64;
        $output[$vi + 7] = (byte4 & 31) as i64;
        $vi += 8;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(5)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(5).zip(values.chunks_exact_mut(8)) {
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

        $output[$vi] = (block0 >> 59) as i32;
        $output[$vi + 1] = ((block0 >> 54) & 31) as i32;
        $output[$vi + 2] = ((block0 >> 49) & 31) as i32;
        $output[$vi + 3] = ((block0 >> 44) & 31) as i32;
        $output[$vi + 4] = ((block0 >> 39) & 31) as i32;
        $output[$vi + 5] = ((block0 >> 34) & 31) as i32;
        $output[$vi + 6] = ((block0 >> 29) & 31) as i32;
        $output[$vi + 7] = ((block0 >> 24) & 31) as i32;
        $output[$vi + 8] = ((block0 >> 19) & 31) as i32;
        $output[$vi + 9] = ((block0 >> 14) & 31) as i32;
        $output[$vi + 10] = ((block0 >> 9) & 31) as i32;
        $output[$vi + 11] = ((block0 >> 4) & 31) as i32;

        let block1 = $input[$bi];
        $bi += 1;

        $output[$vi + 12] = (((block0 & 15) << 1) | (block1 >> 63)) as i32;

        $output[$vi + 13] = ((block1 >> 58) & 31) as i32;
        $output[$vi + 14] = ((block1 >> 53) & 31) as i32;
        $output[$vi + 15] = ((block1 >> 48) & 31) as i32;
        $output[$vi + 16] = ((block1 >> 43) & 31) as i32;
        $output[$vi + 17] = ((block1 >> 38) & 31) as i32;
        $output[$vi + 18] = ((block1 >> 33) & 31) as i32;
        $output[$vi + 19] = ((block1 >> 28) & 31) as i32;
        $output[$vi + 20] = ((block1 >> 23) & 31) as i32;
        $output[$vi + 21] = ((block1 >> 18) & 31) as i32;
        $output[$vi + 22] = ((block1 >> 13) & 31) as i32;
        $output[$vi + 23] = ((block1 >> 8) & 31) as i32;
        $output[$vi + 24] = ((block1 >> 3) & 31) as i32;

        let block2 = $input[$bi];
        $bi += 1;

        $output[$vi + 25] = (((block1 & 7) << 2) | (block2 >> 62)) as i32;

        $output[$vi + 26] = ((block2 >> 57) & 31) as i32;
        $output[$vi + 27] = ((block2 >> 52) & 31) as i32;
        $output[$vi + 28] = ((block2 >> 47) & 31) as i32;
        $output[$vi + 29] = ((block2 >> 42) & 31) as i32;
        $output[$vi + 30] = ((block2 >> 37) & 31) as i32;
        $output[$vi + 31] = ((block2 >> 32) & 31) as i32;
        $output[$vi + 32] = ((block2 >> 27) & 31) as i32;
        $output[$vi + 33] = ((block2 >> 22) & 31) as i32;
        $output[$vi + 34] = ((block2 >> 17) & 31) as i32;
        $output[$vi + 35] = ((block2 >> 12) & 31) as i32;
        $output[$vi + 36] = ((block2 >> 7) & 31) as i32;
        $output[$vi + 37] = ((block2 >> 2) & 31) as i32;

        let block3 = $input[$bi];
        $bi += 1;

        $output[$vi + 38] = (((block2 & 3) << 3) | (block3 >> 61)) as i32;

        $output[$vi + 39] = ((block3 >> 56) & 31) as i32;
        $output[$vi + 40] = ((block3 >> 51) & 31) as i32;
        $output[$vi + 41] = ((block3 >> 46) & 31) as i32;
        $output[$vi + 42] = ((block3 >> 41) & 31) as i32;
        $output[$vi + 43] = ((block3 >> 36) & 31) as i32;
        $output[$vi + 44] = ((block3 >> 31) & 31) as i32;
        $output[$vi + 45] = ((block3 >> 26) & 31) as i32;
        $output[$vi + 46] = ((block3 >> 21) & 31) as i32;
        $output[$vi + 47] = ((block3 >> 16) & 31) as i32;
        $output[$vi + 48] = ((block3 >> 11) & 31) as i32;
        $output[$vi + 49] = ((block3 >> 6) & 31) as i32;
        $output[$vi + 50] = ((block3 >> 1) & 31) as i32;

        let block4 = $input[$bi];
        $bi += 1;

        $output[$vi + 51] = (((block3 & 1) << 4) | (block4 >> 60)) as i32;

        for i in 0..12 {
          $output[$vi + 52 + i] = ((block4 >> (55 - i * 5)) & 31) as i32;
        }
        $vi += 64;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(5)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(5).zip(values.chunks_exact_mut(64)) {
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
        $output[$vi] = byte0 >> 3;
        $vi += 1;

        let byte1 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte0 & 7) << 2) | (byte1 >> 6);
        $vi += 1;
        $output[$vi] = (byte1 >> 1) & 31;
        $vi += 1;

        let byte2 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte1 & 1) << 4) | (byte2 >> 4);
        $vi += 1;

        let byte3 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte2 & 15) << 1) | (byte3 >> 7);
        $vi += 1;
        $output[$vi] = (byte3 >> 2) & 31;
        $vi += 1;

        let byte4 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte3 & 3) << 3) | (byte4 >> 5);
        $vi += 1;
        $output[$vi] = byte4 & 31;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(5)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(5).zip(values.chunks_exact_mut(8)) {
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
impl_bulk_operation_packed_encoder!(BulkOperationPacked5);
impl BulkOperation for BulkOperationPacked5 {}
