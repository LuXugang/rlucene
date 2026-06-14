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
use std::collections::VecDeque;
use std::io::{Cursor, Seek};

use byteorder::WriteBytesExt;

use crate::core::store::DataInput;
use crate::core::store::byte_buffers_data_input::{
  ByteBuffersDataInput, ByteBuffersDataInputOwned,
};
use crate::core::store::data_output::DataOutput;
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{ReadableCursorExt, TryIntoInt, WritableCursorExt};

/// A [`DataOutput`] storing data in a list of [`Cursor<Vec<u8>>`](Cursor).
pub struct ByteBuffersDataOutput {
  //In Rust Lucene, all data within each block is considered valid.
  // However, in Java Lucene, the valid data range can be controlled
  // by the `limit` parameter of the `java.nio.ByteBuffer` encapsulation.
  blocks: VecDeque<Cursor<Vec<u8>>>,
  max_bits_per_block: i32,
  block_bits: i32,
  ram_bytes_used: i64,
  // it is necessary when we want to reuse the data output
  current_block_index: usize,
  reuse: bool,
}

impl Default for ByteBuffersDataOutput {
  fn default() -> Self {
    Self::new()
  }
}

impl ByteBuffersDataOutput {
  /// Smallest `minBitsPerBlock` allowed
  pub const LIMIT_MIN_BITS_PER_BLOCK: i32 = 1;
  /// Largest `maxBitsPerBlock` allowed
  pub const LIMIT_MAX_BITS_PER_BLOCK: i32 = 31;
  ///Maximum number of blocks at the current `blockBits` block size before we
  /// increase the block size (and thus decrease the number of blocks).
  pub const MAX_BLOCKS_BEFORE_BLOCK_EXPANSION: i32 = 100;
  ///Default `maxBitsPerBlock`
  pub const DEFAULT_MAX_BITS_PER_BLOCK: i32 = 26;
  /// Default `minBitsPerBlock`
  pub const DEFAULT_MIN_BITS_PER_BLOCK: i32 = 10;

  pub fn new() -> Self {
    let result = Self::with_reuse(
      Self::DEFAULT_MIN_BITS_PER_BLOCK,
      Self::DEFAULT_MAX_BITS_PER_BLOCK,
      false,
    );
    debug_assert!(result.is_ok());
    result.unwrap()
  }
  ///Creates a new output with all defaults.
  pub fn new_resettable_instance() -> Self {
    let result = Self::with_reuse(
      Self::DEFAULT_MIN_BITS_PER_BLOCK,
      Self::DEFAULT_MAX_BITS_PER_BLOCK,
      true,
    );
    debug_assert!(result.is_ok());
    result.unwrap()
  }
  /// Expert: Creates a new output with custom parameters.
  ///
  /// # Arguments
  /// * `min_bits_per_block` - Minimum bits per block.
  /// * `max_bits_per_block` - Maximum bits per block.
  /// * `reuse` - Reuse this Instance.
  pub fn with_reuse(min_bits_per_block: i32, max_bits_per_block: i32, reuse: bool) -> Result<Self> {
    if min_bits_per_block < Self::LIMIT_MIN_BITS_PER_BLOCK {
      return Err(LuceneError::illegal_argument(format!(
        "minBitsPerBlock ({}) too small, must be at least {}",
        min_bits_per_block,
        Self::LIMIT_MIN_BITS_PER_BLOCK
      )));
    }
    if max_bits_per_block > Self::LIMIT_MAX_BITS_PER_BLOCK {
      return Err(LuceneError::illegal_argument(format!(
        "maxBitsPerBlock ({}) too large, must not exceed {}",
        max_bits_per_block,
        Self::LIMIT_MAX_BITS_PER_BLOCK
      )));
    }
    if min_bits_per_block > max_bits_per_block {
      return Err(LuceneError::illegal_argument(format!(
        "minBitsPerBlock ({min_bits_per_block}) cannot exceed maxBitsPerBlock ({max_bits_per_block})"
      )));
    }
    let block = Cursor::new(vec![0u8; 1 << min_bits_per_block]);
    let mut blocks = VecDeque::new();
    blocks.push_back(block);
    Ok(Self {
      max_bits_per_block,
      block_bits: min_bits_per_block,
      blocks,
      ram_bytes_used: 0,
      current_block_index: 0,
      reuse,
    })
  }
  /// Creates a new output, suitable for writing a file of approximately
  /// `expected_size` bytes.
  ///
  /// Memory allocation will be optimized based on the `expected_size` hint to
  /// reduce overhead for larger files.
  ///
  /// # Arguments
  /// * `expected_size` - Estimated size of the output file.
  pub fn with_size(expected_size: i64) -> Result<Self> {
    let block_bits = compute_block_size_bits_for(expected_size);
    Self::with_reuse(block_bits, Self::DEFAULT_MAX_BITS_PER_BLOCK, false)
  }

