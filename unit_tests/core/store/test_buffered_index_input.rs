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
use crate::test_framework::core::util::lucene_test_case::random;
use std::clone::Clone;
use std::io::Cursor;

use byteorder::WriteBytesExt;
use rand::RngExt;

use crate::core::store::index_input::IndexInput;
use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::store::{BUFFER_SIZE, BufferedIndexInput, BufferedIndexInputBase, DataInput};
use crate::core::util::ReadableCursorExt;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::clone::TryClone as OtherClone;
use crate::core::util::error::lucene_error::{LuceneError, Result};

#[allow(dead_code)] // for quick search
struct TestBufferedIndexInput;

const TEST_FILE_LENGTH: usize = 100 * 1024;

#[test]
// Call readByte() repeatedly, past the buffer boundary, and see that it
// is working as expected.
// Our input comes from a dynamically generated/ "file" - see
// MyBufferedIndexInput below.
fn test_read_byte() -> Result<()> {
  let sub_index_input = MyBufferedIndexInput::new();
  let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
  let mut input =
    BufferedIndexInput::with_buffer_size(sub_index_input, &resource_description, BUFFER_SIZE)?;
  for i in 0..BUFFER_SIZE * 10 {
    assert_eq!(byten(i), DataInput::read_byte(&mut input)?);
  }

  Ok(())
}

#[test]
fn test_read_bytes() -> Result<()> {
  let mut random = random();
  let sub_index_input = MyBufferedIndexInput::new();
  let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
  let mut input =
    BufferedIndexInput::with_buffer_size(sub_index_input, &resource_description, BUFFER_SIZE)?;

  let mut pos = 0;

  // Gradually increasing size
  let mut size = 1;
  while size < BUFFER_SIZE * 10 {
    let mut buffer: Vec<u8> = vec![0; 10];
    check_read_bytes(&mut input, size, pos, &mut buffer)?;
    pos += size;
    if pos >= TEST_FILE_LENGTH {
      // Wrap around
      pos = 0;
      input.seek(0)?;
    }
    size += size / 200 + 1;
  }

  // Wildly fluctuating size
  for _ in 0..100 {
    let size = random.random_range(1..=10000);
    let mut buffer: Vec<u8> = vec![0; 10];
    check_read_bytes(&mut input, size, pos, &mut buffer)?;
    pos += size;
    if pos >= TEST_FILE_LENGTH {
      // Wrap around
      pos = 0;
      input.seek(0)?;
    }
  }

  // Constant small size (7 bytes)
  for _ in 0..BUFFER_SIZE {
    let mut buffer: Vec<u8> = vec![0; 10];
    check_read_bytes(&mut input, 7, pos, &mut buffer)?;
    pos += 7;
    if pos >= TEST_FILE_LENGTH {
      // Wrap around
      pos = 0;
      input.seek(0)?;
    }
  }

  Ok(())
}

fn check_read_bytes(
  input: &mut impl IndexInput,
  size: usize,
  pos: usize,
  buffer: &mut Vec<u8>,
) -> Result<()> {
  // Just to see that "offset" is treated properly in read_bytes(), we
  // add an arbitrary offset at the beginning of the array
  let offset = size % 10; // arbitrary offset
  if buffer.len() < (offset + size) {
    buffer.resize(offset + size, 0); // Grow the buffer as
    // needed
  }

  assert_eq!(
    pos,
    input.get_file_pointer()?,
    "File pointer does not match expected position"
  );

  let left = TEST_FILE_LENGTH.saturating_sub(input.get_file_pointer()?);
  if left == 0 {
    return Ok(()); // No data left to read
  }

  let size_to_read = if left < size { left } else { size };

  input.read_bytes(&mut buffer[offset..offset + size_to_read], 0, size_to_read)?;

  assert_eq!(
    pos + size_to_read,
    input.get_file_pointer()?,
    "File pointer does not match after reading"
  );

  for i in 0..size_to_read {
    let file_pos = pos + i;
    let expected_byte = byten(file_pos);
    let actual_byte = buffer[offset + i];
    assert_eq!(
      expected_byte, actual_byte,
      "Mismatch at pos={}, filepos={}",
      i, file_pos
    );
  }

  Ok(())
}

