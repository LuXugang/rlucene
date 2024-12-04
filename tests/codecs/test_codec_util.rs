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
use crate::util::test_error::TestError;
use rlucene::codecs::codec_util::{check_header, check_sum_entire_file, header_length, write_footer, write_header};
use rlucene::store::data_output::DataOutput;
use rlucene::store::index_input::IndexInput;
use rlucene::store::{
    ByteBuffersDataOutput, ByteBuffersIndexInput, ByteBuffersIndexOutput, DataInput,
};
use rlucene::util::error::data_io_error_enum::DataIOError;

#[allow(dead_code)] // for quick search
struct TestCodecUtil;

#[test]
fn test_header_length() -> Result<(), TestError> {
    let mut out = ByteBuffersDataOutput::new_resettable_instance()?;
    {
        let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);
        write_header(&mut output, "FooBar", 5)?;
        output.write_string("this is the data")?;
    }

    let mut input = ByteBuffersIndexInput::new(out.get_data_input(), "temp");
    input.seek(header_length("FooBar") as u64)?;
    assert_eq!(input.read_string()?, "this is the data");
    Ok(())
}

#[test]
fn test_write_too_long_header() -> Result<(), TestError>{
    let too_long: String = "a".repeat(128);

    let mut output = ByteBuffersDataOutput::new_resettable_instance()?;
    let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut output);

    let result = write_header(&mut output, &too_long, 5);
    matches!(result, Err(DataIOError::IllegalArgument(_)));
    Ok(())
}

#[test]
fn test_write_non_ascii_header() -> Result<(), TestError> {
    let non_ascii_header = "\u{1234}".to_string();

    let mut out = ByteBuffersDataOutput::new_resettable_instance()?;
    let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);

    let result = write_header(&mut output, &non_ascii_header, 5);
    matches!(result, Err(DataIOError::IllegalArgument(_)));
    Ok(())
}

#[test]
fn test_read_header_wrong_magic() -> Result<(), TestError> {
    let mut output = ByteBuffersDataOutput::new_resettable_instance()?;
    {
        let mut index_output = ByteBuffersIndexOutput::new("temp", "temp", &mut output);
        index_output.write_int(1234)?;
    }

    // 创建输入对象
    let input_data = output.get_data_input();
    let mut input = ByteBuffersIndexInput::new(input_data,"temp");

    let result = check_header(&mut input, "bogus", 1, 1);
    assert!( matches!(result, Err(DataIOError::CorruptIndex(_))));
    Ok(())
}

#[test]
fn test_checksum_entire_file() -> Result<(), TestError>{
    let mut output = ByteBuffersDataOutput::new_resettable_instance()?;
    {
        let mut index_output = ByteBuffersIndexOutput::new("temp", "temp", &mut output);
        write_header(&mut index_output, "FooBar", 5)?;
        index_output.write_string("this is the data")?;
        write_footer(&mut index_output)?;
    }

    let mut input_data = ByteBuffersIndexInput::new(output.get_data_input(), "temp");
    check_sum_entire_file(&mut input_data)?;
    Ok(())
}










