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
use crate::core::analysis::char_filter::CharFilter;
use crate::core::analysis::reader::{Reader, ReaderEnum};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::collections::BTreeMap;

/// the purpose of this charfilter is to send offsets out of bounds if the analyzer doesn't use correctOffset or does incorrect offset math.
#[derive(Clone, Debug)]
pub struct MockCharFilter {
  input: Box<ReaderEnum>,
  remainder: i32,
  current_offset: i32,
  delta: i32,
  buffered_ch: i32,
  corrections: BTreeMap<i32, i32>,
}

impl MockCharFilter {
  pub fn new(input: ReaderEnum, remainder: i32) -> Result<Self> {
    if !(0..10).contains(&remainder) {
      return Err(LuceneError::illegal_argument(format!(
        "invalid remainder parameter (must be 0..10): {remainder}"
      )));
    }

    Ok(Self {
      input: Box::new(input),
      remainder,
      current_offset: -1,
      delta: 0,
      buffered_ch: -1,
      corrections: BTreeMap::new(),
    })
  }

  fn add_off_correct_map(&mut self, off: i32, cumulative_diff: i32) {
    self.corrections.insert(off, cumulative_diff);
  }
}

impl Reader for MockCharFilter {
  fn read(&mut self) -> Result<i32> {
    if self.buffered_ch >= 0 {
      let ch = self.buffered_ch;
      self.buffered_ch = -1;
      self.current_offset += 1;

      self.add_off_correct_map(self.current_offset, self.delta - 1);
      self.delta -= 1;
      return Ok(ch);
    }

    let ch = self.input.read()?;
    if ch < 0 {
      return Ok(ch);
    }

    self.current_offset += 1;
    if (ch % 10) != self.remainder {
      return Ok(ch);
    }

    self.buffered_ch = ch;
    Ok(ch)
  }

  fn read_range(&mut self, cbuf: &mut [char], off: usize, len: usize) -> Result<i32> {
    let mut num_read = 0;
    for slot in cbuf.iter_mut().skip(off).take(len) {
      let c = self.read()?;
      if c == -1 {
        break;
      }
      *slot = char::from_u32(c as u32).unwrap_or(char::REPLACEMENT_CHARACTER);
      num_read += 1;
    }

    Ok(if num_read == 0 { -1 } else { num_read })
  }

  fn close(&mut self) -> Result<()> {
    CharFilter::close(self)
  }
}

impl CharFilter for MockCharFilter {
  fn get_reader(&self) -> &ReaderEnum {
    &self.input
  }

  fn get_reader_mut(&mut self) -> &mut ReaderEnum {
    &mut self.input
  }

  fn correct(&self, current_off: i32) -> i32 {
    let ret = self
      .corrections
      .range(..=current_off)
      .next_back()
      .map_or(current_off, |(_, diff)| current_off + *diff);
    debug_assert!(
      ret >= 0,
      "currentOff={current_off},diff={}",
      ret - current_off
    );
    ret
  }
}
