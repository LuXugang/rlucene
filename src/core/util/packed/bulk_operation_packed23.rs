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

define_bulk_operation_packed_specialized!(BulkOperationPacked23, 23);
impl Decoder for BulkOperationPacked23 {
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
        $output[$vi] = (block0 >> 41) as i64;
        $vi += 1;
        $output[$vi] = ((block0 >> 18) & 0x7FFFFF) as i64;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 0x3FFFF) << 5) | (block1 >> 59)) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 36) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block1 >> 13) & 0x7FFFFF) as i64;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 0x1FFF) << 10) | (block2 >> 54)) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 31) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block2 >> 8) & 0x7FFFFF) as i64;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 0xFF) << 15) | (block3 >> 49)) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 26) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block3 >> 3) & 0x7FFFFF) as i64;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 0x7) << 20) | (block4 >> 44)) as i64;
        $vi += 1;
        $output[$vi] = ((block4 >> 21) & 0x7FFFFF) as i64;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 0x1FFFFF) << 2) | (block5 >> 62)) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 39) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block5 >> 16) & 0x7FFFFF) as i64;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 0xFFFF) << 7) | (block6 >> 57)) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 34) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block6 >> 11) & 0x7FFFFF) as i64;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 0x7FF) << 12) | (block7 >> 52)) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 29) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block7 >> 6) & 0x7FFFFF) as i64;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 0x3F) << 17) | (block8 >> 47)) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 24) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block8 >> 1) & 0x7FFFFF) as i64;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 0x1) << 22) | (block9 >> 42)) as i64;
        $vi += 1;
        $output[$vi] = ((block9 >> 19) & 0x7FFFFF) as i64;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 0x7FFFF) << 4) | (block10 >> 60)) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 37) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block10 >> 14) & 0x7FFFFF) as i64;
        $vi += 1;

        let block11 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block10 & 0x3FFF) << 9) | (block11 >> 55)) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 32) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block11 >> 9) & 0x7FFFFF) as i64;
        $vi += 1;

        let block12 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block11 & 0x1FF) << 14) | (block12 >> 50)) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 27) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block12 >> 4) & 0x7FFFFF) as i64;
        $vi += 1;

        let block13 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block12 & 0xF) << 19) | (block13 >> 45)) as i64;
        $vi += 1;
        $output[$vi] = ((block13 >> 22) & 0x7FFFFF) as i64;
        $vi += 1;

        let block14 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block13 & 0x3FFFFF) << 1) | (block14 >> 63)) as i64;
        $vi += 1;
        $output[$vi] = ((block14 >> 40) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block14 >> 17) & 0x7FFFFF) as i64;
        $vi += 1;

        let block15 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block14 & 0x1FFFF) << 6) | (block15 >> 58)) as i64;
        $vi += 1;
        $output[$vi] = ((block15 >> 35) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block15 >> 12) & 0x7FFFFF) as i64;
        $vi += 1;

        let block16 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block15 & 0xFFF) << 11) | (block16 >> 53)) as i64;
        $vi += 1;
        $output[$vi] = ((block16 >> 30) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block16 >> 7) & 0x7FFFFF) as i64;
        $vi += 1;

        let block17 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block16 & 0x7F) << 16) | (block17 >> 48)) as i64;
        $vi += 1;
        $output[$vi] = ((block17 >> 25) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block17 >> 2) & 0x7FFFFF) as i64;
        $vi += 1;

        let block18 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block17 & 0x3) << 21) | (block18 >> 43)) as i64;
        $vi += 1;
        $output[$vi] = ((block18 >> 20) & 0x7FFFFF) as i64;
        $vi += 1;

        let block19 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block18 & 0xFFFFF) << 3) | (block19 >> 61)) as i64;
        $vi += 1;
        $output[$vi] = ((block19 >> 38) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block19 >> 15) & 0x7FFFFF) as i64;
        $vi += 1;

        let block20 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block19 & 0x7FFF) << 8) | (block20 >> 56)) as i64;
        $vi += 1;
        $output[$vi] = ((block20 >> 33) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block20 >> 10) & 0x7FFFFF) as i64;
        $vi += 1;

        let block21 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block20 & 0x3FF) << 13) | (block21 >> 51)) as i64;
        $vi += 1;
        $output[$vi] = ((block21 >> 28) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = ((block21 >> 5) & 0x7FFFFF) as i64;
        $vi += 1;

        let block22 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block21 & 0x1F) << 18) | (block22 >> 46)) as i64;
        $vi += 1;
        $output[$vi] = ((block22 >> 23) & 0x7FFFFF) as i64;
        $vi += 1;
        $output[$vi] = (block22 & 0x7FFFFF) as i64;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(23)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(23).zip(values.chunks_exact_mut(64)) {
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
        $output[$vi] = (byte0 << 15) | (byte1 << 7) | (byte2 >> 1);
        $vi += 1;

        let byte3 = $input[$bi] as i64;
        $bi += 1;
        let byte4 = $input[$bi] as i64;
        $bi += 1;
        let byte5 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte2 & 1) << 22) | (byte3 << 14) | (byte4 << 6) | (byte5 >> 2);
        $vi += 1;

        let byte6 = $input[$bi] as i64;
        $bi += 1;
        let byte7 = $input[$bi] as i64;
        $bi += 1;
        let byte8 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte5 & 3) << 21) | (byte6 << 13) | (byte7 << 5) | (byte8 >> 3);
        $vi += 1;

        let byte9 = $input[$bi] as i64;
        $bi += 1;
        let byte10 = $input[$bi] as i64;
        $bi += 1;
        let byte11 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte8 & 7) << 20) | (byte9 << 12) | (byte10 << 4) | (byte11 >> 4);
        $vi += 1;

        let byte12 = $input[$bi] as i64;
        $bi += 1;
        let byte13 = $input[$bi] as i64;
        $bi += 1;
        let byte14 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte11 & 15) << 19) | (byte12 << 11) | (byte13 << 3) | (byte14 >> 5);
        $vi += 1;

        let byte15 = $input[$bi] as i64;
        $bi += 1;
        let byte16 = $input[$bi] as i64;
        $bi += 1;
        let byte17 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte14 & 31) << 18) | (byte15 << 10) | (byte16 << 2) | (byte17 >> 6);
        $vi += 1;

        let byte18 = $input[$bi] as i64;
        $bi += 1;
        let byte19 = $input[$bi] as i64;
        $bi += 1;
        let byte20 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte17 & 63) << 17) | (byte18 << 9) | (byte19 << 1) | (byte20 >> 7);
        $vi += 1;

        let byte21 = $input[$bi] as i64;
        $bi += 1;
        let byte22 = $input[$bi] as i64;
        $bi += 1;
        $output[$vi] = ((byte20 & 127) << 16) | (byte21 << 8) | byte22;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(23)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(23).zip(values.chunks_exact_mut(8)) {
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
        $output[$vi] = (block0 >> 41) as i32;
        $vi += 1;
        $output[$vi] = ((block0 >> 18) & 0x7FFFFF) as i32;
        $vi += 1;

        let block1 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block0 & 0x3FFFF) << 5) | (block1 >> 59)) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 36) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block1 >> 13) & 0x7FFFFF) as i32;
        $vi += 1;

        let block2 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block1 & 0x1FFF) << 10) | (block2 >> 54)) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 31) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block2 >> 8) & 0x7FFFFF) as i32;
        $vi += 1;

        let block3 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block2 & 0xFF) << 15) | (block3 >> 49)) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 26) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block3 >> 3) & 0x7FFFFF) as i32;
        $vi += 1;

        let block4 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block3 & 0x7) << 20) | (block4 >> 44)) as i32;
        $vi += 1;
        $output[$vi] = ((block4 >> 21) & 0x7FFFFF) as i32;
        $vi += 1;

        let block5 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block4 & 0x1FFFFF) << 2) | (block5 >> 62)) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 39) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block5 >> 16) & 0x7FFFFF) as i32;
        $vi += 1;

        let block6 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block5 & 0xFFFF) << 7) | (block6 >> 57)) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 34) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block6 >> 11) & 0x7FFFFF) as i32;
        $vi += 1;

        let block7 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block6 & 0x7FF) << 12) | (block7 >> 52)) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 29) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block7 >> 6) & 0x7FFFFF) as i32;
        $vi += 1;

        let block8 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block7 & 0x3F) << 17) | (block8 >> 47)) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 24) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block8 >> 1) & 0x7FFFFF) as i32;
        $vi += 1;

        let block9 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block8 & 0x1) << 22) | (block9 >> 42)) as i32;
        $vi += 1;
        $output[$vi] = ((block9 >> 19) & 0x7FFFFF) as i32;
        $vi += 1;

        let block10 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block9 & 0x7FFFF) << 4) | (block10 >> 60)) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 37) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block10 >> 14) & 0x7FFFFF) as i32;
        $vi += 1;

        let block11 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block10 & 0x3FFF) << 9) | (block11 >> 55)) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 32) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block11 >> 9) & 0x7FFFFF) as i32;
        $vi += 1;

        let block12 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block11 & 0x1FF) << 14) | (block12 >> 50)) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 27) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block12 >> 4) & 0x7FFFFF) as i32;
        $vi += 1;

        let block13 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block12 & 0xF) << 19) | (block13 >> 45)) as i32;
        $vi += 1;
        $output[$vi] = ((block13 >> 22) & 0x7FFFFF) as i32;
        $vi += 1;

        let block14 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block13 & 0x3FFFFF) << 1) | (block14 >> 63)) as i32;
        $vi += 1;
        $output[$vi] = ((block14 >> 40) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block14 >> 17) & 0x7FFFFF) as i32;
        $vi += 1;

        let block15 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block14 & 0x1FFFF) << 6) | (block15 >> 58)) as i32;
        $vi += 1;
        $output[$vi] = ((block15 >> 35) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block15 >> 12) & 0x7FFFFF) as i32;
        $vi += 1;

        let block16 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block15 & 0xFFF) << 11) | (block16 >> 53)) as i32;
        $vi += 1;
        $output[$vi] = ((block16 >> 30) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block16 >> 7) & 0x7FFFFF) as i32;
        $vi += 1;

        let block17 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block16 & 0x7F) << 16) | (block17 >> 48)) as i32;
        $vi += 1;
        $output[$vi] = ((block17 >> 25) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block17 >> 2) & 0x7FFFFF) as i32;
        $vi += 1;

        let block18 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block17 & 0x3) << 21) | (block18 >> 43)) as i32;
        $vi += 1;
        $output[$vi] = ((block18 >> 20) & 0x7FFFFF) as i32;
        $vi += 1;

        let block19 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block18 & 0xFFFFF) << 3) | (block19 >> 61)) as i32;
        $vi += 1;
        $output[$vi] = ((block19 >> 38) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block19 >> 15) & 0x7FFFFF) as i32;
        $vi += 1;

        let block20 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block19 & 0x7FFF) << 8) | (block20 >> 56)) as i32;
        $vi += 1;
        $output[$vi] = ((block20 >> 33) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block20 >> 10) & 0x7FFFFF) as i32;
        $vi += 1;

        let block21 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block20 & 0x3FF) << 13) | (block21 >> 51)) as i32;
        $vi += 1;
        $output[$vi] = ((block21 >> 28) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = ((block21 >> 5) & 0x7FFFFF) as i32;
        $vi += 1;

        let block22 = $input[$bi];
        $bi += 1;
        $output[$vi] = (((block21 & 0x1F) << 18) | (block22 >> 46)) as i32;
        $vi += 1;
        $output[$vi] = ((block22 >> 23) & 0x7FFFFF) as i32;
        $vi += 1;
        $output[$vi] = (block22 & 0x7FFFFF) as i32;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(23)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(64)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(23).zip(values.chunks_exact_mut(64)) {
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
        $output[$vi] = (byte0 << 15) | (byte1 << 7) | (byte2 >> 1);
        $vi += 1;

        let byte3 = $input[$bi] as i32;
        $bi += 1;
        let byte4 = $input[$bi] as i32;
        $bi += 1;
        let byte5 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte2 & 1) << 22) | (byte3 << 14) | (byte4 << 6) | (byte5 >> 2);
        $vi += 1;

        let byte6 = $input[$bi] as i32;
        $bi += 1;
        let byte7 = $input[$bi] as i32;
        $bi += 1;
        let byte8 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte5 & 3) << 21) | (byte6 << 13) | (byte7 << 5) | (byte8 >> 3);
        $vi += 1;

        let byte9 = $input[$bi] as i32;
        $bi += 1;
        let byte10 = $input[$bi] as i32;
        $bi += 1;
        let byte11 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte8 & 7) << 20) | (byte9 << 12) | (byte10 << 4) | (byte11 >> 4);
        $vi += 1;

        let byte12 = $input[$bi] as i32;
        $bi += 1;
        let byte13 = $input[$bi] as i32;
        $bi += 1;
        let byte14 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte11 & 15) << 19) | (byte12 << 11) | (byte13 << 3) | (byte14 >> 5);
        $vi += 1;

        let byte15 = $input[$bi] as i32;
        $bi += 1;
        let byte16 = $input[$bi] as i32;
        $bi += 1;
        let byte17 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte14 & 31) << 18) | (byte15 << 10) | (byte16 << 2) | (byte17 >> 6);
        $vi += 1;

        let byte18 = $input[$bi] as i32;
        $bi += 1;
        let byte19 = $input[$bi] as i32;
        $bi += 1;
        let byte20 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte17 & 63) << 17) | (byte18 << 9) | (byte19 << 1) | (byte20 >> 7);
        $vi += 1;

        let byte21 = $input[$bi] as i32;
        $bi += 1;
        let byte22 = $input[$bi] as i32;
        $bi += 1;
        $output[$vi] = ((byte20 & 127) << 16) | (byte21 << 8) | byte22;
        $vi += 1;
      }};
    }

    if let (Some(blocks_end), Some(values_end)) = (
      iterations
        .checked_mul(23)
        .and_then(|n| blocks_offset.checked_add(n)),
      iterations
        .checked_mul(8)
        .and_then(|n| values_offset.checked_add(n)),
    ) {
      if let (Some(blocks), Some(values)) = (
        blocks.get(blocks_offset..blocks_end),
        values.get_mut(values_offset..values_end),
      ) {
        for (blocks, values) in blocks.chunks_exact(23).zip(values.chunks_exact_mut(8)) {
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
impl_bulk_operation_packed_encoder!(BulkOperationPacked23);
impl BulkOperation for BulkOperationPacked23 {}
