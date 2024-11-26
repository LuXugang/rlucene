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
use crate::store::data_output::DataOutput;
use crate::store::DataInput;
use crate::util::accountable::Accountable;
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::error::runtime_error::RuntimeError;
use byteorder::WriteBytesExt;
use std::collections::VecDeque;
use std::io::{Cursor};

/** Smallest `minBitsPerBlock` allowed */
const LIMIT_MIN_BITS_PER_BLOCK: usize = 1;
/** Largest `maxBitsPerBlock` allowed */
const LIMIT_MAX_BITS_PER_BLOCK: usize = 30;
/**
 * Maximum number of blocks at the current `blockBits` block size before we increase the
 * block size (and thus decrease the number of blocks).
 */
const MAX_BLOCKS_BEFORE_BLOCK_EXPANSION: usize = 100;
/** Default `maxBitsPerBlock` */
const DEFAULT_MAX_BITS_PER_BLOCK: u32 = 15;
/** Default `minBitsPerBlock` */
const DEFAULT_MIN_BITS_PER_BLOCK: u32 = 10;

/** A `DataOutput` storing data in a list of `vec<u8>`. */
pub struct ByteBuffersDataOutput {
    blocks: VecDeque<Cursor<Vec<u8>>>,
    max_bits_per_block: usize,
    block_bits: usize,
    current_block: usize,
    ram_bytes_used: i64,
    reuse: bool,
}
impl ByteBuffersDataOutput {
    pub fn new(
        min_bits_per_block: usize,
        max_bits_per_block: usize,
        reuse: bool,
    ) -> Result<Self, RuntimeError> {
        if min_bits_per_block < LIMIT_MIN_BITS_PER_BLOCK {
            return Err(RuntimeError::argument(format!(
                "minBitsPerBlock ({}) too small, must be at least {}",
                min_bits_per_block, LIMIT_MIN_BITS_PER_BLOCK
            )));
        }
        if max_bits_per_block > LIMIT_MAX_BITS_PER_BLOCK {
            return Err(RuntimeError::argument(format!(
                "maxBitsPerBlock ({}) too large, must not exceed {}",
                max_bits_per_block, LIMIT_MAX_BITS_PER_BLOCK
            )));
        }
        if min_bits_per_block > max_bits_per_block {
            return Err(RuntimeError::argument(format!(
                "minBitsPerBlock ({}) cannot exceed maxBitsPerBlock ({})",
                min_bits_per_block, max_bits_per_block
            )));
        }
        let block = Cursor::new(vec![0u8; 1 << min_bits_per_block]);
        let mut bocks = VecDeque::new();
        bocks.push_back(block);
        Ok(Self {
            max_bits_per_block,
            block_bits: min_bits_per_block,
            blocks: bocks,
            current_block: 0,
            ram_bytes_used: 0,
            reuse: false,
        })
    }
    fn new_with_expected_size(expected_size: u64, reuse: bool) -> Result<Self, RuntimeError> {
        let block_bits = compute_block_size_bits_for(expected_size);
        Self::new(block_bits, DEFAULT_MAX_BITS_PER_BLOCK as usize, reuse)
    }

