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
use std::fmt::{Display, Formatter};

use crate::index::byte_slice_pool::ByteSlicePool;
use crate::store::{DataInput, DataOutput};
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{ByteBlockPool, ByteBlockPoolBorrow, SliceCopyOps};

pub(crate) struct ByteSliceReader {
    pool: Option<ByteBlockPoolBorrow>,
    buffer_upto: i32,
    upto: i32,
    limit: i32,
    level: i32,
    buffer_offset: i32,
    end_index: i32,
}
#[allow(unused)]
impl ByteSliceReader {
    pub(crate) fn new() -> Self {
        ByteSliceReader {
            pool: None,
            buffer_upto: 0,
            upto: 0,
            limit: 0,
            level: 0,
            buffer_offset: 0,
            end_index: 0,
        }
    }
    pub(crate) fn init(&mut self, pool: ByteBlockPoolBorrow, start_index: i32, end_index: i32) {
        debug_assert!(end_index - start_index >= 0);
        debug_assert!(start_index >= 0);
        debug_assert!(end_index >= 0);
        self.pool = Some(pool);
        self.end_index = end_index;

        self.level = 0;
        self.buffer_upto = start_index / ByteBlockPool::BYTE_BLOCK_SIZE;
        self.buffer_offset = self.buffer_upto * ByteBlockPool::BYTE_BLOCK_SIZE;
        self.upto = start_index & ByteBlockPool::BYTE_BLOCK_MASK;

        let first_size = ByteSlicePool::LEVEL_SIZE_ARRAY[0];

        if start_index + first_size >= end_index {
            // There is only this one slice to read
            self.limit = end_index & ByteBlockPool::BYTE_BLOCK_MASK;
        } else {
            self.limit = self.upto + first_size - 4;
        }
    }

    pub(crate) fn eof(&self) -> bool {
        debug_assert!(self.upto + self.buffer_offset <= self.end_index);
        self.upto + self.buffer_offset == self.end_index
    }
    /// # Note
    /// not used in Java Lucene, so it is not implemented here
    pub(crate) fn write(&self, _out: &mut impl DataOutput) -> i64 {
        0
    }

    pub(crate) fn next_slice(&mut self) {
        // Skip to our next slice

        debug_assert!(self.pool.is_some());
        let mut pool = self.pool.as_ref().unwrap().borrow_mut();
        let buffer = pool.get_buffer(self.buffer_upto);
        let next_index = BitUtil::get_i32_le(buffer, self.limit as usize);
        self.level = ByteSlicePool::NEXT_LEVEL_ARRAY[self.level as usize];
        let new_size = ByteSlicePool::LEVEL_SIZE_ARRAY[self.level as usize];

        self.buffer_upto = next_index / ByteBlockPool::BYTE_BLOCK_SIZE;
        self.buffer_offset = self.buffer_upto * ByteBlockPool::BYTE_BLOCK_SIZE;

        self.upto = next_index & ByteBlockPool::BYTE_BLOCK_MASK;

        if next_index + new_size >= self.end_index {
            // We are advancing to the final slice
            debug_assert!(self.end_index - next_index > 0);
            self.limit = self.end_index - self.buffer_offset;
        } else {
            // This is not the final slice (subtract 4 for the forwarding
            // address at the end of this new slice)
            self.limit = self.upto + new_size - 4;
        }
    }
}

impl Display for ByteSliceReader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ByteSliceReader")
    }
}

impl DataInput for ByteSliceReader {
    fn read_byte(&mut self) -> Result<u8> {
        debug_assert!(!self.eof());
        debug_assert!(self.upto <= self.limit);
        if self.upto == self.limit {
            self.next_slice();
        }
        debug_assert!(self.pool.is_some());
        let mut pool = self.pool.as_ref().unwrap().borrow_mut();
        let byte = pool.get_buffer(self.buffer_upto)[self.upto as usize];
        self.upto += 1;
        Ok(byte)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, mut len: i32) -> Result<()> {
        let mut offset = offset as usize;
        debug_assert!(self.pool.is_some());
        while len > 0 {
            let num_left = self.limit - self.upto;
            if num_left < len {
                // Read entire slice
                {
                    let mut pool = self.pool.as_ref().unwrap().borrow_mut();
                    let buffer = pool.get_buffer(self.buffer_upto);
                    b.copy_from(
                        &buffer[self.upto as usize..self.upto as usize + num_left as usize],
                        offset,
                    );
                }
                offset += num_left as usize;
                len -= num_left;
                self.next_slice();
            } else {
                let mut pool = self.pool.as_ref().unwrap().borrow_mut();
                // This slice is the last one
                let buffer = pool.get_buffer(self.buffer_upto);
                b.copy_from(
                    &buffer[self.upto as usize..(self.upto + len) as usize],
                    offset,
                );
                self.upto += len;
                break;
            }
        }
        Ok(())
    }

