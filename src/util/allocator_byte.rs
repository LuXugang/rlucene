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
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::util::access::SharedAccess;
use crate::util::{BYTE_BLOCK_SIZE, Counter, CounterEnum, CounterEnumBorrow};

/// A simple `Allocator` that never recycles, but tracks how much total RAM is
/// in use.  */
#[derive(Debug)]
pub struct DirectTrackingAllocatorByte<C: SharedAccess<CounterEnum>> {
    block_size: usize,
    pub(crate) byte_used: C,
}

impl<C: SharedAccess<CounterEnum>> DirectTrackingAllocatorByte<C> {
    pub fn new(byte_used: C) -> Self {
        DirectTrackingAllocatorByte {
            block_size: BYTE_BLOCK_SIZE as usize,
            byte_used,
        }
    }
    pub fn allocator_enum(byte_used: C) -> AllocatorByteEnum<C> {
        AllocatorByteEnum::DTA(DirectTrackingAllocatorByte::new(byte_used))
    }
}

impl<C: SharedAccess<CounterEnum>> AllocatorByte for DirectTrackingAllocatorByte<C> {
    fn recycle_byte_blocks(&mut self, _blocks: &[Vec<u8>], start: usize, end: usize) {
        let delta = (end - start) as i64 * self.block_size as i64;
        self.byte_used
            .access_mut(|byte_used| byte_used.add_and_get(-delta));
    }

    fn get_byte_block(&mut self) -> Vec<u8> {
        self.byte_used
            .access_mut(|byte_used| byte_used.add_and_get(self.block_size as i64));
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
pub enum AllocatorByteEnum<C>
where
    C: SharedAccess<CounterEnum>,
{
    DA(DirectAllocatorByte),
    DTA(DirectTrackingAllocatorByte<C>),
}
impl<C> AllocatorByteEnum<C>
where
    C: SharedAccess<CounterEnum>,
{
    pub fn get_used(&self) -> i64 {
        match self {
            AllocatorByteEnum::DA(_da) => 0,
            AllocatorByteEnum::DTA(dta) => dta.byte_used.access_mut(|byte_used| byte_used.get()),
        }
    }
}
impl<C> AllocatorByte for AllocatorByteEnum<C>
where
    C: SharedAccess<CounterEnum>,
{
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
/// for single-threaded scenarios
pub type STAllocatorByteEnum = AllocatorByteEnum<CounterEnumBorrow>;
/// for multi-threaded scenarios
pub type MTAllocatorByteEnum = AllocatorByteEnum<Arc<Mutex<CounterEnum>>>;

pub(crate) trait Allocator {
    type CounterAccess: SharedAccess<CounterEnum>;
    type Handle: Clone;

    fn new_allocator(allocator: AllocatorByteEnum<Self::CounterAccess>) -> Self::Handle;
    fn recycle_byte_blocks(&mut self, blocks: &[Vec<u8>], start: usize, end: usize);
    fn get_byte_block(&mut self) -> Vec<u8>;
    fn get_block_size(&self) -> usize;
    fn get_used(&self) -> i64;
}

impl<C> Allocator for Rc<RefCell<AllocatorByteEnum<C>>>
where
    C: SharedAccess<CounterEnum>,
{
    type CounterAccess = C;
    type Handle = Rc<RefCell<AllocatorByteEnum<C>>>;

    fn new_allocator(allocator: AllocatorByteEnum<C>) -> Self::Handle {
        Rc::new(RefCell::new(allocator))
    }

    fn recycle_byte_blocks(&mut self, blocks: &[Vec<u8>], start: usize, end: usize) {
        self.borrow_mut().recycle_byte_blocks(blocks, start, end)
    }

    fn get_byte_block(&mut self) -> Vec<u8> {
        self.borrow_mut().get_byte_block()
    }

    fn get_block_size(&self) -> usize {
        self.borrow().get_block_size()
    }

    fn get_used(&self) -> i64 {
        self.borrow().get_used()
    }
}

impl<C> Allocator for Arc<Mutex<AllocatorByteEnum<C>>>
where
    C: SharedAccess<CounterEnum>,
{
    type CounterAccess = C;
    type Handle = Arc<Mutex<AllocatorByteEnum<C>>>;

    fn new_allocator(allocator: AllocatorByteEnum<C>) -> Self::Handle {
        Arc::new(Mutex::new(allocator))
    }

    fn recycle_byte_blocks(&mut self, blocks: &[Vec<u8>], start: usize, end: usize) {
        self.lock().recycle_byte_blocks(blocks, start, end)
    }

    fn get_byte_block(&mut self) -> Vec<u8> {
        self.lock().get_byte_block()
    }

    fn get_block_size(&self) -> usize {
        self.lock().get_block_size()
    }

    fn get_used(&self) -> i64 {
        self.lock().get_used()
    }
}
