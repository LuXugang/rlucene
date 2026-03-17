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
use std::fmt::Display;

use crate::core::store::DataInput;
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::longs_ref::LongsRef;
use crate::core::util::packed::bulk_operation::{BulkOperation, of};
use crate::core::util::packed::bulk_operation_packed_enum::BulkOperationPackedEnum;
use crate::core::util::packed::format_behavior::FormatBehavior;
use crate::core::util::packed::{Decoder, Format, ReaderIterator};

pub struct PackedReaderIterator<'a, D>
where
  D: DataInput,
{
  packed_ints_version: i32,
  format: Format,
  bulk_operation: &'static BulkOperationPackedEnum,
  next_blocks: Vec<u8>,
  next_values: LongsRef,
  iterations: i32,
  position: i32,
  value_count: i32,
  bits_per_value: i32,
  data_input: &'a mut D,
}
impl<'a, D> PackedReaderIterator<'a, D>
where
  D: DataInput + 'a,
{
  pub fn new(
    format: Format,
    packed_ints_version: i32,
    value_count: i32,
    bits_per_value: i32,
    data_input: &'a mut D,
    mem: i32,
  ) -> Result<Self> {
    let bulk_operation = of(format, bits_per_value);
    let iterations = bulk_operation.compute_iterations(value_count, mem);

    debug_assert!(
      value_count == 0 || iterations > 0,
      "Value count must be 0 or iterations must be greater than 0."
    );

    let next_blocks = vec![0u8; iterations as usize * bulk_operation.byte_block_count() as usize];
    let next_values_long_length = (iterations * bulk_operation.byte_value_count()).try_convert()?;
    let next_values = LongsRef::from_slice(
      vec![0i64; next_values_long_length as usize],
      next_values_long_length,
      0,
    );

    Ok(Self {
      packed_ints_version,
      format,
      bulk_operation,
      next_blocks,
      next_values,
      iterations,
      position: -1,
      value_count,
      bits_per_value,
      data_input,
    })
  }
}
impl<'a, D> ReaderIterator for PackedReaderIterator<'a, D>
where
  D: DataInput + 'a,
{
  fn next_batch(&mut self, mut count: i32) -> Result<&mut LongsRef> {
    debug_assert!(count > 0);
    debug_assert!(
      (self.next_values.offset + self.next_values.length) <= self.next_values.longs.len(),
      "Offset and length should be within the bounds of longs"
    );
    self.next_values.offset += self.next_values.length;

    let remaining = self.value_count - self.position - 1;
    if remaining <= 0 {
      return Err(LuceneError::eof("No more values to read"));
    }

    count = count.min(remaining);

    if self.next_values.offset == self.next_values.longs.len() {
      let remaining_blocks =
        self
          .format
          .byte_count(self.packed_ints_version, remaining, self.bits_per_value);
      let blocks_to_read = remaining_blocks.min(self.next_blocks.len() as i64);
      debug_assert!(blocks_to_read <= i32::MAX as i64);
      self.data_input.read_bytes(
        &mut self.next_blocks[..blocks_to_read as usize],
        0,
        blocks_to_read as usize,
      )?;

      if (blocks_to_read as usize) < self.next_blocks.len() {
        self.next_blocks[blocks_to_read as usize..].fill(0);
      }

      self.bulk_operation.decode_u8_to_i64(
        &self.next_blocks,
        0,
        self.next_values.longs.as_mut_slice(),
        0,
        self.iterations,
      );

      self.next_values.offset = 0;
    }

    self.next_values.length =
      (self.next_values.longs.len() - self.next_values.offset).min(count as usize);
    let v: i32 = self.next_values.length.try_convert()?;
    self.position += v;

    Ok(&mut self.next_values)
  }
  fn ord(&self) -> i32 {
    self.position
  }
}
impl<D> Display for PackedReaderIterator<'_, D>
where
  D: DataInput,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}
