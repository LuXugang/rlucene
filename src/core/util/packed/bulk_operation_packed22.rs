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

define_bulk_operation_packed_specialized!(BulkOperationPacked22, 22);
impl Decoder for BulkOperationPacked22 {
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
        $output[$vi] = (block0 >> 42) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 20) & 4_194_303) as i64;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 1_048_575) << 2) | (block1 >> 62)) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 40) & 4_194_303) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 18) & 4_194_303) as i64;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 262_143) << 4) | (block2 >> 60)) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 38) & 4_194_303) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 16) & 4_194_303) as i64;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 65_535) << 6) | (block3 >> 58)) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 36) & 4_194_303) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 14) & 4_194_303) as i64;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 16_383) << 8) | (block4 >> 56)) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 34) & 4_194_303) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 12) & 4_194_303) as i64;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 4_095) << 10) | (block5 >> 54)) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 32) & 4_194_303) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 10) & 4_194_303) as i64;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 1_023) << 12) | (block6 >> 52)) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 30) & 4_194_303) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 8) & 4_194_303) as i64;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 255) << 14) | (block7 >> 50)) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 28) & 4_194_303) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 6) & 4_194_303) as i64;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 63) << 16) | (block8 >> 48)) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 26) & 4_194_303) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 4) & 4_194_303) as i64;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 15) << 18) | (block9 >> 46)) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 24) & 4_194_303) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 2) & 4_194_303) as i64;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 3) << 20) | (block10 >> 44)) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 22) & 4_194_303) as i64;
        $vi += 1;
        $output[$vi] = (block10 & 4_194_303) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(11)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(32)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(11).zip(values.chunks_exact_mut(32)) {
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
        $output[$vi] = (byte0 << 14) | (byte1 << 6) | (byte2 >> 2);
        $vi += 1;

        let byte3 = $input[$bi] as i64;
        $bi += 1;
        let byte4 = $input[$bi] as i64;
        $bi += 1;
        let byte5 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte2 & 3) << 20) | (byte3 << 12) | (byte4 << 4) | (byte5 >> 4);
        $vi += 1;

        let byte6 = $input[$bi] as i64;
        $bi += 1;
        let byte7 = $input[$bi] as i64;
        $bi += 1;
        let byte8 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte5 & 15) << 18) | (byte6 << 10) | (byte7 << 2) | (byte8 >> 6);
        $vi += 1;

        let byte9 = $input[$bi] as i64;
        $bi += 1;
        let byte10 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte8 & 63) << 16) | (byte9 << 8) | byte10;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(11)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(4)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(11).zip(values.chunks_exact_mut(4)) {
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
        $output[$vi] = (block0 >> 42) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 20) & 0x3FFFFF) as i32;
        $vi += 1;
        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 0xFFFFF) << 2) | (block1 >> 62)) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 40) & 0x3FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 18) & 0x3FFFFF) as i32;
        $vi += 1;
        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 0x3FFFF) << 4) | (block2 >> 60)) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 38) & 0x3FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 16) & 0x3FFFFF) as i32;
        $vi += 1;
        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 0xFFFF) << 6) | (block3 >> 58)) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 36) & 0x3FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 14) & 0x3FFFFF) as i32;
        $vi += 1;
        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 0x3FFF) << 8) | (block4 >> 56)) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 34) & 0x3FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 12) & 0x3FFFFF) as i32;
        $vi += 1;
        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 0xFFF) << 10) | (block5 >> 54)) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 32) & 0x3FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 10) & 0x3FFFFF) as i32;
        $vi += 1;
        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 0x3FF) << 12) | (block6 >> 52)) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 30) & 0x3FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 8) & 0x3FFFFF) as i32;
        $vi += 1;
        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 0xFF) << 14) | (block7 >> 50)) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 28) & 0x3FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 6) & 0x3FFFFF) as i32;
        $vi += 1;
        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 0x3F) << 16) | (block8 >> 48)) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 26) & 0x3FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 4) & 0x3FFFFF) as i32;
        $vi += 1;
        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 0xF) << 18) | (block9 >> 46)) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 24) & 0x3FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 2) & 0x3FFFFF) as i32;
        $vi += 1;
        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 3) << 20) | (block10 >> 44)) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 22) & 0x3FFFFF) as i32;
        $vi += 1;
        $output[$vi] = (block10 & 0x3FFFFF) as i32;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(11)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(32)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(11).zip(values.chunks_exact_mut(32)) {
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
        $output[$vi] = (byte0 << 14) | (byte1 << 6) | (byte2 >> 2);
        $vi += 1;

        let byte3 = $input[$bi] as i32;
        $bi += 1;
        let byte4 = $input[$bi] as i32;
        $bi += 1;
        let byte5 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte2 & 3) << 20) | (byte3 << 12) | (byte4 << 4) | (byte5 >> 4);
        $vi += 1;

        let byte6 = $input[$bi] as i32;
        $bi += 1;
        let byte7 = $input[$bi] as i32;
        $bi += 1;
        let byte8 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte5 & 15) << 18) | (byte6 << 10) | (byte7 << 2) | (byte8 >> 6);
        $vi += 1;

        let byte9 = $input[$bi] as i32;
        $bi += 1;
        let byte10 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte8 & 63) << 16) | (byte9 << 8) | byte10;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(11)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(4)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(11).zip(values.chunks_exact_mut(4)) {
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
impl_bulk_operation_packed_encoder!(BulkOperationPacked22);
impl BulkOperation for BulkOperationPacked22 {}
