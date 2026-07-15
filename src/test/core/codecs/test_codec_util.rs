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
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::atomic::AtomicI64;

use crate::core::codecs::CodecUtil;
use crate::core::store::buffered_checksum_index_input::BufferedChecksumIndexInput;
use crate::core::store::directory::Directory;
use crate::core::store::index_input::IndexInput;
use crate::core::store::{
  ByteBuffersDataOutput, ByteBuffersIndexInput, ByteBuffersIndexOutput, DataInput, DataOutput,
  IOContext, IndexOutput,
};
use crate::core::util::StringHelper;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::util::lucene_test_case::{new_directory, random};

#[allow(dead_code)] // for quick search
struct TestCodecUtil;

#[test]
fn test_header_length() -> Result<()> {
  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");
  CodecUtil::write_header(&mut output, "FooBar", 5)?;
  output.write_string("this is the data")?;

  let mut input = ByteBuffersIndexInput::new(output.delegate_mut()?.get_data_input_ref()?, "temp");
  input.seek(CodecUtil::header_length("FooBar"))?;
  assert_eq!(input.read_string()?, "this is the data");
  Ok(())
}

#[test]
fn test_write_too_long_header() -> Result<()> {
  let too_long: String = "a".repeat(128);

  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");

  let result = CodecUtil::write_header(&mut output, &too_long, 5);
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
  Ok(())
}

#[test]
fn test_write_non_ascii_header() -> Result<()> {
  let non_ascii_header = "\u{1234}".to_string();

  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");

  let result = CodecUtil::write_header(&mut output, &non_ascii_header, 5);
  assert!(result.is_ok());
  Ok(())
}

#[test]
fn test_read_header_wrong_magic() -> Result<()> {
  let mut index_output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");
  index_output.write_int(1234)?;

  // 创建输入对象
  let input_data = index_output.delegate_mut()?.get_data_input_ref()?;
  let mut input = ByteBuffersIndexInput::new(input_data, "temp");

  let result = CodecUtil::check_header(&mut input, "bogus", 1, 1);
  assert!(matches!(result, Err(LuceneError::CorruptIndex(_))));
  Ok(())
}

#[test]
fn test_checksum_entire_file() -> Result<()> {
  let mut index_output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");
  CodecUtil::write_header(&mut index_output, "FooBar", 5)?;
  index_output.write_string("this is the data")?;
  CodecUtil::write_footer(&mut index_output)?;

  let input_data =
    ByteBuffersIndexInput::new(index_output.delegate_mut()?.get_data_input_ref()?, "temp");
  CodecUtil::checksum_entire_file(&input_data)?;
  Ok(())
}
#[test]
fn test_check_footer_valid() -> Result<()> {
  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");
  CodecUtil::write_header(&mut output, "FooBar", 5)?;
  output.write_string("this is the data")?;
  CodecUtil::write_footer(&mut output)?;

  let mut input = BufferedChecksumIndexInput::new(ByteBuffersIndexInput::new(
    output.delegate_mut()?.get_data_input_ref()?,
    "temp",
  ));
  let mine = LuceneError::illegal_argument("fake exception");
  let result = CodecUtil::check_footer_with_error(&mut input, mine);
  match result.get_suppressed()? {
    Some(suppressed) => {
      let suppressed_message = suppressed.to_string();
      assert!(suppressed_message.contains("checksum passed"));
    },
    None => unreachable!(""),
  }
  Ok(())
}

