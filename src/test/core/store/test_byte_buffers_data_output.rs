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
use crate::test_framework::core::util::lucene_test_case::{
  is_night_mode, random, random_from_seed,
};
use std::io::Cursor;
use std::mem::size_of;

use rand::RngExt;

use crate::core::store::data_output::DataOutput;
use crate::core::store::{ByteArrayDataInput, ByteBuffersDataOutput};
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::store::base_data_output_test_case::{
  BaseDataOutputTestCase, add_random_data,
};

struct TestByteBuffersDataOutput;
impl BaseDataOutputTestCase for TestByteBuffersDataOutput {
  type DO = ByteBuffersDataOutput;

  fn new_instance(&self) -> Result<Self::DO> {
    Ok(ByteBuffersDataOutput::new_resettable_instance())
  }

  fn get_bytes(&mut self, instance: Self::DO) -> Vec<u8> {
    instance.get_array_copy()
  }
}

#[test]
fn test_reuse() -> Result<()> {
  let mut random = random();
  let mut o = ByteBuffersDataOutput::with_reuse(
    ByteBuffersDataOutput::DEFAULT_MIN_BITS_PER_BLOCK,
    ByteBuffersDataOutput::DEFAULT_MAX_BITS_PER_BLOCK,
    true,
  )?;
  // add some random data first
  let gen_seed: u64 = random.random();
  let mut random1 = random_from_seed(gen_seed);
  let mut random2 = random_from_seed(gen_seed);
  let add_count = random.random_range(1000..=5000);
  add_random_data(&mut o, &mut random1, add_count)?;
  let dta = match random.random_bool(0.5) {
    true => o.get_array_copy(),
    false => o.try_get_array_ownership(),
  };

  o.reset();
  add_random_data(&mut o, &mut random2, add_count)?;
  match random.random_bool(0.5) {
    true => {
      assert_eq!(dta, o.get_array_copy());
    },
    false => {
      assert_eq!(dta, o.try_get_array_ownership());
    },
  }
  Ok(())
}
#[test]
fn test_constructor_with_expected_size() -> Result<()> {
  let mut random = random();
  let mut o = ByteBuffersDataOutput::with_size(0)?;
  o.write_byte(0)?;
  let (_length, mut result) = o.to_buffer_list_ref();
  let capacity = result.get_mut(0).unwrap().get_ref().len();
  assert_eq!(
    1 << ByteBuffersDataOutput::DEFAULT_MIN_BITS_PER_BLOCK,
    capacity
  );

  let mb = 1024 * 1024;
  let expected_size: i64 = random.random_range(mb..mb * 1024);
  let mut o = ByteBuffersDataOutput::with_size(expected_size)?;
  o.write_byte(0)?;
  let (_length, mut result) = o.to_buffer_list_ref();
  let cap = result.get_mut(0).unwrap().get_ref().len();
  assert!(
    ((cap >> 1) * ByteBuffersDataOutput::MAX_BLOCKS_BEFORE_BLOCK_EXPANSION as usize)
      < expected_size as usize
  );
  assert!(
    cap * ByteBuffersDataOutput::MAX_BLOCKS_BEFORE_BLOCK_EXPANSION as usize
      >= expected_size as usize
  );
  Ok(())
}

#[test]
fn test_randomized_writes() -> Result<()> {
  let mut test = TestByteBuffersDataOutput;
  let mut random = random();
  // here could use any DataInput impl because this test does not test
  // ByteArrayDataInput
  test.test_randomized_writes::<ByteArrayDataInput<Vec<u8>>, _>(&mut random)
}