    fn skip_bytes(&mut self, mut num_bytes: i64) -> Result<()> {
        if num_bytes < 0 {
            return Err(LuceneError::illegal_argument(
                "numBytes must be >= 0".to_string(),
            ));
        }
        while num_bytes > 0 {
            let num_left = (self.limit - self.upto) as i64;
            if num_left < num_bytes {
                num_bytes -= num_left;
                self.next_slice();
            } else {
                debug_assert!(num_bytes <= i32::MAX as i64);
                self.upto += num_bytes as i32;
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use rand::rngs::StdRng;
    use rand::Rng;

    use crate::index::byte_slice_pool::ByteSlicePool;
    use crate::index::byte_slice_reader::ByteSliceReader;
    use crate::store::DataInput;
    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::allocator_byte::{AllocatorByteEnum, DirectAllocatorByte};
    use crate::util::error::lucene_error::Result;
    use crate::util::{ByteBlockPool, ByteBlockPoolBorrow};

    #[allow(dead_code)] // for quick search
    struct TestByteSliceReader;

    #[allow(clippy::type_complexity)]
    pub fn before_class(random: &mut StdRng) -> Result<(Vec<u8>, ByteBlockPoolBorrow, i32)> {
        let len = 100; // You can adjust this value if needed
        let random_data: Vec<u8> = (0..len).map(|_| random.random()).collect(); // Fill RANDOM_DATA with random bytes

        let allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
        let block_pool = Rc::new(RefCell::new(ByteBlockPool::new(allocator)));
        block_pool.borrow_mut().next_buffer()?;

        let mut slice_pool = ByteSlicePool::new(block_pool.clone());
        let mut buffer_upto = block_pool.borrow().buffer_upto;
        let mut upto = slice_pool.new_slice(ByteSlicePool::FIRST_LEVEL_SIZE)?;
        for &random_byte in random_data.iter() {
            let mut pool_guard = block_pool.borrow_mut();
            let mut buffer = pool_guard.get_buffer(buffer_upto);
            let value = buffer[upto as usize];
            drop(pool_guard);
            if (value & 16) != 0 {
                upto = slice_pool.alloc_slice(buffer_upto, upto)?;
            }
            pool_guard = block_pool.borrow_mut();
            buffer_upto = pool_guard.buffer_upto;
            buffer = pool_guard.get_buffer(buffer_upto);
            buffer[upto as usize] = random_byte;
            upto += 1;
        }
        let block_pool_end = upto;
        Ok((random_data, block_pool, block_pool_end))
    }
    #[test]
    fn test_read_byte() -> Result<()> {
        let mut random = random();
        let (random_data, block_pool, block_pool_end) = before_class(&mut random)?;
        let mut reader = ByteSliceReader::new();
        reader.init(block_pool.clone(), 0, block_pool_end);
        for &expected in random_data.iter() {
            let byte = reader.read_byte()?;
            assert_eq!(byte, expected);
        }
        Ok(())
    }
    #[test]
    fn test_skip_bytes() -> Result<()> {
        let mut random = random();
        let (random_data, block_pool, block_pool_end) = before_class(&mut random)?;
        let mut slice_reader = ByteSliceReader::new();
        let max_skip_to = random_data.len() as i32 - 1;
        let iterations = at_least(&mut random, 10);
        for _ in 0..iterations {
            slice_reader.init(block_pool.clone(), 0, block_pool_end);
            // Skip random chunks of bytes until exhausted
            let mut curr = 0;
            while curr < max_skip_to {
                let skip_to = TestUtil::next_int(&mut random, curr, max_skip_to);
                let step = skip_to - curr;
                slice_reader.skip_bytes(step as i64)?;
                assert_eq!(random_data[skip_to as usize], slice_reader.read_byte()?);
                curr = skip_to + 1; // +1 for read byte
            }
        }
        Ok(())
    }
}
