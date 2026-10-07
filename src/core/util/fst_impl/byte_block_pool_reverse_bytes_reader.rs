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

use crate::core::store::data_input_ext::DataInputExt;
use std::fmt::{Display, Formatter};

use crate::core::store::DataInput;
use crate::core::util::ByteBlockPool;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fst_impl::fst::BytesReader;
use crate::core::util::group_vint_util::GroupVIntUtil;

/// Reads in reverse from a ByteBlockPool.
pub struct ByteBlockPoolReverseBytesReader {
  pub(crate) buf: ByteBlockPool,
  // the difference between the FST node address and the hash table copied
  // node address
  pos_delta: i64,
  pos: i64,
}
impl ByteBlockPoolReverseBytesReader {
  pub fn new(buf: ByteBlockPool) -> Self {
    Self {
      buf,
      pos_delta: 0,
      pos: 0,
    }
  }
  pub fn set_pos_delta(&mut self, pos_delta: i64) {
    self.pos_delta = pos_delta;
  }
}

impl crate::core::util::close::Closeable for ByteBlockPoolReverseBytesReader {}

impl DataInput for ByteBlockPoolReverseBytesReader {
  fn read_byte(&mut self) -> Result<u8> {
    debug_assert!(self.pos >= 0);
    let b = self.buf.read_byte(self.pos as usize);
    self.pos -= 1;
    Ok(b)
  }

  fn get_bytes(&mut self, len: usize) -> Result<std::borrow::Cow<'_, [u8]>> {
    if len > 1 && self.pos >= 0 {
      let pos = self.pos as usize;
      let end = (pos & crate::core::util::byte_block_pool::BYTE_BLOCK_MASK as usize) + 1;
      let block = pos >> crate::core::util::BYTE_BLOCK_SHIFT;
      if len <= end && self.buf.buffer_upto.is_some_and(|last| block <= last) {
        let buffer = self.buf.get_buffer(block);
        if let Some(range) = buffer.get(end - len..end) {
          let bytes = range.iter().rev().copied().collect::<Vec<u8>>();
          self.pos -= len as i64;
          return Ok(std::borrow::Cow::Owned(bytes));
        }
      }
    }
    let mut bytes = Vec::with_capacity(len);
    while bytes.len() < len {
      if self.pos >= 0 {
        let pos = self.pos as usize;
        let end = (pos & crate::core::util::byte_block_pool::BYTE_BLOCK_MASK as usize) + 1;
        let block = pos >> crate::core::util::BYTE_BLOCK_SHIFT;
        let buffer = self.buf.get_buffer(block);
        let count = (len - bytes.len())
          .min(end)
          .min(bytes.capacity() - bytes.len());
        if let Some(range) = buffer.get(end - count..end) {
          bytes.extend(range.iter().rev().copied());
          self.pos -= count as i64;
          continue;
        }
      }
      bytes.extend(std::iter::once(self.read_byte()?).take(bytes.capacity() - bytes.len()));
    }
    Ok(std::borrow::Cow::Owned(bytes))
  }

  fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    for i in 0..len {
      debug_assert!(self.pos >= 0);
      b[offset + i] = self.buf.read_byte(self.pos as usize);
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

impl DataInputExt for ByteBlockPoolReverseBytesReader {}

impl Display for ByteBlockPoolReverseBytesReader {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl BytesReader for ByteBlockPoolReverseBytesReader {
  fn get_position(&self) -> i64 {
    let pos = self.pos + self.pos_delta;
    debug_assert!(pos >= 0);
    pos
  }

  fn set_position(&mut self, pos: i64) {
    self.pos = pos - self.pos_delta;
  }
}
