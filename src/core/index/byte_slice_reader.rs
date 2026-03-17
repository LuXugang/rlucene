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
use crate::core::index::byte_slice_pool::ByteSlicePool;
use crate::core::store::{DataInput, DataOutput};
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::group_vint_util::GroupVIntUtil;
use crate::core::util::{
  BYTE_BLOCK_MASK, BYTE_BLOCK_SIZE, ByteBlockPool, SliceCopyOps, TryIntoInt,
};
use std::fmt::{Display, Formatter};
use std::ops::Deref;

/// IndexInput that knows how to read the byte slices written by Posting and PostingVector.
/// We read the bytes in each slice until we hit the end of that slice at which point we read the forwarding address of the next slice and then jump to it.
pub(crate) struct ByteSliceReader<P> {
  pool: P,
  buffer_upto: usize,
  upto: usize,
  limit: usize,
  level: usize,
  buffer_offset: usize,
  end_index: usize,
}

impl<P> ByteSliceReader<P> {
  pub(crate) fn new(pool: P) -> Self {
    Self {
      pool,
      buffer_upto: 0,
      upto: 0,
      limit: 0,
      level: 0,
      buffer_offset: 0,
      end_index: 0,
    }
  }

  pub(crate) fn eof(&self) -> bool {
    debug_assert!(self.upto + self.buffer_offset <= self.end_index);
    self.upto + self.buffer_offset == self.end_index
  }

  /// # Note
  /// Not used in Java Lucene; kept for API completeness.
  #[allow(dead_code)]
  pub(crate) fn write(&self, _out: &mut impl DataOutput) -> i64 {
    0
  }
}

impl<P> ByteSliceReader<P>
where
  P: Deref<Target = ByteBlockPool>,
{
  pub(crate) fn init(&mut self, start_index: usize, end_index: usize) {
    debug_assert!(end_index >= start_index);

    self.end_index = end_index;
    self.level = 0;

    self.buffer_upto = start_index / BYTE_BLOCK_SIZE as usize;
    self.buffer_offset = self.buffer_upto * BYTE_BLOCK_SIZE as usize;
    self.upto = start_index & BYTE_BLOCK_MASK as usize;

    let first_size = ByteSlicePool::LEVEL_SIZE_ARRAY[0] as usize;

    if start_index + first_size >= end_index {
      // Only one slice
      self.limit = end_index & BYTE_BLOCK_MASK as usize;
    } else {
      self.limit = self.upto + first_size - 4;
    }
  }

  pub(crate) fn next_slice(&mut self) -> Result<()> {
    let buffer = self.pool.get_buffer(self.buffer_upto);
    let next_index = BitUtil::get_i32_le(buffer, self.limit).try_convert()?;

    self.level = ByteSlicePool::NEXT_LEVEL_ARRAY[self.level] as usize;
    let new_size = ByteSlicePool::LEVEL_SIZE_ARRAY[self.level] as usize;

    self.buffer_upto = next_index / BYTE_BLOCK_SIZE as usize;
    self.buffer_offset = self.buffer_upto * BYTE_BLOCK_SIZE as usize;
    self.upto = next_index & BYTE_BLOCK_MASK as usize;

    if next_index + new_size >= self.end_index {
      // Final slice
      debug_assert!(self.end_index - next_index > 0);
      self.limit = self.end_index - self.buffer_offset;
    } else {
      // Intermediate slice (reserve 4 bytes for forwarding address)
      self.limit = self.upto + new_size - 4;
    }
    Ok(())
  }
}

