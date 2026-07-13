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
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::store::DataInput;
use crate::core::store::byte_buffers_data_input::{
  ByteBuffersDataInput, ByteBuffersDataInputBlock,
};
use crate::core::store::index_input::IndexInput;
use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// An [`IndexInput`] implementing [`RandomAccessInput`]
/// and backed by a [`ByteBuffersDataInput`].
pub type ByteBuffersIndexInputRef<'a> = ByteBuffersIndexInput<&'a [u8]>;
pub type ByteBuffersIndexInputOwned = ByteBuffersIndexInput<Vec<u8>>;

pub struct ByteBuffersIndexInput<B: ByteBuffersDataInputBlock> {
  in_: ByteBuffersDataInput<B>,
  resource_description: String,
  closed: AtomicBool,
}
impl<B> ByteBuffersIndexInput<B>
where
  B: ByteBuffersDataInputBlock,
{
  pub fn new(data_input: ByteBuffersDataInput<B>, resource_description: &str) -> Self {
    Self {
      in_: data_input,
      resource_description: resource_description.to_string(),
      closed: AtomicBool::new(false),
    }
  }

  fn ensure_open(&self) -> Result<()> {
    if self.closed.load(Ordering::Relaxed) {
      Err(LuceneError::already_closed("Already closed."))
    } else {
      Ok(())
    }
  }
}

impl<B> crate::core::util::close::CloseableRef for ByteBuffersIndexInput<B>
where
  B: ByteBuffersDataInputBlock,
{
  fn close(&self) -> Result<()> {
    self.closed.store(true, Ordering::Relaxed);
    Ok(())
  }
}

impl<B> DataInput for ByteBuffersIndexInput<B>
where
  B: ByteBuffersDataInputBlock + Clone,
{
  fn read_byte(&mut self) -> Result<u8> {
    self.ensure_open()?;
    DataInput::read_byte(&mut self.in_)
  }

  fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    DataInput::read_bytes(&mut self.in_, b, offset, len)
  }

  fn read_bytes_with_buffer(
    &mut self,
    b: &mut [u8],
    offset: usize,
    len: usize,
    _use_buffer: bool,
  ) -> Result<()> {
    self.ensure_open()?;
    self.in_.read_bytes_with_buffer(b, offset, len, false)
  }

  fn read_short(&mut self) -> Result<i16> {
    self.ensure_open()?;
    DataInput::read_short(&mut self.in_)
  }

  fn read_int(&mut self) -> Result<i32> {
    self.ensure_open()?;
    DataInput::read_int(&mut self.in_)
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    self.ensure_open()?;
    self.in_.read_group_vint(dst, offset)
  }

  fn read_vint(&mut self) -> Result<i32> {
    self.ensure_open()?;
    DataInput::read_vint(&mut self.in_)
  }

  fn read_zint(&mut self) -> Result<i32> {
    self.ensure_open()?;
    DataInput::read_zint(&mut self.in_)
  }

  fn read_long(&mut self) -> Result<i64> {
    self.ensure_open()?;
    DataInput::read_long(&mut self.in_)
  }

  fn read_longs(&mut self, dst: &mut [i64], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    self.in_.read_longs(dst, offset, len)
  }

  fn read_floats(&mut self, dst: &mut [f32], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    self.in_.read_floats(dst, offset, len)
  }

  fn read_vlong(&mut self) -> Result<i64> {
    self.ensure_open()?;
    self.in_.read_vlong()
  }

  fn read_zlong(&mut self) -> Result<i64> {
    self.ensure_open()?;
    self.in_.read_zlong()
  }

  fn read_string(&mut self) -> Result<String> {
    self.ensure_open()?;
    self.in_.read_string()
  }

  fn read_map_of_strings(&mut self) -> Result<HashMap<String, String>> {
    self.ensure_open()?;
    self.in_.read_map_of_strings()
  }

  fn read_set_of_strings(&mut self) -> Result<HashSet<String>> {
    self.ensure_open()?;
    self.in_.read_set_of_strings()
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    self.ensure_open()?;
    DataInput::skip_bytes(&mut self.in_, num_bytes)
  }

  fn is_index_input(&self) -> bool {
    true
  }

  fn seek_in_data_input(&mut self, _pos: usize) -> Result<()> {
    debug_assert!(self.is_index_input());
    IndexInput::seek(self, _pos)
  }

  fn get_file_pointer_in_data_input(&self) -> Result<usize> {
    debug_assert!(self.is_index_input());
    IndexInput::get_file_pointer(self)
  }
}
impl<B> RandomAccessInput for ByteBuffersIndexInput<B>
where
  B: ByteBuffersDataInputBlock,
{
  fn length(&self) -> Result<usize> {
    self.ensure_open()?;
    RandomAccessInput::length(&self.in_)
  }

  fn read_byte(&mut self, pos: usize) -> Result<u8> {
    self.ensure_open()?;
    RandomAccessInput::read_byte(&mut self.in_, pos)
  }

  fn read_bytes(&mut self, pos: usize, buf: &mut [u8], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    RandomAccessInput::read_bytes(&mut self.in_, pos, buf, offset, len)
  }

  fn read_short(&mut self, pos: usize) -> Result<i16> {
    self.ensure_open()?;
    RandomAccessInput::read_short(&mut self.in_, pos)
  }

  fn read_int(&mut self, pos: usize) -> Result<i32> {
    self.ensure_open()?;
    RandomAccessInput::read_int(&mut self.in_, pos)
  }

  fn read_long(&mut self, pos: usize) -> Result<i64> {
    self.ensure_open()?;
    RandomAccessInput::read_long(&mut self.in_, pos)
  }

  fn prefetch(&mut self, _pos: usize, _len: usize) -> Result<()> {
    self.ensure_open()?;
    Ok(())
  }
}

impl<B> Display for ByteBuffersIndexInput<B>
where
  B: ByteBuffersDataInputBlock,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.resource_description)
  }
}

impl<B> crate::core::util::clone::TryClone for ByteBuffersIndexInput<B>
where
  B: ByteBuffersDataInputBlock + Clone,
{
  fn try_clone(&self) -> Result<Self>
  where
    Self: Sized,
  {
    self.ensure_open()?;
    let slice = self.in_.slice(0, self.in_.length())?;
    let mut cloned = ByteBuffersIndexInput::new(slice, format!("(clone of) {self}").as_str());
    cloned.seek(self.get_file_pointer()?)?;
    Ok(cloned)
  }
}

impl<B> IndexInput for ByteBuffersIndexInput<B>
where
  B: ByteBuffersDataInputBlock + Clone,
{
  type IndexInput = ByteBuffersIndexInput<B>;

  fn get_file_pointer(&self) -> Result<usize> {
    self.ensure_open()?;
    self.in_.position()
  }

  fn seek(&mut self, pos: usize) -> Result<()> {
    self.ensure_open()?;
    self.in_.seek(pos)
  }

  fn length(&self) -> Result<usize> {
    self.ensure_open()?;
    Ok(self.in_.length())
  }

  fn slice(
    &self,
    slice_description: &str,
    offset: usize,
    length: usize,
  ) -> Result<Self::IndexInput> {
    self.ensure_open()?;
    Ok(ByteBuffersIndexInput::new(
      self.in_.slice(offset, length)?,
      slice_description,
    ))
  }

  type RandomAccessSlice = ByteBuffersIndexInput<B>;

  fn random_access_slice(&self, offset: usize, length: usize) -> Result<Self::RandomAccessSlice> {
    self.ensure_open()?;
    self.slice("", offset, length)
  }
}
