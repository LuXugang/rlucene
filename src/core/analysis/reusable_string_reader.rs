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
use std::sync::Arc;

use crate::core::analysis::reader::Reader;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// Internal reader that allows a string reader to be reused by an analyzer's token stream.
#[derive(Debug, Clone)]
pub struct ReusableStringReader {
  pos: usize,
  s: Option<Arc<str>>,
}

impl Default for ReusableStringReader {
  fn default() -> Self {
    Self::new()
  }
}

impl ReusableStringReader {
  pub fn new() -> Self {
    Self { pos: 0, s: None }
  }

  pub fn set_value(&mut self, s: &str) {
    self.pos = 0;
    self.s = Some(Arc::from(s));
  }
}

impl Reader for ReusableStringReader {
  fn read(&mut self) -> Result<i32> {
    if let Some(ref s) = self.s
      && self.pos < s.len()
    {
      let ch = expect_invariant!(
        s[self.pos..].chars().next(),
        "reader position stays on a character boundary"
      );
      self.pos += ch.len_utf8();
      return Ok(ch as i32);
    }
    self.s = None;
    Ok(-1)
  }

  fn read_range(&mut self, buf: &mut [char], off: usize, len: usize) -> Result<i32> {
    if let Some(ref s) = self.s
      && self.pos < s.len()
    {
      if off > buf.len() || off + len > buf.len() {
        return Err(LuceneError::illegal_argument(
          "IndexOutOfBounds: off+len exceeds buffer length",
        ));
      }
      let mut read = 0;
      let mut consumed = 0;
      for ch in s[self.pos..].chars().take(len) {
        buf[off + read] = ch;
        read += 1;
        consumed += ch.len_utf8();
      }
      self.pos += consumed;
      return Ok(read as i32);
    }
    self.s = None;
    Ok(-1)
  }

  fn close(&mut self) -> Result<()> {
    self.pos = 0;
    self.s = None;
    Ok(())
  }
}
impl Drop for ReusableStringReader {
  fn drop(&mut self) {
    let _ = self.close();
  }
}
