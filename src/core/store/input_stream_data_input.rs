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
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom};

use crate::core::store::data_input::DataInput;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::group_vint_util::GroupVIntUtil;

/// A [`DataInput`] wrapping a plain [`Read`] and [`Seek`].
pub struct InputStreamDataInput<R> {
  is: R,
}

impl<R> InputStreamDataInput<R> {
  pub fn new(is: R) -> Self {
    Self { is }
  }
}

impl<R> crate::core::util::close::Closeable for InputStreamDataInput<R> {}

impl<R: Read + Seek> DataInput for InputStreamDataInput<R> {
  fn read_byte(&mut self) -> Result<u8> {
    let mut b = [0u8; 1];
    let cnt = self.is.read(&mut b)?;
    if cnt == 0 {
      return Err(LuceneError::eof(""));
    }
    Ok(b[0])
  }

  fn read_bytes(&mut self, b: &mut [u8], mut offset: usize, mut len: usize) -> Result<()> {
    while len > 0 {
      let cnt = self.is.read(&mut b[offset..offset + len])?;
      if cnt == 0 {
        return Err(LuceneError::eof(""));
      }
      len -= cnt;
      offset += cnt;
    }
    Ok(())
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    GroupVIntUtil::read_group_vint_i32(self, dst, offset)
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    if num_bytes < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "numBytes must be >= 0, got {num_bytes}"
      )));
    }
    let skipped = self.is.seek(SeekFrom::Current(num_bytes))?;
    let current = skipped;
    let end = self.is.seek(SeekFrom::End(0))?;
    if current > end {
      self.is.seek(SeekFrom::Start(end))?;
      return Err(LuceneError::eof(""));
    }
    self.is.seek(SeekFrom::Start(current))?;
    Ok(())
  }
}

impl<R> Display for InputStreamDataInput<R> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}
