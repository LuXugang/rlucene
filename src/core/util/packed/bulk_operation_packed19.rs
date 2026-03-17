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
use crate::core::util::packed::bulk_operation::BulkOperation;
use crate::core::util::packed::{Decoder, Encoder};

#[derive(Default)]
pub(crate) struct BulkOperationPacked19;
impl Decoder for BulkOperationPacked19 {
  fn decode_u64_to_i64(
    &self,
    blocks: &[u64],
    mut blocks_offset: usize,
    values: &mut [i64],
    mut values_offset: usize,
    iterations: i32,
  ) {
    for _ in 0..iterations {
      let block0 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (block0 >> 45) as i64;
      values_offset += 1;
      values[values_offset] = ((block0 >> 26) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block0 >> 7) & 524_287) as i64;
      values_offset += 1;

      let block1 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block0 & 127) << 12) | (block1 >> 52)) as i64;
      values_offset += 1;
      values[values_offset] = ((block1 >> 33) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block1 >> 14) & 524_287) as i64;
      values_offset += 1;

      let block2 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block1 & 16_383) << 5) | (block2 >> 59)) as i64;
      values_offset += 1;
      values[values_offset] = ((block2 >> 40) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block2 >> 21) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block2 >> 2) & 524_287) as i64;
      values_offset += 1;

      let block3 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block2 & 3) << 17) | (block3 >> 47)) as i64;
      values_offset += 1;
      values[values_offset] = ((block3 >> 28) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block3 >> 9) & 524_287) as i64;
      values_offset += 1;

      let block4 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block3 & 511) << 10) | (block4 >> 54)) as i64;
      values_offset += 1;
      values[values_offset] = ((block4 >> 35) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block4 >> 16) & 524_287) as i64;
      values_offset += 1;

      let block5 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block4 & 65_535) << 3) | (block5 >> 61)) as i64;
      values_offset += 1;
      values[values_offset] = ((block5 >> 42) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block5 >> 23) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block5 >> 4) & 524_287) as i64;
      values_offset += 1;

      let block6 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block5 & 15) << 15) | (block6 >> 49)) as i64;
      values_offset += 1;
      values[values_offset] = ((block6 >> 30) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block6 >> 11) & 524_287) as i64;
      values_offset += 1;

      let block7 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block6 & 2_047) << 8) | (block7 >> 56)) as i64;
      values_offset += 1;
      values[values_offset] = ((block7 >> 37) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block7 >> 18) & 524_287) as i64;
      values_offset += 1;

      let block8 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block7 & 262_143) << 1) | (block8 >> 63)) as i64;
      values_offset += 1;
      values[values_offset] = ((block8 >> 44) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block8 >> 25) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block8 >> 6) & 524_287) as i64;
      values_offset += 1;

      let block9 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block8 & 63) << 13) | (block9 >> 51)) as i64;
      values_offset += 1;
      values[values_offset] = ((block9 >> 32) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block9 >> 13) & 524_287) as i64;
      values_offset += 1;

      let block10 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block9 & 8_191) << 6) | (block10 >> 58)) as i64;
      values_offset += 1;
      values[values_offset] = ((block10 >> 39) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block10 >> 20) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block10 >> 1) & 524_287) as i64;
      values_offset += 1;

      let block11 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block10 & 1) << 18) | (block11 >> 46)) as i64;
      values_offset += 1;
      values[values_offset] = ((block11 >> 27) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block11 >> 8) & 524_287) as i64;
      values_offset += 1;

      let block12 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block11 & 255) << 11) | (block12 >> 53)) as i64;
      values_offset += 1;
      values[values_offset] = ((block12 >> 34) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block12 >> 15) & 524_287) as i64;
      values_offset += 1;

      let block13 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block12 & 32_767) << 4) | (block13 >> 60)) as i64;
      values_offset += 1;
      values[values_offset] = ((block13 >> 41) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block13 >> 22) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block13 >> 3) & 524_287) as i64;
      values_offset += 1;

      let block14 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block13 & 7) << 16) | (block14 >> 48)) as i64;
      values_offset += 1;
      values[values_offset] = ((block14 >> 29) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block14 >> 10) & 524_287) as i64;
      values_offset += 1;

      let block15 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block14 & 1_023) << 9) | (block15 >> 55)) as i64;
      values_offset += 1;
      values[values_offset] = ((block15 >> 36) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block15 >> 17) & 524_287) as i64;
      values_offset += 1;

      let block16 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block15 & 131_071) << 2) | (block16 >> 62)) as i64;
      values_offset += 1;
      values[values_offset] = ((block16 >> 43) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block16 >> 24) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block16 >> 5) & 524_287) as i64;
      values_offset += 1;

      let block17 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block16 & 31) << 14) | (block17 >> 50)) as i64;
      values_offset += 1;
      values[values_offset] = ((block17 >> 31) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block17 >> 12) & 524_287) as i64;
      values_offset += 1;

      let block18 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block17 & 4_095) << 7) | (block18 >> 57)) as i64;
      values_offset += 1;
      values[values_offset] = ((block18 >> 38) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = ((block18 >> 19) & 524_287) as i64;
      values_offset += 1;
      values[values_offset] = (block18 & 524_287) as i64;
      values_offset += 1;
    }
  }
  fn decode_u8_to_i64(
    &self,
    blocks: &[u8],
    mut blocks_offset: usize,
    values: &mut [i64],
    mut values_offset: usize,
    iterations: i32,
  ) {
    for _ in 0..iterations {
      let byte0 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte1 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte2 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = (byte0 << 11) | (byte1 << 3) | (byte2 >> 5);
      values_offset += 1;

      let byte3 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte4 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte2 & 31) << 14) | (byte3 << 6) | (byte4 >> 2);
      values_offset += 1;

      let byte5 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte6 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte7 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte4 & 3) << 17) | (byte5 << 9) | (byte6 << 1) | (byte7 >> 7);
      values_offset += 1;

      let byte8 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte9 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte7 & 127) << 12) | (byte8 << 4) | (byte9 >> 4);
      values_offset += 1;

      let byte10 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte11 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte9 & 15) << 15) | (byte10 << 7) | (byte11 >> 1);
      values_offset += 1;

      let byte12 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte13 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte14 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte11 & 1) << 18) | (byte12 << 10) | (byte13 << 2) | (byte14 >> 6);
      values_offset += 1;

      let byte15 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte16 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte14 & 63) << 13) | (byte15 << 5) | (byte16 >> 3);
      values_offset += 1;

      let byte17 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte18 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte16 & 7) << 16) | (byte17 << 8) | byte18;
      values_offset += 1;
    }
  }
  fn decode_u64_to_i32(
    &self,
    blocks: &[u64],
    mut blocks_offset: usize,
    values: &mut [i32],
    mut values_offset: usize,
    iterations: i32,
  ) {
    for _ in 0..iterations {
      let block0 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (block0 >> 45) as i32;
      values_offset += 1;
      values[values_offset] = ((block0 >> 26) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block0 >> 7) & 524_287) as i32;
      values_offset += 1;

      let block1 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block0 & 127) << 12) | (block1 >> 52)) as i32;
      values_offset += 1;
      values[values_offset] = ((block1 >> 33) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block1 >> 14) & 524_287) as i32;
      values_offset += 1;

      let block2 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block1 & 16_383) << 5) | (block2 >> 59)) as i32;
      values_offset += 1;
      values[values_offset] = ((block2 >> 40) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block2 >> 21) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block2 >> 2) & 524_287) as i32;
      values_offset += 1;

      let block3 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block2 & 3) << 17) | (block3 >> 47)) as i32;
      values_offset += 1;
      values[values_offset] = ((block3 >> 28) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block3 >> 9) & 524_287) as i32;
      values_offset += 1;

      let block4 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block3 & 511) << 10) | (block4 >> 54)) as i32;
      values_offset += 1;
      values[values_offset] = ((block4 >> 35) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block4 >> 16) & 524_287) as i32;
      values_offset += 1;

      let block5 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block4 & 65_535) << 3) | (block5 >> 61)) as i32;
      values_offset += 1;
      values[values_offset] = ((block5 >> 42) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block5 >> 23) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block5 >> 4) & 524_287) as i32;
      values_offset += 1;

      let block6 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block5 & 15) << 15) | (block6 >> 49)) as i32;
      values_offset += 1;
      values[values_offset] = ((block6 >> 30) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block6 >> 11) & 524_287) as i32;
      values_offset += 1;

      let block7 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block6 & 2_047) << 8) | (block7 >> 56)) as i32;
      values_offset += 1;
      values[values_offset] = ((block7 >> 37) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block7 >> 18) & 524_287) as i32;
      values_offset += 1;

      let block8 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block7 & 262_143) << 1) | (block8 >> 63)) as i32;
      values_offset += 1;
      values[values_offset] = ((block8 >> 44) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block8 >> 25) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block8 >> 6) & 524_287) as i32;
      values_offset += 1;

      let block9 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block8 & 63) << 13) | (block9 >> 51)) as i32;
      values_offset += 1;
      values[values_offset] = ((block9 >> 32) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block9 >> 13) & 524_287) as i32;
      values_offset += 1;

      let block10 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block9 & 8_191) << 6) | (block10 >> 58)) as i32;
      values_offset += 1;
      values[values_offset] = ((block10 >> 39) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block10 >> 20) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block10 >> 1) & 524_287) as i32;
      values_offset += 1;

      let block11 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block10 & 1) << 18) | (block11 >> 46)) as i32;
      values_offset += 1;
      values[values_offset] = ((block11 >> 27) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block11 >> 8) & 524_287) as i32;
      values_offset += 1;

      let block12 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block11 & 255) << 11) | (block12 >> 53)) as i32;
      values_offset += 1;
      values[values_offset] = ((block12 >> 34) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block12 >> 15) & 524_287) as i32;
      values_offset += 1;

      let block13 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block12 & 32_767) << 4) | (block13 >> 60)) as i32;
      values_offset += 1;
      values[values_offset] = ((block13 >> 41) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block13 >> 22) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block13 >> 3) & 524_287) as i32;
      values_offset += 1;

      let block14 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block13 & 7) << 16) | (block14 >> 48)) as i32;
      values_offset += 1;
      values[values_offset] = ((block14 >> 29) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block14 >> 10) & 524_287) as i32;
      values_offset += 1;

      let block15 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block14 & 1_023) << 9) | (block15 >> 55)) as i32;
      values_offset += 1;
      values[values_offset] = ((block15 >> 36) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block15 >> 17) & 524_287) as i32;
      values_offset += 1;

      let block16 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block15 & 131_071) << 2) | (block16 >> 62)) as i32;
      values_offset += 1;
      values[values_offset] = ((block16 >> 43) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block16 >> 24) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block16 >> 5) & 524_287) as i32;
      values_offset += 1;

      let block17 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block16 & 31) << 14) | (block17 >> 50)) as i32;
      values_offset += 1;
      values[values_offset] = ((block17 >> 31) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block17 >> 12) & 524_287) as i32;
      values_offset += 1;

      let block18 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block17 & 4_095) << 7) | (block18 >> 57)) as i32;
      values_offset += 1;
      values[values_offset] = ((block18 >> 38) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = ((block18 >> 19) & 524_287) as i32;
      values_offset += 1;
      values[values_offset] = (block18 & 524_287) as i32;
      values_offset += 1;
    }
  }
  fn decode_u8_to_i32(
    &self,
    blocks: &[u8],
    mut blocks_offset: usize,
    values: &mut [i32],
    mut values_offset: usize,
    iterations: i32,
  ) {
    for _ in 0..iterations {
      let byte0 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte1 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte2 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = (byte0 << 11) | (byte1 << 3) | (byte2 >> 5);
      values_offset += 1;

      let byte3 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte4 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte2 & 31) << 14) | (byte3 << 6) | (byte4 >> 2);
      values_offset += 1;

      let byte5 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte6 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte7 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte4 & 3) << 17) | (byte5 << 9) | (byte6 << 1) | (byte7 >> 7);
      values_offset += 1;

      let byte8 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte9 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte7 & 127) << 12) | (byte8 << 4) | (byte9 >> 4);
      values_offset += 1;

      let byte10 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte11 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte9 & 15) << 15) | (byte10 << 7) | (byte11 >> 1);
      values_offset += 1;

      let byte12 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte13 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte14 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte11 & 1) << 18) | (byte12 << 10) | (byte13 << 2) | (byte14 >> 6);
      values_offset += 1;

      let byte15 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte16 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte14 & 63) << 13) | (byte15 << 5) | (byte16 >> 3);
      values_offset += 1;

      let byte17 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte18 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte16 & 7) << 16) | (byte17 << 8) | byte18;
      values_offset += 1;
    }
  }
}
impl Encoder for BulkOperationPacked19 {}
impl BulkOperation for BulkOperationPacked19 {}
