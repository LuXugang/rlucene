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

define_bulk_operation_packed_specialized!(BulkOperationPacked12, 12);
impl Decoder for BulkOperationPacked12 {
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
        $output[$vi] = (block0 >> 52) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 40) & 4095) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 28) & 4095) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 16) & 4095) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 4) & 4095) as i64;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 15) << 8) | (block1 >> 56)) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 44) & 4095) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 32) & 4095) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 20) & 4095) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 8) & 4095) as i64;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 255) << 4) | (block2 >> 60)) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 48) & 4095) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 36) & 4095) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 24) & 4095) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 12) & 4095) as i64;
        $vi += 1;
        $output[$vi] = (block2 & 4095) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(3)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(16)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(3).zip(values.chunks_exact_mut(16)) {
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
  fn decode_u8_to_i64(
    &self,
    blocks: &[u8],
    mut blocks_offset: usize,
    values: &mut [i64],
    mut values_offset: usize,
    iterations: usize,
  ) {
    for _ in 0..iterations {
      let byte0 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte1 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = (byte0 << 4) | (byte1 >> 4);
      values_offset += 1;

      let byte2 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte1 & 15) << 8) | byte2;
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
        $output[$vi] = (block0 >> 52) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 40) & 4095) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 28) & 4095) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 16) & 4095) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 4) & 4095) as i32;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 15) << 8) | (block1 >> 56)) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 44) & 4095) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 32) & 4095) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 20) & 4095) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 8) & 4095) as i32;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 255) << 4) | (block2 >> 60)) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 48) & 4095) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 36) & 4095) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 24) & 4095) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 12) & 4095) as i32;
        $vi += 1;
        $output[$vi] = (block2 & 4095) as i32;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(3)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(16)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(3).zip(values.chunks_exact_mut(16)) {
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
      values[values_offset] = (byte0 << 4) | (byte1 >> 4);
      values_offset += 1;

      let byte2 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte1 & 15) << 8) | byte2;
      values_offset += 1;
    }
    Ok(())
  }
}
impl_bulk_operation_packed_encoder!(BulkOperationPacked12);
impl BulkOperation for BulkOperationPacked12 {}
