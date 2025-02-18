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
use crate::util::error::lucene_error::LuceneError;
use crate::util::{ByteBlockPool, Counter, DirectTrackingAllocator};

pub struct IntBlockPool;

/// Abstract trait for allocating and freeing byte blocks.
pub trait Allocator {
    fn recycle_byte_blocks(
        &mut self,
        blocks: &[Vec<u8>],
        start: i32,
        end: i32,
    ) -> Result<(), LuceneError>;
    fn get_byte_block(&mut self) -> Result<Vec<u8>, LuceneError>;
    fn get_block_size(&self) -> i32;
}

/// A simple [`Allocator`] that never recycles. */
pub struct DirectAllocator {
    block_size: i32,
}

impl Default for DirectAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectAllocator {
    pub fn new() -> Self {
        DirectAllocator {
            block_size: ByteBlockPool::BYTE_BLOCK_SIZE,
        }
    }
}

impl Allocator for DirectAllocator {
    fn recycle_byte_blocks(
        &mut self,
        _blocks: &[Vec<u8>],
        _start: i32,
        _end: i32,
    ) -> Result<(), LuceneError> {
        Ok(())
    }

    fn get_byte_block(&mut self) -> Result<Vec<u8>, LuceneError> {
        Ok(vec![0; self.block_size as usize])
    }

    fn get_block_size(&self) -> i32 {
        self.block_size
    }
}

pub enum AllocatorEnum {
    DA(DirectAllocator),
    DTA(DirectTrackingAllocator),
}

impl AllocatorEnum {
    pub fn get_used(&self) -> Result<i64, LuceneError> {
        match self {
            AllocatorEnum::DA(_da) => Ok(0),
            AllocatorEnum::DTA(dta) => Ok(dta
                .byte_used
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
                .get()),
        }
    }
    pub fn recycle_byte_blocks(
        &mut self,
        blocks: &[Vec<u8>],
        start: i32,
        end: i32,
    ) -> Result<(), LuceneError> {
        match self {
            AllocatorEnum::DA(da) => da.recycle_byte_blocks(blocks, start, end),
            AllocatorEnum::DTA(dta) => dta.recycle_byte_blocks(blocks, start, end),
        }
    }
    pub fn get_block_size(&self) -> i32 {
        match self {
            AllocatorEnum::DA(da) => da.get_block_size(),
            AllocatorEnum::DTA(dta) => dta.get_block_size(),
        }
    }
    pub fn get_byte_block(&mut self) -> Result<Vec<u8>, LuceneError> {
        match self {
            AllocatorEnum::DA(da) => da.get_byte_block(),
            AllocatorEnum::DTA(dta) => dta.get_byte_block(),
        }
    }
}
