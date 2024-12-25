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
use crate::store::DataInput;
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::longs_ref::LongsRef;
use crate::util::packed::bulk_operation::{of, BulkOperation};
use crate::util::packed::bulk_operation_packed_enum::BulkOperationPackedEnum;
use crate::util::packed::{Decoder, Format, ReaderIterator};
use crate::util::packed::format_behavior::FormatBehavior;

pub(crate) struct PackedReaderIterator<'a, D>
where
    D: DataInput + 'a,
{
    packed_ints_version: u32,
    format: Format,
    bulk_operation: &'static BulkOperationPackedEnum,
    next_blocks: Vec<u8>,
    next_values: LongsRef,
    iterations: u32,
    position: i32,
    value_count: u32,
    bits_per_value: u32,
    data_input : &'a mut D,
}
impl<'a, D> PackedReaderIterator<'a, D>
where
    D: DataInput + 'a,
{
    pub fn new(
        format: Format,
        packed_ints_version: u32,
        value_count: u32,
        bits_per_value: u32,
        data_input: &'a mut D,
        mem: u32,
    ) -> Self {
        let bulk_operation = of(format, bits_per_value);
        let iterations = bulk_operation.compute_iterations(value_count, mem);

        assert!(
            value_count == 0 || iterations > 0,
            "Value count must be 0 or iterations must be greater than 0."
        );

        let next_blocks =
            vec![0u8; iterations as usize * bulk_operation.byte_block_count() as usize];
        let next_values = LongsRef::from_slice(
            vec![0i64; iterations as usize * bulk_operation.byte_value_count() as usize],
            0,
            0,
        );

        Self {
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
        }
    }
}
impl <'a, D> ReaderIterator for PackedReaderIterator<'a, D>
where
    D: DataInput + 'a,
{

    fn next_batch(&mut self, mut count: u32) -> Result<LongsRef, DataIOError> {
        debug_assert!(self.next_values.longs.len() >= 0, "Next values length should be >= 0");
        debug_assert!(
            self.next_values.offset + self.next_values.length <= self.next_values.longs.len(),
            "Offset and length should be within the bounds of longs"
        );
        self.next_values.offset += self.next_values.length;

        let remaining = self.value_count as i32 - self.position - 1;
        if remaining <= 0 {
            return Err(DataIOError::eof("No more values to read"));
        }

        count = count.min(remaining as u32);

        if self.next_values.offset == self.next_values.longs.len() {
            let remaining_blocks = self.format.byte_count(
                self.packed_ints_version,
                remaining as u32,
                self.bits_per_value,
            );
            let blocks_to_read = remaining_blocks.min(self.next_blocks.len() as u64) as usize;

            self.data_input.read_bytes(&mut self.next_blocks[..blocks_to_read], 0, blocks_to_read as u32)?;

            if blocks_to_read < self.next_blocks.len() {
                self.next_blocks[blocks_to_read..].fill(0);
            }

            self.bulk_operation.decode_byte_to_long(
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
        debug_assert!(self.next_values.length <= u32::MAX as usize);
        self.position += self.next_values.length as i32;

        Ok(self.next_values.clone())
    }
    fn ord(&self) -> i32 {
        self.position
    }
}