  fn append_block(&mut self) -> Result<()> {
    if self.blocks.len() > Self::MAX_BLOCKS_BEFORE_BLOCK_EXPANSION as usize
      && self.block_bits < self.max_bits_per_block
    {
      self.rewrite_to_block_size(self.block_bits + 1)?;
      if self
        .blocks
        .get_mut(self.current_block_index)
        .unwrap()
        .remain()?
        > 0
      {
        return Ok(());
      }
    }
    let required_block_size = 1 << self.block_bits;
    self
      .blocks
      .push_back(Cursor::new(vec![0u8; required_block_size]));
    // TODO: self.ramBytesUsed += 0;
    self.ram_bytes_used += 0;
    self.current_block_index += 1;
    Ok(())
  }
  fn rewrite_to_block_size(&mut self, target_block_bits: i32) -> Result<()> {
    debug_assert!(target_block_bits <= self.max_bits_per_block);
    self.rewrite_blocks(target_block_bits)?;
    // TODO:
    self.ram_bytes_used += 0;
    Ok(())
  }
  // create larger blocks and copy data from smaller blocks
  // TODO: the first old_block's data could be reused ,first do expansion by
  // `push_back` and then move to tail and continue copy the second
  // old_block's data to it
  pub fn rewrite_blocks(&mut self, target_block_bits: i32) -> Result<()> {
    debug_assert!(target_block_bits > self.block_bits);
    self.block_bits = target_block_bits;
    let block_size = 1 << self.block_bits;
    let mut new_block = Cursor::new(vec![0; block_size]);
    let mut old_block_count = self.blocks.len();
    while let Some(mut old_block) = self.blocks.pop_front() {
      // read from head
      old_block.set_position(0);
      while old_block.remain()? > 0 {
        let mut available_space = new_block.remain()?;
        if available_space == 0 {
          self.blocks.push_back(new_block);
          new_block = Cursor::new(vec![0; block_size]);
          available_space = 1 << self.block_bits;
        }
        let bytes_to_copy = available_space.min(old_block.remain()?) as usize;
        let old_position = old_block.position() as usize;
        let old_data = &old_block.get_ref()[old_position..old_position + bytes_to_copy];
        debug_assert!(
          new_block.remain()? as usize >= bytes_to_copy,
          "Insufficient space in new_block: remaining={}, required={}",
          new_block.remain()?,
          bytes_to_copy
        );
        new_block.write_from_slice(old_data).unwrap();
        old_block.set_position((old_position + bytes_to_copy) as u64);
      }
      old_block_count -= 1;
      if old_block_count == 0 {
        break;
      }
    }
    if new_block.position() > 0 {
      self.blocks.push_back(new_block);
    }
    self.current_block_index = self.blocks.len() - 1;
    Ok(())
  }
  /// Copies the current content of this object into another [`DataOutput`].
  pub(crate) fn copy_to<DA>(&self, output: &mut DA) -> Result<()>
  where
    DA: DataOutput,
  {
    debug_assert!(!self.blocks.is_empty());
    for (index, block) in self.blocks.iter().enumerate() {
      if index == self.current_block_index {
        let end = block.position() as usize;
        output.write_bytes_range(block.get_ref(), 0, end)?;
      } else {
        // this block is reused
        if block.position() == 0 {
          continue;
        }
        let len = block.get_ref().len();
        debug_assert!(len == 1 << self.block_bits);
        output.write_bytes_with_len(block.get_ref(), len)?;
      }
    }
    Ok(())
  }
  /// The number of bytes written to this output so far.
  pub fn size(&self) -> usize {
    let mut size = 0;
    let block_count = self.current_block_index + 1;
    if block_count >= 1 {
      let full_block_size = (block_count - 1) * self.block_size();
      let last_block_size = self
        .blocks
        .get(self.current_block_index)
        .unwrap()
        .position();
      size = full_block_size + last_block_size as usize;
    }
    size
  }
  fn block_size(&self) -> usize {
    1 << self.block_bits
  }
  /// Resets this object to a clean (zero-size) state and publishes any
  /// currently allocated buffers for reuse according to the reuse
  /// strategy provided when this value was created.
  ///
  /// # Warning
  /// Sharing byte buffers for reads and writes is dangerous and may lead to
  /// hard-to-debug issues. Use with great caution.
  pub fn reset(&mut self) {
    if self.reuse {
      for block in &mut self.blocks {
        let _ = block.rewind();
      }
    }
    self.current_block_index = 0;
    self.ram_bytes_used = 0;
  }

