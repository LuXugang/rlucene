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
use std::rc::Rc;

use crate::core::store::DataInput;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fst_impl::fst::BytesReader;
use crate::core::util::group_vint_util::GroupVIntUtil;

/// Reads in reverse from a single byte array.
pub struct ReverseBytesReader {
  bytes: Rc<Vec<u8>>,
  pos: usize,
}

impl ReverseBytesReader {
  pub fn new(bytes: Rc<Vec<u8>>) -> Self {
    Self { bytes, pos: 0 }
  }
}

impl DataInput for ReverseBytesReader {
  fn read_byte(&mut self) -> Result<u8> {
    let b = self.bytes[self.pos];
    self.pos -= 1;
    Ok(b)
  }

  fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    for i in 0..len {
      b[offset + i] = self.bytes[self.pos];
      self.pos -= 1;
    }
    Ok(())
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    GroupVIntUtil::read_group_vint_i32(self, dst, offset)
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    if num_bytes >= 0 {
      let v = num_bytes as usize;
      self.pos = self.pos.checked_sub(v).ok_or_else(|| {
        LuceneError::illegal_state(format!(
          "underflow, pos {}, num_num_bytes {} ",
          self.pos, num_bytes
        ))
      })?;
    } else {
      let v = -num_bytes as usize;
      self.pos += v;
    }

    Ok(())
  }
}

impl BytesReader for ReverseBytesReader {
  fn get_position(&self) -> usize {
    self.pos
  }

  fn set_position(&mut self, pos: usize) {
    self.pos = pos;
  }
}

impl std::fmt::Display for ReverseBytesReader {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}
