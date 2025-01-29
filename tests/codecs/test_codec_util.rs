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
use rlucene::codecs::CodecUtil;
use rlucene::store::buffered_checksum_index_input::BufferedChecksumIndexInput;
use rlucene::store::data_output::DataOutput;
use rlucene::store::index_input::IndexInput;
use rlucene::store::{
    ByteBuffersDataOutput, ByteBuffersIndexInput, ByteBuffersIndexOutput, DataInput, IndexOutput,
};
use rlucene::util::error::lucene_error::LuceneError;
use rlucene::util::StringHelper;
use std::fmt::{Display, Formatter};
use std::sync::atomic::AtomicI64;

#[allow(dead_code)] // for quick search
struct TestCodecUtil;

#[test]
fn test_header_length() -> Result<(), TestError> {
    let mut out = ByteBuffersDataOutput::with_resettable_instance()?;
    {
        let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);
        CodecUtil::write_header(&mut output, "FooBar", 5)?;
        output.write_string("this is the data")?;
    }

    let mut input = ByteBuffersIndexInput::new(out.get_data_input(), "temp");
    input.seek(CodecUtil::header_length("FooBar") as i64)?;
    assert_eq!(input.read_string()?, "this is the data");
    Ok(())
}

#[test]
fn test_write_too_long_header() -> Result<(), TestError> {
    let too_long: String = "a".repeat(128);

    let mut output = ByteBuffersDataOutput::with_resettable_instance()?;
    let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut output);

    let result = CodecUtil::write_header(&mut output, &too_long, 5);
    matches!(result, Err(LuceneError::IllegalArgument(_)));
    Ok(())
}

#[test]
fn test_write_non_ascii_header() -> Result<(), TestError> {
    let non_ascii_header = "\u{1234}".to_string();

    let mut out = ByteBuffersDataOutput::with_resettable_instance()?;
    let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);

    let result = CodecUtil::write_header(&mut output, &non_ascii_header, 5);
    matches!(result, Err(LuceneError::IllegalArgument(_)));
    Ok(())
}

#[test]
fn test_read_header_wrong_magic() -> Result<(), TestError> {
    let mut output = ByteBuffersDataOutput::with_resettable_instance()?;
    {
        let mut index_output = ByteBuffersIndexOutput::new("temp", "temp", &mut output);
        index_output.write_int(1234)?;
    }

    // 创建输入对象
    let input_data = output.get_data_input();
    let mut input = ByteBuffersIndexInput::new(input_data, "temp");

    let result = CodecUtil::check_header(&mut input, "bogus", 1, 1);
    assert!(matches!(result, Err(LuceneError::CorruptIndex(_))));
    Ok(())
}

#[test]
fn test_checksum_entire_file() -> Result<(), TestError> {
    let mut output = ByteBuffersDataOutput::with_resettable_instance()?;
    {
        let mut index_output = ByteBuffersIndexOutput::new("temp", "temp", &mut output);
        CodecUtil::write_header(&mut index_output, "FooBar", 5)?;
        index_output.write_string("this is the data")?;
        CodecUtil::write_footer(&mut index_output)?;
    }

    let mut input_data = ByteBuffersIndexInput::new(output.get_data_input(), "temp");
    CodecUtil::checksum_entire_file(&mut input_data)?;
    Ok(())
}
#[test]
// TODO:This test does not reflect the nested error; it needs to be improved.
fn test_check_footer_valid() -> Result<(), TestError> {
    let mut out = ByteBuffersDataOutput::with_resettable_instance()?;
    {
        let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);
        CodecUtil::write_header(&mut output, "FooBar", 5)?;
        output.write_string("this is the data")?;
        CodecUtil::write_footer(&mut output)?;
    }

    let mut input =
        BufferedChecksumIndexInput::new(ByteBuffersIndexInput::new(out.get_data_input(), "temp"));
    let mut mine = LuceneError::illegal_argument("fake exception");
    let result = CodecUtil::check_footer_with_error(&mut input, &mut mine);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("checksum passed"));
    Ok(())
}

