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
// Migrated from src/core/util/paged_bytes.rs

use crate::test::core::util::lucene_test_case::{
  at_least, is_night_mode, new_directory_shared, random,
};
use rand::RngExt;

use crate::core::index::BytesRef;
#[cfg(feature = "nightly")]
use crate::core::store::IndexOutput;
use crate::core::store::directory::Directory;
use crate::core::store::{DataInput, DataOutput, IOContext, IndexInput};
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::paged_bytes::{PagedBytes, get_data_input, get_data_output};
use crate::test::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestPagedBytes;
#[test]
fn test_data_input_output() -> Result<()> {
  let mut random = random();
  let num_iters = at_least(&mut random, 1);

  for _ in 0..num_iters {
    // TODO: BaseDirectoryWrapper not implement
    let dir = new_directory_shared(&mut random)?;
    let block_bits = TestUtil::next_int(&mut random, 1, 20);
    let block_size = 1 << block_bits;
    let mut paged_bytes = PagedBytes::new(block_bits as usize);

    let num_bytes = if is_night_mode() {
      TestUtil::next_usize(&mut random, 2, 10_000_000)
    } else {
      TestUtil::next_usize(&mut random, 2, 1_000_000)
    };

    let mut answer = vec![0u8; num_bytes];
    random.fill(&mut answer[..]);

    {
      let mut out = dir.create_output("foo", &IOContext::default_io_context()?)?;
      let mut written: usize = 0;
      while written < num_bytes {
        if random.random_range(0..100) == 7 {
          out.write_byte(answer[written])?;
          written += 1;
        } else {
          let chunk = std::cmp::min(random.random_range(1..1000), num_bytes - written);
          out.write_bytes_range(&answer, written, chunk)?;
          written += chunk;
        }
      }
    }

    let mut input = dir.open_input("foo", &IOContext::default_io_context()?)?;
    let mut clone_input = input.try_clone()?;

    let len = input.length()?;
    paged_bytes.copy_with_input(&mut input, len)?;
    let reader = paged_bytes.freeze(random.random_bool(0.5))?;

    let mut verify = vec![0u8; num_bytes];
    let mut read = 0;
    while read < num_bytes {
      if random.random_range(0..100) == 7 {
        verify[read] = clone_input.read_byte()?;
        read += 1;
      } else {
        let chunk = std::cmp::min(random.random_range(1..1000), num_bytes - read);
        clone_input.read_bytes(&mut verify, read, chunk)?;
        read += chunk;
      }
    }

    assert_eq!(answer, verify);

    let mut slice = BytesRef::new();
    for _ in 0..100 {
      let pos = random.random_range(0..num_bytes - 1);
      assert_eq!(reader.get_byte(pos), answer[pos]);

      let len = random.random_range(0..std::cmp::min(block_size + 1, num_bytes - pos));
      reader.fill_slice(&mut slice, pos, len);

      for i in 0..len {
        assert_eq!(
          slice.bytes[slice.offset + i],
          answer[pos + i],
          "byte mismatch at pos {} + {}",
          pos,
          i
        );
      }
    }
  }

  Ok(())
}
// Writes random byte/s into PagedBytes via
// .getDataOutput(), then verifies with
// PagedBytes.getDataInput():
#[test]
fn test_data_input_output_2() -> Result<()> {
  let mut random = random();
  let num_iters = at_least(&mut random, 1);

  for _ in 0..num_iters {
    let block_bits = TestUtil::next_int(&mut random, 1, 20);
    let block_size = 1 << block_bits;
    let paged_bytes = PagedBytes::new(block_bits as usize);
    let mut out = get_data_output(paged_bytes)?;

    let num_bytes = if is_night_mode() {
      TestUtil::next_int(&mut random, 1, 10_000_000)
    } else {
      TestUtil::next_int(&mut random, 1, 1_000_000)
    } as usize;

    let mut answer = vec![0u8; num_bytes];
    random.fill(&mut answer[..]);

    let mut written = 0;
    while written < num_bytes {
      if random.random_range(0..10) == 7 {
        out.write_byte(answer[written])?;
        written += 1;
      } else {
        let chunk = std::cmp::min(random.random_range(1..1000), num_bytes - written);
        out.write_bytes_range(&answer, written, chunk)?;
        written += chunk;
      }
    }

    let reader = out.paged_bytes.freeze(random.random_bool(0.5))?;
    let paged_bytes = std::mem::take(&mut out.paged_bytes);
    let mut input = get_data_input(&paged_bytes)?;

    let mut verify = vec![0u8; num_bytes];
    let mut read = 0;
    while read < num_bytes {
      if random.random_range(0..10) == 7 {
        verify[read] = input.read_byte()?;
        read += 1;
      } else {
        let chunk = std::cmp::min(random.random_range(1..1000), num_bytes - read);
        input.read_bytes(&mut verify, read, chunk)?;
        read += chunk;
      }
    }

    assert_eq!(answer, verify);

    let mut slice = BytesRef::new();
    for _ in 0..100 {
      let pos = random.random_range(0..num_bytes - 1);
      let len = random.random_range(0..std::cmp::min(block_size + 1, num_bytes - pos));
      reader.fill_slice(&mut slice, pos, len);
      for byte_upto in 0..len {
        assert_eq!(
          slice.bytes[slice.offset + byte_upto],
          answer[pos + byte_upto],
          "byte mismatch at pos {} + {}",
          pos,
          byte_upto
        );
      }
    }

    let mut input2 = get_data_input(&paged_bytes)?;
    let mut curr = 0;
    let max_skip_to = num_bytes - 1;
    while curr < max_skip_to {
      let skip_to = TestUtil::next_usize(&mut random, curr, max_skip_to);
      let step = skip_to - curr;
      input2.skip_bytes(step as i64)?;
      assert_eq!(answer[skip_to], input2.read_byte()?);
      curr = skip_to + 1;
    }
  }

  Ok(())
}
#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"] // memory hole
fn test_overflow() -> Result<()> {
  let mut random = random();
  // TODO: BaseDirectoryWrapper not implement
  let dir = new_directory_shared(&mut random)?;
  let block_bits = TestUtil::next_int(&mut random, 14, 28);
  let block_size = 1 << block_bits;

  let arr_len = TestUtil::next_usize(&mut random, block_size / 2, block_size * 2);
  let mut arr = vec![0u8; arr_len];
  for (i, byte) in arr.iter_mut().enumerate().take(arr_len) {
    *byte = i as u8;
  }

  let extra = TestUtil::next_usize(&mut random, 1, block_size * 3);
  let num_bytes = (1 << 31) + extra;

  let mut paged_bytes = PagedBytes::new(block_bits as usize);
  {
    let mut out = dir.create_output("foo", &IOContext::default_io_context()?)?;

    let mut written = 0;
    while written < num_bytes {
      assert_eq!(written, out.get_file_pointer()?);
      let len = std::cmp::min(arr.len(), num_bytes - written) as usize;
      out.write_bytes_range(&arr, 0, len)?;
      written += len;
    }
    assert_eq!(num_bytes, out.get_file_pointer()?);
  }

  let mut input = dir.open_input("foo", &IOContext::default_io_context()?)?;
  paged_bytes.copy_with_input(&mut input, num_bytes)?;
  let reader = paged_bytes.freeze(random.random_bool(0.5))?;

  let test_offsets = [
    0,
    i32::MAX as usize,
    num_bytes - 1,
    TestUtil::next_usize(&mut random, 1, num_bytes - 2),
  ];

  let mut b = BytesRef::new();
  for offset in test_offsets.into_iter() {
    reader.fill_slice(&mut b, offset, 1);
    let expected = arr[offset % arr.len()];
    assert_eq!(expected, b.bytes[b.offset], "Mismatch at offset {}", offset);
  }
  Ok(())
}
#[test]
fn test_ram_bytes_used() -> Result<()> {
  // TODO: memory calculation not implement
  Ok(())
}