impl<P> DataInput for ByteSliceReader<P>
where
  P: Deref<Target = ByteBlockPool>,
{
  fn read_byte(&mut self) -> Result<u8> {
    debug_assert!(!self.eof());
    debug_assert!(self.upto <= self.limit);

    if self.upto == self.limit {
      self.next_slice()?;
    }

    let byte = self.pool.get_buffer(self.buffer_upto)[self.upto];
    self.upto += 1;
    Ok(byte)
  }

  fn read_bytes(&mut self, b: &mut [u8], mut offset: usize, mut len: usize) -> Result<()> {
    while len > 0 {
      let num_left = self.limit - self.upto;
      if num_left < len {
        {
          let buffer = self.pool.get_buffer(self.buffer_upto);
          b.copy_from(&buffer[self.upto..self.upto + num_left], offset);
        }
        offset += num_left;
        len = len.checked_sub(num_left).ok_or_else(|| {
          LuceneError::illegal_state(format!("underflow, len {}, num_left {} ", len, num_left))
        })?;
        self.next_slice()?;
      } else {
        let buffer = self.pool.get_buffer(self.buffer_upto);
        b.copy_from(&buffer[self.upto..(self.upto + len)], offset);
        self.upto += len;
        break;
      }
    }
    Ok(())
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    GroupVIntUtil::read_group_vint_i32(self, dst, offset)
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    if num_bytes < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "num_bytes must be >= 0, got {num_bytes}"
      )));
    }
    let mut num_bytes = num_bytes.try_convert()?;
    while num_bytes > 0 {
      let num_left = self.limit - self.upto;
      if num_left < num_bytes {
        num_bytes = num_bytes.checked_sub(num_left).ok_or_else(|| {
          LuceneError::illegal_state(format!(
            "underflow, num_bytes {}, num_left {} ",
            num_bytes, num_left
          ))
        })?;
        self.next_slice()?;
      } else {
        self.upto += num_bytes;
        break;
      }
    }
    Ok(())
  }
}

impl<P> Display for ByteSliceReader<P> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

#[cfg(test)]
mod tests {
  use rand::Rng;
  use rand::RngExt;
  use std::rc::Rc;

  use crate::core::index::byte_slice_pool::ByteSlicePool;
  use crate::core::index::byte_slice_reader::ByteSliceReader;
  use crate::core::store::DataInput;
  use crate::core::util::allocator_byte::{AllocatorByteEnum, DirectAllocatorByte};
  use crate::core::util::error::lucene_error::Result;
  use crate::core::util::{ByteBlockPool, TryIntoInt};
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, random};
  use crate::test::core::util::test_util::TestUtil;

  #[allow(dead_code)] // for quick search
  struct TestByteSliceReader;

  #[allow(clippy::type_complexity)]
  pub fn before_class<R: Rng + ?Sized>(random: &mut R) -> Result<(Vec<u8>, ByteBlockPool, i32)> {
    let len = 100; // You can adjust this value if needed
    let random_data: Vec<u8> = (0..len).map(|_| random.random()).collect(); // Fill RANDOM_DATA with random bytes

    let allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
    let mut block_pool = ByteBlockPool::new(allocator);
    block_pool.next_buffer()?;

    let mut slice_pool = ByteSlicePool;
    let mut buffer_upto = block_pool.buffer_upto()?;
    let mut upto = slice_pool.new_slice(ByteSlicePool::FIRST_LEVEL_SIZE, &mut block_pool)?;
    for &random_byte in random_data.iter() {
      let mut buffer = block_pool.get_buffer_mut(buffer_upto);
      let value = buffer[upto as usize];
      if (value & 16) != 0 {
        upto = slice_pool.alloc_slice(buffer_upto, upto, &mut block_pool)?;
      }
      buffer_upto = block_pool.buffer_upto()?;
      buffer = block_pool.get_buffer_mut(buffer_upto);
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
    let mut reader = ByteSliceReader::new(&block_pool);
    reader.init(0, block_pool_end.try_convert()?);
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
    let mut slice_reader = ByteSliceReader::new(Rc::new(block_pool));
    let max_skip_to = random_data.len() as i32 - 1;
    let iterations = at_least(&mut random, 10);
    for _ in 0..iterations {
      slice_reader.init(0, block_pool_end.try_convert()?);
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
