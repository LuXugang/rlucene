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
use std::fmt::{Display, Formatter};
use std::io::Cursor;
use std::rc::Rc;
use std::sync::Arc;

use byteorder::{ByteOrder, LE};

use crate::core::store::DataInput;
use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::util::accountable::Accountable;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::group_vint_util::{GroupVIntUtil, IntReader};
use crate::core::util::ram_usage_estimator::size_of_vec;
use crate::core::util::{SliceCopyOps, TryIntoInt};
pub type ByteBuffersDataInputRef<'a> = ByteBuffersDataInput<&'a [u8]>;
pub type ByteBuffersDataInputOwned = ByteBuffersDataInput<Vec<u8>>;

pub trait ByteBuffersDataInputBlock {
  fn as_slice(&self) -> &[u8];
}

impl ByteBuffersDataInputBlock for Vec<u8> {
  fn as_slice(&self) -> &[u8] {
    self
  }
}

impl ByteBuffersDataInputBlock for &[u8] {
  fn as_slice(&self) -> &[u8] {
    self
  }
}

impl ByteBuffersDataInputBlock for Rc<Vec<u8>> {
  fn as_slice(&self) -> &[u8] {
    self.as_ref()
  }
}

impl ByteBuffersDataInputBlock for Arc<Vec<u8>> {
  fn as_slice(&self) -> &[u8] {
    self.as_ref()
  }
}

/// A [`DataInput`] implementing [`RandomAccessInput`]
/// and reading data from a list of [`Cursor<Vec<u8>>`](Cursor).
pub struct ByteBuffersDataInput<B> {
  blocks: Vec<Cursor<B>>,
  block_mask: usize,
  block_bits: usize,
  length: usize,
  offset: usize,
  pos: usize,
}
/// Reads data from a set of contiguous buffers.
/// All data buffers except for the last one must have an identical number of
/// remaining bytes (which must be a power of two). The last buffer can have an
/// arbitrary remaining length.
impl<B: ByteBuffersDataInputBlock> ByteBuffersDataInput<B> {
  pub fn new(blocks: Vec<Cursor<B>>, length: usize) -> Result<Self> {
    let (block_bits, block_mask) = if blocks.len() <= 1 {
      (32, !0)
    } else {
      let block_bytes = blocks[0].get_ref().as_slice().len();
      let block_bits = block_bytes.trailing_zeros() as usize;
      (block_bits, (1 << block_bits) - 1)
    };
    // The initial "position" of this stream is shifted by the position of
    // the first block.
    let offset = blocks
      .first()
      .map_or(0, |block| block.position())
      .try_convert()?;
    Ok(Self {
      blocks,
      block_mask,
      block_bits,
      length,
      offset,
      pos: offset,
    })
  }
  fn block_index(&self, pos: usize) -> usize {
    pos >> self.block_bits
  }
  fn block_offset(&self, pos: usize) -> usize {
    pos & (self.block_mask)
  }
  fn read_buffer<T, C>(
    &self,
    mut pos: usize,
    len: usize,
    output: &mut [T],
    type_size: usize,
    converter: C,
  ) -> Result<()>
  where
    C: Fn(&[u8]) -> T,
  {
    let mut bytes_read = len * type_size;
    // TODO: use This bytes would made additional data copy
    // TODO: we should convert directly from block
    let mut bytes = vec![0; bytes_read];
    let mut bytes_offset = 0;
    while bytes_read > 0 {
      let block_index = self.block_index(pos);
      let block_offset = self.block_offset(pos);

      if block_index >= self.blocks.len() || pos + bytes_read > self.length + self.offset {
        return Err(LuceneError::eof(format!("{pos}")));
      }

      let block = self.blocks.get(block_index).ok_or_else(|| {
        LuceneError::eof(format!("missing block {block_index} at position {pos}"))
      })?;
      let block_bytes = block.get_ref().as_slice();
      let available = block_bytes.len().saturating_sub(block_offset);

      let chunk = bytes_read.min(available);
      if block_offset > block_bytes.len() || block_offset + chunk > block_bytes.len() {
        return Err(LuceneError::illegal_argument(format!(
          "Read out of bounds: position={}, len={}, total={}",
          block_offset,
          chunk,
          block_bytes.len()
        )));
      }
      if bytes_offset + chunk > bytes.len() {
        return Err(LuceneError::illegal_argument(format!(
          "Destination buffer out of bounds: offset={}, len={}, total={}",
          bytes_offset,
          chunk,
          bytes.len()
        )));
      }
      bytes[bytes_offset..bytes_offset + chunk]
        .copy_from_slice(&block_bytes[block_offset..block_offset + chunk]);
      bytes_offset += chunk;
      pos += chunk;
      bytes_read -= chunk;
    }

    debug_assert!(bytes.len().is_multiple_of(type_size));
    if type_size == 1 {
      let output_bytes =
        unsafe { std::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u8, output.len()) };
      output_bytes.copy_from(&bytes, 0);
    } else {
      output
        .iter_mut()
        .zip(bytes.chunks_exact(type_size).map(converter))
        .for_each(|(out, value)| *out = value);
    }

