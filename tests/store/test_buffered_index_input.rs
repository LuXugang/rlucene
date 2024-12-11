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
use crate::common::my_random;
use crate::util::test_error::TestError;
use byteorder::WriteBytesExt;
use rand::Rng;
use rlucene::store::index_input::IndexInput;
use rlucene::store::random_access_input::RandomAccessInput;
use rlucene::store::{BufferedIndexInput, BufferedIndexInputBase, DataInput, BUFFER_SIZE};
use rlucene::util::bit_util::{FLOAT_BYTES, INT_BYTES, LONG_BYTES};
use rlucene::util::error::data_io_error_enum::DataIOError;
use rlucene::util::ReadableCursorExt;
use std::io::Cursor;

#[allow(dead_code)] // for quick search
struct TestBufferedIndexInput;

const TEST_FILE_LENGTH: u64 = 1000;

#[test]
// Call readByte() repeatedly, past the buffer boundary, and see that it
// is working as expected.
// Our input comes from a dynamically generated/ "file" - see
// MyBufferedIndexInput below.
fn test_read_byte() -> Result<(), TestError> {
    let sub_index_input = MyBufferedIndexInput::new();
    let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
    let mut input = BufferedIndexInput::new_with_buffer_size(
        sub_index_input,
        &resource_description,
        BUFFER_SIZE,
    );
    for i in 0..BUFFER_SIZE * 10 {
        assert_eq!(byten(i as u64), DataInput::read_byte(&mut input)?);
    }

    Ok(())
}

