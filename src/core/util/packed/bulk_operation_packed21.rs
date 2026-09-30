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

define_bulk_operation_packed_specialized!(BulkOperationPacked21, 21);
impl Decoder for BulkOperationPacked21 {
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
        $output[$vi] = (block0 >> 43) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 22) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 1) & 2097151) as i64;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 1) << 20) | (block1 >> 44)) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 23) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 2) & 2097151) as i64;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 3) << 19) | (block2 >> 45)) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 24) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 3) & 2097151) as i64;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 7) << 18) | (block3 >> 46)) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 25) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 4) & 2097151) as i64;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 15) << 17) | (block4 >> 47)) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 26) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 5) & 2097151) as i64;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 31) << 16) | (block5 >> 48)) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 27) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 6) & 2097151) as i64;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 63) << 15) | (block6 >> 49)) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 28) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 7) & 2097151) as i64;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 127) << 14) | (block7 >> 50)) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 29) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 8) & 2097151) as i64;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 255) << 13) | (block8 >> 51)) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 30) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 9) & 2097151) as i64;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 511) << 12) | (block9 >> 52)) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 31) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 10) & 2097151) as i64;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 1023) << 11) | (block10 >> 53)) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 32) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 11) & 2097151) as i64;
        $vi += 1;

        let block11 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block10 & 2047) << 10) | (block11 >> 54)) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 33) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 12) & 2097151) as i64;
        $vi += 1;

        let block12 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block11 & 4095) << 9) | (block12 >> 55)) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 34) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 13) & 2097151) as i64;
        $vi += 1;

        let block13 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block12 & 8191) << 8) | (block13 >> 56)) as i64;
        $vi += 1;
        $output[$vi] = ((block13 >> 35) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block13 >> 14) & 2097151) as i64;
        $vi += 1;

        let block14 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block13 & 16383) << 7) | (block14 >> 57)) as i64;
        $vi += 1;
        $output[$vi] = ((block14 >> 36) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block14 >> 15) & 2097151) as i64;
        $vi += 1;

        let block15 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block14 & 32767) << 6) | (block15 >> 58)) as i64;
        $vi += 1;
        $output[$vi] = ((block15 >> 37) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block15 >> 16) & 2097151) as i64;
        $vi += 1;

        let block16 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block15 & 65535) << 5) | (block16 >> 59)) as i64;
        $vi += 1;
        $output[$vi] = ((block16 >> 38) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block16 >> 17) & 2097151) as i64;
        $vi += 1;

        let block17 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block16 & 131071) << 4) | (block17 >> 60)) as i64;
        $vi += 1;
        $output[$vi] = ((block17 >> 39) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block17 >> 18) & 2097151) as i64;
        $vi += 1;

        let block18 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block17 & 262143) << 3) | (block18 >> 61)) as i64;
        $vi += 1;
        $output[$vi] = ((block18 >> 40) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block18 >> 19) & 2097151) as i64;
        $vi += 1;

        let block19 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block18 & 524287) << 2) | (block19 >> 62)) as i64;
        $vi += 1;
        $output[$vi] = ((block19 >> 41) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block19 >> 20) & 2097151) as i64;
        $vi += 1;

        let block20 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block19 & 1048575) << 1) | (block20 >> 63)) as i64;
        $vi += 1;
        $output[$vi] = ((block20 >> 42) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = ((block20 >> 21) & 2097151) as i64;
        $vi += 1;
        $output[$vi] = (block20 & 2097151) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(21)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(21).zip(values.chunks_exact_mut(64)) {
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
        let byte2 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = (byte0 << 13) | (byte1 << 5) | (byte2 >> 3);
        $vi += 1;

        let byte3 = $input[$bi] as i64;
        $bi += 1;
        let byte4 = $input[$bi] as i64;
        $bi += 1;
        let byte5 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte2 & 7) << 18) | (byte3 << 10) | (byte4 << 2) | (byte5 >> 6);
        $vi += 1;

        let byte6 = $input[$bi] as i64;
        $bi += 1;
        let byte7 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte5 & 63) << 15) | (byte6 << 7) | (byte7 >> 1);
        $vi += 1;

        let byte8 = $input[$bi] as i64;
        $bi += 1;
        let byte9 = $input[$bi] as i64;
        $bi += 1;
        let byte10 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte7 & 1) << 20) | (byte8 << 12) | (byte9 << 4) | (byte10 >> 4);
        $vi += 1;

        let byte11 = $input[$bi] as i64;
        $bi += 1;
        let byte12 = $input[$bi] as i64;
        $bi += 1;
        let byte13 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte10 & 15) << 17) | (byte11 << 9) | (byte12 << 1) | (byte13 >> 7);
        $vi += 1;

        let byte14 = $input[$bi] as i64;
        $bi += 1;
        let byte15 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte13 & 127) << 14) | (byte14 << 6) | (byte15 >> 2);
        $vi += 1;

        let byte16 = $input[$bi] as i64;
        $bi += 1;
        let byte17 = $input[$bi] as i64;
        $bi += 1;
        let byte18 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte15 & 3) << 19) | (byte16 << 11) | (byte17 << 3) | (byte18 >> 5);
        $vi += 1;

        let byte19 = $input[$bi] as i64;
        $bi += 1;
        let byte20 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte18 & 31) << 16) | (byte19 << 8) | byte20;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(21)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(21).zip(values.chunks_exact_mut(8)) {
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
        $output[$vi] = (block0 >> 43) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 22) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 1) & 2_097_151) as i32;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 1) << 20) | (block1 >> 44)) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 23) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 2) & 2_097_151) as i32;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 3) << 19) | (block2 >> 45)) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 24) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 3) & 2_097_151) as i32;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 7) << 18) | (block3 >> 46)) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 25) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 4) & 2_097_151) as i32;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 15) << 17) | (block4 >> 47)) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 26) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 5) & 2_097_151) as i32;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 31) << 16) | (block5 >> 48)) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 27) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 6) & 2_097_151) as i32;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 63) << 15) | (block6 >> 49)) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 28) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 7) & 2_097_151) as i32;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 127) << 14) | (block7 >> 50)) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 29) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 8) & 2_097_151) as i32;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 255) << 13) | (block8 >> 51)) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 30) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 9) & 2_097_151) as i32;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 511) << 12) | (block9 >> 52)) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 31) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 10) & 2_097_151) as i32;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 1023) << 11) | (block10 >> 53)) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 32) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 11) & 2_097_151) as i32;
        $vi += 1;

        let block11 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block10 & 2047) << 10) | (block11 >> 54)) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 33) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 12) & 2_097_151) as i32;
        $vi += 1;

        let block12 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block11 & 4095) << 9) | (block12 >> 55)) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 34) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 13) & 2_097_151) as i32;
        $vi += 1;

        let block13 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block12 & 8191) << 8) | (block13 >> 56)) as i32;
        $vi += 1;
        $output[$vi] = ((block13 >> 35) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block13 >> 14) & 2_097_151) as i32;
        $vi += 1;

        let block14 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block13 & 16_383) << 7) | (block14 >> 57)) as i32;
        $vi += 1;
        $output[$vi] = ((block14 >> 36) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block14 >> 15) & 2_097_151) as i32;
        $vi += 1;

        let block15 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block14 & 32_767) << 6) | (block15 >> 58)) as i32;
        $vi += 1;
        $output[$vi] = ((block15 >> 37) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block15 >> 16) & 2_097_151) as i32;
        $vi += 1;

        let block16 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block15 & 65_535) << 5) | (block16 >> 59)) as i32;
        $vi += 1;
        $output[$vi] = ((block16 >> 38) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block16 >> 17) & 2_097_151) as i32;
        $vi += 1;

        let block17 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block16 & 131_071) << 4) | (block17 >> 60)) as i32;
        $vi += 1;
        $output[$vi] = ((block17 >> 39) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block17 >> 18) & 2_097_151) as i32;
        $vi += 1;

        let block18 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block17 & 262_143) << 3) | (block18 >> 61)) as i32;
        $vi += 1;
        $output[$vi] = ((block18 >> 40) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block18 >> 19) & 2_097_151) as i32;
        $vi += 1;

        let block19 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block18 & 524_287) << 2) | (block19 >> 62)) as i32;
        $vi += 1;
        $output[$vi] = ((block19 >> 41) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block19 >> 20) & 2_097_151) as i32;
        $vi += 1;

        let block20 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block19 & 1_048_575) << 1) | (block20 >> 63)) as i32;
        $vi += 1;
        $output[$vi] = ((block20 >> 42) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = ((block20 >> 21) & 2_097_151) as i32;
        $vi += 1;
        $output[$vi] = (block20 & 2_097_151) as i32;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(21)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(21).zip(values.chunks_exact_mut(64)) {
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
        let byte2 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = (byte0 << 13) | (byte1 << 5) | (byte2 >> 3);
        $vi += 1;

        let byte3 = $input[$bi] as i32;
        $bi += 1;
        let byte4 = $input[$bi] as i32;
        $bi += 1;
        let byte5 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte2 & 7) << 18) | (byte3 << 10) | (byte4 << 2) | (byte5 >> 6);
        $vi += 1;

        let byte6 = $input[$bi] as i32;
        $bi += 1;
        let byte7 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte5 & 63) << 15) | (byte6 << 7) | (byte7 >> 1);
        $vi += 1;

        let byte8 = $input[$bi] as i32;
        $bi += 1;
        let byte9 = $input[$bi] as i32;
        $bi += 1;
        let byte10 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte7 & 1) << 20) | (byte8 << 12) | (byte9 << 4) | (byte10 >> 4);
        $vi += 1;

        let byte11 = $input[$bi] as i32;
        $bi += 1;
        let byte12 = $input[$bi] as i32;
        $bi += 1;
        let byte13 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte10 & 15) << 17) | (byte11 << 9) | (byte12 << 1) | (byte13 >> 7);
        $vi += 1;

        let byte14 = $input[$bi] as i32;
        $bi += 1;
        let byte15 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte13 & 127) << 14) | (byte14 << 6) | (byte15 >> 2);
        $vi += 1;

        let byte16 = $input[$bi] as i32;
        $bi += 1;
        let byte17 = $input[$bi] as i32;
        $bi += 1;
        let byte18 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte15 & 3) << 19) | (byte16 << 11) | (byte17 << 3) | (byte18 >> 5);
        $vi += 1;

        let byte19 = $input[$bi] as i32;
        $bi += 1;
        let byte20 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte18 & 31) << 16) | (byte19 << 8) | byte20;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(21)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(21).zip(values.chunks_exact_mut(8)) {
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
impl_bulk_operation_packed_encoder!(BulkOperationPacked21);
impl BulkOperation for BulkOperationPacked21 {}
