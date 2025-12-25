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

use crate::core::util::{BYTE_BLOCK_SIZE, Counter, SharedCounter};

/// A simple `Allocator` that never recycles, but tracks how much total RAM is
/// in use.  */
#[derive(Debug)]
pub struct DirectTrackingAllocatorByte {
    block_size: usize,
    pub(crate) byte_used: SharedCounter,
}

impl DirectTrackingAllocatorByte {
    pub fn new(byte_used: SharedCounter) -> Self {
        DirectTrackingAllocatorByte {
            block_size: BYTE_BLOCK_SIZE as usize,
            byte_used,
        }
    }
    pub fn allocator_enum(byte_used: SharedCounter) -> AllocatorByteEnum {
        AllocatorByteEnum::DTA(DirectTrackingAllocatorByte::new(byte_used))
    }
}

impl AllocatorByte for DirectTrackingAllocatorByte {
    fn recycle_byte_blocks(&mut self, _blocks: &[Vec<u8>], start: usize, end: usize) {
        let delta = (end - start) as i64 * self.block_size as i64;
        self.byte_used.add_and_get(-delta);
    }

    fn get_byte_block(&mut self) -> Vec<u8> {
        self.byte_used.add_and_get(self.block_size as i64);
        vec![0; self.block_size]
    }

    fn get_block_size(&self) -> usize {
        self.block_size
    }
}

/// Abstract trait for allocating and freeing byte blocks.
pub trait AllocatorByte {
    fn recycle_byte_blocks(&mut self, blocks: &[Vec<u8>], start: usize, end: usize);
    fn get_byte_block(&mut self) -> Vec<u8>;
    fn get_block_size(&self) -> usize;
}

/// A simple [`AllocatorByte`] that never recycles.  */
#[derive(Debug)]
pub struct DirectAllocatorByte {
    block_size: usize,
}

impl Default for DirectAllocatorByte {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectAllocatorByte {
    pub fn new() -> Self {
        DirectAllocatorByte {
            block_size: BYTE_BLOCK_SIZE as usize,
        }
    }
}

impl AllocatorByte for DirectAllocatorByte {
    fn recycle_byte_blocks(&mut self, _blocks: &[Vec<u8>], _start: usize, _end: usize) {}

    fn get_byte_block(&mut self) -> Vec<u8> {
        vec![0; self.block_size]
    }

    fn get_block_size(&self) -> usize {
        self.block_size
    }
}

#[derive(Debug)]
pub enum AllocatorByteEnum {
    DA(DirectAllocatorByte),
    DTA(DirectTrackingAllocatorByte),
}
impl AllocatorByteEnum {
    pub fn get_used(&self) -> i64 {
        match self {
            AllocatorByteEnum::DA(_da) => 0,
            AllocatorByteEnum::DTA(dta) => dta.byte_used.get(),
        }
    }
}
impl AllocatorByte for AllocatorByteEnum {
    fn recycle_byte_blocks(&mut self, blocks: &[Vec<u8>], start: usize, end: usize) {
        match self {
            AllocatorByteEnum::DA(da) => da.recycle_byte_blocks(blocks, start, end),
            AllocatorByteEnum::DTA(dta) => dta.recycle_byte_blocks(blocks, start, end),
        }
    }
    fn get_byte_block(&mut self) -> Vec<u8> {
        match self {
            AllocatorByteEnum::DA(da) => da.get_byte_block(),
            AllocatorByteEnum::DTA(dta) => dta.get_byte_block(),
        }
    }
    fn get_block_size(&self) -> usize {
        match self {
            AllocatorByteEnum::DA(da) => da.get_block_size(),
            AllocatorByteEnum::DTA(dta) => dta.get_block_size(),
        }
    }
}
