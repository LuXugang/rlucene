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
use byteorder::{LittleEndian, ReadBytesExt};
use rlucene::store::data_output::DataOutput;
use rlucene::store::index_output::IndexOutput;
use rlucene::store::output_stream_index_output::OutputStreamIndexOutput;
use rlucene::util::error::data_io_error_enum::DataIOError;
use std::io::Cursor;

#[allow(dead_code)]
struct TestOutputStreamIndexOutput;

#[test]
fn test_data_types() -> Result<(), DataIOError> {
    for offset in 0..12 {
        do_test_data_types(offset)?;
    }
    Ok(())
}

fn do_test_data_types(offset: usize) -> Result<(), DataIOError> {
    use crc32fast::Hasher;

    let mut buffer = Vec::new();
    {
        let mut out = OutputStreamIndexOutput::new("test", "test", &mut buffer, 12);
        let mut hasher = Hasher::new();
        for i in 0..offset {
            out.write_byte(i as u8)?;
            hasher.update(&[i as u8]);
        }
        out.write_short(12345)?;
        hasher.update(&12345u16.to_le_bytes());

        out.write_int(1234567890)?;
        hasher.update(&1234567890u32.to_le_bytes());

        out.write_long(1234567890123456789)?;
        hasher.update(&1234567890123456789u64.to_le_bytes());
        assert_eq!(out.get_file_pointer(), (offset + 14) as i64);
        assert_eq!(
            out.get_check_sum() as u32,
            hasher.finalize(),
            "Checksum mismatch"
        );
    }

    let mut reader = Cursor::new(buffer);
    for i in 0..offset {
        assert_eq!(reader.read_u8()?, i as u8);
    }

    assert_eq!(reader.read_i16::<LittleEndian>()?, 12345);
    assert_eq!(reader.read_i32::<LittleEndian>()?, 1234567890);
    assert_eq!(reader.read_i64::<LittleEndian>()?, 1234567890123456789);
    assert_eq!(reader.position() as usize, reader.get_ref().len());

    Ok(())
}

#[test]
fn test_write_exceeding_buffer() -> Result<(), DataIOError> {
    use crc32fast::Hasher;

    let buffer_size = 8;
    let large_data: Vec<u8> = (0..16).collect();
    let mut buffer = Vec::new();
    {
        let mut out = OutputStreamIndexOutput::new("test", "test", &mut buffer, buffer_size);

        let mut hasher = Hasher::new();

        out.write_bytes_range(&large_data, 0, large_data.len())?;
        hasher.update(&large_data);

        assert_eq!(out.get_file_pointer(), large_data.len() as i64);
        assert_eq!(
            out.get_check_sum() as u32,
            hasher.finalize(),
            "Checksum mismatch"
        );
    }

    assert_eq!(buffer, large_data);

    Ok(())
}
#[test]
fn test_multiple_writes_with_checksum() -> Result<(), DataIOError> {
    use crc32fast::Hasher;

    let mut buffer = Vec::new();
    let combined_data: Vec<u8>;
    {
        let mut out = OutputStreamIndexOutput::new("test", "test", &mut buffer, 8);

        let data1 = b"Hello";
        let data2 = b"World";
        let mut hasher = Hasher::new();

        out.write_bytes_range(data1, 0, data1.len())?;
        hasher.update(data1);
        let sum1 = out.get_check_sum();
        out.write_bytes_range(data2, 0, data2.len())?;
        hasher.update(data2);
        let sum2 = out.get_check_sum();
        assert_ne!(sum1, sum2, "Checksum mismatch");

        assert_eq!(
            out.get_check_sum() as u32,
            hasher.finalize(),
            "Checksum mismatch"
        );
        combined_data = [data1.as_slice(), data2.as_slice()].concat();
    }

    assert_eq!(buffer, combined_data);

    Ok(())
}

trait MyTrait {
    fn method_a(&self) {
        println!("Default implementation of method_a");
    }
}

struct MyStruct;

impl MyTrait for MyStruct {
    fn method_a(&self) {}
}

#[test]
fn main() {
    let instance = MyStruct;

    println!("Calling method_a:");
    instance.method_a();
}