#[test]
fn test_eof() -> Result<()> {
  let sub_index_input = MyBufferedIndexInput::with_len(1024);
  let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
  let mut input =
    BufferedIndexInput::with_buffer_size(sub_index_input, &resource_description, BUFFER_SIZE)?;
  let mut buffer = vec![];

  // Verify we can read all bytes in one go
  let mut length = IndexInput::length(&input)?;
  check_read_bytes(&mut input, length, 0, &mut buffer)?;

  // Attempt to read more than the available bytes for small and large
  // overflows
  length = IndexInput::length(&input)?;
  let pos = length - 10;
  input.seek(pos)?;

  // Small overflow: read exactly remaining bytes
  check_read_bytes(&mut input, 10, pos, &mut buffer)?;

  input.seek(pos)?;

  // Test block read past end of file
  let mut result = check_read_bytes(&mut input, 11, pos, &mut buffer);
  assert!(matches!(result, Err(LuceneError::Eof(_))));

  input.seek(pos)?;

  result = check_read_bytes(&mut input, 50, pos, &mut buffer);
  // Test large block read past end of file
  assert!(matches!(result, Err(LuceneError::Eof(_))));

  input.seek(pos)?;

  result = check_read_bytes(&mut input, 100000, pos, &mut buffer);
  // Test massive block read past end of file
  assert!(matches!(result, Err(LuceneError::Eof(_))));

  Ok(())
}

#[test]
fn test_backwards_byte_reads() -> Result<()> {
  let mut random = random();
  let sub_index_input = MyBufferedIndexInput::with_len(1024 * 8);
  let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
  let mut input =
    BufferedIndexInput::with_buffer_size(sub_index_input, &resource_description, BUFFER_SIZE)?;

  let mut i = 2048;
  while i > 0 {
    assert_eq!(byten(i), RandomAccessInput::read_byte(&mut input, i)?);
    let v = random.random_range(1..16) as usize;
    let next = i.saturating_sub(v);
    if next == 0 {
      break;
    }
    i = next;
  }

  assert_eq!(3, input.get_sub_index_input().read_count);

  Ok(())
}

#[test]
fn test_backwards_short_reads() -> Result<()> {
  let mut random = random();
  let sub_index_input = MyBufferedIndexInput::with_len(1024 * 8);
  let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
  let mut input =
    BufferedIndexInput::with_buffer_size(sub_index_input, &resource_description, BUFFER_SIZE)?;

  let mut i = 2048;
  while i > 0 {
    let bb = [byten(i), byten(i + 1)];
    let expected_value = i16::from_le_bytes(bb);
    assert_eq!(
      expected_value,
      RandomAccessInput::read_short(&mut input, i)?
    );
    let v = random.random_range(1..17) as usize;
    let next = i.saturating_sub(v);
    if next == 0 {
      break;
    }
    i = next;
  }

  let actual_read_count = input.get_sub_index_input().read_count;
  assert!(
    actual_read_count == 3 || actual_read_count == 4,
    "Expected 3 or 4, got {}",
    actual_read_count
  );

  Ok(())
}

#[test]
fn test_backwards_int_reads() -> Result<()> {
  let mut random = random();
  let sub_index_input = MyBufferedIndexInput::with_len(1024 * 8);
  let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
  let mut input =
    BufferedIndexInput::with_buffer_size(sub_index_input, &resource_description, BUFFER_SIZE)?;

  let mut i = 2048;
  while i > 0 {
    let mut bb = vec![0u8; 4];
    bb[0] = byten(i);
    bb[1] = byten(i + 1);
    bb[2] = byten(i + 2);
    bb[3] = byten(i + 3);

    let expected_value = i32::from_le_bytes(bb.try_into().unwrap());
    assert_eq!(expected_value, RandomAccessInput::read_int(&mut input, i)?);
    let v = random.random_range(3..19) as usize;
    let next = i.saturating_sub(v);
    if next == 0 {
      break;
    }
    i = next;
  }

  let actual_read_count = input.get_sub_index_input().read_count;
  assert!(
    actual_read_count == 3 || actual_read_count == 4,
    "Expected 3 or 4, got {}",
    actual_read_count
  );

  Ok(())
}

