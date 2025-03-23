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
use crate::util::error::lucene_error::{LuceneError, Result};
use std::cell::RefCell;
use std::rc::Rc;
/// # Internal
/// A pool for int blocks similar to `ByteBlockPool`.
pub struct IntBlockPool {
    /// array of buffers currently used in the pool. Buffers are allocated if needed don't modify this
    /// outside of this class
    buffers: Vec<Vec<i32>>,
    /// index into the buffers array pointing to the current buffer used as the head.
    pub(crate) buffer_upto: i32,
    /// Pointer to the current position in head buffer.
    pub(crate) int_upto: i32,
    /// Current head offset.
    pub(crate) int_offset: i32,
    allocator: Rc<RefCell<AllocatorIntEnum>>,
}
impl Default for IntBlockPool {
    fn default() -> Self {
        Self::new()
    }
}

impl IntBlockPool {
    pub(crate) const INT_BLOCK_SHIFT: i32 = 13;
    pub(crate) const INT_BLOCK_SIZE: i32 = 1 << Self::INT_BLOCK_SHIFT;
    #[allow(unused)]
    pub(crate) const INT_BLOCK_MASK: i32 = Self::INT_BLOCK_SIZE - 1;
    /// Creates a new `IntBlockPool` with a default `Allocator`.
    ///
    /// See `IntBlockPool::next_buffer()` for more details.
    pub fn new() -> Self {
        let allocator = Rc::new(RefCell::new(
            AllocatorIntEnum::DA(DirectAllocatorI32::new()),
        ));
        Self::with_allocator(allocator)
    }
    /// Creates a new `IntBlockPool` with the given `Allocator`.
    ///
    /// See `IntBlockPool::next_buffer()` for more details.
    pub fn with_allocator(allocator: Rc<RefCell<AllocatorIntEnum>>) -> Self {
        IntBlockPool {
            buffers: vec![],
            buffer_upto: -1,
            int_upto: Self::INT_BLOCK_SIZE,
            int_offset: -Self::INT_BLOCK_SIZE,
            allocator,
        }
    }
    /// Expert: Resets the pool to its initial state, while optionally reusing the first buffer.
    /// Buffers that are not reused are reclaimed by `ByteBlockPool::Allocator::recycleByteBlocks(byte[][], int, int)`.
    /// Buffers can be filled with zeros before recycling them. This is useful if a slice pool works on top of this int pool
    /// and relies on the buffers being filled with zeros to find the non-zero end of slices.
    ///
    /// # Parameters
    /// - `zero_fill_buffers`: if `true`, the buffers are filled with `0`.
    /// - `reuse_first`: if `true`, the first buffer will be reused and calling `IntBlockPool::nextBuffer()` is not needed after reset
    ///   if the block pool was used before, i.e., `IntBlockPool::next_buffer()` was called before.
    pub fn reset(&mut self, zero_fill_buffers: bool, reuse_first: bool) {
        if self.buffer_upto != -1 {
            if zero_fill_buffers {
                for i in 0..(self.buffer_upto + 1) as usize {
                    self.buffers[i].fill(0);
                }
            }
            if self.buffer_upto > 0 || !reuse_first {
                let offset = if reuse_first { 1 } else { 0 };
                self.allocator.borrow_mut().recycle_byte_blocks(
                    &self.buffers,
                    offset,
                    self.buffer_upto + 1,
                );
                for _i in offset as usize..(self.buffer_upto + 1) as usize {
                    self.buffers.pop();
                }
            }

            if reuse_first {
                self.buffer_upto = 0;
                self.int_upto = 0;
                self.int_offset = 0;
            } else {
                self.buffer_upto = -1;
                self.int_upto = Self::INT_BLOCK_SIZE;
                self.int_offset = -Self::INT_BLOCK_SIZE;
            }
        }
    }
    /// Advances the pool to its next buffer. This method should be called once after the constructor
    /// to initialize the pool. In contrast to the constructor, a `IntBlockPool::reset(boolean, boolean)`
    /// call will advance the pool to its first buffer immediately.
    pub fn next_buffer(&mut self) -> Result<()> {
        if self.buffer_upto + 1 == self.buffers.len() as i32 {
            self.buffers
                .push(self.allocator.borrow_mut().get_byte_block());
        }
        // Allocate new buffer and advance the pool to it
        self.buffer_upto += 1;
        self.int_upto = 0;
        match self.int_offset.checked_add(Self::INT_BLOCK_SIZE) {
            Some(val) => {
                self.int_offset = val;
                Ok(())
            }
            None => Err(LuceneError::integer_overflow(
                "Overflow when calculating byte offset.".to_string(),
            )),
        }
    }
    pub fn get_buffer(&mut self, buffer_index: i32) -> &mut Vec<i32> {
        &mut self.buffers[buffer_index as usize]
    }
}

