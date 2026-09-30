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

define_bulk_operation_packed_specialized!(BulkOperationPacked14, 14);
impl Decoder for BulkOperationPacked14 {
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
        $output[$vi] = (block0 >> 50) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 36) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 22) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 8) & 16383) as i64;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 255) << 6) | (block1 >> 58)) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 44) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 30) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 16) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 2) & 16383) as i64;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 3) << 12) | (block2 >> 52)) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 38) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 24) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 10) & 16383) as i64;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 1023) << 4) | (block3 >> 60)) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 46) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 32) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 18) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 4) & 16383) as i64;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 15) << 10) | (block4 >> 54)) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 40) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 26) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 12) & 16383) as i64;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 4095) << 2) | (block5 >> 62)) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 48) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 34) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 20) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 6) & 16383) as i64;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 63) << 8) | (block6 >> 56)) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 42) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 28) & 16383) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 14) & 16383) as i64;
        $vi += 1;
        $output[$vi] = (block6 & 16383) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(7)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(32)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(7).zip(values.chunks_exact_mut(32)) {
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
  #[allow(clippy::chunks_exact_to_as_chunks)]
  fn decode_u8_to_i64(
    &self,
    blocks: &[u8],
    mut blocks_offset: usize,
    values: &mut [i64],
    mut values_offset: usize,
    iterations: usize,
  ) {
    // A complete seven-byte group forms four 14-bit values. Keep the
    // original per-access sequence when either range is incomplete.
    if iterations >= 2
      && let (Some(blocks_end), Some(values_end)) = (
        iterations
          .checked_mul(7)
          .and_then(|n| blocks_offset.checked_add(n)),
        iterations
          .checked_mul(4)
          .and_then(|n| values_offset.checked_add(n)),
      )
      && let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      )
    {
      for (blocks, values) in blocks.chunks_exact(7).zip(values.chunks_exact_mut(4)) {
        let bits = u64::from_be_bytes([
          0, blocks[0], blocks[1], blocks[2], blocks[3], blocks[4], blocks[5], blocks[6],
        ]);
        values[0] = ((bits >> 42) & 16_383) as i64;
        values[1] = ((bits >> 28) & 16_383) as i64;
        values[2] = ((bits >> 14) & 16_383) as i64;
        values[3] = (bits & 16_383) as i64;
      }
      return;
    }

    for _ in 0..iterations {
      let byte0 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte1 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = (byte0 << 6) | (byte1 >> 2);
      values_offset += 1;

      let byte2 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte3 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte1 & 3) << 12) | (byte2 << 4) | (byte3 >> 4);
      values_offset += 1;

      let byte4 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte5 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte3 & 15) << 10) | (byte4 << 2) | (byte5 >> 6);
      values_offset += 1;

      let byte6 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte5 & 63) << 8) | byte6;
      values_offset += 1;
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
        $output[$vi] = (block0 >> 50) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 36) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 22) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 8) & 16383) as i32;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 255) << 6) | (block1 >> 58)) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 44) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 30) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 16) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 2) & 16383) as i32;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 3) << 12) | (block2 >> 52)) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 38) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 24) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 10) & 16383) as i32;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 1023) << 4) | (block3 >> 60)) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 46) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 32) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 18) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 4) & 16383) as i32;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 15) << 10) | (block4 >> 54)) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 40) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 26) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 12) & 16383) as i32;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 4095) << 2) | (block5 >> 62)) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 48) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 34) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 20) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 6) & 16383) as i32;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 63) << 8) | (block6 >> 56)) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 42) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 28) & 16383) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 14) & 16383) as i32;
        $vi += 1;
        $output[$vi] = (block6 & 16383) as i32;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(7)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(32)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(7).zip(values.chunks_exact_mut(32)) {
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
      values[values_offset] = (byte0 << 6) | (byte1 >> 2);
      values_offset += 1;

      let byte2 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte3 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte1 & 3) << 12) | (byte2 << 4) | (byte3 >> 4);
      values_offset += 1;

      let byte4 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte5 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte3 & 15) << 10) | (byte4 << 2) | (byte5 >> 6);
      values_offset += 1;

      let byte6 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte5 & 63) << 8) | byte6;
      values_offset += 1;
    }
    Ok(())
  }
}
impl_bulk_operation_packed_encoder!(BulkOperationPacked14);
impl BulkOperation for BulkOperationPacked14 {}
