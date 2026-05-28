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
use rand::Rng;
use rand::RngExt;
use std::rc::Rc;

use crate::core::index::byte_slice_pool::ByteSlicePool;
use crate::core::index::byte_slice_reader::ByteSliceReader;
use crate::core::store::DataInput;
use crate::core::util::allocator_byte::DirectAllocatorByte;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{ByteBlockPool, TryIntoInt};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, random};
use crate::test::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestByteSliceReader;

#[allow(clippy::type_complexity)]
pub fn set_up<R>(random: &mut R) -> Result<(Vec<u8>, ByteBlockPool, i32)>
where
  R: Rng + ?Sized,
{
  let len = 100; // You can adjust this value if needed
  let random_data: Vec<u8> = (0..len).map(|_| random.random()).collect(); // Fill RANDOM_DATA with random bytes

  let allocator = DirectAllocatorByte::new();
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
  let (random_data, block_pool, block_pool_end) = set_up(&mut random)?;
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
  let (random_data, block_pool, block_pool_end) = set_up(&mut random)?;
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
