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
use crate::core::util::packed::bulk_operation_packed::BulkOperationPacked;
use crate::core::util::packed::bulk_operation_packed_enum::BulkOperationPackedEnum;
use crate::core::util::packed::bulk_operation_packed_single_block::BulkOperationPackedSingleBlock;
use crate::core::util::packed::bulk_operation_packed1::BulkOperationPacked1;
use crate::core::util::packed::bulk_operation_packed2::BulkOperationPacked2;
use crate::core::util::packed::bulk_operation_packed3::BulkOperationPacked3;
use crate::core::util::packed::bulk_operation_packed4::BulkOperationPacked4;
use crate::core::util::packed::bulk_operation_packed5::BulkOperationPacked5;
use crate::core::util::packed::bulk_operation_packed6::BulkOperationPacked6;
use crate::core::util::packed::bulk_operation_packed7::BulkOperationPacked7;
use crate::core::util::packed::bulk_operation_packed8::BulkOperationPacked8;
use crate::core::util::packed::bulk_operation_packed9::BulkOperationPacked9;
use crate::core::util::packed::bulk_operation_packed10::BulkOperationPacked10;
use crate::core::util::packed::bulk_operation_packed11::BulkOperationPacked11;
use crate::core::util::packed::bulk_operation_packed12::BulkOperationPacked12;
use crate::core::util::packed::bulk_operation_packed13::BulkOperationPacked13;
use crate::core::util::packed::bulk_operation_packed14::BulkOperationPacked14;
use crate::core::util::packed::bulk_operation_packed15::BulkOperationPacked15;
use crate::core::util::packed::bulk_operation_packed16::BulkOperationPacked16;
use crate::core::util::packed::bulk_operation_packed17::BulkOperationPacked17;
use crate::core::util::packed::bulk_operation_packed18::BulkOperationPacked18;
use crate::core::util::packed::bulk_operation_packed19::BulkOperationPacked19;
use crate::core::util::packed::bulk_operation_packed20::BulkOperationPacked20;
use crate::core::util::packed::bulk_operation_packed21::BulkOperationPacked21;
use crate::core::util::packed::bulk_operation_packed22::BulkOperationPacked22;
use crate::core::util::packed::bulk_operation_packed23::BulkOperationPacked23;
use crate::core::util::packed::bulk_operation_packed24::BulkOperationPacked24;
use crate::core::util::packed::{Decoder, Encoder};
pub(crate) const PACKED_BULK_OPS: [BulkOperationPackedEnum; 64] = [
  BulkOperationPackedEnum::Packed1(BulkOperationPacked1::new()),
  BulkOperationPackedEnum::Packed2(BulkOperationPacked2::new()),
  BulkOperationPackedEnum::Packed3(BulkOperationPacked3::new()),
  BulkOperationPackedEnum::Packed4(BulkOperationPacked4::new()),
  BulkOperationPackedEnum::Packed5(BulkOperationPacked5::new()),
  BulkOperationPackedEnum::Packed6(BulkOperationPacked6::new()),
  BulkOperationPackedEnum::Packed7(BulkOperationPacked7::new()),
  BulkOperationPackedEnum::Packed8(BulkOperationPacked8::new()),
  BulkOperationPackedEnum::Packed9(BulkOperationPacked9::new()),
  BulkOperationPackedEnum::Packed10(BulkOperationPacked10::new()),
  BulkOperationPackedEnum::Packed11(BulkOperationPacked11::new()),
  BulkOperationPackedEnum::Packed12(BulkOperationPacked12::new()),
  BulkOperationPackedEnum::Packed13(BulkOperationPacked13::new()),
  BulkOperationPackedEnum::Packed14(BulkOperationPacked14::new()),
  BulkOperationPackedEnum::Packed15(BulkOperationPacked15::new()),
  BulkOperationPackedEnum::Packed16(BulkOperationPacked16::new()),
  BulkOperationPackedEnum::Packed17(BulkOperationPacked17::new()),
  BulkOperationPackedEnum::Packed18(BulkOperationPacked18::new()),
  BulkOperationPackedEnum::Packed19(BulkOperationPacked19::new()),
  BulkOperationPackedEnum::Packed20(BulkOperationPacked20::new()),
  BulkOperationPackedEnum::Packed21(BulkOperationPacked21::new()),
  BulkOperationPackedEnum::Packed22(BulkOperationPacked22::new()),
  BulkOperationPackedEnum::Packed23(BulkOperationPacked23::new()),
  BulkOperationPackedEnum::Packed24(BulkOperationPacked24::new()),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(25)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(26)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(27)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(28)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(29)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(30)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(31)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(32)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(33)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(34)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(35)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(36)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(37)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(38)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(39)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(40)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(41)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(42)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(43)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(44)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(45)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(46)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(47)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(48)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(49)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(50)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(51)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(52)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(53)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(54)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(55)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(56)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(57)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(58)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(59)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(60)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(61)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(62)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(63)),
  BulkOperationPackedEnum::Packed(BulkOperationPacked::new(64)),
];
pub(crate) const PACKED_SINGLE_BLOCK_BULK_OPS: [Option<BulkOperationPackedEnum>; 32] = [
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(1),
  )),
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(2),
  )),
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(3),
  )),
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(4),
  )),
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(5),
  )),
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(6),
  )),
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(7),
  )),
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(8),
  )),
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(9),
  )),
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(10),
  )),
  None,
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(12),
  )),
  None,
  None,
  None,
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(16),
  )),
  None,
  None,
  None,
  None,
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(21),
  )),
  None,
  None,
  None,
  None,
  None,
  None,
  None,
  None,
  None,
  None,
  Some(BulkOperationPackedEnum::SinglePacked(
    BulkOperationPackedSingleBlock::new(32),
  )),
];
pub(crate) trait BulkOperation: Decoder + Encoder {
  fn write_long(&self, block: u64, blocks: &mut [u8], mut blocks_offset: usize) -> usize {
    for j in 1..=8 {
      blocks[blocks_offset] = (block >> (64 - (j << 3))) as u8;
      blocks_offset += 1;
    }
    blocks_offset
  }
  /// For every number of bits per value, there is a minimum number of blocks
  /// (b) / values (v) you need to write in order to reach the next block
  /// boundary:
  ///
  /// - 16 bits per value -> b=2, v=1
  /// - 24 bits per value -> b=3, v=1
  /// - 50 bits per value -> b=25, v=4
  /// - 63 bits per value -> b=63, v=8
  ///
  /// A bulk read consists of copying `iterations * v` values that are
  /// contained in `iterations * b` blocks into a `Vec<i64>` (higher
  /// values of `iterations` are likely to yield a better throughput):
  /// this requires `iterations * (b + 8v)` bytes of memory.
  ///
  /// This method computes `iterations` as `ram_budget / (b + 8v)` (since an
  /// i64 is 8 bytes).
  ///
  /// # Arguments
  /// - `value_count`: The total number of values.
  /// - `ram_budget`: The available RAM budget in bytes.
  ///
  /// # Returns
  /// The number of iterations to perform.
  fn compute_iterations(&self, value_count: i32, ram_budget: i32) -> i32 {
    let byte_value_count = Decoder::byte_value_count(self);
    let iterations = ram_budget / (Decoder::byte_block_count(self) + 8 * byte_value_count);
    if iterations == 0 {
      // At least 1 iteration is required
      1
    } else if (iterations - 1) * byte_value_count >= value_count {
      // Don't allocate for more than the size of the reader
      (value_count as f64 / byte_value_count as f64).ceil() as i32
    } else {
      iterations
    }
  }
}
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::packed::Format;

pub(crate) fn of(format: Format, bits_per_value: i32) -> Result<&'static BulkOperationPackedEnum> {
  match format {
    Format::Packed(..) => PACKED_BULK_OPS
      .get(bits_per_value.wrapping_sub(1) as usize)
      .ok_or_else(|| {
        LuceneError::array_index_out_of_bounds(format!(
          "Invalid bits_per_value for PACKED: {bits_per_value}"
        ))
      }),
    Format::PackedSingleBlock(..) => {
      let operation = PACKED_SINGLE_BLOCK_BULK_OPS
        .get(bits_per_value.wrapping_sub(1) as usize)
        .ok_or_else(|| {
          LuceneError::array_index_out_of_bounds(format!(
            "Invalid bits_per_value for PACKED_SINGLE_BLOCK: {bits_per_value}"
          ))
        })?;
      debug_assert!(
        operation.is_some(),
        "unsupported bits_per_value for PACKED_SINGLE_BLOCK: {bits_per_value}"
      );
      operation.as_ref().ok_or_else(|| {
        LuceneError::illegal_argument(format!(
          "unsupported bits_per_value for PACKED_SINGLE_BLOCK: {bits_per_value}"
        ))
      })
    },
  }
}
