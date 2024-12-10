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
use std::io::Cursor;
use byteorder::WriteBytesExt;
use rand::Rng;
use rlucene::store::{BufferedIndexInput, BufferedIndexInputBase, DataInput, BUFFER_SIZE};
use rlucene::store::index_input::IndexInput;
use rlucene::store::random_access_input::RandomAccessInput;
use rlucene::util::error::data_io_error_enum::DataIOError;
use rlucene::util::ReadableCursorExt;
use crate::common::my_random;
use crate::util::test_error::TestError;

#[allow(dead_code)] // for quick search
struct TestBufferedIndexInput;

const TEST_FILE_LENGTH: u64 = 1000;

#[test]
// Call readByte() repeatedly, past the buffer boundary, and see that it
// is working as expected.
// Our input comes from a dynamically generated/ "file" - see
// MyBufferedIndexInput below.
fn test_read_byte() -> Result<(), TestError>{
    let sub_index_input= MyBufferedIndexInput::new();
    let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
    let mut input = BufferedIndexInput::new_with_buffer_size(sub_index_input, &resource_description, BUFFER_SIZE);
    for i in 0..BUFFER_SIZE* 10 {
        assert_eq!(byten(i as u64), DataInput::read_byte(&mut input)?);
    }
    
   Ok(())
}

#[test]
fn test_read_bytes() -> Result<(), TestError>{
    let mut random = my_random("test_read_bytes".to_string());
    let sub_index_input= MyBufferedIndexInput::new();
    let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
    let mut input = BufferedIndexInput::new_with_buffer_size(sub_index_input, &resource_description, BUFFER_SIZE);

    let mut pos = 0;

    // Gradually increasing size
    let mut size = 1;
    while size < BUFFER_SIZE* 10 {
        let mut buffer:Vec<u8> = vec![0; 10];
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
        let mut buffer:Vec<u8> = vec![0; 10];
        check_read_bytes(&mut input, size, pos as u64, &mut buffer)?;
        pos += size as u32;
        if pos as u64 >= TEST_FILE_LENGTH {
            // Wrap around
            pos = 0;
            input.seek(0)?;
        }
    }

    // Constant small size (7 bytes)
    for _ in 0..BUFFER_SIZE{
        let mut buffer:Vec<u8> = vec![0; 10];
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

    assert_eq!(pos, input.get_file_pointer(), "File pointer does not match expected position");

    let left = TEST_FILE_LENGTH - input.get_file_pointer();
    if left == 0 {
        return Ok(()); // No data left to read
    }

    let size_to_read = if left < size as u64 {
        left as usize // Adjust size to remaining bytes
    } else {
        size
    };

    input.read_bytes(&mut buffer[offset..offset + size_to_read], 0, size_to_read as u32)?;

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
    let mut random = my_random("test_read_bytes".to_string());
    let sub_index_input= MyBufferedIndexInput::new_with_len(1024);
    let resource_description = format!("MyBufferedIndexInput(len= {})", sub_index_input.len);
    let mut input = BufferedIndexInput::new_with_buffer_size(sub_index_input, &resource_description, BUFFER_SIZE);
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
    assert!(matches!(
        result,
        Err(TestError::DataIOError(_))
    ));

    input.seek(pos as u64)?;

    // Test large block read past end of file
    assert!(matches!(
        check_read_bytes(&mut input, 50, pos as u64, &mut buffer),
        Err(TestError::Eof(_))
    ));

    input.seek(pos as u64)?;

    // Test massive block read past end of file
    assert!(matches!(
        check_read_bytes(&mut input, 100000, pos as u64, &mut buffer),
        Err(TestError::Eof(_))
    ));

    Ok(())
}

struct MyBufferedIndexInput{
    pos: u64,
    len: u64,
    read_count: u64
}

impl MyBufferedIndexInput {
    fn new_with_len(len: u64) -> Self {
        Self {
            pos: 0,
            len,
            read_count: 0
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

impl BufferedIndexInputBase for MyBufferedIndexInput {
    fn seek_internal(&mut self, pos: u64) -> Result<(), DataIOError> {
        Ok(self.pos = pos)
    }

    fn read_internal(&mut self, b: &mut Cursor<Vec<u8>>) -> Result<(), DataIOError> {
        self.read_count +=1;
        while b.remain() > 0 {
           b.write_u8(byten(self.pos))?;
            self.pos += 1;
        }
        Ok(())
    }
}

impl DataInput for MyBufferedIndexInput {
    fn read_byte(&mut self) -> Result<u8, DataIOError> {
        Ok(0)
    }

    fn read_bytes(&mut self, _b: &mut [u8], _offset: u32, _len: u32) -> Result<(), DataIOError> {
        Ok(())
    }

    fn skip_bytes(&mut self, _num_bytes: u64) -> Result<(), DataIOError> {
        Ok(())
    }
}

impl Display for MyBufferedIndexInput {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Clone for MyBufferedIndexInput {
    fn clone(&self) -> Self {
        MyBufferedIndexInput::new_with_len(self.len)
    }
}

impl IndexInput for MyBufferedIndexInput {
    fn get_file_pointer(&self) -> u64 {
        0
    }

    fn seek(&mut self, _pos: u64) -> Result<(), DataIOError> {
        Ok(())
    }

    fn length(&self) -> u64 {
        self.len
    }

    fn slice(&self, _slice_description: &str, _offset: u64, _length: u64) -> Result<MyBufferedIndexInput, DataIOError> {
        unreachable!("MyBufferedIndexInput does not support slicing")
    }

    fn is_random_access(&self) -> bool {
        false
    }

    fn get_random_access_slice(&self, _offset: u64, _length: u64) -> Result<MyBufferedIndexInput, DataIOError> {
        Ok(MyBufferedIndexInput::new())
    }
}
impl RandomAccessInput for MyBufferedIndexInput {
    fn length(&self) -> u64 {
        0
    }

    fn read_byte(&mut self, _pos: u64) -> Result<u8, DataIOError> {
        Ok(0)
    }

    fn read_short(&mut self, _pos: u64) -> Result<i16, DataIOError> {
        Ok(0)
    }

    fn read_int(&mut self, _pos: u64) -> Result<i32, DataIOError> {
        Ok(0)
    }

    fn read_long(&mut self, _pos: u64) -> Result<i64, DataIOError> {
        Ok(0)
    }

    fn pre_fetch(&mut self, _pos: u64, _len: u64) -> Result<(), DataIOError> {
        Ok(())
    }
}