#[test]
// TODO:This test does not reflect the nested error; it needs to be improved.
fn test_check_footer_valid_at_footer() -> Result<(), TestError> {
    let mut out = ByteBuffersDataOutput::with_resettable_instance()?;
    {
        let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);
        CodecUtil::write_header(&mut output, "FooBar", 5)?;
        output.write_string("this is the data")?;
        CodecUtil::write_footer(&mut output)?;
    }

    let mut input =
        BufferedChecksumIndexInput::new(ByteBuffersIndexInput::new(out.get_data_input(), "temp"));
    CodecUtil::check_header(&mut input, "FooBar", 5, 5)?;
    let read_data = input.read_string()?;
    assert_eq!(read_data, "this is the data");
    let mut mine = LuceneError::illegal_argument("fake exception");
    let result = CodecUtil::check_footer_with_error(&mut input, &mut mine);
    assert!(result.is_err());
    let err_message = result.unwrap_err().to_string();
    assert!(err_message.contains("fake exception"));
    assert!(err_message.contains("checksum passed"));
    Ok(())
}
#[test]
// TODO: This test does not fully reflect the nested error; it needs to be improved.
fn test_check_footer_valid_past_footer() -> Result<(), TestError> {
    let mut out = ByteBuffersDataOutput::with_resettable_instance()?;
    {
        let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);
        CodecUtil::write_header(&mut output, "FooBar", 5)?;
        output.write_string("this is the data")?;
        CodecUtil::write_footer(&mut output)?;
    }

    let mut input =
        BufferedChecksumIndexInput::new(ByteBuffersIndexInput::new(out.get_data_input(), "temp"));

    CodecUtil::check_header(&mut input, "FooBar", 5, 5)?;
    let read_data = input.read_string()?;
    assert_eq!(read_data, "this is the data");

    // Bogusly read a byte too far
    input.read_byte()?;

    let mut mine = LuceneError::illegal_argument("fake exception");
    let result = CodecUtil::check_footer_with_error(&mut input, &mut mine);

    assert!(result.is_err());
    let err_message = result.unwrap_err().to_string();
    assert!(err_message.contains("checksum status indeterminate"));
    assert!(err_message.contains("fake exception"));

    Ok(())
}
#[test]
// TODO: This test does not fully reflect the nested error; it needs to be improved.
fn test_check_footer_invalid() -> Result<(), TestError> {
    let mut out = ByteBuffersDataOutput::with_resettable_instance()?;
    {
        let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);
        CodecUtil::write_header(&mut output, "FooBar", 5)?;
        output.write_string("this is the data")?;
        CodecUtil::write_be_int(&mut output, CodecUtil::FOOTER_MAGIC)?;
        CodecUtil::write_be_int(&mut output, 0)?;
        CodecUtil::write_be_long(&mut output, 1234567)?; // write a bogus checksum
    }
    let mut input =
        BufferedChecksumIndexInput::new(ByteBuffersIndexInput::new(out.get_data_input(), "temp"));
    CodecUtil::check_header(&mut input, "FooBar", 5, 5)?;
    let read_data = input.read_string()?;
    assert_eq!(read_data, "this is the data");
    let mut mine = LuceneError::illegal_argument("fake exception");
    let result = CodecUtil::check_footer_with_error(&mut input, &mut mine);
    assert!(result.is_err());
    let err_message = result.unwrap_err().to_string();
    assert!(err_message.contains("checksum failed"));
    assert!(err_message.contains("fake exception"));
    Ok(())
}
#[test]
fn test_segment_header_length() -> Result<(), TestError> {
    let mut out = ByteBuffersDataOutput::with_resettable_instance()?;
    {
        let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);
        let id = StringHelper::random_id();
        CodecUtil::write_index_header(&mut output, "FooBar", 5, &id, "xyz")?;
        output.write_string("this is the data")?;
    }
    let mut input = ByteBuffersIndexInput::new(out.get_data_input(), "temp");

    input.seek(CodecUtil::index_header_length("FooBar", "xyz") as i64)?;

    let read_data = input.read_string()?;
    assert_eq!(read_data, "this is the data");

    Ok(())
}
#[test]
fn test_write_too_long_suffix() {
    let too_long: String = "a".repeat(256);
    let mut out = ByteBuffersDataOutput::with_resettable_instance().unwrap();
    let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);

    let result = CodecUtil::write_index_header(
        &mut output,
        "foobar",
        5,
        &StringHelper::random_id(),
        &too_long,
    );
    assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
}
#[test]
fn test_write_very_long_suffix() -> Result<(), TestError> {
    let just_long_enough: String = "a".repeat(255);

    let mut out = ByteBuffersDataOutput::with_resettable_instance()?;
    let id = StringHelper::random_id();
    {
        let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);
        CodecUtil::write_index_header(&mut output, "foobar", 5, &id, &just_long_enough)?;
    }

    let mut input = ByteBuffersIndexInput::new(out.get_data_input(), "temp");
    CodecUtil::check_index_header(&mut input, "foobar", 5, 5, &id, &just_long_enough)?;

    assert_eq!(input.get_file_pointer(), input.length());
    assert_eq!(
        input.get_file_pointer(),
        CodecUtil::index_header_length("foobar", &just_long_enough) as i64
    );

    Ok(())
}
#[test]
fn test_write_non_ascii_suffix() {
    let mut out = ByteBuffersDataOutput::with_resettable_instance().unwrap();
    let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);

    let non_ascii_suffix = "\u{1234}";

    let result = CodecUtil::write_index_header(
        &mut output,
        "foobar",
        5,
        &StringHelper::random_id(),
        non_ascii_suffix,
    );
    assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
}
#[test]
fn test_read_bogus_crc() -> Result<(), TestError> {
    let mut out = ByteBuffersDataOutput::with_resettable_instance()?;
    {
        let mut output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);

        CodecUtil::write_be_long(&mut output, -1_i64)?; // bad
        CodecUtil::write_be_long(&mut output, 1_i64 << 32)?; // bad
        CodecUtil::write_be_long(&mut output, -(1_i64 << 32))?; // bad
        CodecUtil::write_be_long(&mut output, (1_i64 << 32) - 1)?; // ok
    }

    let mut input =
        BufferedChecksumIndexInput::new(ByteBuffersIndexInput::new(out.get_data_input(), "temp"));

    for _ in 0..3 {
        let result = CodecUtil::read_crc(&mut input);
        assert!(matches!(result, Err(LuceneError::CorruptIndex(_))));
    }

    let result = CodecUtil::read_crc(&mut input);
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_write_bogus_crc() -> Result<(), TestError> {
    let mut out = ByteBuffersDataOutput::with_resettable_instance()?;
    let output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);
    let fake_checksum = AtomicI64::new(0);
    let mut fake_output = FakeOutput::new(output, &fake_checksum);

    fake_checksum.store(-1, std::sync::atomic::Ordering::Relaxed); // bad
    let result = CodecUtil::write_crc(&mut fake_output);
    assert!(result.is_err());
    assert!(matches!(result, Err(LuceneError::IllegalState(_))));

    fake_checksum.store(1 << 32, std::sync::atomic::Ordering::Relaxed); // bad
    let result = CodecUtil::write_crc(&mut fake_output);
    assert!(result.is_err());
    assert!(matches!(result, Err(LuceneError::IllegalState(_))));

    fake_checksum.store(-(1 << 32), std::sync::atomic::Ordering::Relaxed); // bad
    let result = CodecUtil::write_crc(&mut fake_output);
    assert!(result.is_err());
    assert!(matches!(result, Err(LuceneError::IllegalState(_))));

    fake_checksum.store((1 << 32) - 1, std::sync::atomic::Ordering::Relaxed); // ok
    let result = CodecUtil::write_crc(&mut fake_output);
    assert!(result.is_ok());

    Ok(())
}
#[test]
// TODO: This test does not fully reflect the nested error; it needs to be improved.
fn test_truncated_file_throws_corrupt_index_exception() -> Result<(), TestError> {
    let mut out = ByteBuffersDataOutput::with_resettable_instance()?;
    let _output = ByteBuffersIndexOutput::new("temp", "temp", &mut out);

    let mut input = ByteBuffersIndexInput::new(out.get_data_input(), "temp");

    let result = CodecUtil::checksum_entire_file(&mut input);
    assert!(matches!(result, Err(LuceneError::CorruptIndex(_))));
    assert!(result.unwrap_err().to_string().contains(
        "misplaced codec footer (file truncated?): length=0 but footerLength==16 (resource"
    ));

    let result = CodecUtil::retrieve_checksum(&mut input);
    assert!(matches!(result, Err(LuceneError::CorruptIndex(_))));
    assert!(result.unwrap_err().to_string().contains(
        "misplaced codec footer (file truncated?): length=0 but footerLength==16 (resource"
    ));

    Ok(())
}