    Ok(())
  }
  fn do_read_longs(&self, pos: usize, len: usize, output: &mut [i64]) -> Result<()> {
    self.read_buffer(pos, len, output, BitUtil::LONG_BYTES, LE::read_i64)
  }
  fn do_read_bytes(&self, mut pos: usize, mut len: usize, output: &mut [u8]) -> Result<()> {
    let mut output_offset = 0;
    while len > 0 {
      let block_index = self.block_index(pos);
      let block_offset = self.block_offset(pos);

      if block_index >= self.blocks.len() || pos + len > self.length + self.offset {
        return Err(LuceneError::eof(format!("{pos}")));
      }

      let block = self.blocks.get(block_index).ok_or_else(|| {
        LuceneError::eof(format!("missing block {block_index} at position {pos}"))
      })?;
      let block_bytes = block.get_ref().as_slice();
      let chunk = len.min(block_bytes.len().saturating_sub(block_offset));
      if chunk == 0 {
        return Err(LuceneError::eof(format!("{pos}")));
      }

      output[output_offset..output_offset + chunk]
        .copy_from_slice(&block_bytes[block_offset..block_offset + chunk]);

      pos += chunk;
      len -= chunk;
      output_offset += chunk;
    }
    Ok(())
  }
  fn do_read_floats(&self, pos: usize, len: usize, output: &mut [f32]) -> Result<()> {
    self.read_buffer(pos, len, output, BitUtil::FLOAT_BYTES, LE::read_f32)
  }

  pub fn seek(&mut self, position: usize) -> Result<()> {
    self.pos = position + self.offset;
    if position > self.length() {
      self.pos = self.length;
      return Err(LuceneError::eof(format!("{}", self.pos)));
    }
    Ok(())
  }
}

impl<B> ByteBuffersDataInput<B> {
  pub fn position(&self) -> Result<usize> {
    self.pos.checked_sub(self.offset).ok_or_else(|| {
      LuceneError::illegal_state(format!(
        "underflow, pos {} offset {}",
        self.pos, self.offset
      ))
    })
  }

  pub fn length(&self) -> usize {
    self.length
  }
}
impl<B> ByteBuffersDataInput<B>
where
  B: ByteBuffersDataInputBlock + Clone,
{
  pub fn slice(&self, offset: usize, length: usize) -> Result<ByteBuffersDataInput<B>> {
    if offset + length > self.length {
      return Err(LuceneError::illegal_argument(format!(
        "slice(offset={}, length={}) is out of bounds: {}",
        offset, length, self.length
      )));
    }
    let blocks = Self::slice_buffer_list(&self.blocks, offset, length);
    Self::new(blocks, length)
  }
  pub fn slice_buffer_list(blocks: &[Cursor<B>], offset: usize, length: usize) -> Vec<Cursor<B>> {
    debug_assert!(!blocks.is_empty(), "blocks cannot be empty");

    if blocks.len() == 1 {
      let mut new_cursor = Cursor::new(blocks[0].get_ref().clone());
      new_cursor.set_position(blocks[0].position() + offset as u64);
      return vec![new_cursor];
    }

    let abs_start = blocks[0].position() + offset as u64;
    let abs_end = abs_start + length as u64;

    let block_bytes = blocks[0].get_ref().as_slice().len() as u64;
    debug_assert!(block_bytes.is_power_of_two());
    let block_bits = block_bytes.trailing_zeros() as u64;
    let block_mask = (1u64 << block_bits) - 1;

    let start_block_index = (abs_start / block_bytes) as usize;
    let end_block_index = ((abs_end / block_bytes) as usize).min(blocks.len() - 1);

    // Create a new Cursor for each block and adjust the position and underlying
    // data range as needed
    blocks[start_block_index..=end_block_index]
      .iter()
      .enumerate()
      .map(|(i, block)| {
        let vec_data = block.get_ref().clone();

        let mut new_cursor = Cursor::new(vec_data);
        if i == 0 {
          // first block we need to set position to start_offset to keep
          // al blocks same length
          let block_offset = abs_start & block_mask;
          new_cursor.set_position(block_offset);
        } else {
          // other blocks we can use full block, so we only need set
          // position to 0
          new_cursor.set_position(0);
        }
        new_cursor
      })
      .collect()
  }
}

