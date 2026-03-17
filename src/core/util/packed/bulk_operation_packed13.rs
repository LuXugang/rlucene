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
pub(crate) struct BulkOperationPacked13;
impl Decoder for BulkOperationPacked13 {
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
      values[values_offset] = (block0 >> 51) as i64;
      values_offset += 1;
      values[values_offset] = ((block0 >> 38) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block0 >> 25) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block0 >> 12) & 8191) as i64;
      values_offset += 1;

      let block1 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block0 & 4095) << 1) | (block1 >> 63)) as i64;
      values_offset += 1;
      values[values_offset] = ((block1 >> 50) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block1 >> 37) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block1 >> 24) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block1 >> 11) & 8191) as i64;
      values_offset += 1;

      let block2 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block1 & 2047) << 2) | (block2 >> 62)) as i64;
      values_offset += 1;
      values[values_offset] = ((block2 >> 49) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block2 >> 36) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block2 >> 23) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block2 >> 10) & 8191) as i64;
      values_offset += 1;

      let block3 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block2 & 1023) << 3) | (block3 >> 61)) as i64;
      values_offset += 1;
      values[values_offset] = ((block3 >> 48) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block3 >> 35) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block3 >> 22) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block3 >> 9) & 8191) as i64;
      values_offset += 1;

      let block4 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block3 & 511) << 4) | (block4 >> 60)) as i64;
      values_offset += 1;
      values[values_offset] = ((block4 >> 47) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block4 >> 34) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block4 >> 21) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block4 >> 8) & 8191) as i64;
      values_offset += 1;

      let block5 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block4 & 255) << 5) | (block5 >> 59)) as i64;
      values_offset += 1;
      values[values_offset] = ((block5 >> 46) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block5 >> 33) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block5 >> 20) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block5 >> 7) & 8191) as i64;
      values_offset += 1;

      let block6 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block5 & 127) << 6) | (block6 >> 58)) as i64;
      values_offset += 1;
      values[values_offset] = ((block6 >> 45) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block6 >> 32) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block6 >> 19) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block6 >> 6) & 8191) as i64;
      values_offset += 1;

      let block7 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block6 & 63) << 7) | (block7 >> 57)) as i64;
      values_offset += 1;
      values[values_offset] = ((block7 >> 44) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block7 >> 31) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block7 >> 18) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block7 >> 5) & 8191) as i64;
      values_offset += 1;

      let block8 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block7 & 31) << 8) | (block8 >> 56)) as i64;
      values_offset += 1;
      values[values_offset] = ((block8 >> 43) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block8 >> 30) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block8 >> 17) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block8 >> 4) & 8191) as i64;
      values_offset += 1;

      let block9 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block8 & 15) << 9) | (block9 >> 55)) as i64;
      values_offset += 1;
      values[values_offset] = ((block9 >> 42) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block9 >> 29) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block9 >> 16) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block9 >> 3) & 8191) as i64;
      values_offset += 1;

      let block10 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block9 & 7) << 10) | (block10 >> 54)) as i64;
      values_offset += 1;
      values[values_offset] = ((block10 >> 41) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block10 >> 28) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block10 >> 15) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block10 >> 2) & 8191) as i64;
      values_offset += 1;

      let block11 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block10 & 3) << 11) | (block11 >> 53)) as i64;
      values_offset += 1;
      values[values_offset] = ((block11 >> 40) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block11 >> 27) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block11 >> 14) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block11 >> 1) & 8191) as i64;
      values_offset += 1;

      let block12 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block11 & 1) << 12) | (block12 >> 52)) as i64;
      values_offset += 1;
      values[values_offset] = ((block12 >> 39) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block12 >> 26) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = ((block12 >> 13) & 8191) as i64;
      values_offset += 1;
      values[values_offset] = (block12 & 8191) as i64;
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
      values[values_offset] = (byte0 << 5) | (byte1 >> 3);
      values_offset += 1;

      let byte2 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte3 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte1 & 7) << 10) | (byte2 << 2) | (byte3 >> 6);
      values_offset += 1;

      let byte4 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte3 & 63) << 7) | (byte4 >> 1);
      values_offset += 1;

      let byte5 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte6 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte4 & 1) << 12) | (byte5 << 4) | (byte6 >> 4);
      values_offset += 1;

      let byte7 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte8 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte6 & 15) << 9) | (byte7 << 1) | (byte8 >> 7);
      values_offset += 1;

      let byte9 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte8 & 127) << 6) | (byte9 >> 2);
      values_offset += 1;

      let byte10 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      let byte11 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte9 & 3) << 11) | (byte10 << 3) | (byte11 >> 5);
      values_offset += 1;

      let byte12 = blocks[blocks_offset] as i64;
      blocks_offset += 1;
      values[values_offset] = ((byte11 & 31) << 8) | byte12;
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
      values[values_offset] = (block0 >> 51) as i32;
      values_offset += 1;
      values[values_offset] = ((block0 >> 38) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block0 >> 25) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block0 >> 12) & 8191) as i32;
      values_offset += 1;

      let block1 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block0 & 4095) << 1) | (block1 >> 63)) as i32;
      values_offset += 1;
      values[values_offset] = ((block1 >> 50) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block1 >> 37) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block1 >> 24) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block1 >> 11) & 8191) as i32;
      values_offset += 1;

      let block2 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block1 & 2047) << 2) | (block2 >> 62)) as i32;
      values_offset += 1;
      values[values_offset] = ((block2 >> 49) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block2 >> 36) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block2 >> 23) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block2 >> 10) & 8191) as i32;
      values_offset += 1;

      let block3 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block2 & 1023) << 3) | (block3 >> 61)) as i32;
      values_offset += 1;
      values[values_offset] = ((block3 >> 48) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block3 >> 35) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block3 >> 22) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block3 >> 9) & 8191) as i32;
      values_offset += 1;

      let block4 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block3 & 511) << 4) | (block4 >> 60)) as i32;
      values_offset += 1;
      values[values_offset] = ((block4 >> 47) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block4 >> 34) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block4 >> 21) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block4 >> 8) & 8191) as i32;
      values_offset += 1;

      let block5 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block4 & 255) << 5) | (block5 >> 59)) as i32;
      values_offset += 1;
      values[values_offset] = ((block5 >> 46) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block5 >> 33) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block5 >> 20) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block5 >> 7) & 8191) as i32;
      values_offset += 1;

      let block6 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block5 & 127) << 6) | (block6 >> 58)) as i32;
      values_offset += 1;
      values[values_offset] = ((block6 >> 45) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block6 >> 32) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block6 >> 19) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block6 >> 6) & 8191) as i32;
      values_offset += 1;

      let block7 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block6 & 63) << 7) | (block7 >> 57)) as i32;
      values_offset += 1;
      values[values_offset] = ((block7 >> 44) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block7 >> 31) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block7 >> 18) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block7 >> 5) & 8191) as i32;
      values_offset += 1;

      let block8 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block7 & 31) << 8) | (block8 >> 56)) as i32;
      values_offset += 1;
      values[values_offset] = ((block8 >> 43) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block8 >> 30) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block8 >> 17) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block8 >> 4) & 8191) as i32;
      values_offset += 1;

      let block9 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block8 & 15) << 9) | (block9 >> 55)) as i32;
      values_offset += 1;
      values[values_offset] = ((block9 >> 42) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block9 >> 29) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block9 >> 16) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block9 >> 3) & 8191) as i32;
      values_offset += 1;

      let block10 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block9 & 7) << 10) | (block10 >> 54)) as i32;
      values_offset += 1;
      values[values_offset] = ((block10 >> 41) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block10 >> 28) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block10 >> 15) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block10 >> 2) & 8191) as i32;
      values_offset += 1;

      let block11 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block10 & 3) << 11) | (block11 >> 53)) as i32;
      values_offset += 1;
      values[values_offset] = ((block11 >> 40) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block11 >> 27) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block11 >> 14) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block11 >> 1) & 8191) as i32;
      values_offset += 1;

      let block12 = blocks[blocks_offset];
      blocks_offset += 1;
      values[values_offset] = (((block11 & 1) << 12) | (block12 >> 52)) as i32;
      values_offset += 1;
      values[values_offset] = ((block12 >> 39) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block12 >> 26) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = ((block12 >> 13) & 8191) as i32;
      values_offset += 1;
      values[values_offset] = (block12 & 8191) as i32;
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
      values[values_offset] = (byte0 << 5) | (byte1 >> 3);
      values_offset += 1;

      let byte2 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte3 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte1 & 7) << 10) | (byte2 << 2) | (byte3 >> 6);
      values_offset += 1;

      let byte4 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte3 & 63) << 7) | (byte4 >> 1);
      values_offset += 1;

      let byte5 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte6 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte4 & 1) << 12) | (byte5 << 4) | (byte6 >> 4);
      values_offset += 1;

      let byte7 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte8 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte6 & 15) << 9) | (byte7 << 1) | (byte8 >> 7);
      values_offset += 1;

      let byte9 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte8 & 127) << 6) | (byte9 >> 2);
      values_offset += 1;

      let byte10 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      let byte11 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte9 & 3) << 11) | (byte10 << 3) | (byte11 >> 5);
      values_offset += 1;

      let byte12 = blocks[blocks_offset] as i32;
      blocks_offset += 1;
      values[values_offset] = ((byte11 & 31) << 8) | byte12;
      values_offset += 1;
    }
  }
}
impl Encoder for BulkOperationPacked13 {}
impl BulkOperation for BulkOperationPacked13 {}
