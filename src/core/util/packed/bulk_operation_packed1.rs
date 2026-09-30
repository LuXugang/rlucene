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

define_bulk_operation_packed_specialized!(BulkOperationPacked1, 1);
impl Decoder for BulkOperationPacked1 {
  delegate_bulk_operation_packed_decoder_counts!();
  /// Decodes blocks of type `u64` into `u64` values.
  fn decode_u64_to_i64(
    &self,
    blocks: &[u64],
    mut blocks_offset: usize,
    values: &mut [i64],
    mut values_offset: usize,
    iterations: usize,
  ) {
    for _ in 0..iterations {
      let block = blocks[blocks_offset];
      blocks_offset += 1;

      for shift in (0..64).rev() {
        values[values_offset] = ((block >> shift) & 1) as i64;
        values_offset += 1;
      }
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
        let block = $input[$bi];
        $bi += 1;

        $output[$vi] = ((block >> 7) & 1) as i64;
        $output[$vi + 1] = ((block >> 6) & 1) as i64;
        $output[$vi + 2] = ((block >> 5) & 1) as i64;
        $output[$vi + 3] = ((block >> 4) & 1) as i64;
        $output[$vi + 4] = ((block >> 3) & 1) as i64;
        $output[$vi + 5] = ((block >> 2) & 1) as i64;
        $output[$vi + 6] = ((block >> 1) & 1) as i64;
        $output[$vi + 7] = (block & 1) as i64;

        $vi += 8;
      }};
    }

    if iterations >= 2 {
      if let (Some(blocks_end), Some(values_end)) = (
        iterations
          .checked_mul(1)
          .and_then(|n| blocks_offset.checked_add(n)),
        iterations
          .checked_mul(8)
          .and_then(|n| values_offset.checked_add(n)),
      ) {
        if let (Some(blocks), Some(values)) = (
          blocks.get(blocks_offset..blocks_end),
          values.get_mut(values_offset..values_end),
        ) {
          for (blocks, values) in blocks.chunks_exact(1).zip(values.chunks_exact_mut(8)) {
            let mut blocks_offset = 0;
            let mut values_offset = 0;
            decode_group!(blocks, blocks_offset, values, values_offset);
            let _ = (blocks_offset, values_offset);
          }
          return;
        }
      }
    }

    for _ in 0..iterations {
      decode_group!(blocks, blocks_offset, values, values_offset);
    }
  }

  /// Decodes blocks of type `u64` into `i32` values.
  fn decode_u64_to_i32(
    &self,
    blocks: &[u64],
    mut blocks_offset: usize,
    values: &mut [i32],
    mut values_offset: usize,
    iterations: usize,
  ) -> Result<()> {
    for _ in 0..iterations {
      let block = blocks[blocks_offset];
      blocks_offset += 1;

      for shift in (0..64).rev() {
        values[values_offset] = ((block >> shift) & 1) as i32;
        values_offset += 1;
      }
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
        let block = $input[$bi];
        $bi += 1;

        $output[$vi] = ((block >> 7) & 1) as i32;
        $output[$vi + 1] = ((block >> 6) & 1) as i32;
        $output[$vi + 2] = ((block >> 5) & 1) as i32;
        $output[$vi + 3] = ((block >> 4) & 1) as i32;
        $output[$vi + 4] = ((block >> 3) & 1) as i32;
        $output[$vi + 5] = ((block >> 2) & 1) as i32;
        $output[$vi + 6] = ((block >> 1) & 1) as i32;
        $output[$vi + 7] = (block & 1) as i32;

        $vi += 8;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(1)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(1).zip(values.chunks_exact_mut(8)) {
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
impl_bulk_operation_packed_encoder!(BulkOperationPacked1);
impl BulkOperation for BulkOperationPacked1 {}