/// Abstract trait for allocating and freeing byte blocks.
pub trait AllocatorI32 {
    fn recycle_byte_blocks(&mut self, blocks: &[Vec<i32>], start: i32, end: i32);
    fn get_byte_block(&mut self) -> Vec<i32>;
    fn get_block_size(&self) -> i32;
}

/// A simple [`AllocatorByte`] that never recycles. */
pub struct DirectAllocatorI32 {
    block_size: i32,
}

impl Default for DirectAllocatorI32 {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectAllocatorI32 {
    pub fn new() -> Self {
        DirectAllocatorI32 {
            block_size: IntBlockPool::INT_BLOCK_SIZE,
        }
    }
}

impl AllocatorI32 for DirectAllocatorI32 {
    fn recycle_byte_blocks(&mut self, _blocks: &[Vec<i32>], _start: i32, _end: i32) {}

    fn get_byte_block(&mut self) -> Vec<i32> {
        vec![0; self.block_size as usize]
    }

    fn get_block_size(&self) -> i32 {
        self.block_size
    }
}
pub enum AllocatorIntEnum {
    DA(DirectAllocatorI32),
}
impl AllocatorI32 for AllocatorIntEnum {
    fn recycle_byte_blocks(&mut self, blocks: &[Vec<i32>], start: i32, end: i32) {
        match self {
            AllocatorIntEnum::DA(da) => da.recycle_byte_blocks(blocks, start, end),
        }
    }

    fn get_byte_block(&mut self) -> Vec<i32> {
        match self {
            AllocatorIntEnum::DA(da) => da.get_byte_block(),
        }
    }

    fn get_block_size(&self) -> i32 {
        match self {
            AllocatorIntEnum::DA(da) => da.get_block_size(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::util::lucene_test_case::random;
    use crate::util::error::lucene_error::{LuceneError, Result};
    use crate::util::int_block_pool::{AllocatorIntEnum, DirectAllocatorI32, IntBlockPool};
    use rand::Rng;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_write_read_reset() -> Result<()> {
        let mut random = random();
        let allocator = Rc::new(RefCell::new(
            AllocatorIntEnum::DA(DirectAllocatorI32::new()),
        ));
        let mut pool = IntBlockPool::with_allocator(allocator);
        pool.next_buffer()?;

        // Write <count> consecutive ints to the buffer, possibly allocating a new buffer
        let count = random.random_range(0..2 * IntBlockPool::INT_BLOCK_SIZE);
        for i in 0..count {
            if pool.int_upto == IntBlockPool::INT_BLOCK_SIZE {
                pool.next_buffer()?;
            }
            let buffer_index = pool.buffer_upto;
            let int_upto = pool.int_upto as usize;
            pool.get_buffer(buffer_index)[int_upto] = i;
            pool.int_upto += 1;
        }

        // Check that all the ints are present in the buffer pool
        for i in 0..count {
            assert_eq!(
                i,
                pool.buffers[(i / IntBlockPool::INT_BLOCK_SIZE) as usize]
                    [(i % IntBlockPool::INT_BLOCK_SIZE) as usize]
            );
        }

        // Reset without filling with zeros and check that the first buffer still has the ints
        let count = count.min(IntBlockPool::INT_BLOCK_SIZE);
        pool.reset(false, true);
        for i in 0..count {
            assert_eq!(i, pool.buffers[0][i as usize]);
        }

        // Reset and fill with zeros, then check there is no data left
        pool.int_upto = count;
        pool.reset(true, true);
        for i in 0..count {
            assert_eq!(0, pool.buffers[0][i as usize]);
        }
        Ok(())
    }
    #[test]
    fn test_too_many_allocs() -> Result<()> {
        let allocator = Rc::new(RefCell::new(
            AllocatorIntEnum::DA(DirectAllocatorI32::new()),
        ));
        let mut pool = IntBlockPool::with_allocator(allocator);
        pool.next_buffer()?;

        let result = (|| {
            for _ in 0..(i32::MAX / IntBlockPool::INT_BLOCK_SIZE + 1) {
                pool.next_buffer()?;
            }
            Ok(())
        })();

        assert!(matches!(result, Err(LuceneError::IntegerOverflow(_))));
        assert!(pool.int_offset + IntBlockPool::INT_BLOCK_SIZE < pool.int_offset);

        Ok(())
    }
}
