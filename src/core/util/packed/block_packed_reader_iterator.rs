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
use crate::core::store::DataInput;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::longs_ref::LongsRef;
use crate::core::util::packed::abstract_block_packed_writer::{
    BPV_SHIFT, MAX_BLOCK_SIZE, MIN_BLOCK_SIZE, MIN_VALUE_EQUALS_0,
};
use crate::core::util::packed::{Decoder, Format, FormatBehavior, PackedImpl, PackedInts};

/// Reader for sequences of longs written with
/// [`BlockPackedWriter`](crate::core::util::packed::block_packed_writer::BlockPackedWriter).
///
///
/// # See Also
/// [`BlockPackedWriter`](crate::core::util::packed::block_packed_writer::BlockPackedWriter)
///
/// # Note
/// This is an internal implementation detail.
pub struct BlockPackedReaderIterator {
    packed_ints_version: i32,
    value_count: usize,
    block_size: usize,
    values_ref: LongsRef,
    blocks: Vec<u8>,
    off: usize,
    ord: usize,
}

impl BlockPackedReaderIterator {
    /// Reads a variable-length long value (supports negative values).
    ///
    /// # Arguments
    ///
    /// * `data_input` - The input from which to read the value.
    ///
    /// # Returns
    ///
    /// A signed long value.
    ///
    /// # Errors
    ///
    /// Returns an `IoError` if the reading fails.
    fn read_vlong(data_input: &mut impl DataInput) -> Result<i64> {
        let mut l = 0u64;
        for shift in (0..56).step_by(7) {
            let b = data_input.read_byte()?;
            l |= ((b & 0x7F) as u64) << shift;
            if b as i8 >= 0 {
                return Ok(l as i64);
            }
        }
        let last_byte = data_input.read_byte()?;
        Ok(l as i64 | ((last_byte as i64 & 0xFF) << 56))
    }

    pub fn new(packed_ints_version: i32, block_size: usize, value_count: usize) -> Result<Self> {
        PackedInts::check_block_size(block_size as i32, MIN_BLOCK_SIZE, MAX_BLOCK_SIZE)?;
        let values = vec![0; block_size];
        let long_ref = LongsRef::from_slice(values, 0, 0);
        Ok(Self {
            packed_ints_version,
            value_count,
            block_size,
            values_ref: long_ref,
            blocks: vec![],
            off: block_size,
            ord: 0,
        })
    }
    /// Reset the current reader to wrap a stream of `valueCount` values
    /// contained in `data_input`. The block size remains unchanged.
    ///
    /// # Arguments
    ///
    /// * `data_input` - The new input stream to read from.
    /// * `value_count` - The number of values to read from the input.
    pub fn reset(&mut self, value_count: usize) {
        self.value_count = value_count;
        self.off = self.block_size;
        self.ord = 0;
    }
    /// Skip exactly `count` values.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of values to skip.
    ///
    /// # Errors
    ///
    /// Returns a `LuceneError` if `count` is invalid or if there is an issue
    /// reading the input.
    pub fn skip(&mut self, mut count: usize, data_input: &mut impl DataInput) -> Result<()> {
        // debug_assert!(count >= 0);
        if self.ord + count > self.value_count {
            return Err(LuceneError::eof("Attempt to skip past end of file"));
        }

        // 1. Skip buffered values
        let skip_buffer = std::cmp::min(count, self.block_size - self.off);
        self.off += skip_buffer;
        self.ord += skip_buffer;
        count -= skip_buffer;
        if count == 0 {
            return Ok(());
        }

        // 2. Skip as many blocks as necessary
        debug_assert_eq!(self.off, self.block_size);
        while count >= self.block_size {
            let token = data_input.read_byte()? as i32;
            let bits_per_value = token >> BPV_SHIFT;

            if bits_per_value > 64 {
                return Err(LuceneError::corrupt_index("Corrupted: bits_per_value > 64"));
            }

            if (token & MIN_VALUE_EQUALS_0) == 0 {
                Self::read_vlong(data_input)?;
            }

            let block_bytes = Format::Packed(PackedImpl::new(0)).byte_count(
                self.packed_ints_version,
                self.block_size as i32,
                bits_per_value,
            );
            self.skip_bytes(block_bytes, data_input)?;
            self.ord += self.block_size;
            count -= self.block_size;
        }

        if count == 0 {
            return Ok(());
        }
        // 3. Skip last values
        debug_assert!(count < self.block_size);
        self.refill(data_input)?;
        self.ord += count;
        debug_assert!(count <= i32::MAX as usize);
        self.off += count;
        Ok(())
    }
    fn skip_bytes(&mut self, count: i64, data_input: &mut impl DataInput) -> Result<()> {
        if data_input.is_index_input() {
            let new_position = data_input.get_file_pointer_in_data_input()? as i64 + count;
            data_input.seek_in_data_input(new_position as usize)?;
        } else {
            // Use a temporary buffer to skip bytes
            if self.blocks.is_empty() {
                self.blocks = vec![0u8; self.block_size];
            }

            let mut skipped = 0;
            while skipped < count {
                let to_skip = std::cmp::min(self.blocks.len() as i64, count - skipped);
                debug_assert!(to_skip <= i32::MAX as i64);
                data_input.read_bytes(&mut self.blocks, 0, to_skip as usize)?;
                skipped += to_skip;
            }
        }

        Ok(())
    }
    /// Reads the next value from the stream.
    ///
    /// # Errors
    ///
    /// Returns an `EOFError` if the reader has reached the end of the value
    /// stream.
    ///
    /// # Behavior
    /// - If the current block is exhausted (`off == block_size`), it will
    ///   refill the block.
    /// - Increments the `ord` to track the current position in the stream.
    /// - Returns the next value from the `values` buffer.
    pub fn next_value(&mut self, data_input: &mut impl DataInput) -> Result<i64> {
        if self.ord == self.value_count {
            return Err(LuceneError::eof("Reached end of value stream"));
        }
        if self.off == self.block_size {
            self.refill(data_input)?;
        }
        let value = self.values_ref.longs[self.off];
        self.off += 1;
        self.ord += 1;
        Ok(value)
    }

