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

define_bulk_operation_packed_specialized!(BulkOperationPacked19, 19);
impl Decoder for BulkOperationPacked19 {
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
        $output[$vi] = (block0 >> 45) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 26) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 7) & 524_287) as i64;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 127) << 12) | (block1 >> 52)) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 33) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 14) & 524_287) as i64;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 16_383) << 5) | (block2 >> 59)) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 40) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 21) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 2) & 524_287) as i64;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 3) << 17) | (block3 >> 47)) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 28) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 9) & 524_287) as i64;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 511) << 10) | (block4 >> 54)) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 35) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 16) & 524_287) as i64;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 65_535) << 3) | (block5 >> 61)) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 42) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 23) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 4) & 524_287) as i64;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 15) << 15) | (block6 >> 49)) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 30) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 11) & 524_287) as i64;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 2_047) << 8) | (block7 >> 56)) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 37) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 18) & 524_287) as i64;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 262_143) << 1) | (block8 >> 63)) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 44) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 25) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 6) & 524_287) as i64;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 63) << 13) | (block9 >> 51)) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 32) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 13) & 524_287) as i64;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 8_191) << 6) | (block10 >> 58)) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 39) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 20) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 1) & 524_287) as i64;
        $vi += 1;

        let block11 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block10 & 1) << 18) | (block11 >> 46)) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 27) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 8) & 524_287) as i64;
        $vi += 1;

        let block12 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block11 & 255) << 11) | (block12 >> 53)) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 34) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 15) & 524_287) as i64;
        $vi += 1;

        let block13 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block12 & 32_767) << 4) | (block13 >> 60)) as i64;
        $vi += 1;
        $output[$vi] = ((block13 >> 41) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block13 >> 22) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block13 >> 3) & 524_287) as i64;
        $vi += 1;

        let block14 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block13 & 7) << 16) | (block14 >> 48)) as i64;
        $vi += 1;
        $output[$vi] = ((block14 >> 29) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block14 >> 10) & 524_287) as i64;
        $vi += 1;

        let block15 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block14 & 1_023) << 9) | (block15 >> 55)) as i64;
        $vi += 1;
        $output[$vi] = ((block15 >> 36) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block15 >> 17) & 524_287) as i64;
        $vi += 1;

        let block16 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block15 & 131_071) << 2) | (block16 >> 62)) as i64;
        $vi += 1;
        $output[$vi] = ((block16 >> 43) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block16 >> 24) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block16 >> 5) & 524_287) as i64;
        $vi += 1;

        let block17 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block16 & 31) << 14) | (block17 >> 50)) as i64;
        $vi += 1;
        $output[$vi] = ((block17 >> 31) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block17 >> 12) & 524_287) as i64;
        $vi += 1;

        let block18 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block17 & 4_095) << 7) | (block18 >> 57)) as i64;
        $vi += 1;
        $output[$vi] = ((block18 >> 38) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = ((block18 >> 19) & 524_287) as i64;
        $vi += 1;
        $output[$vi] = (block18 & 524_287) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(19)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(19).zip(values.chunks_exact_mut(64)) {
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
        $output[$vi] = (byte0 << 11) | (byte1 << 3) | (byte2 >> 5);
        $vi += 1;

        let byte3 = $input[$bi] as i64;
        $bi += 1;
        let byte4 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte2 & 31) << 14) | (byte3 << 6) | (byte4 >> 2);
        $vi += 1;

        let byte5 = $input[$bi] as i64;
        $bi += 1;
        let byte6 = $input[$bi] as i64;
        $bi += 1;
        let byte7 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte4 & 3) << 17) | (byte5 << 9) | (byte6 << 1) | (byte7 >> 7);
        $vi += 1;

        let byte8 = $input[$bi] as i64;
        $bi += 1;
        let byte9 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte7 & 127) << 12) | (byte8 << 4) | (byte9 >> 4);
        $vi += 1;

        let byte10 = $input[$bi] as i64;
        $bi += 1;
        let byte11 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte9 & 15) << 15) | (byte10 << 7) | (byte11 >> 1);
        $vi += 1;

        let byte12 = $input[$bi] as i64;
        $bi += 1;
        let byte13 = $input[$bi] as i64;
        $bi += 1;
        let byte14 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte11 & 1) << 18) | (byte12 << 10) | (byte13 << 2) | (byte14 >> 6);
        $vi += 1;

        let byte15 = $input[$bi] as i64;
        $bi += 1;
        let byte16 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte14 & 63) << 13) | (byte15 << 5) | (byte16 >> 3);
        $vi += 1;

        let byte17 = $input[$bi] as i64;
        $bi += 1;
        let byte18 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte16 & 7) << 16) | (byte17 << 8) | byte18;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(19)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(19).zip(values.chunks_exact_mut(8)) {
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
        $output[$vi] = (block0 >> 45) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 26) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 7) & 524_287) as i32;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 127) << 12) | (block1 >> 52)) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 33) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 14) & 524_287) as i32;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 16_383) << 5) | (block2 >> 59)) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 40) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 21) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 2) & 524_287) as i32;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 3) << 17) | (block3 >> 47)) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 28) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 9) & 524_287) as i32;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 511) << 10) | (block4 >> 54)) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 35) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 16) & 524_287) as i32;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 65_535) << 3) | (block5 >> 61)) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 42) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 23) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 4) & 524_287) as i32;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 15) << 15) | (block6 >> 49)) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 30) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 11) & 524_287) as i32;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 2_047) << 8) | (block7 >> 56)) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 37) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 18) & 524_287) as i32;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 262_143) << 1) | (block8 >> 63)) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 44) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 25) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 6) & 524_287) as i32;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 63) << 13) | (block9 >> 51)) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 32) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 13) & 524_287) as i32;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 8_191) << 6) | (block10 >> 58)) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 39) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 20) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 1) & 524_287) as i32;
        $vi += 1;

        let block11 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block10 & 1) << 18) | (block11 >> 46)) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 27) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 8) & 524_287) as i32;
        $vi += 1;

        let block12 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block11 & 255) << 11) | (block12 >> 53)) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 34) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 15) & 524_287) as i32;
        $vi += 1;

        let block13 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block12 & 32_767) << 4) | (block13 >> 60)) as i32;
        $vi += 1;
        $output[$vi] = ((block13 >> 41) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block13 >> 22) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block13 >> 3) & 524_287) as i32;
        $vi += 1;

        let block14 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block13 & 7) << 16) | (block14 >> 48)) as i32;
        $vi += 1;
        $output[$vi] = ((block14 >> 29) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block14 >> 10) & 524_287) as i32;
        $vi += 1;

        let block15 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block14 & 1_023) << 9) | (block15 >> 55)) as i32;
        $vi += 1;
        $output[$vi] = ((block15 >> 36) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block15 >> 17) & 524_287) as i32;
        $vi += 1;

        let block16 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block15 & 131_071) << 2) | (block16 >> 62)) as i32;
        $vi += 1;
        $output[$vi] = ((block16 >> 43) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block16 >> 24) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block16 >> 5) & 524_287) as i32;
        $vi += 1;

        let block17 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block16 & 31) << 14) | (block17 >> 50)) as i32;
        $vi += 1;
        $output[$vi] = ((block17 >> 31) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block17 >> 12) & 524_287) as i32;
        $vi += 1;

        let block18 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block17 & 4_095) << 7) | (block18 >> 57)) as i32;
        $vi += 1;
        $output[$vi] = ((block18 >> 38) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = ((block18 >> 19) & 524_287) as i32;
        $vi += 1;
        $output[$vi] = (block18 & 524_287) as i32;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(19)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(19).zip(values.chunks_exact_mut(64)) {
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
        $output[$vi] = (byte0 << 11) | (byte1 << 3) | (byte2 >> 5);
        $vi += 1;

        let byte3 = $input[$bi] as i32;
        $bi += 1;
        let byte4 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte2 & 31) << 14) | (byte3 << 6) | (byte4 >> 2);
        $vi += 1;

        let byte5 = $input[$bi] as i32;
        $bi += 1;
        let byte6 = $input[$bi] as i32;
        $bi += 1;
        let byte7 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte4 & 3) << 17) | (byte5 << 9) | (byte6 << 1) | (byte7 >> 7);
        $vi += 1;

        let byte8 = $input[$bi] as i32;
        $bi += 1;
        let byte9 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte7 & 127) << 12) | (byte8 << 4) | (byte9 >> 4);
        $vi += 1;

        let byte10 = $input[$bi] as i32;
        $bi += 1;
        let byte11 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte9 & 15) << 15) | (byte10 << 7) | (byte11 >> 1);
        $vi += 1;

        let byte12 = $input[$bi] as i32;
        $bi += 1;
        let byte13 = $input[$bi] as i32;
        $bi += 1;
        let byte14 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte11 & 1) << 18) | (byte12 << 10) | (byte13 << 2) | (byte14 >> 6);
        $vi += 1;

        let byte15 = $input[$bi] as i32;
        $bi += 1;
        let byte16 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte14 & 63) << 13) | (byte15 << 5) | (byte16 >> 3);
        $vi += 1;

        let byte17 = $input[$bi] as i32;
        $bi += 1;
        let byte18 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte16 & 7) << 16) | (byte17 << 8) | byte18;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(19)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(19).zip(values.chunks_exact_mut(8)) {
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
impl_bulk_operation_packed_encoder!(BulkOperationPacked19);
impl BulkOperation for BulkOperationPacked19 {}