    fn append_block(&mut self) {
        if self.blocks.len() > MAX_BLOCKS_BEFORE_BLOCK_EXPANSION
            && self.block_bits < self.max_bits_per_block
        {
            self.rewrite_to_block_size(self.block_bits + 1);
            if self.blocks.back_mut().unwrap().remain() > 0 {
                return;
            }
        }
        let required_block_size = 1 << self.block_bits;
        self.blocks
            .push_back(Cursor::new(vec![0u8; required_block_size]));
        self.current_block = self.blocks.len() - 1;
        debug_assert!(self.current_block == self.blocks.len());
        // TODO: self.ramBytesUsed += 0;
        self.ram_bytes_used += 0;
    }
    fn rewrite_to_block_size(&mut self, target_block_bits: usize) {
        debug_assert!(target_block_bits <= self.max_bits_per_block);
        let old_blocks_size = self.blocks.len();
        debug_assert!(
            self.blocks.len() < old_blocks_size
                || (old_blocks_size == self.blocks.len()
                    && self.blocks.back_mut().unwrap().remain() > 0)
        );
        self.rewrite_blocks(target_block_bits);
        // TODO:
        self.ram_bytes_used += 0;
    }
    // create larger blocks and copy data from smaller blocks
    pub fn rewrite_blocks(&mut self, target_block_bits: usize) {
        debug_assert!(target_block_bits > self.block_bits);
        self.block_bits = target_block_bits;
        let block_size = 1 << self.block_bits;
        let mut new_block = Cursor::new(vec![0; block_size]);
        while let Some(mut old_block) = self.blocks.pop_front() {
            while old_block.remain() > 0 {
                let available_space = new_block.remain();
                if available_space == 0 {
                    self.blocks.push_back(new_block);
                    new_block = Cursor::new(vec![0; block_size]);
                }
                let bytes_to_copy = available_space.min(old_block.remain()) as usize;
                let old_position = old_block.position() as usize;
                let new_position = new_block.position() as usize;
                let old_data = &old_block.get_ref()[old_position..old_position + bytes_to_copy];
                // TODO: maybe we should use `memcpy` to improve performance
                new_block.get_mut()[new_position..new_position + bytes_to_copy]
                    .copy_from_slice(old_data);
                old_block.set_position((old_position + bytes_to_copy) as u64);
                new_block.set_position((new_position + bytes_to_copy) as u64);
            }
        }
        if new_block.position() > 0 {
            self.blocks.push_back(new_block);
        }
        self.current_block = self.blocks.len() - 1;
    }
    #[cfg(feature = "test_only")]
    pub fn write_byte(&mut self, b: u8) -> Result<(), DataIOError> {
        self.write_bytes_range(&[b], 0, 1)
    }
}

impl DataOutput for ByteBuffersDataOutput {
    fn write_byte(&mut self, b: u8) -> Result<(), DataIOError> {
        if self.blocks.get_mut(self.current_block).unwrap().remain() == 0 {
            self.append_block();
        }
        Ok(self
            .blocks
            .get_mut(self.current_block)
            .unwrap()
            .write_u8(b)?)
    }

    fn write_bytes_with_len(&mut self, b: &[u8], len: i32) -> Result<(), DataIOError> {
        self.write_bytes_range(b, 0, len)
    }

    fn write_bytes_range(
        &mut self,
        b: &[u8],
        mut offset: i32,
        mut length: i32,
    ) -> Result<(), DataIOError> {
        debug_assert!(length >= 0);
        while length > 0 {
            let mut block = self.blocks.get_mut(self.current_block).unwrap();
            let mut available_space = block.remain();
            if available_space == 0 {
                self.append_block();
                block = self.blocks.get_mut(self.current_block).unwrap();
                available_space = 1 << self.block_bits;
            }
            let chunk = available_space.min(length as u64) as usize;
            let block_position = block.position() as usize;
            block.get_mut()[block_position..block_position + chunk]
                .copy_from_slice(&b[offset as usize..offset as usize + chunk]);
            length -= chunk as i32;
            offset += chunk as i32;
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
        todo!()
    }

    fn copy_bytes<T: DataInput>(
        &mut self,
        input: &mut T,
        num_bytes: i64,
    ) -> Result<(), DataIOError> {
        todo!()
    }
}

impl Accountable for ByteBuffersDataOutput {
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

trait CursorExt {
    fn remain(&self) -> u64;
}

impl CursorExt for Cursor<Vec<u8>> {
    fn remain(&self) -> u64 {
        (self.get_ref().len() as u64) - self.position()
    }
}
fn compute_block_size_bits_for(bytes: u64) -> usize {
    let avg_block_size = bytes / MAX_BLOCKS_BEFORE_BLOCK_EXPANSION as u64;
    let power_of_two = avg_block_size.next_power_of_two();
    if power_of_two == 0 {
        return DEFAULT_MIN_BITS_PER_BLOCK as usize;
    }
    let mut block_bits = power_of_two.trailing_zeros();
    block_bits = block_bits.clamp(DEFAULT_MIN_BITS_PER_BLOCK, DEFAULT_MAX_BITS_PER_BLOCK);
    block_bits as usize
}
