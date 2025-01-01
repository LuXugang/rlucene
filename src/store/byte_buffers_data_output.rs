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
use crate::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::store::data_output::DataOutput;
use crate::store::DataInput;
use crate::util::accountable::Accountable;
use crate::util::error::data_io_error_enum::DataIOError;

use crate::util::{ReadableCursorExt, WritableCursorExt};
use byteorder::WriteBytesExt;
use std::collections::VecDeque;
use std::io::{Cursor, Seek};

/// A [`DataOutput`] storing data in a list of [`Cursor<Vec<u8>>`](std::io::Cursor).
pub struct ByteBuffersDataOutput {
    //In Rust Lucene, all data within each block is considered valid.
    // However, in Java Lucene, the valid data range can be controlled
    // by the `limit` parameter of the `java.nio.ByteBuffer` encapsulation.
    blocks: VecDeque<Cursor<Vec<u8>>>,
    max_bits_per_block: u32,
    block_bits: u32,
    ram_bytes_used: i64,
    // it is needed when we want to reuse the dataoutput
    current_block_index: u32,
    reuse: bool,
}
impl ByteBuffersDataOutput {
    /// Smallest `minBitsPerBlock` allowed
    pub const LIMIT_MIN_BITS_PER_BLOCK: u32 = 1;
    /// Largest `maxBitsPerBlock` allowed
    pub const LIMIT_MAX_BITS_PER_BLOCK: u32 = 31;
    ///Maximum number of blocks at the current `blockBits` block size before we increase the
    ///block size (and thus decrease the number of blocks).
    pub const MAX_BLOCKS_BEFORE_BLOCK_EXPANSION: u32 = 100;
    ///Default `maxBitsPerBlock`
    pub const DEFAULT_MAX_BITS_PER_BLOCK: u32 = 26;
    /// Default `minBitsPerBlock`
    pub const DEFAULT_MIN_BITS_PER_BLOCK: u32 = 10;

    ///Creates a new output with all defaults.
    pub fn new_resettable_instance() -> Result<Self, DataIOError> {
        Self::new(
            Self::DEFAULT_MIN_BITS_PER_BLOCK,
            Self::DEFAULT_MAX_BITS_PER_BLOCK,
            true,
        )
    }
    /// Expert: Creates a new output with custom parameters.
    ///
    /// # Arguments
    /// * `min_bits_per_block` - Minimum bits per block.
    /// * `max_bits_per_block` - Maximum bits per block.
    /// * `reuse` - Reuse this Instance.
    pub fn new(
        min_bits_per_block: u32,
        max_bits_per_block: u32,
        reuse: bool,
    ) -> Result<Self, DataIOError> {
        if min_bits_per_block < Self::LIMIT_MIN_BITS_PER_BLOCK {
            return Err(DataIOError::illegal_argument(format!(
                "minBitsPerBlock ({}) too small, must be at least {}",
                min_bits_per_block,
                Self::LIMIT_MIN_BITS_PER_BLOCK
            )));
        }
        if max_bits_per_block > Self::LIMIT_MAX_BITS_PER_BLOCK {
            return Err(DataIOError::illegal_argument(format!(
                "maxBitsPerBlock ({}) too large, must not exceed {}",
                max_bits_per_block,
                Self::LIMIT_MAX_BITS_PER_BLOCK
            )));
        }
        if min_bits_per_block > max_bits_per_block {
            return Err(DataIOError::illegal_argument(format!(
                "minBitsPerBlock ({}) cannot exceed maxBitsPerBlock ({})",
                min_bits_per_block, max_bits_per_block
            )));
        }
        let block = Cursor::new(vec![0u8; 1 << min_bits_per_block]);
        let mut blocks = VecDeque::new();
        blocks.push_back(block);
        Ok(Self {
            max_bits_per_block,
            block_bits: min_bits_per_block,
            blocks,
            ram_bytes_used: 0,
            current_block_index: 0,
            reuse,
        })
    }
    /// Creates a new output, suitable for writing a file of approximately `expected_size` bytes.
    ///
    /// Memory allocation will be optimized based on the `expected_size` hint to reduce overhead for larger files.
    ///
    /// # Arguments
    /// * `expected_size` - Estimated size of the output file.
    pub fn new_with_expected_size(expected_size: u64) -> Result<Self, DataIOError> {
        let block_bits = compute_block_size_bits_for(expected_size);
        Self::new(block_bits, Self::DEFAULT_MAX_BITS_PER_BLOCK, false)
    }

