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

define_bulk_operation_packed_specialized!(BulkOperationPacked15, 15);
impl Decoder for BulkOperationPacked15 {
  delegate_bulk_operation_packed_decoder_counts!();
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
        $output[$vi] = (block0 >> 49) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 34) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 19) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 4) & 32767) as i64;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 15) << 11) | (block1 >> 53)) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 38) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 23) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 8) & 32767) as i64;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 255) << 7) | (block2 >> 57)) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 42) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 27) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 12) & 32767) as i64;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 4095) << 3) | (block3 >> 61)) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 46) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 31) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 16) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 1) & 32767) as i64;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 1) << 14) | (block4 >> 50)) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 35) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 20) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 5) & 32767) as i64;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 31) << 10) | (block5 >> 54)) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 39) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 24) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 9) & 32767) as i64;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 511) << 6) | (block6 >> 58)) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 43) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 28) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 13) & 32767) as i64;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 8191) << 2) | (block7 >> 62)) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 47) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 32) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 17) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 2) & 32767) as i64;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 3) << 13) | (block8 >> 51)) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 36) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 21) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 6) & 32767) as i64;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 63) << 9) | (block9 >> 55)) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 40) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 25) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 10) & 32767) as i64;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 1023) << 5) | (block10 >> 59)) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 44) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 29) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 14) & 32767) as i64;
        $vi += 1;

        let block11 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block10 & 16383) << 1) | (block11 >> 63)) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 48) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 33) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 18) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 3) & 32767) as i64;
        $vi += 1;

        let block12 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block11 & 7) << 12) | (block12 >> 52)) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 37) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 22) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 7) & 32767) as i64;
        $vi += 1;

        let block13 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block12 & 127) << 8) | (block13 >> 56)) as i64;
        $vi += 1;
        $output[$vi] = ((block13 >> 41) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block13 >> 26) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block13 >> 11) & 32767) as i64;
        $vi += 1;

        let block14 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block13 & 2047) << 4) | (block14 >> 60)) as i64;
        $vi += 1;
        $output[$vi] = ((block14 >> 45) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block14 >> 30) & 32767) as i64;
        $vi += 1;
        $output[$vi] = ((block14 >> 15) & 32767) as i64;
        $vi += 1;
        $output[$vi] = (block14 & 32767) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(15)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(15).zip(values.chunks_exact_mut(64)) {
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
        let byte0 = $input[$bi] as i64;
        $bi += 1;
        let byte1 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = (byte0 << 7) | (byte1 >> 1);
        $vi += 1;

        let byte2 = $input[$bi] as i64;
        $bi += 1;
        let byte3 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte1 & 1) << 14) | (byte2 << 6) | (byte3 >> 2);
        $vi += 1;

        let byte4 = $input[$bi] as i64;
        $bi += 1;
        let byte5 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte3 & 3) << 13) | (byte4 << 5) | (byte5 >> 3);
        $vi += 1;

        let byte6 = $input[$bi] as i64;
        $bi += 1;
        let byte7 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte5 & 7) << 12) | (byte6 << 4) | (byte7 >> 4);
        $vi += 1;

        let byte8 = $input[$bi] as i64;
        $bi += 1;
        let byte9 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte7 & 15) << 11) | (byte8 << 3) | (byte9 >> 5);
        $vi += 1;

        let byte10 = $input[$bi] as i64;
        $bi += 1;
        let byte11 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte9 & 31) << 10) | (byte10 << 2) | (byte11 >> 6);
        $vi += 1;

        let byte12 = $input[$bi] as i64;
        $bi += 1;
        let byte13 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte11 & 63) << 9) | (byte12 << 1) | (byte13 >> 7);
        $vi += 1;

        let byte14 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte13 & 127) << 8) | byte14;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(15)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(15).zip(values.chunks_exact_mut(8)) {
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
        $output[$vi] = (block0 >> 49) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 34) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 19) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 4) & 32767) as i32;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 15) << 11) | (block1 >> 53)) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 38) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 23) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 8) & 32767) as i32;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 255) << 7) | (block2 >> 57)) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 42) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 27) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 12) & 32767) as i32;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 4095) << 3) | (block3 >> 61)) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 46) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 31) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 16) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 1) & 32767) as i32;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 1) << 14) | (block4 >> 50)) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 35) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 20) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 5) & 32767) as i32;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 31) << 10) | (block5 >> 54)) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 39) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 24) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 9) & 32767) as i32;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 511) << 6) | (block6 >> 58)) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 43) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 28) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 13) & 32767) as i32;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 8191) << 2) | (block7 >> 62)) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 47) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 32) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 17) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 2) & 32767) as i32;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 3) << 13) | (block8 >> 51)) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 36) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 21) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 6) & 32767) as i32;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 63) << 9) | (block9 >> 55)) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 40) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 25) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 10) & 32767) as i32;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 1023) << 5) | (block10 >> 59)) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 44) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 29) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 14) & 32767) as i32;
        $vi += 1;

        let block11 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block10 & 16383) << 1) | (block11 >> 63)) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 48) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 33) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 18) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 3) & 32767) as i32;
        $vi += 1;

        let block12 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block11 & 7) << 12) | (block12 >> 52)) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 37) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 22) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 7) & 32767) as i32;
        $vi += 1;

        let block13 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block12 & 127) << 8) | (block13 >> 56)) as i32;
        $vi += 1;
        $output[$vi] = ((block13 >> 41) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block13 >> 26) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block13 >> 11) & 32767) as i32;
        $vi += 1;

        let block14 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block13 & 2047) << 4) | (block14 >> 60)) as i32;
        $vi += 1;
        $output[$vi] = ((block14 >> 45) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block14 >> 30) & 32767) as i32;
        $vi += 1;
        $output[$vi] = ((block14 >> 15) & 32767) as i32;
        $vi += 1;
        $output[$vi] = (block14 & 32767) as i32;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(15)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(15).zip(values.chunks_exact_mut(64)) {
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
        let byte1 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = (byte0 << 7) | (byte1 >> 1);
        $vi += 1;

        let byte2 = $input[$bi] as i32;
        $bi += 1;
        let byte3 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte1 & 1) << 14) | (byte2 << 6) | (byte3 >> 2);
        $vi += 1;

        let byte4 = $input[$bi] as i32;
        $bi += 1;
        let byte5 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte3 & 3) << 13) | (byte4 << 5) | (byte5 >> 3);
        $vi += 1;

        let byte6 = $input[$bi] as i32;
        $bi += 1;
        let byte7 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte5 & 7) << 12) | (byte6 << 4) | (byte7 >> 4);
        $vi += 1;

        let byte8 = $input[$bi] as i32;
        $bi += 1;
        let byte9 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte7 & 15) << 11) | (byte8 << 3) | (byte9 >> 5);
        $vi += 1;

        let byte10 = $input[$bi] as i32;
        $bi += 1;
        let byte11 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte9 & 31) << 10) | (byte10 << 2) | (byte11 >> 6);
        $vi += 1;

        let byte12 = $input[$bi] as i32;
        $bi += 1;
        let byte13 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte11 & 63) << 9) | (byte12 << 1) | (byte13 >> 7);
        $vi += 1;

        let byte14 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte13 & 127) << 8) | byte14;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(15)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(15).zip(values.chunks_exact_mut(8)) {
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
impl_bulk_operation_packed_encoder!(BulkOperationPacked15);
impl BulkOperation for BulkOperationPacked15 {}