#[test]
fn test_read_bytes() -> Result<(), TestError> {
    let mut random = my_random("test_read_bytes".to_string());
    let sub_index_input = MyBufferedIndexInput::new();
    let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
    let mut input = BufferedIndexInput::new_with_buffer_size(
        sub_index_input,
        &resource_description,
        BUFFER_SIZE,
    );

    let mut pos = 0;

    // Gradually increasing size
    let mut size = 1;
    while size < BUFFER_SIZE * 10 {
        let mut buffer: Vec<u8> = vec![0; 10];
        check_read_bytes(&mut input, size as usize, pos as u64, &mut buffer)?;
        pos += size;
        if pos as u64 >= TEST_FILE_LENGTH {
            // Wrap around
            pos = 0;
            input.seek(0)?;
        }
        size += size / 200 + 1;
    }

    // Wildly fluctuating size
    for _ in 0..100 {
        let size = random.gen_range(1..=10000);
        let mut buffer: Vec<u8> = vec![0; 10];
        check_read_bytes(&mut input, size, pos as u64, &mut buffer)?;
        pos += size as u32;
        if pos as u64 >= TEST_FILE_LENGTH {
            // Wrap around
            pos = 0;
            input.seek(0)?;
        }
    }

    // Constant small size (7 bytes)
    for _ in 0..BUFFER_SIZE {
        let mut buffer: Vec<u8> = vec![0; 10];
        check_read_bytes(&mut input, 7, pos as u64, &mut buffer)?;
        pos += 7;
        if pos as u64 >= TEST_FILE_LENGTH {
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
    pos: u64,
    buffer: &mut Vec<u8>,
) -> Result<(), TestError> {
    // Just to see that "offset" is treated properly in read_bytes(), we
    // add an arbitrary offset at the beginning of the array
    let offset = size % 10; // arbitrary offset
    if buffer.len() < offset + size {
        buffer.resize(offset + size, 0); // Grow the buffer as needed
    }

    assert_eq!(
        pos,
        input.get_file_pointer(),
        "File pointer does not match expected position"
    );

    let left = TEST_FILE_LENGTH - input.get_file_pointer();
    if left == 0 {
        return Ok(()); // No data left to read
    }

    let size_to_read = if left < size as u64 {
        left as usize // Adjust size to remaining bytes
    } else {
        size
    };

    input.read_bytes(
        &mut buffer[offset..offset + size_to_read],
        0,
        size_to_read as u32,
    )?;

    assert_eq!(
        pos + size_to_read as u64,
        input.get_file_pointer(),
        "File pointer does not match after reading"
    );

    for i in 0..size_to_read {
        let file_pos = pos + i as u64;
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
fn test_eof() -> Result<(), TestError> {
    let random = my_random("test_read_bytes".to_string());
    let sub_index_input = MyBufferedIndexInput::new_with_len(1024);
    let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
    let mut input = BufferedIndexInput::new_with_buffer_size(
        sub_index_input,
        &resource_description,
        BUFFER_SIZE,
    );
    let mut buffer = vec![];

    // Verify we can read all bytes in one go
    let mut length = IndexInput::length(&input) as usize;
    check_read_bytes(&mut input, length, 0, &mut buffer)?;

    // Attempt to read more than the available bytes for small and large overflows
    length = IndexInput::length(&input) as usize;
    let pos = length - 10;
    input.seek(pos as u64)?;

    // Small overflow: read exactly remaining bytes
    check_read_bytes(&mut input, 10, pos as u64, &mut buffer)?;

    input.seek(pos as u64)?;

    // Test block read past end of file
    let mut result = check_read_bytes(&mut input, 11, pos as u64, &mut buffer);
    assert!(matches!(result, Err(TestError::DataIOError(_))));

    input.seek(pos as u64)?;

    result = check_read_bytes(&mut input, 50, pos as u64, &mut buffer);
    // Test large block read past end of file
    assert!(matches!(result, Err(TestError::DataIOError(_))));

    input.seek(pos as u64)?;

    result = check_read_bytes(&mut input, 100000, pos as u64, &mut buffer);
    // Test massive block read past end of file
    assert!(matches!(result, Err(TestError::DataIOError(_))));

    Ok(())
}

#[test]
fn test_backwards_byte_reads() -> Result<(), TestError> {
    let mut random = my_random("test_backwards_byte_reads".to_string());
    let sub_index_input = MyBufferedIndexInput::new_with_len(1024 * 8);
    let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
    let mut input = BufferedIndexInput::new_with_buffer_size(
        sub_index_input,
        &resource_description,
        BUFFER_SIZE,
    );

    let mut read_count = 0;

    let mut i: i64 = 2048;
    while i > 0 {
        assert_eq!(
            byten(i as u64),
            RandomAccessInput::read_byte(&mut input, i as u64)?
        );
        read_count += 1;
        i -= random.gen_range(1..16);
    }

    assert_eq!(3, input.get_sub_index_input().read_count);

    Ok(())
}

#[test]
fn test_backwards_int_reads() -> Result<(), TestError> {
    let mut random = my_random("test_backwards_int_reads".to_string());
    let sub_index_input = MyBufferedIndexInput::new_with_len(1024 * 8);
    let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
    let mut input = BufferedIndexInput::new_with_buffer_size(
        sub_index_input,
        &resource_description,
        BUFFER_SIZE,
    );

    let mut read_count = 0;

    let mut i = 2048;
    while i > 0 {
        let mut bb = vec![0u8; 4];
        bb[0] = byten(i as u64);
        bb[1] = byten(i as u64 + 1);
        bb[2] = byten(i as u64 + 2);
        bb[3] = byten(i as u64 + 3);

        let expected_value = i32::from_le_bytes(bb.try_into().unwrap());
        assert_eq!(
            expected_value,
            RandomAccessInput::read_int(&mut input, i as u64)?
        );

        read_count += 1;
        i -= random.gen_range(3..19);
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
fn test_backwards_long_reads() -> Result<(), TestError> {
    let mut random = my_random("test_backwards_long_reads".to_string());
    let sub_index_input = MyBufferedIndexInput::new_with_len(1024 * 8);
    let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
    let mut input = BufferedIndexInput::new_with_buffer_size(
        sub_index_input,
        &resource_description,
        BUFFER_SIZE,
    );

    let mut i = 2048;
    while i > 0 {
        let mut bb = vec![0u8; 8];
        bb[0] = byten(i as u64);
        bb[1] = byten(i as u64 + 1);
        bb[2] = byten(i as u64 + 2);
        bb[3] = byten(i as u64 + 3);
        bb[4] = byten(i as u64 + 4);
        bb[5] = byten(i as u64 + 5);
        bb[6] = byten(i as u64 + 6);
        bb[7] = byten(i as u64 + 7);

        let expected_value = i64::from_le_bytes(bb.try_into().unwrap());
        assert_eq!(
            expected_value,
            RandomAccessInput::read_long(&mut input, i as u64)?
        );

        i -= random.gen_range(7..23);
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
fn test_read_floats() -> Result<(), TestError> {
    let length: usize = 1024 * 8;
    let buffer_length: usize = 128;
    let mut random = my_random("test_read_floats".to_string());
    let sub_index_input = MyBufferedIndexInput::new_with_len(length as u64);
    let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
    let mut input = BufferedIndexInput::new_with_buffer_size(
        sub_index_input.clone(),
        &resource_description,
        BUFFER_SIZE,
    );
    let mut bb = vec![0u8; FLOAT_BYTES];
    let mut float_buffer = vec![0f32; buffer_length];

    for alignment in 0..FLOAT_BYTES {
        input.seek(0)?;
        for _ in 0..alignment {
            DataInput::read_byte(&mut input)?;
        }

        let bulk_reads = length / (buffer_length * FLOAT_BYTES) - 1;
        for i in 0..bulk_reads {
            let pos = alignment + i * buffer_length * FLOAT_BYTES;
            let float_offset: usize = random.gen_range(0..3);
            DataInput::skip_bytes(&mut input, (float_offset * FLOAT_BYTES) as u64)?;

            input.read_floats(
                &mut float_buffer[float_offset..],
                0,
                (buffer_length - float_offset) as u32,
            )?;

            for idx in float_offset as usize..buffer_length {
                let offset = pos + idx * FLOAT_BYTES;
                bb[0] = byten(offset as u64);
                bb[1] = byten(offset as u64 + 1);
                bb[2] = byten(offset as u64 + 2);
                bb[3] = byten(offset as u64 + 3);

                let bb_clone = bb.clone();
                let expected_bits = f32::from_le_bytes(bb_clone.try_into().unwrap()).to_bits();
                let actual_bits = float_buffer[idx].to_bits();
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
fn test_read_ints() -> Result<(), TestError> {
    let length: usize = 1024 * 8;
    let buffer_length: usize = 128;
    let mut random = my_random("test_read_ints".to_string());
    let sub_index_input = MyBufferedIndexInput::new_with_len(length as u64);
    let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
    let mut input = BufferedIndexInput::new_with_buffer_size(
        sub_index_input.clone(),
        &resource_description,
        BUFFER_SIZE,
    );
    let mut bb = vec![0u8; INT_BYTES];
    let mut int_buffer = vec![0i32; buffer_length];

    for alignment in 0..INT_BYTES {
        input.seek(0)?;
        for _ in 0..alignment {
            DataInput::read_byte(&mut input)?;
        }

        let bulk_reads = length / (buffer_length * INT_BYTES) - 1;
        for i in 0..bulk_reads {
            let pos = alignment + i * buffer_length * INT_BYTES;
            let int_offset: usize = random.gen_range(0..3);
            DataInput::skip_bytes(&mut input, (int_offset * INT_BYTES) as u64)?;

            input.read_ints(
                &mut int_buffer[int_offset..],
                0,
                (buffer_length - int_offset) as u32,
            )?;

            for idx in int_offset..buffer_length {
                let offset = pos + idx * INT_BYTES;
                bb[0] = byten(offset as u64);
                bb[1] = byten(offset as u64 + 1);
                bb[2] = byten(offset as u64 + 2);
                bb[3] = byten(offset as u64 + 3);

                let bb_clone = bb.clone();
                let expected_value = i32::from_le_bytes(bb_clone.try_into().unwrap());
                let actual_value = int_buffer[idx];
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
fn test_read_longs() -> Result<(), TestError> {
    let length: usize = 1024 * 8;
    let buffer_length: usize = 128;
    let mut random = my_random("test_read_longs".to_string());
    let sub_index_input = MyBufferedIndexInput::new_with_len(length as u64);
    let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
    let mut input = BufferedIndexInput::new_with_buffer_size(
        sub_index_input.clone(),
        &resource_description,
        BUFFER_SIZE,
    );
    let mut bb = vec![0u8; LONG_BYTES];
    let mut long_buffer = vec![0i64; buffer_length];

    for alignment in 0..LONG_BYTES {
        input.seek(0)?;
        for _ in 0..alignment {
            DataInput::read_byte(&mut input)?;
        }

        let bulk_reads = length / (buffer_length * LONG_BYTES) - 1;
        for i in 0..bulk_reads {
            let pos = alignment + i * buffer_length * LONG_BYTES;
            let long_offset: usize = random.gen_range(0..3);
            DataInput::skip_bytes(&mut input, (long_offset * LONG_BYTES) as u64)?;

            input.read_longs(
                &mut long_buffer[long_offset..],
                0,
                (buffer_length - long_offset) as u32,
            )?;

            for idx in long_offset..buffer_length {
                let offset = pos + idx * LONG_BYTES;
                bb[0] = byten(offset as u64);
                bb[1] = byten(offset as u64 + 1);
                bb[2] = byten(offset as u64 + 2);
                bb[3] = byten(offset as u64 + 3);
                bb[4] = byten(offset as u64 + 4);
                bb[5] = byten(offset as u64 + 5);
                bb[6] = byten(offset as u64 + 6);
                bb[7] = byten(offset as u64 + 7);

                let bb_clone = bb.clone();
                let expected_value = i64::from_le_bytes(bb_clone.try_into().unwrap());
                let actual_value = long_buffer[idx];
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
struct MyBufferedIndexInput {
    pos: u64,
    len: u64,
    read_count: u64,
}

impl MyBufferedIndexInput {
    fn new_with_len(len: u64) -> Self {
        Self {
            pos: 0,
            len,
            read_count: 0,
        }
    }
    fn new() -> Self {
        Self::new_with_len(u64::MAX)
    }
}

/// Simulates a file where each byte is determined by a mathematical function.
/// This function emulates reading the n'th byte in that file.
///
/// # Arguments
/// * `n` - The position in the file.
///
/// # Returns
/// The byte value at the given position.
fn byten(n: u64) -> u8 {
    ((n * n) % 256) as u8
}

impl Clone for MyBufferedIndexInput {
    fn clone(&self) -> Self {
        MyBufferedIndexInput::new_with_len(self.len)
    }
}

impl BufferedIndexInputBase for MyBufferedIndexInput {
    fn seek_internal(&mut self, pos: u64) -> Result<(), DataIOError> {
        self.pos = pos;
        Ok(())
    }

    fn read_internal(
        &mut self,
        b: &mut Cursor<Vec<u8>>,
        len: u64,
        _file_pointer: u64,
    ) -> Result<(), DataIOError> {
        let mut i = 0;
        self.read_count += 1;
        while b.remain() > 0 && i < len {
            b.write_u8(byten(self.pos))?;
            self.pos += 1;
            i += 1;
        }
        Ok(())
    }

    fn slice(
        &self,
        _slice_description: &str,
        _offset: u64,
        _length: u64,
    ) -> Result<MyBufferedIndexInput, DataIOError> {
        unreachable!("MyBufferedIndexInput does not support slicing")
    }

    fn length(&self) -> u64 {
        self.len
    }
}