    fn append_block(&mut self) {
        if self.blocks.len() > Self::MAX_BLOCKS_BEFORE_BLOCK_EXPANSION as usize
            && self.block_bits < self.max_bits_per_block
        {
            self.rewrite_to_block_size(self.block_bits + 1);
            if self
                .blocks
                .get_mut(self.current_block_index as usize)
                .unwrap()
                .remain()
                > 0
            {
                return;
            }
        }
        let required_block_size = 1 << self.block_bits;
        self.blocks
            .push_back(Cursor::new(vec![0u8; required_block_size]));
        // TODO: self.ramBytesUsed += 0;
        self.ram_bytes_used += 0;
        self.current_block_index += 1;
    }
    fn rewrite_to_block_size(&mut self, target_block_bits: u32) {
        debug_assert!(target_block_bits <= self.max_bits_per_block);
        self.rewrite_blocks(target_block_bits);
        // TODO:
        self.ram_bytes_used += 0;
    }
    // create larger blocks and copy data from smaller blocks
    // TODO: the first old_block's data could be reused ,first do expansion by `push_back` and then move to tail and continue copy the second old_block's data to it
    pub fn rewrite_blocks(&mut self, target_block_bits: u32) {
        debug_assert!(target_block_bits > self.block_bits);
        self.block_bits = target_block_bits;
        let block_size = 1 << self.block_bits;
        let mut new_block = Cursor::new(vec![0; block_size]);
        let mut old_block_count = self.blocks.len();
        while let Some(mut old_block) = self.blocks.pop_front() {
            // read from head
            old_block.set_position(0);
            while old_block.remain() > 0 {
                let mut available_space = new_block.remain();
                if available_space == 0 {
                    self.blocks.push_back(new_block);
                    new_block = Cursor::new(vec![0; block_size]);
                    available_space = 1 << self.block_bits;
                }
                let bytes_to_copy = available_space.min(old_block.remain()) as usize;
                let old_position = old_block.position() as usize;
                let old_data = &old_block.get_ref()[old_position..old_position + bytes_to_copy];
                debug_assert!(
                    new_block.remain() as usize >= bytes_to_copy,
                    "Insufficient space in new_block: remaining={}, required={}",
                    new_block.remain(),
                    bytes_to_copy
                );
                new_block.write_from_slice(old_data).unwrap();
                old_block.set_position((old_position + bytes_to_copy) as u64);
            }
            old_block_count -= 1;
            if old_block_count == 0 {
                break;
            }
        }
        if new_block.position() > 0 {
            self.blocks.push_back(new_block);
        }
        debug_assert!(self.blocks.len() <= u32::MAX as usize);
        self.current_block_index = (self.blocks.len() - 1) as u32;
    }
    /// Copies the current content of this object into another [`DataOutput`].
    #[allow(unused)]
    fn copy_to<T: DataInput>(&mut self, _output: T) -> Result<(), DataIOError> {
        unimplemented!("")
    }
    /// The number of bytes written to this output so far.
    pub fn size(&self) -> u64 {
        let mut size = 0;
        let block_count = self.current_block_index + 1;
        if block_count >= 1 {
            let full_block_size = (block_count - 1) as u64 * self.block_size();
            let last_block_size = self
                .blocks
                .get(self.current_block_index as usize)
                .unwrap()
                .position();
            size = full_block_size + last_block_size;
        }
        size
    }
    fn block_size(&self) -> u64 {
        1 << self.block_bits
    }
    /// Resets this object to a clean (zero-size) state and publishes any currently allocated buffers
    /// for reuse according to the reuse strategy provided in the constructor.
    ///
    /// # Warning
    /// Sharing byte buffers for reads and writes is dangerous and may lead to hard-to-debug issues.
    /// Use with great caution.
    pub fn reset(&mut self) {
        if self.reuse {
            for block in &mut self.blocks {
                let _ = block.rewind();
            }
        }
        self.current_block_index = 0;
        self.ram_bytes_used = 0;
    }

    /// Returns a list of read-only views of [`Cursor<Vec<u8>>`](Cursor) blocks over the current content written
    /// to the output.
    pub fn to_buffer_list(&self) -> (u64, Vec<Cursor<&[u8]>>) {
        let data = self
            .blocks
            .iter()
            .map(|cursor| {
                let slice: &[u8] = cursor.get_ref().as_slice();
                let mut new_cursor = Cursor::new(slice);
                new_cursor.set_position(0);
                new_cursor
            })
            .collect();
        (self.size(), data)
    }
    #[allow(unused)]
    pub fn get_writeable_buffer_list(&mut self) -> Vec<&mut Cursor<Vec<u8>>> {
        todo!()
    }
    /// Returns a contiguous array containing the current content written to the output.
    /// The returned array is always a copy and can be safely mutated.
    pub fn get_array_copy(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(self.size() as usize);

        for block in &self.blocks {
            let end = block.position() as usize;
            buffer.extend_from_slice(&block.get_ref()[..end]);
        }
        buffer
    }

    pub fn get_data_input(&mut self) -> ByteBuffersDataInput {
        let (length, data) = self.to_buffer_list();
        ByteBuffersDataInput::new(data, length)
    }