#[test]
fn test_backwards_long_reads() -> Result<()> {
  let mut random = random();
  let sub_index_input = MyBufferedIndexInput::with_len(1024 * 8);
  let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
  let mut input =
    BufferedIndexInput::with_buffer_size(sub_index_input, &resource_description, BUFFER_SIZE)?;

  let mut i = 2048;
  while i > 0 {
    let mut bb = vec![0u8; 8];
    bb[0] = byten(i);
    bb[1] = byten(i + 1);
    bb[2] = byten(i + 2);
    bb[3] = byten(i + 3);
    bb[4] = byten(i + 4);
    bb[5] = byten(i + 5);
    bb[6] = byten(i + 6);
    bb[7] = byten(i + 7);

    let expected_value = i64::from_le_bytes(bb.try_into().unwrap());
    assert_eq!(expected_value, RandomAccessInput::read_long(&mut input, i)?);

    let v = random.random_range(7..23) as usize;
    let next = i.saturating_sub(v);
    if next == 0 {
      break;
    }
    i = next;
  }

  let actual_read_count = input.get_sub_index_input().read_count;
  assert!(
    actual_read_count == 3 || actual_read_count == 4,
    "Expected 3 or 4, got {}",
    actual_read_count
  );

  Ok(())
}
#[test]
fn test_read_floats() -> Result<()> {
  let mut random = random();
  let length: usize = 1024 * 8;
  let buffer_length: usize = random.random_range(128..length / 8);
  let sub_index_input = MyBufferedIndexInput::with_len(length);
  let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
  let mut input = BufferedIndexInput::with_buffer_size(
    sub_index_input.try_clone()?,
    &resource_description,
    BUFFER_SIZE,
  )?;
  let mut float_buffer = vec![0f32; buffer_length];

  for alignment in 0..BitUtil::FLOAT_BYTES {
    input.seek(0)?;
    for _ in 0..alignment {
      DataInput::read_byte(&mut input)?;
    }

    let bulk_reads = length / (buffer_length * BitUtil::FLOAT_BYTES) - 1;
    for i in 0..bulk_reads {
      let pos = alignment + i * buffer_length * BitUtil::FLOAT_BYTES;
      let float_offset: usize = random.random_range(0..3);
      DataInput::skip_bytes(&mut input, (float_offset * BitUtil::FLOAT_BYTES) as i64)?;

      DataInput::read_floats(
        &mut input,
        &mut float_buffer[float_offset..],
        0,
        buffer_length - float_offset,
      )?;

      for (idx, &actual_float) in float_buffer
        .iter()
        .enumerate()
        .skip(float_offset)
        .take(buffer_length - float_offset)
      {
        let offset = pos + idx * BitUtil::FLOAT_BYTES;

        let bb = [
          byten(offset),
          byten(offset + 1),
          byten(offset + 2),
          byten(offset + 3),
        ];

        let expected_bits = f32::from_le_bytes(bb).to_bits();
        let actual_bits = actual_float.to_bits();

        assert_eq!(
          expected_bits, actual_bits,
          "Mismatch at alignment={}, bulk_read={}, idx={}",
          alignment, i, idx
        );
      }
    }
  }

  Ok(())
}
#[test]
fn test_read_ints() -> Result<()> {
  let mut random = random();
  let length: usize = 1024 * 8;
  let buffer_length: usize = random.random_range(128..length / 8);
  let sub_index_input = MyBufferedIndexInput::with_len(length);
  let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
  let mut input = BufferedIndexInput::with_buffer_size(
    sub_index_input.try_clone()?,
    &resource_description,
    BUFFER_SIZE,
  )?;
  let mut int_buffer = vec![0i32; buffer_length];

  for alignment in 0..BitUtil::INT_BYTES {
    input.seek(0)?;
    for _ in 0..alignment {
      DataInput::read_byte(&mut input)?;
    }

    let bulk_reads = length / (buffer_length * BitUtil::INT_BYTES) - 1;
    for i in 0..bulk_reads {
      let pos = alignment + i * buffer_length * BitUtil::INT_BYTES;
      let int_offset: usize = random.random_range(0..3);
      DataInput::skip_bytes(&mut input, (int_offset * BitUtil::INT_BYTES) as i64)?;

      DataInput::read_ints(
        &mut input,
        &mut int_buffer[int_offset..],
        0,
        buffer_length - int_offset,
      )?;

      for (idx, &actual_value) in int_buffer
        .iter()
        .enumerate()
        .skip(int_offset)
        .take(buffer_length - int_offset)
      {
        let offset = pos + idx * BitUtil::INT_BYTES;

        let bb = [
          byten(offset),
          byten(offset + 1),
          byten(offset + 2),
          byten(offset + 3),
        ];

        let expected_value = i32::from_le_bytes(bb);
        assert_eq!(
          expected_value, actual_value,
          "Mismatch at alignment={}, bulk_read={}, idx={}",
          alignment, i, idx
        );
      }
    }
  }

  Ok(())
}
#[test]
fn test_read_longs() -> Result<()> {
  let mut random = random();
  let length: usize = 1024 * 8;
  let buffer_length: usize = random.random_range(128..length / 8);
  let sub_index_input = MyBufferedIndexInput::with_len(length);
  let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
  let mut input = BufferedIndexInput::with_buffer_size(
    sub_index_input.try_clone()?,
    &resource_description,
    BUFFER_SIZE,
  )?;
  let mut bb = vec![0u8; BitUtil::LONG_BYTES];
  let mut long_buffer = vec![0i64; buffer_length];

  for alignment in 0..BitUtil::LONG_BYTES {
    input.seek(0)?;
    for _ in 0..alignment {
      DataInput::read_byte(&mut input)?;
    }

    let bulk_reads = length / (buffer_length * BitUtil::LONG_BYTES) - 1;
    for i in 0..bulk_reads {
      let pos = alignment + i * buffer_length * BitUtil::LONG_BYTES;
      let long_offset: usize = random.random_range(0..3);
      DataInput::skip_bytes(&mut input, (long_offset * BitUtil::LONG_BYTES) as i64)?;

      DataInput::read_longs(
        &mut input,
        &mut long_buffer[long_offset..],
        0,
        buffer_length - long_offset,
      )?;

      for (idx, actual_value) in long_buffer
        .iter()
        .enumerate()
        .take(buffer_length)
        .skip(long_offset)
      {
        let offset = pos + idx * BitUtil::LONG_BYTES;
        bb[0] = byten(offset);
        bb[1] = byten(offset + 1);
        bb[2] = byten(offset + 2);
        bb[3] = byten(offset + 3);
        bb[4] = byten(offset + 4);
        bb[5] = byten(offset + 5);
        bb[6] = byten(offset + 6);
        bb[7] = byten(offset + 7);

        let expected_value = i64::from_le_bytes(bb.clone().try_into().unwrap());
        assert_eq!(
          expected_value, *actual_value,
          "Mismatch at alignment={}, bulk_read={}, idx={}",
          alignment, i, idx
        );
      }
    }
  }

  Ok(())
}
struct MyBufferedIndexInput {
  pos: usize,
  len: usize,
  read_count: usize,
}

