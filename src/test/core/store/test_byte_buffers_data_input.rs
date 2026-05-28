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
use rand::RngExt;
use rand_xoshiro::Xoroshiro128Plus;
use rand_xoshiro::rand_core::SeedableRng;

use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::store::{ByteBuffersDataOutput, DataInput};
use crate::core::util::error::lucene_error::Result;
use crate::test::core::store::base_data_output_test_case::add_random_data;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::is_night_mode;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;

#[allow(dead_code)] // for quick search
struct TestByteBuffersDataInput;

#[test]
fn test_sanity() -> Result<()> {
  let mut out = ByteBuffersDataOutput::new();
  let mut o1 = out.get_data_input_ref()?;
  assert_eq!(0, o1.length());
  let mut result = DataInput::read_byte(&mut o1);
  assert!(result.is_err());

  out.write_byte(1)?;
  // TODO: how to assert o1's length not modified?
  // assert_eq!(0, o1.length());
  let mut o2 = out.get_data_input_ref()?;
  assert_eq!(1, o2.length());
  assert_eq!(0, o2.position()?);

  //TODO
  // assert!(o2.ram_bytes_used() > 0)
  assert_eq!(1, DataInput::read_byte(&mut o2)? as i32);
  assert_eq!(1, o2.position()?);
  assert_eq!(1, RandomAccessInput::read_byte(&mut o2, 0)? as i32);

  result = DataInput::read_byte(&mut o2);
  assert!(result.is_err());
  assert_eq!(1, o2.position()?);
  Ok(())
}

#[test]
fn test_random_reads() -> Result<()> {
  let mut random = random();
  let mut dst = ByteBuffersDataOutput::new();
  let seed: u64 = random.random();
  let mut random1 = Xoroshiro128Plus::seed_from_u64(seed);
  let max = if is_night_mode() { 1000000 } else { 100000 };
  let reply = add_random_data(&mut dst, &mut random1, max);
  let mut src = dst.get_data_input_ref()?;
  for action in reply {
    action.verify(&mut src);
  }
  let result = DataInput::read_byte(&mut src);
  assert!(result.is_err());
  Ok(())
}

#[test]
fn test_random_reads_on_slices() -> Result<()> {
  let mut random = random();
  let reps = random.random_range(1..=20);
  for _i in 0..=reps {
    let mut dst = ByteBuffersDataOutput::new();
    let prefix = vec![0; random.random_range(0..=1024 * 8)];
    let prefix_len = prefix.len();
    dst.write_bytes(prefix.as_slice())?;
    let seed: u64 = random.random();
    let max = 10000;
    let mut random1 = Xoroshiro128Plus::seed_from_u64(seed);
    let reply = add_random_data(&mut dst, &mut random1, max);
    let suffix = vec![0; random.random_range(0..=1024 * 8)];
    let suffix_len = suffix.len();
    dst.write_bytes(suffix.as_slice())?;
    let size = dst.size();
    let mut src = dst
      .get_data_input_ref()?
      .slice(prefix_len, size - suffix_len - prefix_len)?;
    assert_eq!(0, src.position()?);
    assert_eq!(size - prefix_len - suffix_len, src.length());
    for action in reply {
      action.verify(&mut src);
    }
    let result = DataInput::read_byte(&mut src);
    assert!(result.is_err());
  }
  Ok(())
}
#[test]
fn test_seek_empty() -> Result<()> {
  let mut dst = ByteBuffersDataOutput::new();
  let mut data_input = dst.get_data_input_ref()?;
  let mut result = data_input.seek(0);
  assert!(result.is_ok());
  result = data_input.seek(1);
  assert!(result.is_err());
  result = data_input.seek(0);
  assert!(result.is_ok());
  let read_result = DataInput::read_byte(&mut data_input);
  assert!(read_result.is_err());
  Ok(())
}

