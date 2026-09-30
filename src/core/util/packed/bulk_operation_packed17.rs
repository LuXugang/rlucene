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

define_bulk_operation_packed_specialized!(BulkOperationPacked17, 17);
impl Decoder for BulkOperationPacked17 {
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
        $output[$vi] = (block0 >> 47) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 30) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 13) & 131_071) as i64;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 8_191) << 4) | (block1 >> 60)) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 43) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 26) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 9) & 131_071) as i64;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 511) << 8) | (block2 >> 56)) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 39) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 22) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 5) & 131_071) as i64;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 31) << 12) | (block3 >> 52)) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 35) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 18) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 1) & 131_071) as i64;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 1) << 16) | (block4 >> 48)) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 31) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 14) & 131_071) as i64;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 16_383) << 3) | (block5 >> 61)) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 44) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 27) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 10) & 131_071) as i64;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 1_023) << 7) | (block6 >> 57)) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 40) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 23) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 6) & 131_071) as i64;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 63) << 11) | (block7 >> 53)) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 36) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 19) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 2) & 131_071) as i64;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 3) << 15) | (block8 >> 49)) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 32) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 15) & 131_071) as i64;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 32_767) << 2) | (block9 >> 62)) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 45) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 28) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 11) & 131_071) as i64;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 2_047) << 6) | (block10 >> 58)) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 41) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 24) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 7) & 131_071) as i64;
        $vi += 1;

        let block11 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block10 & 127) << 10) | (block11 >> 54)) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 37) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 20) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 3) & 131_071) as i64;
        $vi += 1;

        let block12 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block11 & 7) << 14) | (block12 >> 50)) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 33) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 16) & 131_071) as i64;
        $vi += 1;

        let block13 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block12 & 65_535) << 1) | (block13 >> 63)) as i64;
        $vi += 1;
        $output[$vi] = ((block13 >> 46) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block13 >> 29) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block13 >> 12) & 131_071) as i64;
        $vi += 1;

        let block14 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block13 & 4_095) << 5) | (block14 >> 59)) as i64;
        $vi += 1;
        $output[$vi] = ((block14 >> 42) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block14 >> 25) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block14 >> 8) & 131_071) as i64;
        $vi += 1;

        let block15 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block14 & 255) << 9) | (block15 >> 55)) as i64;
        $vi += 1;
        $output[$vi] = ((block15 >> 38) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block15 >> 21) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block15 >> 4) & 131_071) as i64;
        $vi += 1;

        let block16 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block15 & 15) << 13) | (block16 >> 51)) as i64;
        $vi += 1;
        $output[$vi] = ((block16 >> 34) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = ((block16 >> 17) & 131_071) as i64;
        $vi += 1;
        $output[$vi] = (block16 & 131_071) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(17)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(17).zip(values.chunks_exact_mut(64)) {
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
        $output[$vi] = (byte0 << 9) | (byte1 << 1) | (byte2 >> 7);
        $vi += 1;

        let byte3 = $input[$bi] as i64;
        $bi += 1;
        let byte4 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte2 & 127) << 10) | (byte3 << 2) | (byte4 >> 6);
        $vi += 1;

        let byte5 = $input[$bi] as i64;
        $bi += 1;
        let byte6 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte4 & 63) << 11) | (byte5 << 3) | (byte6 >> 5);
        $vi += 1;

        let byte7 = $input[$bi] as i64;
        $bi += 1;
        let byte8 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte6 & 31) << 12) | (byte7 << 4) | (byte8 >> 4);
        $vi += 1;

        let byte9 = $input[$bi] as i64;
        $bi += 1;
        let byte10 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte8 & 15) << 13) | (byte9 << 5) | (byte10 >> 3);
        $vi += 1;

        let byte11 = $input[$bi] as i64;
        $bi += 1;
        let byte12 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte10 & 7) << 14) | (byte11 << 6) | (byte12 >> 2);
        $vi += 1;

        let byte13 = $input[$bi] as i64;
        $bi += 1;
        let byte14 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte12 & 3) << 15) | (byte13 << 7) | (byte14 >> 1);
        $vi += 1;

        let byte15 = $input[$bi] as i64;
        $bi += 1;
        let byte16 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte14 & 1) << 16) | (byte15 << 8) | byte16;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(17)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(17).zip(values.chunks_exact_mut(8)) {
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
        $output[$vi] = (block0 >> 47) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 30) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 13) & 131071) as i32;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 8191) << 4) | (block1 >> 60)) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 43) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 26) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 9) & 131071) as i32;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 511) << 8) | (block2 >> 56)) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 39) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 22) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 5) & 131071) as i32;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 31) << 12) | (block3 >> 52)) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 35) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 18) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 1) & 131071) as i32;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 1) << 16) | (block4 >> 48)) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 31) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 14) & 131071) as i32;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 16383) << 3) | (block5 >> 61)) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 44) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 27) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 10) & 131071) as i32;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 1023) << 7) | (block6 >> 57)) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 40) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 23) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 6) & 131071) as i32;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 63) << 11) | (block7 >> 53)) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 36) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 19) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 2) & 131071) as i32;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 3) << 15) | (block8 >> 49)) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 32) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 15) & 131071) as i32;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 32767) << 2) | (block9 >> 62)) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 45) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 28) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 11) & 131071) as i32;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 2047) << 6) | (block10 >> 58)) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 41) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 24) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 7) & 131071) as i32;
        $vi += 1;

        let block11 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block10 & 127) << 10) | (block11 >> 54)) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 37) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 20) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 3) & 131071) as i32;
        $vi += 1;

        let block12 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block11 & 7) << 14) | (block12 >> 50)) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 33) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 16) & 131071) as i32;
        $vi += 1;

        let block13 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block12 & 65535) << 1) | (block13 >> 63)) as i32;
        $vi += 1;
        $output[$vi] = ((block13 >> 46) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block13 >> 29) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block13 >> 12) & 131071) as i32;
        $vi += 1;

        let block14 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block13 & 4095) << 5) | (block14 >> 59)) as i32;
        $vi += 1;
        $output[$vi] = ((block14 >> 42) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block14 >> 25) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block14 >> 8) & 131071) as i32;
        $vi += 1;

        let block15 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block14 & 255) << 9) | (block15 >> 55)) as i32;
        $vi += 1;
        $output[$vi] = ((block15 >> 38) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block15 >> 21) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block15 >> 4) & 131071) as i32;
        $vi += 1;

        let block16 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block15 & 15) << 13) | (block16 >> 51)) as i32;
        $vi += 1;
        $output[$vi] = ((block16 >> 34) & 131071) as i32;
        $vi += 1;
        $output[$vi] = ((block16 >> 17) & 131071) as i32;
        $vi += 1;
        $output[$vi] = (block16 & 131071) as i32;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(17)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(17).zip(values.chunks_exact_mut(64)) {
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
        $output[$vi] = (byte0 << 9) | (byte1 << 1) | (byte2 >> 7);
        $vi += 1;

        let byte3 = $input[$bi] as i32;
        $bi += 1;
        let byte4 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte2 & 127) << 10) | (byte3 << 2) | (byte4 >> 6);
        $vi += 1;

        let byte5 = $input[$bi] as i32;
        $bi += 1;
        let byte6 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte4 & 63) << 11) | (byte5 << 3) | (byte6 >> 5);
        $vi += 1;

        let byte7 = $input[$bi] as i32;
        $bi += 1;
        let byte8 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte6 & 31) << 12) | (byte7 << 4) | (byte8 >> 4);
        $vi += 1;

        let byte9 = $input[$bi] as i32;
        $bi += 1;
        let byte10 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte8 & 15) << 13) | (byte9 << 5) | (byte10 >> 3);
        $vi += 1;

        let byte11 = $input[$bi] as i32;
        $bi += 1;
        let byte12 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte10 & 7) << 14) | (byte11 << 6) | (byte12 >> 2);
        $vi += 1;

        let byte13 = $input[$bi] as i32;
        $bi += 1;
        let byte14 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte12 & 3) << 15) | (byte13 << 7) | (byte14 >> 1);
        $vi += 1;

        let byte15 = $input[$bi] as i32;
        $bi += 1;
        let byte16 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte14 & 1) << 16) | (byte15 << 8) | byte16;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(17)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(17).zip(values.chunks_exact_mut(8)) {
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
impl_bulk_operation_packed_encoder!(BulkOperationPacked17);
impl BulkOperation for BulkOperationPacked17 {}