#[test]
#[cfg(feature = "wait_other_impl")]
fn test_retrieve_checksum() {}

struct FakeOutput<'a> {
    output: ByteBuffersIndexOutput<'a>,
    fake_checksum: &'a AtomicI64,
}
impl<'a> FakeOutput<'a> {
    fn new(output: ByteBuffersIndexOutput<'a>, fake_checksum: &'a AtomicI64) -> Self {
        FakeOutput {
            output,
            fake_checksum,
        }
    }
}

impl DataOutput for FakeOutput<'_> {
    fn write_byte(&mut self, b: u8) -> Result<(), LuceneError> {
        self.output.write_byte(b)
    }

    fn write_bytes_range(&mut self, b: &[u8], offset: i32, length: i32) -> Result<(), LuceneError> {
        self.output.write_bytes_range(b, offset, length)
    }
}

impl Display for FakeOutput<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FakeOutput({})", self.output)
    }
}

impl IndexOutput for FakeOutput<'_> {
    fn get_file_pointer(&self) -> i64 {
        self.output.get_file_pointer()
    }

    fn get_checksum(&mut self) -> u64 {
        self.fake_checksum
            .load(std::sync::atomic::Ordering::Relaxed) as u64
    }

    fn get_name(&self) -> &str {
        unreachable!()
    }
}