  /// Returns a list of read-only views of [`Cursor<Vec<u8>>`](Cursor) blocks
  /// over the current content written to the output.
  pub fn to_buffer_list_ref(&self) -> (usize, Vec<Cursor<&[u8]>>) {
    let data = self
      .blocks
      .iter()
      .map(|cursor| {
        let slice: &[u8] = cursor.get_ref().as_slice();
        let mut new_cursor = Cursor::new(slice);
        new_cursor.set_position(0);
        new_cursor
      })
      .collect();
    (self.size(), data)
  }
  /// Moves the blocks out of the current object, transferring ownership.
  /// # Parameters
  /// - `init_blocks`: If init_blocks is true, then after taking ownership of blocks, we pre-allocate the space so it can be reused.
  pub fn to_buffer_list_owner(&mut self, init_blocks: bool) -> (usize, Vec<Cursor<Vec<u8>>>) {
    let size = self.size();

    let old_blocks = {
      if init_blocks {
        let cap = self.blocks.capacity();
        let len = self.blocks.len();

        let mut new_blocks = VecDeque::with_capacity(cap);
        for _ in 0..len {
          new_blocks.push_back(Cursor::new(vec![0u8; 1 << self.block_bits]));
        }
        std::mem::replace(&mut self.blocks, new_blocks)
      } else {
        std::mem::take(&mut self.blocks)
      }
    };

    let data = old_blocks
      .into_iter()
      .map(|mut cursor| {
        cursor.set_position(0);
        cursor
      })
      .collect();

    (size, data)
  }
  pub fn get_writeable_buffer_list(&mut self) -> Vec<&mut Cursor<Vec<u8>>> {
    todo!()
  }
  /// Returns a contiguous array containing the current content written to the
  /// output. The returned array is always a copy and can be safely
  /// mutated. # Note
  /// If reset is called immediately after get_array_copy,
  /// or if ByteBuffersDataOutput will no longer be used,
  /// then [`try_get_array_ownership`](Self::try_get_array_ownership) should
  /// be used instead. If the number of blocks is 1, we take ownership to
  /// avoid copying. See
  /// [`try_get_array_ownership`](Self::try_get_array_ownership)
  pub fn get_array_copy(&self) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(self.size());
    for block in &self.blocks {
      let end = block.position() as usize;
      buffer.extend_from_slice(&block.get_ref()[..end]);
    }
    buffer
  }
  /// See [`get_array_copy`](Self::get_array_copy) Before use this method.
  pub fn try_get_array_ownership(&mut self) -> Vec<u8> {
    match self.blocks.len() {
      0 => vec![0u8; 1 << self.block_bits],
      // If the number of blocks is 1, take ownership to avoid copying.
      1 => {
        let cursor = self.blocks.front_mut().unwrap();
        let end = cursor.position() as usize;

        let old_vec = std::mem::replace(cursor.get_mut(), vec![0u8; 1 << self.block_bits]);

        old_vec.into_iter().take(end).collect()
      },
      _ => {
        let mut buffer = Vec::with_capacity(self.size());
        for block in &self.blocks {
          let end = block.position() as usize;
          buffer.extend_from_slice(&block.get_ref()[..end]);
        }
        buffer
      },
    }
  }

  /// Returns a `ByteBuffersDataInput` backed by references to internal
  /// buffers.
  ///
  /// This method borrows the internal buffer data as `&[u8]`,
  /// and constructs a read-only view over the current written content.
  ///
  /// The returned input is only valid as long as `self` is not mutated.
  pub fn get_data_input_ref(&mut self) -> Result<ByteBuffersDataInput<&[u8]>> {
    let (length, data) = self.to_buffer_list_ref();
    ByteBuffersDataInput::new(data, length)
  }

  /// Returns a `ByteBuffersDataInput` that owns its internal buffers.
  ///
  /// This method consumes the written buffer content into owned `[u8]`
  /// vectors, and constructs a self-contained input stream that can
  /// outlive `self`.
  ///
  /// Use this when the data needs to be retained or passed independently.
  /// # Parameters
  /// - `init_blocks`: If init_blocks is true, then after taking ownership of blocks, we pre-allocate the space so it can be reused.
  pub fn get_data_input_owner(&mut self, init_blocks: bool) -> Result<ByteBuffersDataInputOwned> {
    let (length, data) = self.to_buffer_list_owner(init_blocks);
    ByteBuffersDataInput::new(data, length)
  }

  fn append_block_if_needed(&mut self) -> Result<usize> {
    let mut last_block = self.blocks.get_mut(self.current_block_index).unwrap();
    if last_block.remain()? == 0 {
      if self.reuse && self.current_block_index < self.blocks.len() - 1 {
        self.current_block_index += 1;
        last_block = self.blocks.get_mut(self.current_block_index).unwrap();
      } else {
        self.append_block()?;
        // it is safe to get by `back_mut` because blocks are not reused
        last_block = self.blocks.back_mut().unwrap();
      }
    }
    last_block.remain()
  }
  #[cfg(debug_assertions)]
  pub fn write_bytes(&mut self, b: &[u8]) -> Result<()> {
    debug_assert!(b.len() <= u32::MAX as usize);
    self.write_bytes_range(b, 0, b.len())
  }

  #[cfg(debug_assertions)]
  pub fn write_byte(&mut self, b: u8) -> Result<()> {
    self.write_bytes_range(&[b], 0, 1)
  }
}

