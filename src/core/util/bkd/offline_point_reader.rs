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

use crate::core::codecs::CodecUtil;
use crate::core::index::BytesRef;
use crate::core::store::buffered_checksum_index_input::BufferedChecksumIndexInput;
use crate::core::store::directory::Directory;
use crate::core::store::{DataInput, IOContext, IndexInput};
use crate::core::util::bit_util::BitUtil;
use crate::core::util::bkd::bkd_config::BKDConfig;
use crate::core::util::bkd::point_reader::PointReader;
use crate::core::util::bkd::point_value::PointValue;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;

pub(crate) trait OfflineInput<I: IndexInput> {
  type Input: IndexInput;
  const CHECKSUM: bool;
  fn check_footer(input: &mut Self::Input) -> Result<()>;
}
pub(crate) struct PlainInput;
pub(crate) struct ChecksumInput;
impl<I: IndexInput> OfflineInput<I> for PlainInput {
  type Input = I;
  const CHECKSUM: bool = false;
  fn check_footer(_input: &mut I) -> Result<()> {
    Ok(())
  }
}
impl<I: IndexInput> OfflineInput<I> for ChecksumInput {
  type Input = BufferedChecksumIndexInput<I>;
  const CHECKSUM: bool = true;
  fn check_footer(input: &mut Self::Input) -> Result<()> {
    CodecUtil::check_footer(input).map(|_| ())
  }
}

pub enum OfflinePointReader<I: IndexInput> {
  Plain(OfflinePointReaderImpl<I, PlainInput>),
  Checksum(OfflinePointReaderImpl<I, ChecksumInput>),
}
impl<I: IndexInput> OfflinePointReader<I> {
  pub fn new<D: Directory<IndexInput = I>>(
    config: BKDConfig,
    temp_dir: &D,
    temp_file_name: &str,
    start: usize,
    length: usize,
    reusable_buffer: Vec<u8>,
  ) -> Result<Self> {
    let bytes_per_doc = config.bytes_per_doc();
    let footer_length = CodecUtil::footer_length();
    let file_length = temp_dir.file_length(temp_file_name)?;
    if ((start + length) * bytes_per_doc + footer_length) > file_length {
      return Err(LuceneError::illegal_argument(format!(
        "requested slice is beyond the length of this file: start={} length={} bytesPerDoc={} fileLength={} tempFileName={}",
        start,
        length,
        config.bytes_per_doc(),
        file_length,
        temp_file_name
      )));
    }
    let reusable_buffer_len = reusable_buffer.len();
    if reusable_buffer_len < config.bytes_per_doc() {
      return Err(LuceneError::illegal_argument(format!(
        "Length of reusableBuffer must be bigger than {}",
        config.bytes_per_doc()
      )));
    }

    debug_assert!(reusable_buffer_len <= i32::MAX as usize);
    let max_point_on_heap = reusable_buffer_len / config.bytes_per_doc();
    let seek_fp = start * bytes_per_doc;

    if start == 0 && length * bytes_per_doc == file_length - footer_length {
      let mut input = temp_dir.open_checksum_input(temp_file_name)?;
      IndexInput::seek(&mut input, seek_fp)?;
      Ok(Self::Checksum(OfflinePointReaderImpl::new(
        config,
        input,
        length,
        reusable_buffer,
        max_point_on_heap,
      )))
    } else {
      let mut input = temp_dir.open_input(temp_file_name, &IOContext::read_once_io_context()?)?;
      input.seek(seek_fp)?;
      Ok(Self::Plain(OfflinePointReaderImpl::new(
        config,
        input,
        length,
        reusable_buffer,
        max_point_on_heap,
      )))
    }
  }
  pub(crate) fn take_buffer(&mut self) -> Vec<u8> {
    match self {
      Self::Plain(reader) => std::mem::take(&mut reader.point_value.value),
      Self::Checksum(reader) => std::mem::take(&mut reader.point_value.value),
    }
  }
}
impl<I: IndexInput> Closeable for OfflinePointReader<I> {
  fn close(&mut self) -> Result<()> {
    match self {
      Self::Plain(reader) => reader.close(),
      Self::Checksum(reader) => reader.close(),
    }
  }
}
impl<I: IndexInput> PointReader for OfflinePointReader<I> {
  fn next(&mut self) -> Result<bool> {
    match self {
      Self::Plain(reader) => reader.next(),
      Self::Checksum(reader) => reader.next(),
    }
  }
  type Value<'a>
    = &'a OfflinePointValue
  where
    Self: 'a;
  fn point_value(&mut self) -> Result<Self::Value<'_>> {
    match self {
      Self::Plain(reader) => reader.point_value(),
      Self::Checksum(reader) => reader.point_value(),
    }
  }
}