#[test]
fn test_seek_and_skip() -> Result<()> {
  let mut random = random();
  let reps = random.random_range(1..=20);
  for _i in 0..reps {
    let mut dst = ByteBuffersDataOutput::new();
    let prefix;
    let mut prefix_len = 0;
    if random.random_bool(0.5) {
      let len = random.random_range(1..=1024 * 8);
      prefix = vec![0; len];
      prefix_len = prefix.len();
      dst.write_bytes(prefix.as_slice())?;
    }
    let seed: u64 = random.random();
    let max = 1000;
    let mut random1 = Xoroshiro128Plus::seed_from_u64(seed);
    let reply = add_random_data(&mut dst, &mut random1, max);
    let size = dst.size();
    let mut array = dst.get_array_copy();
    array = Vec::from(&array[prefix_len..array.len()]);
    let mut data_input = dst
      .get_data_input_ref()?
      .slice(prefix_len, size - prefix_len)?;
    data_input.seek(0)?;
    for action in &reply {
      action.verify(&mut data_input);
    }
    data_input.seek(0)?;
    for action in &reply {
      action.verify(&mut data_input);
    }
    for _i in 0..1000 {
      let offs = random.random_range(0..=array.len() - 1);
      data_input.seek(offs)?;
      assert_eq!(offs, data_input.position()?);
      assert_eq!(array[offs], DataInput::read_byte(&mut data_input)?);
    }
    // test skipping
    let max_skip_to = array.len() - 1;
    data_input.seek(0)?;
    // skip chunks of bytes until exhausted
    let mut curr = 0;
    while curr < max_skip_to {
      let skip_to = random.random_range(curr..=max_skip_to);
      let step = skip_to - curr;
      data_input.skip_bytes(step as i64)?;
      assert_eq!(array[skip_to], DataInput::read_byte(&mut data_input)?);
      curr = skip_to + 1;
    }

    data_input.seek(data_input.length())?;
    assert_eq!(data_input.length(), data_input.position()?);
    let result = DataInput::read_byte(&mut data_input);
    assert!(result.is_err());
  }
  Ok(())
}
#[test]
fn test_slicing_window() -> Result<()> {
  let mut random = random();
  let mut dst = ByteBuffersDataOutput::new();
  assert_eq!(0, dst.get_data_input_ref()?.slice(0, 0)?.length());
  let random_bytes = vec![0; random.random_range(0..=1024 * 8)];
  dst.write_bytes(random_bytes.as_slice())?;
  let max = dst.size();
  let data_input = dst.get_data_input_ref()?;
  let mut offset = 0;
  while offset < max {
    assert_eq!(0, data_input.slice(offset, 0)?.length());
    assert_eq!(1, data_input.slice(offset, 1)?.length());

    let window = (max - offset).min(1024);
    assert_eq!(window, data_input.slice(offset, window)?.length());
    offset += 1;
  }
  assert_eq!(0, data_input.slice(max, 0)?.length());
  Ok(())
}

#[test]
fn test_eof_on_array_read_past_buffer_size() -> Result<()> {
  let mut dst = ByteBuffersDataOutput::new();
  let bytes = vec![0; 10];
  dst.write_bytes(bytes.as_slice())?;
  let mut data_input = dst.get_data_input_ref()?;
  let mut output: Vec<u8> = vec![0; 100];
  let result = DataInput::read_bytes(&mut data_input, &mut output, 0, 100);
  assert!(result.is_err());
  Ok(())
}

#[test]
fn test_slicing_large_buffers() -> Result<()> {
  // Simulate a "large" (> 4GB) input by duplicating
  // buffers with the same content.
  let mut random = random();
  let mb = 1024 * 1024;
  let page_bytes: Vec<u8> = vec![0; 4 * mb];
  let simulated_length = random.random_range(0..2018) + 4 * i32::MAX as usize;
  let mut remaining = simulated_length;
  let mut dst = ByteBuffersDataOutput::new();
  while remaining > 0 {
    let mut block = page_bytes.clone();
    if block.len() > remaining {
      block.truncate(remaining);
    }
    let len = block.len();
    dst.write_bytes(block.as_slice())?;
    remaining -= len;
  }
  let data_input = dst.get_data_input_ref()?;
  assert_eq!(simulated_length, data_input.length());
  let max = data_input.length();
  let mut offset = 0;
  while offset < max {
    assert_eq!(0, data_input.slice(offset, 0)?.length());
    assert_eq!(1, data_input.slice(offset, 1)?.length());

    let window = (max - offset).min(1024);
    let mut slice = data_input.slice(offset, window)?;
    assert_eq!(window, slice.length());
    // Sanity check of the content against original pages.
    for i in 0..window {
      let index = (offset + i) % page_bytes.len();
      let expected = page_bytes[index as usize];
      assert_eq!(expected, RandomAccessInput::read_byte(&mut slice, i)?);
    }
    offset += random.random_range(mb..4 * mb);
  }
  Ok(())
}