impl DataOutput for ByteBuffersDataOutput {
  fn write_byte(&mut self, b: u8) -> Result<()> {
    self.append_block_if_needed()?;
    let last_block = self.blocks.get_mut(self.current_block_index).unwrap();
    Ok(last_block.write_u8(b)?)
  }

  fn write_bytes_with_len(&mut self, b: &[u8], len: usize) -> Result<()> {
    self.write_bytes_range(b, 0, len)
  }

  fn write_bytes_range(&mut self, b: &[u8], mut offset: usize, mut length: usize) -> Result<()> {
    while length > 0 {
      let available_space = self.append_block_if_needed()?;
      let last_block = self.blocks.get_mut(self.current_block_index).unwrap();
      let chunk = available_space.min(length);
      last_block.write_from(b, offset, chunk)?;
      length = length.checked_sub(chunk).ok_or_else(|| {
        LuceneError::illegal_state(format!("underflow, length {}, chunk {} ", length, chunk))
      })?;
      offset += chunk;
    }
    Ok(())
  }

  fn write_int(&mut self, i: i32) -> Result<()> {
    let value = i.to_le_bytes();
    self.write_bytes_range(&value, 0, 4)
  }

  fn write_short(&mut self, i: i16) -> Result<()> {
    let value = i.to_le_bytes();
    self.write_bytes_range(&value, 0, 2)
  }

  fn write_long(&mut self, i: i64) -> Result<()> {
    let value = i.to_le_bytes();
    self.write_bytes_range(&value, 0, 8)
  }

  fn write_string(&mut self, s: &str) -> Result<()> {
    let bytes = s.as_bytes();
    let length = bytes.len();
    self.write_vint(length.try_convert()?)?;
    self.write_bytes_range(bytes, 0, length)
  }

  fn copy_bytes(&mut self, input: &mut impl DataInput, mut num_bytes: usize) -> Result<()> {
    while num_bytes > 0 {
      let available_space = self.append_block_if_needed()?;
      let last_block = self.blocks.get_mut(self.current_block_index).unwrap();
      let bytes_to_copy = available_space.min(num_bytes);

      let current_pos = last_block.position().try_convert()?;
      let current_block_mut = last_block.get_mut();
      input.read_bytes(current_block_mut, current_pos, bytes_to_copy)?;
      last_block.set_position((current_pos + bytes_to_copy).try_convert()?);
      num_bytes = num_bytes.checked_sub(bytes_to_copy).ok_or_else(|| {
        LuceneError::illegal_state(format!(
          "underflow, num_bytes {}, bytes_to_copy {} ",
          num_bytes, bytes_to_copy
        ))
      })?;
    }
    Ok(())
  }
}

impl Accountable for ByteBuffersDataOutput {
  fn ram_bytes_used(&self) -> Result<i64> {
    todo!()
  }
}

fn compute_block_size_bits_for(bytes: i64) -> i32 {
  let avg_block_size =
    (bytes / ByteBuffersDataOutput::MAX_BLOCKS_BEFORE_BLOCK_EXPANSION as i64) as u64;
  let power_of_two = avg_block_size.next_power_of_two();
  if power_of_two == 0 {
    return ByteBuffersDataOutput::DEFAULT_MIN_BITS_PER_BLOCK;
  }
  let mut block_bits = power_of_two.trailing_zeros();
  block_bits = block_bits.min(ByteBuffersDataOutput::DEFAULT_MAX_BITS_PER_BLOCK as u32);
  block_bits = block_bits.max(ByteBuffersDataOutput::DEFAULT_MIN_BITS_PER_BLOCK as u32);
  debug_assert!(block_bits <= i32::MAX as u32);
  block_bits as i32
}

#[allow(dead_code)]
fn write_long_string(_byte_len: usize, _s: String) {
  unimplemented!()
}