#[test]
fn test_check_footer_valid_at_footer() -> Result<()> {
  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");
  CodecUtil::write_header(&mut output, "FooBar", 5)?;
  output.write_string("this is the data")?;
  CodecUtil::write_footer(&mut output)?;

  let mut input = BufferedChecksumIndexInput::new(ByteBuffersIndexInput::new(
    output.delegate_mut()?.get_data_input_ref()?,
    "temp",
  ));
  CodecUtil::check_header(&mut input, "FooBar", 5, 5)?;
  let read_data = input.read_string()?;
  assert_eq!(read_data, "this is the data");
  let mine = LuceneError::illegal_argument("fake exception");
  let result = CodecUtil::check_footer_with_error(&mut input, mine);
  let err_message = result.to_string();
  assert!(err_message.contains("fake exception"));
  match result.get_suppressed()? {
    Some(suppressed) => {
      let suppressed_message = suppressed.to_string();
      assert!(suppressed_message.contains("checksum passed"));
    },
    None => unreachable!(""),
  }
  Ok(())
}
#[test]
fn test_check_footer_valid_past_footer() -> Result<()> {
  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");
  CodecUtil::write_header(&mut output, "FooBar", 5)?;
  output.write_string("this is the data")?;
  CodecUtil::write_footer(&mut output)?;

  let mut input = BufferedChecksumIndexInput::new(ByteBuffersIndexInput::new(
    output.delegate_mut()?.get_data_input_ref()?,
    "temp",
  ));

  CodecUtil::check_header(&mut input, "FooBar", 5, 5)?;
  let read_data = input.read_string()?;
  assert_eq!(read_data, "this is the data");

  // Bogusly read a byte too far
  input.read_byte()?;

  let mine = LuceneError::illegal_argument("fake exception");
  let result = CodecUtil::check_footer_with_error(&mut input, mine);
  let err_message = result.to_string();
  assert!(err_message.contains("checksum status indeterminate"));
  match result.get_suppressed()? {
    Some(suppressed) => {
      let suppressed_message = suppressed.to_string();
      assert!(suppressed_message.contains("fake exception"));
    },
    None => unreachable!(""),
  }
  Ok(())
}
#[test]
fn test_check_footer_invalid() -> Result<()> {
  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");
  CodecUtil::write_header(&mut output, "FooBar", 5)?;
  output.write_string("this is the data")?;
  CodecUtil::write_be_int(&mut output, CodecUtil::FOOTER_MAGIC)?;
  CodecUtil::write_be_int(&mut output, 0)?;
  CodecUtil::write_be_long(&mut output, 1234567)?; // write a bogus
  // checksum
  let mut input = BufferedChecksumIndexInput::new(ByteBuffersIndexInput::new(
    output.delegate_mut()?.get_data_input_ref()?,
    "temp",
  ));
  CodecUtil::check_header(&mut input, "FooBar", 5, 5)?;
  let read_data = input.read_string()?;
  assert_eq!(read_data, "this is the data");
  let mine = LuceneError::illegal_argument("fake exception");
  let result = CodecUtil::check_footer_with_error(&mut input, mine);
  assert!(result.source().is_some());
  let err_message = result.to_string();
  assert!(err_message.contains("checksum failed"));
  match result.get_suppressed()? {
    Some(suppressed) => {
      let suppressed_message = suppressed.to_string();
      assert!(suppressed_message.contains("fake exception"));
    },
    None => {
      unreachable!("suppressed is None");
    },
  }
  Ok(())
}
#[test]
fn test_segment_header_length() -> Result<()> {
  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");
  let id = StringHelper::random_id();
  CodecUtil::write_index_header(&mut output, "FooBar", 5, &id, "xyz")?;
  output.write_string("this is the data")?;
  let mut input = ByteBuffersIndexInput::new(output.delegate_mut()?.get_data_input_ref()?, "temp");

  input.seek(CodecUtil::index_header_length("FooBar", "xyz"))?;

  let read_data = input.read_string()?;
  assert_eq!(read_data, "this is the data");

  Ok(())
}
#[test]
fn test_write_too_long_suffix() {
  let too_long: String = "a".repeat(256);
  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");

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
fn test_write_very_long_suffix() -> Result<()> {
  let just_long_enough: String = "a".repeat(255);

  let id = StringHelper::random_id();
  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");
  CodecUtil::write_index_header(&mut output, "foobar", 5, &id, &just_long_enough)?;

  let mut input = ByteBuffersIndexInput::new(output.delegate_mut()?.get_data_input_ref()?, "temp");
  CodecUtil::check_index_header(&mut input, "foobar", 5, 5, &id, &just_long_enough)?;

  assert_eq!(input.get_file_pointer()?, input.length()?);
  assert_eq!(
    input.get_file_pointer()?,
    CodecUtil::index_header_length("foobar", &just_long_enough)
  );

  Ok(())
}
#[test]
fn test_write_non_ascii_suffix() {
  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");

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
fn test_read_bogus_crc() -> Result<()> {
  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");

  CodecUtil::write_be_long(&mut output, -1_i64)?; // bad
  CodecUtil::write_be_long(&mut output, 1_i64 << 32)?; // bad
  CodecUtil::write_be_long(&mut output, -(1_i64 << 32))?; // bad
  CodecUtil::write_be_long(&mut output, (1_i64 << 32) - 1)?; // ok

  let mut input = BufferedChecksumIndexInput::new(ByteBuffersIndexInput::new(
    output.delegate_mut()?.get_data_input_ref()?,
    "temp",
  ));

  for _ in 0..3 {
    let result = CodecUtil::read_crc(&mut input);
    assert!(matches!(result, Err(LuceneError::CorruptIndex(_))));
  }

  let result = CodecUtil::read_crc(&mut input);
  assert!(result.is_ok());

  Ok(())
}

#[test]
fn test_write_bogus_crc() -> Result<()> {
  let output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");
  let fake_checksum = AtomicI64::new(0);
  let mut fake_output = FakeOutput::new(output, &fake_checksum);

  fake_checksum.store(-1, std::sync::atomic::Ordering::SeqCst); // bad
  let result = CodecUtil::write_crc(&mut fake_output);
  assert!(result.is_err());
  assert!(matches!(result, Err(LuceneError::IllegalState(_))));

  fake_checksum.store(1 << 32, std::sync::atomic::Ordering::SeqCst); // bad
  let result = CodecUtil::write_crc(&mut fake_output);
  assert!(result.is_err());
  assert!(matches!(result, Err(LuceneError::IllegalState(_))));

  fake_checksum.store(-(1 << 32), std::sync::atomic::Ordering::SeqCst); // bad
  let result = CodecUtil::write_crc(&mut fake_output);
  assert!(result.is_err());
  assert!(matches!(result, Err(LuceneError::IllegalState(_))));

  fake_checksum.store((1 << 32) - 1, std::sync::atomic::Ordering::SeqCst); // ok
  let result = CodecUtil::write_crc(&mut fake_output);
  assert!(result.is_ok());

  Ok(())
}
#[test]
fn test_truncated_file_throws_corrupt_index_exception() -> Result<()> {
  let mut output = ByteBuffersIndexOutput::new(ByteBuffersDataOutput::new(), "temp", "temp");

  let mut input = ByteBuffersIndexInput::new(output.delegate_mut()?.get_data_input_ref()?, "temp");

  let result = CodecUtil::checksum_entire_file(&input);
  assert!(matches!(result, Err(LuceneError::CorruptIndex(_))));
  assert!(
    result.unwrap_err().to_string().contains(
      "misplaced codec footer (file truncated?): length=0 but footerLength==16 (resource"
    )
  );

  let result = CodecUtil::retrieve_checksum(&mut input);
  assert!(matches!(result, Err(LuceneError::CorruptIndex(_))));
  assert!(
    result.unwrap_err().to_string().contains(
      "misplaced codec footer (file truncated?): length=0 but footerLength==16 (resource"
    )
  );

  Ok(())
}

#[test]
fn test_retrieve_checksum() -> Result<()> {
  let mut random = random();
  let dir = new_directory(&mut random)?;
  {
    let mut out = dir.create_output("foo", &IOContext::default_io_context()?)?;
    let result = (|| -> Result<()> {
      out.write_byte(42)?;
      CodecUtil::write_footer(&mut out)
    })();
    IOUtils::use_or_suppress_result(result, out.close())?;
  }
  {
    let mut input = dir.open_input("foo", &IOContext::default_io_context()?)?;
    let result = (|| -> Result<()> {
      let length = input.length()?;
      CodecUtil::retrieve_checksum_with_expected(&mut input, length)?; // no exception

      let exception = CodecUtil::retrieve_checksum_with_expected(&mut input, length - 1)
        .expect_err("expected file-too-long corruption");
      assert!(exception.to_string().contains("too long"));
      assert!(exception.get_suppressed()?.is_none());

      let exception = CodecUtil::retrieve_checksum_with_expected(&mut input, length + 1)
        .expect_err("expected truncated-file corruption");
      assert!(exception.to_string().contains("truncated"));
      assert!(exception.get_suppressed()?.is_none());
      Ok(())
    })();
    IOUtils::use_or_suppress_result(result, input.close())?;
  }

  {
    let mut out = dir.create_output("bar", &IOContext::default_io_context()?)?;
    let result = (|| -> Result<()> {
      for i in 0..=CodecUtil::footer_length() {
        out.write_byte(i as u8)?;
      }
      Ok(())
    })();
    IOUtils::use_or_suppress_result(result, out.close())?;
  }
  {
    let mut input = dir.open_input("bar", &IOContext::default_io_context()?)?;
    let result = (|| -> Result<()> {
      let length = input.length()?;
      let exception = CodecUtil::retrieve_checksum_with_expected(&mut input, length)
        .expect_err("expected codec-footer mismatch");
      assert!(exception.to_string().contains("codec footer mismatch"));
      assert!(exception.get_suppressed()?.is_none());

      let exception = CodecUtil::retrieve_checksum_with_expected(&mut input, length - 1)
        .expect_err("expected file-too-long corruption");
      assert!(exception.to_string().contains("too long"));

      let exception = CodecUtil::retrieve_checksum_with_expected(&mut input, length + 1)
        .expect_err("expected truncated-file corruption");
      assert!(exception.to_string().contains("truncated"));
      Ok(())
    })();
    IOUtils::use_or_suppress_result(result, input.close())?;
  }

  dir.close()
}

struct FakeOutput<'a> {
  output: ByteBuffersIndexOutput,
  fake_checksum: &'a AtomicI64,
}
impl<'a> FakeOutput<'a> {
  fn new(output: ByteBuffersIndexOutput, fake_checksum: &'a AtomicI64) -> Self {
    FakeOutput {
      output,
      fake_checksum,
    }
  }
}

impl DataOutput for FakeOutput<'_> {
  fn write_byte(&mut self, b: u8) -> Result<()> {
    self.output.write_byte(b)
  }

  fn write_bytes_range(&mut self, b: &[u8], offset: usize, length: usize) -> Result<()> {
    self.output.write_bytes_range(b, offset, length)
  }
}

impl Display for FakeOutput<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "FakeOutput({})", self.output)
  }
}

impl Closeable for FakeOutput<'_> {
  fn close(&mut self) -> Result<()> {
    Ok(())
  }
}

impl IndexOutput for FakeOutput<'_> {
  fn get_file_pointer(&self) -> Result<usize> {
    self.output.get_file_pointer()
  }

  fn get_checksum(&mut self) -> Result<u64> {
    Ok(self.fake_checksum.load(std::sync::atomic::Ordering::SeqCst) as u64)
  }

  fn get_name(&self) -> &str {
    unreachable!()
  }
}
