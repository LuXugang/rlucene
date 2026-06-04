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
use std::fmt;

use crate::core::analysis::reader::Reader;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;

pub struct CharacterUtils;
impl CharacterUtils {
  pub fn new_character_buffer(buffer_size: usize) -> Result<CharacterBuffer> {
    if buffer_size == 0 {
      return Err(LuceneError::illegal_argument("buffer_size must be > 0"));
    }
    Ok(CharacterBuffer::new(vec!['\0'; buffer_size], 0, 0))
  }
  pub fn convert_to_lower_case(buffer: &mut [char], offset: usize, limit: usize) {
    debug_assert!(buffer.len() >= limit);
    debug_assert!(offset <= buffer.len());

    for ch in &mut buffer[offset..limit] {
      *ch = ch.to_lowercase().next().unwrap_or(*ch);
    }
  }
  pub fn get_upper_case(buffer: &mut [char], offset: usize, limit: usize) {
    debug_assert!(buffer.len() >= limit);
    debug_assert!(offset <= buffer.len());

    for ch in &mut buffer[offset..limit] {
      *ch = ch.to_uppercase().next().unwrap_or(*ch);
    }
  }
  pub fn get_code_points(
    src: &[char],
    src_off: usize,
    src_len: usize,
    dest: &mut [i32],
    dest_off: usize,
  ) -> Result<usize> {
    if src_len > src.len().saturating_sub(src_off) {
      return Err(LuceneError::illegal_argument(
        "src_off + src_len out of bounds",
      ));
    }
    if dest_off > dest.len() || src_len > dest.len().saturating_sub(dest_off) {
      return Err(LuceneError::illegal_argument(
        "dest_off + src_len out of bounds",
      ));
    }

    let mut count = 0;
    for i in 0..src_len {
      dest[dest_off + count] = src[src_off + i] as i32;
      count += 1;
    }
    Ok(count)
  }
  pub fn get_chars(
    src: &[i32],
    src_off: usize,
    src_len: usize,
    dest: &mut [char],
    dest_off: usize,
  ) -> Result<usize> {
    let mut written = 0;
    for &cp_i64 in &src[src_off..src_off + src_len] {
      let cp = u32::try_from(cp_i64)
        .map_err(|_| LuceneError::illegal_argument("code point must be >= 0"))?;
      let ch = std::char::from_u32(cp)
        .ok_or_else(|| LuceneError::illegal_argument("invalid Unicode code point"))?;
      dest[dest_off + written] = ch;
      written += 1;
    }

    Ok(written)
  }
  pub fn fill_with_num<R>(
    buffer: &mut CharacterBuffer,
    reader: &mut R,
    num_chars: usize,
  ) -> Result<bool>
  where
    R: Reader,
  {
    if num_chars < 1 || num_chars > buffer.buffer.len() {
      return Err(LuceneError::illegal_argument(
        "num_chars must be >= 1 and <= buffer size",
      ));
    }
    let offset = 0;
    buffer.offset = 0;
    let read = Self::read_fully(reader, &mut buffer.buffer, offset, num_chars)?;
    buffer.length = read;
    Ok(read == num_chars)
  }
  pub fn fill<R>(buffer: &mut CharacterBuffer, reader: &mut R) -> Result<bool>
  where
    R: Reader,
  {
    Self::fill_with_num(buffer, reader, buffer.buffer.len())
  }
  pub fn read_fully<R>(
    reader: &mut R,
    dest: &mut [char],
    offset: usize,
    len: usize,
  ) -> Result<usize>
  where
    R: Reader,
  {
    let mut read = 0;
    while read < len {
      let r = reader.read_range(dest, offset + read, len - read)?;
      if r == -1 {
        break;
      }
      read += r as usize;
    }
    Ok(read)
  }
}

pub struct CharacterBuffer {
  pub(crate) buffer: Vec<char>,
  pub(crate) offset: usize,
  pub(crate) length: usize,
}
impl CharacterBuffer {
  pub fn new(buffer: Vec<char>, offset: usize, length: usize) -> Self {
    CharacterBuffer {
      buffer,
      offset,
      length,
    }
  }
  /// Returns the internal buffer
  pub fn get_buffer(&self) -> &[char] {
    &self.buffer
  }

  /// Returns the data offset in the internal buffer.
  pub fn get_offset(&self) -> usize {
    self.offset
  }
  /// Return the length of the data in the internal buffer starting at [`getOffset()`](Self::get_offset)
  pub fn get_length(&self) -> usize {
    self.length
  }

  /// Resets the CharacterBuffer. All internals are reset to its default values.
  pub fn reset(&mut self) {
    self.offset = 0;
    self.length = 0;
  }
}

impl fmt::Display for CharacterBuffer {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for ch in &self.buffer[self.offset..self.offset + self.length] {
      write!(f, "{ch}")?;
    }
    Ok(())
  }
}
