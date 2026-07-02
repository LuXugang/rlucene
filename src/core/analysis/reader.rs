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
#[cfg(test)]
use crate::core::analysis::char_filter::CharFilter;
#[cfg(test)]
use crate::test_framework::core::analysis::char_filter::{CharFilter1, CharFilter2};
#[cfg(test)]
use crate::test_framework::core::analysis::mock_char_filter::MockCharFilter;

use crate::core::analysis::reusable_string_reader::ReusableStringReader;
use crate::core::analysis::tokenizer::IllegalStateReader;
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub trait Reader {
  /// Reads a single character. Returns -1 on EOF
  fn read(&mut self) -> Result<i32> {
    let mut cb: Vec<char> = vec![char::from(0); 1];
    if self.read_range(&mut cb, 0, 1)? == -1 {
      return Ok(-1);
    }
    Ok(cb[0] as i32)
  }
  fn read_buf(&mut self, cbuf: &mut [char]) -> Result<i32> {
    self.read_range(cbuf, 0, cbuf.len())
  }
  /// Reads characters into the buffer, starting at `off`,
  /// up to `len` characters. Returns the number of chars read,
  /// or -1 on EOF.
  fn read_range(&mut self, buf: &mut [char], off: usize, len: usize) -> Result<i32>;
  fn close(&mut self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum ReaderEnum {
  ReusedString(ReusableStringReader),
  IllegalState(IllegalStateReader),
  String(StringReader),
  #[cfg(test)]
  CharFilter1(CharFilter1),
  #[cfg(test)]
  CharFilter2(CharFilter2),
  #[cfg(test)]
  MockCharFilter(MockCharFilter),
}
// for std::mem::take
impl Default for ReaderEnum {
  fn default() -> Self {
    ReaderEnum::IllegalState(IllegalStateReader)
  }
}
impl ReaderEnum {
  pub fn correct_offset(&self, corrected: i32) -> i32 {
    match self {
      #[cfg(test)]
      ReaderEnum::CharFilter1(r) => r.correct_offset(corrected),
      #[cfg(test)]
      ReaderEnum::CharFilter2(r) => r.correct_offset(corrected),
      #[cfg(test)]
      ReaderEnum::MockCharFilter(r) => r.correct_offset(corrected),
      // not a CharFilter
      _ => corrected,
    }
  }
}
impl Reader for ReaderEnum {
  fn read(&mut self) -> Result<i32> {
    match self {
      ReaderEnum::ReusedString(r) => r.read(),
      ReaderEnum::IllegalState(r) => r.read(),
      ReaderEnum::String(r) => r.read(),
      #[cfg(test)]
      ReaderEnum::CharFilter1(r) => r.read(),
      #[cfg(test)]
      ReaderEnum::CharFilter2(r) => r.read(),
      #[cfg(test)]
      ReaderEnum::MockCharFilter(r) => r.read(),
    }
  }

  fn read_buf(&mut self, cbuf: &mut [char]) -> Result<i32> {
    match self {
      ReaderEnum::ReusedString(r) => r.read_buf(cbuf),
      ReaderEnum::IllegalState(r) => r.read_buf(cbuf),
      ReaderEnum::String(r) => r.read_buf(cbuf),
      #[cfg(test)]
      ReaderEnum::CharFilter1(r) => r.read_buf(cbuf),
      #[cfg(test)]
      ReaderEnum::CharFilter2(r) => r.read_buf(cbuf),
      #[cfg(test)]
      ReaderEnum::MockCharFilter(r) => r.read_buf(cbuf),
    }
  }

  fn read_range(&mut self, buf: &mut [char], off: usize, len: usize) -> Result<i32> {
    match self {
      ReaderEnum::ReusedString(r) => r.read_range(buf, off, len),
      ReaderEnum::IllegalState(r) => r.read_range(buf, off, len),
      ReaderEnum::String(r) => r.read_range(buf, off, len),
      #[cfg(test)]
      ReaderEnum::CharFilter1(r) => r.read_range(buf, off, len),
      #[cfg(test)]
      ReaderEnum::CharFilter2(r) => r.read_range(buf, off, len),
      #[cfg(test)]
      ReaderEnum::MockCharFilter(r) => r.read_range(buf, off, len),
    }
  }

  fn close(&mut self) -> Result<()> {
    match self {
      ReaderEnum::ReusedString(r) => r.close(),
      ReaderEnum::IllegalState(r) => r.close(),
      ReaderEnum::String(r) => r.close(),
      #[cfg(test)]
      ReaderEnum::CharFilter1(r) => CharFilter::close(r),
      #[cfg(test)]
      ReaderEnum::CharFilter2(r) => CharFilter::close(r),
      #[cfg(test)]
      ReaderEnum::MockCharFilter(r) => CharFilter::close(r),
    }
  }
}

impl<'a> From<&'a str> for ReaderEnum {
  fn from(text: &'a str) -> Self {
    let mut reader = ReusableStringReader::new();
    reader.set_value(text);
    ReaderEnum::ReusedString(reader)
  }
}

impl From<&String> for ReaderEnum {
  fn from(text: &String) -> Self {
    ReaderEnum::from(text.as_str())
  }
}

impl From<String> for ReaderEnum {
  fn from(text: String) -> Self {
    let mut reader = ReusableStringReader::new();
    reader.set_value(&text);
    ReaderEnum::ReusedString(reader)
  }
}
impl From<ReusableStringReader> for ReaderEnum {
  fn from(reader: ReusableStringReader) -> Self {
    ReaderEnum::ReusedString(reader)
  }
}
impl From<IllegalStateReader> for ReaderEnum {
  fn from(reader: IllegalStateReader) -> Self {
    ReaderEnum::IllegalState(reader)
  }
}
impl From<StringReader> for ReaderEnum {
  fn from(reader: StringReader) -> Self {
    ReaderEnum::String(reader)
  }
}

/// A character stream whose source is a string.
#[derive(Debug, Clone)]
pub struct StringReader {
  chars: Option<Vec<char>>,
  next: usize,
}

impl StringReader {
  pub fn new(s: impl Into<String>) -> Self {
    Self {
      chars: Some(s.into().chars().collect()),
      next: 0,
    }
  }

  fn ensure_open(&self) -> Result<&[char]> {
    self
      .chars
      .as_deref()
      .ok_or_else(|| LuceneError::illegal_state("Stream closed"))
  }
}

impl Reader for StringReader {
  fn read(&mut self) -> Result<i32> {
    let chars = self.ensure_open()?;
    if self.next >= chars.len() {
      return Ok(-1);
    }

    let ch = chars[self.next];
    self.next += 1;
    Ok(ch as i32)
  }

  fn read_range(&mut self, buf: &mut [char], off: usize, len: usize) -> Result<i32> {
    let chars = self.ensure_open()?;
    let end = off
      .checked_add(len)
      .ok_or_else(|| LuceneError::illegal_argument("IndexOutOfBounds: off+len overflow"))?;
    if off > buf.len() || end > buf.len() {
      return Err(LuceneError::illegal_argument(
        "IndexOutOfBounds: off+len exceeds buffer length",
      ));
    }
    if len == 0 {
      return Ok(0);
    }
    if self.next >= chars.len() {
      return Ok(-1);
    }

    let to_read = len.min(chars.len() - self.next);
    buf[off..off + to_read].copy_from_slice(&chars[self.next..self.next + to_read]);
    self.next += to_read;
    Ok(to_read as i32)
  }

  fn close(&mut self) -> Result<()> {
    self.chars = None;
    Ok(())
  }
}