impl<B> Display for ByteBuffersDataInput<B> {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    let blocks_len = self.blocks.len();
    let offset_str = if self.offset == 0 {
      String::new()
    } else {
      format!(" [offset: {}]", self.offset)
    };
    let v = match self.position() {
      Ok(p) => p.to_string(),
      Err(_) => "ERR".to_string(),
    };
    write!(
      f,
      "{} bytes, block size: {}, blocks: {}, position: {}{}",
      self.length,
      1u64 << self.block_bits,
      blocks_len,
      v,
      offset_str
    )
  }
}

impl<B> crate::core::util::close::Closeable for ByteBuffersDataInput<B> {}

impl<B> DataInput for ByteBuffersDataInput<B>
where
  B: ByteBuffersDataInputBlock,
{
  fn read_byte(&mut self) -> Result<u8> {
    if self.pos >= self.length + self.offset {
      return Err(LuceneError::eof(format!("{}", self.pos)));
    }
    let block_index = self.block_index(self.pos);
    let block_offset = self.block_offset(self.pos);
    let block = self
      .blocks
      .get(block_index)
      .ok_or_else(|| LuceneError::eof(format!("{}", self.pos)))?;
    let value = *block
      .get_ref()
      .as_slice()
      .get(block_offset)
      .ok_or_else(|| LuceneError::eof(format!("{}", self.pos)))?;
    self.pos += 1;
    Ok(value)
  }
  fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    self.do_read_bytes(self.pos, len, &mut b[offset..(offset + len)])?;
    self.pos += len;
    Ok(())
  }

  fn read_short(&mut self) -> Result<i16> {
    let block_offset = self.block_offset(self.pos);
    let value = if self.pos + BitUtil::SHORT_BYTES <= self.length + self.offset
      && block_offset + BitUtil::SHORT_BYTES <= self.block_mask
    {
      let block = self
        .blocks
        .get(self.block_index(self.pos))
        .ok_or_else(|| LuceneError::eof(format!("{}", self.pos)))?;
      let block_bytes = block.get_ref().as_slice();
      if block_offset + BitUtil::SHORT_BYTES <= block_bytes.len() {
        LE::read_i16(&block_bytes[block_offset..])
      } else {
        let mut bytes = [0; BitUtil::SHORT_BYTES];
        self.do_read_bytes(self.pos, BitUtil::SHORT_BYTES, &mut bytes)?;
        LE::read_i16(&bytes)
      }
    } else {
      let mut bytes = [0; BitUtil::SHORT_BYTES];
      self.do_read_bytes(self.pos, BitUtil::SHORT_BYTES, &mut bytes)?;
      LE::read_i16(&bytes)
    };
    self.pos += BitUtil::SHORT_BYTES;
    Ok(value)
  }

  fn read_int(&mut self) -> Result<i32> {
    let block_offset = self.block_offset(self.pos);
    let value = if self.pos + BitUtil::INT_BYTES <= self.length + self.offset
      && block_offset + BitUtil::INT_BYTES <= self.block_mask
    {
      let block = self
        .blocks
        .get(self.block_index(self.pos))
        .ok_or_else(|| LuceneError::eof(format!("{}", self.pos)))?;
      let block_bytes = block.get_ref().as_slice();
      if block_offset + BitUtil::INT_BYTES <= block_bytes.len() {
        LE::read_i32(&block_bytes[block_offset..])
      } else {
        let mut bytes = [0; BitUtil::INT_BYTES];
        self.do_read_bytes(self.pos, BitUtil::INT_BYTES, &mut bytes)?;
        LE::read_i32(&bytes)
      }
    } else {
      let mut bytes = [0; BitUtil::INT_BYTES];
      self.do_read_bytes(self.pos, BitUtil::INT_BYTES, &mut bytes)?;
      LE::read_i32(&bytes)
    };
    self.pos += BitUtil::INT_BYTES;
    Ok(value)
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    let block_index = self.block_index(self.pos);
    let block_offset = self.block_offset(self.pos);
    let block = self
      .blocks
      .get(block_index)
      .ok_or_else(|| LuceneError::eof(format!("{}", self.pos)))?;
    let block_remain = block
      .get_ref()
      .as_slice()
      .len()
      .saturating_sub(block_offset);
    let input_remain = (self.length + self.offset).saturating_sub(self.pos);
    let remain = block_remain.min(input_remain);
    let pos = self.position()?;
    let len =
      GroupVIntUtil::read_group_vint_i32_with_reader(self, remain as u64, pos, dst, offset)?;
    self.pos += len;
    Ok(())
  }
  fn read_long(&mut self) -> Result<i64> {
    let block_offset = self.block_offset(self.pos);
    let value = if self.pos + BitUtil::LONG_BYTES <= self.length + self.offset
      && block_offset + BitUtil::LONG_BYTES <= self.block_mask
    {
      let block = self
        .blocks
        .get(self.block_index(self.pos))
        .ok_or_else(|| LuceneError::eof(format!("{}", self.pos)))?;
      let block_bytes = block.get_ref().as_slice();
      if block_offset + BitUtil::LONG_BYTES <= block_bytes.len() {
        LE::read_i64(&block_bytes[block_offset..])
      } else {
        let mut bytes = [0; BitUtil::LONG_BYTES];
        self.do_read_bytes(self.pos, BitUtil::LONG_BYTES, &mut bytes)?;
        LE::read_i64(&bytes)
      }
    } else {
      let mut bytes = [0; BitUtil::LONG_BYTES];
      self.do_read_bytes(self.pos, BitUtil::LONG_BYTES, &mut bytes)?;
      LE::read_i64(&bytes)
    };
    self.pos += BitUtil::LONG_BYTES;
    Ok(value)
  }

  fn read_longs(&mut self, dst: &mut [i64], offset: usize, len: usize) -> Result<()> {
    self.do_read_longs(self.pos, len, &mut dst[offset..(offset + len)])?;
    self.pos += len * BitUtil::LONG_BYTES;
    Ok(())
  }

  fn read_floats(&mut self, dst: &mut [f32], offset: usize, len: usize) -> Result<()> {
    self.do_read_floats(self.pos, len, &mut dst[offset..(offset + len)])?;
    self.pos += len * BitUtil::FLOAT_BYTES;
    Ok(())
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    if num_bytes < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "num_bytes must be >= 0, got {num_bytes}"
      )));
    }
    let num_bytes: usize = num_bytes.try_convert()?;
    let skip_to = self.position()? + num_bytes;
    self.seek(skip_to)
  }
}
// TODO: In the current implementation, after performing a random read of a
// specific value, it is not possible to use sequential reads to access the next
// value at the subsequent position. TODO: should we support this feature?
impl<B> RandomAccessInput for ByteBuffersDataInput<B>
where
  B: ByteBuffersDataInputBlock,
{
  fn length(&self) -> Result<usize> {
    Ok(self.length)
  }

  fn read_byte(&mut self, pos: usize) -> Result<u8> {
    let pos = pos + self.offset;
    if pos >= self.length + self.offset {
      return Err(LuceneError::eof(format!("{pos}")));
    }
    let block = self
      .blocks
      .get(self.block_index(pos))
      .ok_or_else(|| LuceneError::eof(format!("{pos}")))?;
    block
      .get_ref()
      .as_slice()
      .get(self.block_offset(pos))
      .copied()
      .ok_or_else(|| LuceneError::eof(format!("{pos}")))
  }

  fn read_bytes(&mut self, pos: usize, buf: &mut [u8], offset: usize, len: usize) -> Result<()> {
    let pos = pos + self.offset;
    self.do_read_bytes(pos, len, &mut buf[offset..offset + len])
  }

  fn read_short(&mut self, pos: usize) -> Result<i16> {
    let pos = pos + self.offset;
    let block_offset = self.block_offset(pos);
    if pos + BitUtil::SHORT_BYTES <= self.length + self.offset
      && block_offset + BitUtil::SHORT_BYTES <= self.block_mask
    {
      let block = self
        .blocks
        .get(self.block_index(pos))
        .ok_or_else(|| LuceneError::eof(format!("{pos}")))?;
      let block_bytes = block.get_ref().as_slice();
      if block_offset + BitUtil::SHORT_BYTES <= block_bytes.len() {
        return Ok(LE::read_i16(&block_bytes[block_offset..]));
      }
    }
    let mut bytes = [0; BitUtil::SHORT_BYTES];
    self.do_read_bytes(pos, BitUtil::SHORT_BYTES, &mut bytes)?;
    Ok(LE::read_i16(&bytes))
  }

  fn read_int(&mut self, pos: usize) -> Result<i32> {
    let pos = pos + self.offset;
    let block_offset = self.block_offset(pos);
    if pos + BitUtil::INT_BYTES <= self.length + self.offset
      && block_offset + BitUtil::INT_BYTES <= self.block_mask
    {
      let block = self
        .blocks
        .get(self.block_index(pos))
        .ok_or_else(|| LuceneError::eof(format!("{pos}")))?;
      let block_bytes = block.get_ref().as_slice();
      if block_offset + BitUtil::INT_BYTES <= block_bytes.len() {
        return Ok(LE::read_i32(&block_bytes[block_offset..]));
      }
    }
    let mut bytes = [0; BitUtil::INT_BYTES];
    self.do_read_bytes(pos, BitUtil::INT_BYTES, &mut bytes)?;
    Ok(LE::read_i32(&bytes))
  }

  fn read_long(&mut self, pos: usize) -> Result<i64> {
    let pos = pos + self.offset;
    let block_offset = self.block_offset(pos);
    if pos + BitUtil::LONG_BYTES <= self.length + self.offset
      && block_offset + BitUtil::LONG_BYTES <= self.block_mask
    {
      let block = self
        .blocks
        .get(self.block_index(pos))
        .ok_or_else(|| LuceneError::eof(format!("{pos}")))?;
      let block_bytes = block.get_ref().as_slice();
      if block_offset + BitUtil::LONG_BYTES <= block_bytes.len() {
        return Ok(LE::read_i64(&block_bytes[block_offset..]));
      }
    }
    let mut bytes = [0; BitUtil::LONG_BYTES];
    self.do_read_bytes(pos, BitUtil::LONG_BYTES, &mut bytes)?;
    Ok(LE::read_i64(&bytes))
  }

  fn prefetch(&mut self, _pos: usize, _len: usize) -> Result<()> {
    Ok(())
  }
}

