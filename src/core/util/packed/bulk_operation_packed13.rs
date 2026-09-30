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

define_bulk_operation_packed_specialized!(BulkOperationPacked13, 13);
impl Decoder for BulkOperationPacked13 {
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
        $output[$vi] = (block0 >> 51) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 38) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 25) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 12) & 8191) as i64;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 4095) << 1) | (block1 >> 63)) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 50) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 37) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 24) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 11) & 8191) as i64;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 2047) << 2) | (block2 >> 62)) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 49) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 36) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 23) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 10) & 8191) as i64;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 1023) << 3) | (block3 >> 61)) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 48) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 35) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 22) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 9) & 8191) as i64;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 511) << 4) | (block4 >> 60)) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 47) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 34) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 21) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 8) & 8191) as i64;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 255) << 5) | (block5 >> 59)) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 46) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 33) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 20) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 7) & 8191) as i64;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 127) << 6) | (block6 >> 58)) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 45) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 32) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 19) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 6) & 8191) as i64;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 63) << 7) | (block7 >> 57)) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 44) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 31) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 18) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 5) & 8191) as i64;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 31) << 8) | (block8 >> 56)) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 43) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 30) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 17) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 4) & 8191) as i64;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 15) << 9) | (block9 >> 55)) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 42) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 29) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 16) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 3) & 8191) as i64;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 7) << 10) | (block10 >> 54)) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 41) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 28) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 15) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 2) & 8191) as i64;
        $vi += 1;

        let block11 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block10 & 3) << 11) | (block11 >> 53)) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 40) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 27) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 14) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 1) & 8191) as i64;
        $vi += 1;

        let block12 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block11 & 1) << 12) | (block12 >> 52)) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 39) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 26) & 8191) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 13) & 8191) as i64;
        $vi += 1;
        $output[$vi] = (block12 & 8191) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(13)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(13).zip(values.chunks_exact_mut(64)) {
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
        $output[$vi] = (byte0 << 5) | (byte1 >> 3);
        $vi += 1;

        let byte2 = $input[$bi] as i64;
        $bi += 1;
        let byte3 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte1 & 7) << 10) | (byte2 << 2) | (byte3 >> 6);
        $vi += 1;

        let byte4 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte3 & 63) << 7) | (byte4 >> 1);
        $vi += 1;

        let byte5 = $input[$bi] as i64;
        $bi += 1;
        let byte6 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte4 & 1) << 12) | (byte5 << 4) | (byte6 >> 4);
        $vi += 1;

        let byte7 = $input[$bi] as i64;
        $bi += 1;
        let byte8 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte6 & 15) << 9) | (byte7 << 1) | (byte8 >> 7);
        $vi += 1;

        let byte9 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte8 & 127) << 6) | (byte9 >> 2);
        $vi += 1;

        let byte10 = $input[$bi] as i64;
        $bi += 1;
        let byte11 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte9 & 3) << 11) | (byte10 << 3) | (byte11 >> 5);
        $vi += 1;

        let byte12 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte11 & 31) << 8) | byte12;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(13)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(13).zip(values.chunks_exact_mut(8)) {
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
        $output[$vi] = (block0 >> 51) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 38) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 25) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 12) & 8191) as i32;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 4095) << 1) | (block1 >> 63)) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 50) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 37) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 24) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 11) & 8191) as i32;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 2047) << 2) | (block2 >> 62)) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 49) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 36) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 23) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 10) & 8191) as i32;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 1023) << 3) | (block3 >> 61)) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 48) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 35) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 22) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 9) & 8191) as i32;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 511) << 4) | (block4 >> 60)) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 47) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 34) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 21) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 8) & 8191) as i32;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 255) << 5) | (block5 >> 59)) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 46) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 33) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 20) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 7) & 8191) as i32;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 127) << 6) | (block6 >> 58)) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 45) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 32) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 19) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 6) & 8191) as i32;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 63) << 7) | (block7 >> 57)) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 44) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 31) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 18) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 5) & 8191) as i32;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 31) << 8) | (block8 >> 56)) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 43) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 30) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 17) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 4) & 8191) as i32;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 15) << 9) | (block9 >> 55)) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 42) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 29) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 16) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 3) & 8191) as i32;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 7) << 10) | (block10 >> 54)) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 41) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 28) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 15) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 2) & 8191) as i32;
        $vi += 1;

        let block11 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block10 & 3) << 11) | (block11 >> 53)) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 40) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 27) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 14) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 1) & 8191) as i32;
        $vi += 1;

        let block12 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block11 & 1) << 12) | (block12 >> 52)) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 39) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 26) & 8191) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 13) & 8191) as i32;
        $vi += 1;
        $output[$vi] = (block12 & 8191) as i32;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(13)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(13).zip(values.chunks_exact_mut(64)) {
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
        $output[$vi] = (byte0 << 5) | (byte1 >> 3);
        $vi += 1;

        let byte2 = $input[$bi] as i32;
        $bi += 1;
        let byte3 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte1 & 7) << 10) | (byte2 << 2) | (byte3 >> 6);
        $vi += 1;

        let byte4 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte3 & 63) << 7) | (byte4 >> 1);
        $vi += 1;

        let byte5 = $input[$bi] as i32;
        $bi += 1;
        let byte6 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte4 & 1) << 12) | (byte5 << 4) | (byte6 >> 4);
        $vi += 1;

        let byte7 = $input[$bi] as i32;
        $bi += 1;
        let byte8 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte6 & 15) << 9) | (byte7 << 1) | (byte8 >> 7);
        $vi += 1;

        let byte9 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte8 & 127) << 6) | (byte9 >> 2);
        $vi += 1;

        let byte10 = $input[$bi] as i32;
        $bi += 1;
        let byte11 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte9 & 3) << 11) | (byte10 << 3) | (byte11 >> 5);
        $vi += 1;

        let byte12 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte11 & 31) << 8) | byte12;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(13)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(13).zip(values.chunks_exact_mut(8)) {
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
impl_bulk_operation_packed_encoder!(BulkOperationPacked13);
impl BulkOperation for BulkOperationPacked13 {}
