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

use crate::test::core::util::lucene_test_case::random;
use rand::RngExt;

use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::int_block_pool::{
  AllocatorIntEnum, DirectAllocatorI32, INT_BLOCK_SIZE, IntBlockPool,
};
#[allow(dead_code)] // for quick search
struct TestIntBlockPool;
#[test]
fn test_write_read_reset() -> Result<()> {
  let mut random = random();
  let allocator = AllocatorIntEnum::DA(DirectAllocatorI32::new());
  let mut pool = IntBlockPool::with_allocator(allocator);
  pool.next_buffer()?;

  // Write <count> consecutive ints to the buffer, possibly allocating a
  // new buffer
  let count = random.random_range(0..2 * INT_BLOCK_SIZE);
  for i in 0..count {
    if pool.int_upto == INT_BLOCK_SIZE {
      pool.next_buffer()?;
    }
    let buffer_index = pool.buffer_upto;
    let int_upto = pool.int_upto as usize;
    pool.get_buffer_mut(buffer_index)[int_upto] = i;
    pool.int_upto += 1;
  }

  // Check that all the ints are present in the buffer pool
  for i in 0..count {
    assert_eq!(
      i,
      pool.buffers[(i / INT_BLOCK_SIZE) as usize][(i % INT_BLOCK_SIZE) as usize]
    );
  }

  // Reset without filling with zeros and check that the first buffer
  // still has the ints
  let count = count.min(INT_BLOCK_SIZE);
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
  let allocator = AllocatorIntEnum::DA(DirectAllocatorI32::new());
  let mut pool = IntBlockPool::with_allocator(allocator);
  pool.next_buffer()?;

  let result = (|| {
    for _ in 0..(i32::MAX / INT_BLOCK_SIZE + 1) {
      pool.next_buffer()?;
    }
    Ok(())
  })();

  assert!(matches!(result, Err(LuceneError::NumberOverflow(_))));
  assert!(pool.int_offset + INT_BLOCK_SIZE < pool.int_offset);

  Ok(())
}
