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

define_bulk_operation_packed_specialized!(BulkOperationPacked18, 18);

impl Decoder for BulkOperationPacked18 {
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
        $output[$vi] = (block0 >> 46) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 28) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 10) & 262_143) as i64;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 1_023) << 8) | (block1 >> 56)) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 38) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 20) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 2) & 262_143) as i64;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 3) << 16) | (block2 >> 48)) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 30) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 12) & 262_143) as i64;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 4_095) << 6) | (block3 >> 58)) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 40) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 22) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 4) & 262_143) as i64;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 15) << 14) | (block4 >> 50)) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 32) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 14) & 262_143) as i64;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 16_383) << 4) | (block5 >> 60)) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 42) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 24) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 6) & 262_143) as i64;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 63) << 12) | (block6 >> 52)) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 34) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 16) & 262_143) as i64;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 65_535) << 2) | (block7 >> 62)) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 44) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 26) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 8) & 262_143) as i64;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 255) << 10) | (block8 >> 54)) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 36) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 18) & 262_143) as i64;
        $vi += 1;
        $output[$vi] = (block8 & 262_143) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(9)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(32)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(9).zip(values.chunks_exact_mut(32)) {
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
        $output[$vi] = (byte0 << 10) | (byte1 << 2) | (byte2 >> 6);
        $vi += 1;

        let byte3 = $input[$bi] as i64;
        $bi += 1;
        let byte4 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte2 & 63) << 12) | (byte3 << 4) | (byte4 >> 4);
        $vi += 1;

        let byte5 = $input[$bi] as i64;
        $bi += 1;
        let byte6 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte4 & 15) << 14) | (byte5 << 6) | (byte6 >> 2);
        $vi += 1;

        let byte7 = $input[$bi] as i64;
        $bi += 1;
        let byte8 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte6 & 3) << 16) | (byte7 << 8) | byte8;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(9)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(4)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(9).zip(values.chunks_exact_mut(4)) {
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
        $output[$vi] = (block0 >> 46) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 28) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 10) & 262_143) as i32;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 1_023) << 8) | (block1 >> 56)) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 38) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 20) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 2) & 262_143) as i32;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 3) << 16) | (block2 >> 48)) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 30) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 12) & 262_143) as i32;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 4_095) << 6) | (block3 >> 58)) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 40) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 22) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 4) & 262_143) as i32;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 15) << 14) | (block4 >> 50)) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 32) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 14) & 262_143) as i32;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 16_383) << 4) | (block5 >> 60)) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 42) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 24) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 6) & 262_143) as i32;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 63) << 12) | (block6 >> 52)) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 34) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 16) & 262_143) as i32;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 65_535) << 2) | (block7 >> 62)) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 44) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 26) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 8) & 262_143) as i32;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 255) << 10) | (block8 >> 54)) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 36) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 18) & 262_143) as i32;
        $vi += 1;
        $output[$vi] = (block8 & 262_143) as i32;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(9)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(32)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(9).zip(values.chunks_exact_mut(32)) {
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
      let byte1 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte2 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = (byte0 << 10) | (byte1 << 2) | (byte2 >> 6);
      values_offset += 1;

      let byte3 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte4 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte2 & 63) << 12) | (byte3 << 4) | (byte4 >> 4);
      values_offset += 1;

      let byte5 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte6 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte4 & 15) << 14) | (byte5 << 6) | (byte6 >> 2);
      values_offset += 1;

      let byte7 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte8 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte6 & 3) << 16) | (byte7 << 8) | byte8;
      values_offset += 1;
    }
    Ok(())
  }
}
impl_bulk_operation_packed_encoder!(BulkOperationPacked18);
impl BulkOperation for BulkOperationPacked18 {}