    fn append_block_if_needed(&mut self) -> u64 {
        let mut last_block = self
            .blocks
            .get_mut(self.current_block_index as usize)
            .unwrap();
        if last_block.remain() == 0 {
            if self.reuse && (self.current_block_index as usize) < self.blocks.len() - 1 {
                self.current_block_index += 1;
                last_block = self
                    .blocks
                    .get_mut(self.current_block_index as usize)
                    .unwrap();
            } else {
                self.append_block();
                // it is safe to get by `back_mut` because blocks are not reused
                last_block = self.blocks.back_mut().unwrap();
            }
        }
        last_block.remain()
    }
    #[cfg(feature = "test_only")]
    pub fn write_bytes(&mut self, b: Vec<u8>) -> Result<(), DataIOError> {
        debug_assert!(b.len() <= u32::MAX as usize);
        self.write_bytes_range(&b, 0, b.len() as u32)
    }

    #[cfg(feature = "test_only")]
    pub fn write_byte(&mut self, b: u8) -> Result<(), DataIOError> {
        self.write_bytes_range(&[b], 0, 1)
    }
}

impl DataOutput for ByteBuffersDataOutput {
    fn write_byte(&mut self, b: u8) -> Result<(), DataIOError> {
        self.append_block_if_needed();
        let last_block = self
            .blocks
            .get_mut(self.current_block_index as usize)
            .unwrap();
        Ok(last_block.write_u8(b)?)
    }

    fn write_bytes_with_len(&mut self, b: &[u8], len: u32) -> Result<(), DataIOError> {
        self.write_bytes_range(b, 0, len)
    }

    fn write_bytes_range(
        &mut self,
        b: &[u8],
        mut offset: u32,
        mut length: u32,
    ) -> Result<(), DataIOError> {
        while length > 0 {
            let available_space = self.append_block_if_needed();
            let last_block = self
                .blocks
                .get_mut(self.current_block_index as usize)
                .unwrap();
            let chunk = available_space.min(length as u64);
            debug_assert!(chunk <= u32::MAX as u64);
            last_block.write_from(b, offset, chunk as u32)?;
            length -= chunk as u32;
            offset += chunk as u32;
        }
        Ok(())
    }

    fn write_int(&mut self, i: i32) -> Result<(), DataIOError> {
        let value = i.to_le_bytes();
        self.write_bytes_range(&value, 0, 4)
    }

    fn write_short(&mut self, i: i16) -> Result<(), DataIOError> {
        let value = i.to_le_bytes();
        self.write_bytes_range(&value, 0, 2)
    }

    fn write_long(&mut self, i: i64) -> Result<(), DataIOError> {
        let value = i.to_le_bytes();
        self.write_bytes_range(&value, 0, 8)
    }

    fn write_string(&mut self, s: &str) -> Result<(), DataIOError> {
        let bytes = s.as_bytes();
        let length = bytes.len();
        debug_assert!(length <= u32::MAX as usize);
        self.write_vint(length as i32)?;
        self.write_bytes_range(bytes, 0, length as u32)
    }

    fn copy_bytes<T: DataInput>(
        &mut self,
        input: &mut T,
        mut num_bytes: u64,
    ) -> Result<(), DataIOError> {
        while num_bytes > 0 {
            let available_space = self.append_block_if_needed();
            let last_block = self
                .blocks
                .get_mut(self.current_block_index as usize)
                .unwrap();
            let bytes_to_copy = available_space.min(num_bytes);
            debug_assert!(bytes_to_copy <= u32::MAX as u64);

            let current_pos = last_block.position();
            debug_assert!(current_pos <= u32::MAX as u64);
            let current_block_mut = last_block.get_mut();
            input.read_bytes(current_block_mut, current_pos as u32, bytes_to_copy as u32)?;
            last_block.set_position(current_pos + bytes_to_copy);
            num_bytes -= bytes_to_copy;
        }
        Ok(())
    }
}

impl Accountable for ByteBuffersDataOutput {
    fn ram_bytes_used(&self) -> u64 {
        todo!()
    }
}

fn compute_block_size_bits_for(bytes: u64) -> u32 {
    let avg_block_size = bytes / ByteBuffersDataOutput::MAX_BLOCKS_BEFORE_BLOCK_EXPANSION as u64;
    let power_of_two = avg_block_size.next_power_of_two();
    if power_of_two == 0 {
        return ByteBuffersDataOutput::DEFAULT_MIN_BITS_PER_BLOCK;
    }
    let mut block_bits = power_of_two.trailing_zeros();
    block_bits = block_bits.min(ByteBuffersDataOutput::DEFAULT_MAX_BITS_PER_BLOCK);
    block_bits = block_bits.max(ByteBuffersDataOutput::DEFAULT_MIN_BITS_PER_BLOCK);
    block_bits
}

#[cfg(feature = "not_required_in_rlucene")]
#[allow(unused)]
fn write_long_string(_byte_len: usize, _s: String) {
    unimplemented!()
}
