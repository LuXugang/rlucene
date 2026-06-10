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
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fst_impl::fst::BytesReader;
use crate::core::util::group_vint_util::GroupVIntUtil;

/// Reads in reverse from a single byte array.
pub struct ReverseBytesReader {
  bytes: Rc<Vec<u8>>,
  pos: i64,
}

impl ReverseBytesReader {
  pub fn new(bytes: Rc<Vec<u8>>) -> Self {
    Self { bytes, pos: 0 }
  }
}

impl DataInput for ReverseBytesReader {
  fn read_byte(&mut self) -> Result<u8> {
    debug_assert!(self.pos >= 0);
    let b = self.bytes[self.pos as usize];
    self.pos -= 1;
    Ok(b)
  }

  fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    for i in 0..len {
      debug_assert!(self.pos >= 0);
      b[offset + i] = self.bytes[self.pos as usize];
      self.pos -= 1;
    }
    Ok(())
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    GroupVIntUtil::read_group_vint_i32(self, dst, offset)
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    self.pos -= num_bytes;
    Ok(())
  }
}

impl BytesReader for ReverseBytesReader {
  fn get_position(&self) -> i64 {
    debug_assert!(self.pos >= 0);
    self.pos
  }

  fn set_position(&mut self, pos: i64) {
    self.pos = pos;
  }
}

impl std::fmt::Display for ReverseBytesReader {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}