    /// Reads between `1` and `count` values and returns a reference to the
    /// values.
    ///
    /// # Arguments
    ///
    /// * `count` - The maximum number of values to read.
    ///
    /// # Returns
    ///
    /// A `LongsRef` containing a reference to the values read and their offset
    /// and length.
    ///
    /// # Errors
    ///
    /// Returns an `EOFError` if the reader has reached the end of the value
    /// stream.
    pub fn next_batch(
        &mut self,
        mut count: usize,
        data_input: &mut impl DataInput,
    ) -> Result<&LongsRef> {
        debug_assert!(count > 0);
        if self.ord == self.value_count {
            return Err(LuceneError::eof("Reached end of value stream"));
        }
        if self.off == self.block_size {
            self.refill(data_input)?;
        }
        count = count.min(self.block_size - self.off);
        count = count.min(self.value_count - self.ord);

        self.values_ref.offset = self.off;
        self.values_ref.length = count;
        self.off += count;
        self.ord += count;
        Ok(&self.values_ref)
    }

    fn refill(&mut self, data_input: &mut impl DataInput) -> Result<()> {
        let token = data_input.read_byte()? as i32;
        let min_equals_0 = (token & MIN_VALUE_EQUALS_0) != 0;
        let bits_per_value = token >> BPV_SHIFT;

        if bits_per_value > 64 {
            return Err(LuceneError::corrupt_index("Corrupted: bits_per_value > 64"));
        }
        let min_value = if min_equals_0 {
            0
        } else {
            BitUtil::zig_zag_decode_i64((1 + Self::read_vlong(data_input)?) as u64)
        };
        debug_assert!(min_equals_0 || min_value != 0);

        if bits_per_value == 0 {
            self.values_ref.longs.fill(min_value);
        } else {
            let decoder = PackedInts::get_decoder(
                Format::Packed(PackedImpl::new(0)),
                self.packed_ints_version,
                bits_per_value,
            )?;

            let iterations = self.block_size as i32 / Decoder::byte_value_count(decoder);
            let blocks_size = iterations * Decoder::byte_block_count(decoder);

            if self.blocks.len() < blocks_size as usize {
                self.blocks = vec![0; blocks_size as usize];
            }

            let value_count = std::cmp::min(self.value_count - self.ord, self.block_size);

            let blocks_count = Format::Packed(PackedImpl::new(0)).byte_count(
                self.packed_ints_version,
                value_count as i32,
                bits_per_value,
            );
            data_input.read_bytes(&mut self.blocks, 0, blocks_count as usize)?;

            decoder.decode_u8_to_i64(&self.blocks, 0, &mut self.values_ref.longs, 0, iterations);
            if min_value != 0 {
                for i in 0..value_count as usize {
                    self.values_ref.longs[i] += min_value;
                }
            }
        }
        self.off = 0;
        Ok(())
    }
    /// Returns the offset of the next value to read.
    ///
    /// # Returns
    /// The current global position (`ord`) in the value stream.
    pub fn ord(&self) -> usize {
        self.ord
    }
}