impl<B> IntReader for ByteBuffersDataInput<B>
where
  B: ByteBuffersDataInputBlock,
{
  fn read(&mut self, pos: usize) -> Result<i32> {
    RandomAccessInput::read_int(self, pos)
  }
}

impl Accountable for ByteBuffersDataInput<Vec<u8>> {
  fn ram_bytes_used(&self) -> Result<i64> {
    let mut size = size_of_vec(&self.blocks);
    for block in &self.blocks {
      size = size.saturating_add(size_of_vec(block.get_ref()));
    }
    Ok(size)
  }
}

impl Accountable for ByteBuffersDataInput<&[u8]> {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(size_of_vec(&self.blocks))
  }
}

impl Accountable for ByteBuffersDataInput<Rc<Vec<u8>>> {
  fn ram_bytes_used(&self) -> Result<i64> {
    let mut size = size_of_vec(&self.blocks);
    for block in &self.blocks {
      size = size
        .saturating_add(std::mem::size_of_val(block.get_ref().as_ref()) as i64)
        .saturating_add(size_of_vec(block.get_ref().as_ref()));
    }
    Ok(size)
  }
}

impl Accountable for ByteBuffersDataInput<Arc<Vec<u8>>> {
  fn ram_bytes_used(&self) -> Result<i64> {
    let mut size = size_of_vec(&self.blocks);
    for block in &self.blocks {
      size = size
        .saturating_add(std::mem::size_of_val(block.get_ref().as_ref()) as i64)
        .saturating_add(size_of_vec(block.get_ref().as_ref()));
    }
    Ok(size)
  }
}