pub struct OfflinePointReaderImpl<I: IndexInput, M: OfflineInput<I>> {
  count_left: usize,
  input: Option<M::Input>,
  offset: usize,
  checked: bool,
  closed: bool,
  config: BKDConfig,
  points_in_buffer: usize,
  max_point_on_heap: usize,
  pub(crate) point_value: OfflinePointValue,
}
impl<I: IndexInput, M: OfflineInput<I>> OfflinePointReaderImpl<I, M> {
  fn new(
    config: BKDConfig,
    input: M::Input,
    count_left: usize,
    buffer: Vec<u8>,
    max_point_on_heap: usize,
  ) -> Self {
    let point_value = OfflinePointValue::new(&config, buffer);
    Self {
      count_left,
      input: Some(input),
      offset: 0,
      checked: false,
      closed: false,
      config,
      points_in_buffer: 0,
      max_point_on_heap,
      point_value,
    }
  }
}
impl<I: IndexInput, M: OfflineInput<I>> Closeable for OfflinePointReaderImpl<I, M> {
  fn close(&mut self) -> Result<()> {
    if self.closed {
      return Ok(());
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
      if self.count_left == 0
        && M::CHECKSUM
        && !self.checked
        && let Some(input) = self.input.as_mut()
      {
        self.checked = true;
        M::check_footer(input)?;
      }
      Ok(())
    }));
    let close_result = if let Some(input) = self.input.take() {
      std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| input.close()))
    } else {
      Ok(Ok(()))
    };
    self.closed = true;
    IOUtils::finally_caught_result(result, close_result)
  }
}
impl<I: IndexInput, M: OfflineInput<I>> PointReader for OfflinePointReaderImpl<I, M> {
  fn next(&mut self) -> Result<bool> {
    let bytes_per_doc = self.config.bytes_per_doc();
    if self.points_in_buffer == 0 {
      if self.count_left == 0 {
        return Ok(false);
      }
      let count = self.count_left.min(self.max_point_on_heap);
      let read_len = count * bytes_per_doc;
      let input = self.input.as_mut().ok_or_else(|| {
        LuceneError::illegal_state(
          "invalid state: exactly one of check_sum_input and input must be Some",
        )
      })?;
      input.read_bytes(&mut self.point_value.value[0..read_len], 0, read_len)?;
      self.points_in_buffer = count - 1;
      self.count_left -= count;
      self.offset = 0;
    } else {
      self.points_in_buffer -= 1;
      self.offset += bytes_per_doc;
    }
    Ok(true)
  }
  type Value<'a>
    = &'a OfflinePointValue
  where
    Self: 'a;
  fn point_value(&mut self) -> Result<Self::Value<'_>> {
    self.point_value.set_offset(self.offset);
    Ok(&self.point_value)
  }
}
impl<I: IndexInput, M: OfflineInput<I>> Drop for OfflinePointReaderImpl<I, M> {
  fn drop(&mut self) {
    let _ = self.close();
  }
}

/// Reusable implementation for a point value offline.
pub(crate) struct OfflinePointValue {
  pub(crate) offset: usize,
  pub(crate) value: Vec<u8>,
  pub(crate) packed_value_length: usize,
  pub(crate) packed_value_doc_id_length: usize,
}
impl OfflinePointValue {
  pub fn new(config: &BKDConfig, value: Vec<u8>) -> Self {
    Self {
      offset: 0,
      value,
      packed_value_length: config.packed_bytes_length(),
      packed_value_doc_id_length: config.bytes_per_doc(),
    }
  }
  pub fn set_offset(&mut self, offset: usize) {
    self.offset = offset;
  }
}
impl PointValue for OfflinePointValue {
  fn packed_value(&self) -> BytesRef<&[u8]> {
    BytesRef {
      bytes: self.value.as_slice(),
      offset: self.offset,
      length: self.packed_value_length,
    }
  }

  fn doc_id(&self) -> i32 {
    let position = self.offset + self.packed_value_length;
    BitUtil::get_i32_be(&self.value[position..], 0)
  }

  fn packed_value_doc_id_bytes(&self) -> BytesRef<&[u8]> {
    BytesRef {
      bytes: self.value.as_slice(),
      offset: self.offset,
      length: self.packed_value_doc_id_length,
    }
  }
}
