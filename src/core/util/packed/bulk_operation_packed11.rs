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

define_bulk_operation_packed_specialized!(BulkOperationPacked11, 11);
impl Decoder for BulkOperationPacked11 {
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
        $output[$vi] = (block0 >> 53) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 42) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 31) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 20) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 9) & 2047) as i64;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 511) << 2) | (block1 >> 62)) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 51) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 40) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 29) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 18) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 7) & 2047) as i64;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 127) << 4) | (block2 >> 60)) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 49) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 38) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 27) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 16) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 5) & 2047) as i64;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 31) << 6) | (block3 >> 58)) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 47) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 36) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 25) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 14) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 3) & 2047) as i64;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 7) << 8) | (block4 >> 56)) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 45) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 34) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 23) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 12) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 1) & 2047) as i64;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 1) << 10) | (block5 >> 54)) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 43) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 32) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 21) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 10) & 2047) as i64;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 1023) << 1) | (block6 >> 63)) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 52) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 41) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 30) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 19) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 8) & 2047) as i64;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 255) << 3) | (block7 >> 61)) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 50) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 39) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 28) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 17) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 6) & 2047) as i64;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 63) << 5) | (block8 >> 59)) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 48) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 37) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 26) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 15) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 4) & 2047) as i64;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 15) << 7) | (block9 >> 57)) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 46) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 35) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 24) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 13) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 2) & 2047) as i64;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 3) << 9) | (block10 >> 55)) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 44) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 33) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 22) & 2047) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 11) & 2047) as i64;
        $vi += 1;
        $output[$vi] = (block10 & 2047) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(11)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(11).zip(values.chunks_exact_mut(64)) {
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
        let byte1 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = ((byte0 << 3) | (byte1 >> 5)) as i64;
        $vi += 1;

        let byte2 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte1 & 31) << 6) | (byte2 >> 2)) as i64;
        $vi += 1;

        let byte3 = $input[$bi] as u64;
        $bi += 1;
        let byte4 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte2 & 3) << 9) | (byte3 << 1) | (byte4 >> 7)) as i64;
        $vi += 1;

        let byte5 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte4 & 127) << 4) | (byte5 >> 4)) as i64;
        $vi += 1;

        let byte6 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte5 & 15) << 7) | (byte6 >> 1)) as i64;
        $vi += 1;

        let byte7 = $input[$bi] as u64;
        $bi += 1;
        let byte8 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte6 & 1) << 10) | (byte7 << 2) | (byte8 >> 6)) as i64;
        $vi += 1;

        let byte9 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte8 & 63) << 5) | (byte9 >> 3)) as i64;
        $vi += 1;

        let byte10 = $input[$bi] as u64;
        $bi += 1;
        $output[$vi] = (((byte9 & 7) << 8) | byte10) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(11)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(11).zip(values.chunks_exact_mut(8)) {
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
        $output[$vi] = (block0 >> 53) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 42) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 31) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 20) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 9) & 2047) as i32;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 511) << 2) | (block1 >> 62)) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 51) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 40) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 29) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 18) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 7) & 2047) as i32;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 127) << 4) | (block2 >> 60)) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 49) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 38) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 27) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 16) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 5) & 2047) as i32;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 31) << 6) | (block3 >> 58)) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 47) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 36) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 25) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 14) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 3) & 2047) as i32;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 7) << 8) | (block4 >> 56)) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 45) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 34) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 23) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 12) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 1) & 2047) as i32;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 1) << 10) | (block5 >> 54)) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 43) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 32) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 21) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 10) & 2047) as i32;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 1023) << 1) | (block6 >> 63)) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 52) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 41) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 30) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 19) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 8) & 2047) as i32;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 255) << 3) | (block7 >> 61)) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 50) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 39) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 28) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 17) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 6) & 2047) as i32;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 63) << 5) | (block8 >> 59)) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 48) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 37) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 26) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 15) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 4) & 2047) as i32;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 15) << 7) | (block9 >> 57)) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 46) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 35) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 24) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 13) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 2) & 2047) as i32;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 3) << 9) | (block10 >> 55)) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 44) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 33) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 22) & 2047) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 11) & 2047) as i32;
        $vi += 1;
        $output[$vi] = (block10 & 2047) as i32;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(11)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(11).zip(values.chunks_exact_mut(64)) {
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
        $output[$vi] = (byte0 << 3) | (byte1 >> 5);
        $vi += 1;

        let byte2 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte1 & 31) << 6) | (byte2 >> 2);
        $vi += 1;

        let byte3 = $input[$bi] as i32;
        $bi += 1;
        let byte4 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte2 & 3) << 9) | (byte3 << 1) | (byte4 >> 7);
        $vi += 1;

        let byte5 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte4 & 127) << 4) | (byte5 >> 4);
        $vi += 1;

        let byte6 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte5 & 15) << 7) | (byte6 >> 1);
        $vi += 1;

        let byte7 = $input[$bi] as i32;
        $bi += 1;
        let byte8 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte6 & 1) << 10) | (byte7 << 2) | (byte8 >> 6);
        $vi += 1;

        let byte9 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte8 & 63) << 5) | (byte9 >> 3);
        $vi += 1;

        let byte10 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte9 & 7) << 8) | byte10;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(11)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(11).zip(values.chunks_exact_mut(8)) {
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
impl_bulk_operation_packed_encoder!(BulkOperationPacked11);
impl BulkOperation for BulkOperationPacked11 {}