impl MyBufferedIndexInput {
  fn with_len(len: usize) -> Self {
    Self {
      pos: 0,
      len,
      read_count: 0,
    }
  }
  fn new() -> Self {
    Self::with_len(i64::MAX as usize)
  }
}

/// Simulates a file where each byte is determined by a mathematical
/// function. This function emulates reading the n'th byte in that file.
///
/// # Arguments
/// * `n` - The position in the file.
///
/// # Returns
/// The byte value at the given position.
fn byten(n: usize) -> u8 {
  ((n * n) % 256) as u8
}

impl crate::core::util::clone::TryClone for MyBufferedIndexInput {
  fn try_clone(&self) -> Result<Self>
  where
    Self: Sized,
  {
    Ok(MyBufferedIndexInput::with_len(self.len))
  }
}

impl crate::core::util::close::CloseableRef for MyBufferedIndexInput {}

impl BufferedIndexInputBase for MyBufferedIndexInput {
  fn seek_internal(&mut self, pos: usize) -> Result<()> {
    self.pos = pos;
    Ok(())
  }

  fn read_internal(
    &mut self,
    b: &mut Cursor<Vec<u8>>,
    len: usize,
    _file_pointer: usize,
  ) -> Result<()> {
    let mut i = 0;
    self.read_count += 1;
    while b.remain()? > 0 && i < len {
      b.write_u8(byten(self.pos))?;
      self.pos += 1;
      i += 1;
    }
    Ok(())
  }

  type Slice = BufferedIndexInput<MyBufferedIndexInput>;

  fn slice(&self, _slice_description: &str, _offset: usize, _length: usize) -> Result<Self::Slice> {
    Err(LuceneError::unsupported_operation(
      "MyBufferedIndexInput method is not supported".to_string(),
    ))
  }

  fn length(&self) -> usize {
    self.len
  }
}