#[test]
fn test_illegal_min_bits_per_block() {
  let o = ByteBuffersDataOutput::with_reuse(
    ByteBuffersDataOutput::LIMIT_MIN_BITS_PER_BLOCK - 1,
    ByteBuffersDataOutput::DEFAULT_MAX_BITS_PER_BLOCK,
    false,
  );
  assert!(o.is_err());
}
#[test]
fn test_illegal_max_bits_per_block() {
  let o = ByteBuffersDataOutput::with_reuse(
    ByteBuffersDataOutput::DEFAULT_MIN_BITS_PER_BLOCK,
    ByteBuffersDataOutput::LIMIT_MIN_BITS_PER_BLOCK + 1,
    false,
  );
  assert!(o.is_err());
}
#[test]
fn test_illegal_bits_per_block_range() {
  let o = ByteBuffersDataOutput::with_reuse(20, 19, false);
  assert!(o.is_err());
}
#[test]
#[ignore = "Java-only: Rust allocator callbacks cannot be null references"]
fn test_null_allocator() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
#[ignore = "Java-only: Rust recycler callbacks cannot be null references"]
fn test_null_recycler() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_sanity() -> Result<()> {
  let mut random = random();
  let case = TestByteBuffersDataOutput;
  let mut o = case.new_instance()?;

  assert_eq!(o.size(), 0);
  assert_eq!(o.get_array_copy().len(), 0);
  // Java allocates its first block lazily and therefore reports zero here.
  // Rust eagerly retains the first reusable block, so it must be accounted for.
  let initial_ram_bytes_used = o.ram_bytes_used()?;
  assert!(initial_ram_bytes_used > 0);

  o.write_byte(1)?;
  assert_eq!(o.size(), 1);
  assert_eq!(initial_ram_bytes_used, o.ram_bytes_used()?);
  assert_eq!(o.get_array_copy(), vec![1]);

  o.write_bytes_with_len(&[2, 3, 4], 3)?;
  assert_eq!(o.size(), 4);

  match random.random_bool(0.5) {
    true => {
      assert_eq!(o.get_array_copy(), vec![1, 2, 3, 4]);
    },
    false => {
      assert_eq!(o.try_get_array_ownership(), vec![1, 2, 3, 4]);
    },
  }
  Ok(())
}
#[test]
#[ignore = "Java-only: Rust has no java.nio.ByteBuffer position/limit write overload"]
fn test_write_byte_buffer() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_large_array_add() -> Result<()> {
  let mut random = random();
  let mut o = ByteBuffersDataOutput::new_resettable_instance();
  let mb = 1024 * 1024;
  let mut bytes = if is_night_mode() {
    let size = random.random_range(5 * mb..=15 * mb);
    vec![0u8; size]
  } else {
    let size = random.random_range(mb / 2..=mb);
    vec![0u8; size]
  };

  bytes.iter_mut().for_each(|byte| *byte = random.random());
  let offset = random.random_range(0..=100);
  let len = bytes.len() - offset;
  o.write_bytes_range(&bytes, offset, len)?;
  assert_eq!(len, o.size());
  let expected = bytes[offset..offset + len].to_vec();
  assert_eq!(expected, o.get_array_copy());
  match random.random_bool(0.5) {
    true => {
      assert_eq!(expected, o.get_array_copy());
    },
    false => {
      assert_eq!(expected, o.try_get_array_ownership());
    },
  }
  Ok(())
}
#[test]
fn test_copy_bytes_on_heap() -> Result<()> {
  let mut random = random();
  let mut bytes = vec![0u8; 1024 * 8 + 10];
  random.fill(&mut bytes[..]);
  let offset = random.random_range(0..=100);
  let len = bytes.len() - offset;
  let mut input = ByteArrayDataInput::with_range(bytes.as_slice(), offset, len);

  let mut o = ByteBuffersDataOutput::with_reuse(
    ByteBuffersDataOutput::DEFAULT_MIN_BITS_PER_BLOCK,
    ByteBuffersDataOutput::DEFAULT_MAX_BITS_PER_BLOCK,
    false,
  )?;
  o.copy_bytes(&mut input, len)?;
  let expected = bytes[offset..offset + len].to_vec();
  match random.random_bool(0.5) {
    true => {
      assert_eq!(o.get_array_copy(), expected);
    },
    false => {
      assert_eq!(o.try_get_array_ownership(), expected);
    },
  }
  Ok(())
}
#[test]
fn test_copy_bytes_on_direct_byte_buffer() -> Result<()> {
  let mut random = random();
  let mut bytes = vec![0u8; 1024 * 8 + 10];
  random.fill(&mut bytes[..]);
  let offset = random.random_range(0..=100);
  let len = bytes.len() - offset;
  let mut input = ByteArrayDataInput::with_range(bytes.as_slice(), offset, len);
  let mut o = ByteBuffersDataOutput::with_reuse(
    ByteBuffersDataOutput::DEFAULT_MIN_BITS_PER_BLOCK,
    ByteBuffersDataOutput::DEFAULT_MAX_BITS_PER_BLOCK,
    false,
  )?;
  o.copy_bytes(&mut input, len)?;
  let expected = bytes[offset..offset + len].to_vec();
  match random.random_bool(0.5) {
    true => {
      assert_eq!(o.get_array_copy(), expected);
    },
    false => {
      assert_eq!(o.try_get_array_ownership(), expected);
    },
  }
  Ok(())
}
#[test]
#[ignore = "Java-only: Rust exposes immutable buffer borrows through the type system"]
fn test_to_buffer_list_returns_read_only_buffers() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
#[ignore = "Java-only: Rust exposes writable buffer borrows through a distinct mutable API"]
fn test_to_writeable_buffer_list_returns_original_buffers() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_ram_bytes_used() -> Result<()> {
  let mut out = ByteBuffersDataOutput::new();
  // Java allocates lazily and expects no retained RAM for an empty output.
  // Rust eagerly allocates its first block, so the empty output retains RAM.
  let empty_ram_bytes_used = out.ram_bytes_used()?;
  assert!(empty_ram_bytes_used >= compute_ram_bytes_used(&out));

  // A non-empty buffer requires RAM, but writing into the eagerly allocated
  // first block does not require another allocation.
  out.write_int(4)?;
  assert_eq!(empty_ram_bytes_used, out.ram_bytes_used()?);
  assert!(out.ram_bytes_used()? >= compute_ram_bytes_used(&out));

  // Make sure this keeps working with multiple backing buffers.
  while out.to_buffer_list_ref().1.len() < 2 {
    out.write_long(42)?;
  }
  let multiple_buffers_ram_bytes_used = out.ram_bytes_used()?;
  assert!(multiple_buffers_ram_bytes_used > empty_ram_bytes_used);
  assert!(multiple_buffers_ram_bytes_used >= compute_ram_bytes_used(&out));

  // Make sure this keeps working when increasing the block size.
  let current_block_capacity = out.to_buffer_list_ref().1[0].get_ref().len();
  while out.to_buffer_list_ref().1[0].get_ref().len() == current_block_capacity {
    out.write_long(42)?;
  }
  let increased_block_ram_bytes_used = out.ram_bytes_used()?;
  assert!(increased_block_ram_bytes_used >= compute_ram_bytes_used(&out));

  // Back to zero after a clear.
  out.reset();
  assert_eq!(0, out.size());
  let reset_ram_bytes_used = out.ram_bytes_used()?;
  assert_eq!(0, reset_ram_bytes_used);
  assert_eq!(compute_ram_bytes_used(&out), reset_ram_bytes_used);

  // And back to non-empty.
  out.write_int(4)?;
  assert!(out.ram_bytes_used()? > reset_ram_bytes_used);
  assert!(out.ram_bytes_used()? >= compute_ram_bytes_used(&out));
  Ok(())
}

fn compute_ram_bytes_used(out: &ByteBuffersDataOutput) -> i64 {
  let (_, buffers) = out.to_buffer_list_ref();
  let buffer_bytes = buffers
    .iter()
    .map(|buffer| buffer.get_ref().len())
    .sum::<usize>();
  let initialized_cursor_bytes = buffers.len().saturating_mul(size_of::<Cursor<Vec<u8>>>());
  buffer_bytes.saturating_add(initialized_cursor_bytes) as i64
}
