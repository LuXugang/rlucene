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
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use crc32fast::Hasher;

use crate::core::store::data_output::DataOutput;
use crate::core::store::{ByteBuffersDataOutput, DataInput, IndexOutput};
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub trait ByteBuffersIndexOutputOnClose: Send + Sync {
  fn on_close(&mut self, output: ByteBuffersDataOutput) -> Result<()>;
}

impl<F> ByteBuffersIndexOutputOnClose for F
where
  F: FnMut(ByteBuffersDataOutput) -> Result<()> + Send + Sync,
{
  fn on_close(&mut self, output: ByteBuffersDataOutput) -> Result<()> {
    self(output)
  }
}

pub struct NoopByteBuffersIndexOutputOnClose;

impl ByteBuffersIndexOutputOnClose for NoopByteBuffersIndexOutputOnClose {
  fn on_close(&mut self, _output: ByteBuffersDataOutput) -> Result<()> {
    Ok(())
  }
}

/// An [`IndexOutput`] writing to a [`ByteBuffersDataOutput`]
pub struct ByteBuffersIndexOutput<C = NoopByteBuffersIndexOutputOnClose>
where
  C: ByteBuffersIndexOutputOnClose,
{
  last_checksum_position: usize,
  last_checksum: u64,
  delegate: ByteBuffersDataOutput,
  on_close: C,
  name: String,
  resource_description: String,
  checksum: Hasher,
  closed: bool,
}

impl ByteBuffersIndexOutput<NoopByteBuffersIndexOutputOnClose> {
  pub fn with_checksum(
    delegate: ByteBuffersDataOutput,
    resource_description: &str,
    name: &str,
    checksum: Hasher,
  ) -> Self {
    Self {
      last_checksum_position: 0,
      last_checksum: 0,
      delegate,
      on_close: NoopByteBuffersIndexOutputOnClose,
      name: name.to_string(),
      resource_description: resource_description.to_string(),
      checksum,
      closed: false,
    }
  }

  pub fn new(delegate: ByteBuffersDataOutput, resource_description: &str, name: &str) -> Self {
    Self::with_checksum(delegate, resource_description, name, Hasher::new())
  }
}

impl<C> ByteBuffersIndexOutput<C>
where
  C: ByteBuffersIndexOutputOnClose,
{
  pub fn with_checksum_and_on_close(
    delegate: ByteBuffersDataOutput,
    resource_description: &str,
    name: &str,
    checksum: Hasher,
    on_close: C,
  ) -> Self
  where
    C: 'static,
  {
    Self {
      last_checksum_position: 0,
      last_checksum: 0,
      delegate,
      on_close,
      name: name.to_string(),
      resource_description: resource_description.to_string(),
      checksum,
      closed: false,
    }
  }

  pub fn get_array_copy(&self) -> Vec<u8> {
    if self.closed {
      Vec::new()
    } else {
      self.delegate.get_array_copy()
    }
  }

  fn ensure_open(&self) -> Result<()> {
    if self.closed {
      Err(LuceneError::already_closed("Already closed."))
    } else {
      Ok(())
    }
  }

  pub fn delegate(&self) -> Result<&ByteBuffersDataOutput> {
    self.ensure_open()?;
    Ok(&self.delegate)
  }

  pub fn delegate_mut(&mut self) -> Result<&mut ByteBuffersDataOutput> {
    self.ensure_open()?;
    Ok(&mut self.delegate)
  }
}

impl<C> DataOutput for ByteBuffersIndexOutput<C>
where
  C: ByteBuffersIndexOutputOnClose,
{
  fn write_byte(&mut self, b: u8) -> Result<()> {
    self.delegate_mut()?.write_byte(b)
  }

  fn write_bytes_with_len(&mut self, b: &[u8], len: usize) -> Result<()> {
    self.delegate_mut()?.write_bytes_with_len(b, len)
  }

  fn write_bytes_range(&mut self, b: &[u8], offset: usize, length: usize) -> Result<()> {
    self.delegate_mut()?.write_bytes_range(b, offset, length)
  }

  fn write_int(&mut self, i: i32) -> Result<()> {
    self.delegate_mut()?.write_int(i)
  }

  fn write_short(&mut self, i: i16) -> Result<()> {
    self.delegate_mut()?.write_short(i)
  }

  fn write_long(&mut self, i: i64) -> Result<()> {
    self.delegate_mut()?.write_long(i)
  }

  fn write_string(&mut self, s: &str) -> Result<()> {
    self.delegate_mut()?.write_string(s)
  }

  fn copy_bytes<I>(&mut self, input: &mut I, num_bytes: usize) -> Result<()>
  where
    I: DataInput + ?Sized,
  {
    self.delegate_mut()?.copy_bytes(input, num_bytes)
  }

  fn write_map_of_strings(&mut self, map: &HashMap<String, String>) -> Result<()> {
    self.delegate_mut()?.write_map_of_strings(map)
  }
}

impl<C> Display for ByteBuffersIndexOutput<C>
where
  C: ByteBuffersIndexOutputOnClose,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.resource_description)
  }
}

impl<C> Closeable for ByteBuffersIndexOutput<C>
where
  C: ByteBuffersIndexOutputOnClose,
{
  fn close(&mut self) -> Result<()> {
    if !self.closed {
      self.closed = true;
      let delegate = std::mem::take(&mut self.delegate);
      self.on_close.on_close(delegate)?;
    }
    Ok(())
  }
}
impl<C> Drop for ByteBuffersIndexOutput<C>
where
  C: ByteBuffersIndexOutputOnClose,
{
  fn drop(&mut self) {
    let _ = self.close();
  }
}

impl<C> IndexOutput for ByteBuffersIndexOutput<C>
where
  C: ByteBuffersIndexOutputOnClose,
{
  fn get_file_pointer(&self) -> Result<usize> {
    Ok(self.delegate()?.size())
  }

  fn get_checksum(&mut self) -> Result<u64> {
    let delegate_size = self.delegate()?.size();
    if self.last_checksum_position != delegate_size {
      self.last_checksum_position = delegate_size;
      self.checksum.reset();
      let mut checksum = self.checksum.clone();
      self.last_checksum = {
        let delegate = self.delegate()?;
        let (length, data) = delegate.to_buffer_list_ref();
        let mut remaining = length;
        for block in data {
          if remaining == 0 {
            break;
          }
          let block_length = remaining.min(block.get_ref().len());
          checksum.update(&block.get_ref()[..block_length]);
          remaining -= block_length;
        }
        checksum.finalize() as u64
      };
    }
    Ok(self.last_checksum)
  }

  fn get_name(&self) -> &str {
    self.name.as_str()
  }
}
