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

use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::access::{SharedAccessVec, WritableVec};
use crate::core::util::accountable::Accountable;
use crate::core::util::allocator_byte::{AllocatorByte, AllocatorByteEnum, DirectAllocatorByte};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{SliceCopyOps, TryIntoInt};

/// This struct enables the allocation of fixed-size buffers and their
/// management as part of a buffer array. Allocation is done through the use of
/// an [`AllocatorByte`] which can
/// be customized, e.g., to allow recycling old buffers. There are methods for
/// writing ([`append`](#method.append)) and reading from the buffers (e.g.,
/// [`read_bytes`](#method.read_bytes)), which handle read/write operations
/// across buffer boundaries.
///
/// # Note
/// This is an internal API.
#[derive(Debug)]
pub struct ByteBlockPool {
  buffers: Vec<Vec<u8>>,
  // Current head buffer's index
  pub(crate) buffer_upto: Option<usize>,
  allocator: AllocatorByteEnum,
  /// Offset from the start of the first buffer to the start of the current
  /// buffer, which is `buffer_upto * BYTE_BLOCK_SIZE`. The buffer pool
  /// maintains this offset because it is the first to overflow if there
  /// are too many allocated blocks.
  pub(crate) byte_offset: i32,
  pub(crate) byte_upto: i32,
}
impl Default for ByteBlockPool {
  fn default() -> Self {
    let allocator = DirectAllocatorByte::new();
    Self::new(allocator)
  }
}
impl ByteBlockPool {
  pub fn new<T>(allocator: T) -> Self
  where
    T: Into<AllocatorByteEnum>,
  {
    let allocator = allocator.into();
    ByteBlockPool {
      buffers: vec![],
      buffer_upto: None,
      allocator,
      byte_offset: -BYTE_BLOCK_SIZE,
      byte_upto: BYTE_BLOCK_SIZE,
    }
  }
  /// Expert: Resets the pool to its initial state, while optionally reusing
  /// the first buffer. Buffers that are not reused are reclaimed by
  /// [`AllocatorByte::recycle_byte_blocks`].
  /// Buffers can be filled with zeros before recycling them. This is
  /// useful if a slice pool works on top of this byte pool and relies on
  /// the buffers being filled with zeros to find the non-zero end of slices.
  ///
  /// # Arguments
  /// * `zero_fill_buffers` - If `true`, the buffers are filled with `0`. This
  ///   should be set to `true` if this pool is used with slices.
  /// * `reuse_first` - If `true`, the first buffer will be reused, and
  ///   calling [`ByteBlockPool::next_buffer`](#method.next_buffer) is not
  ///   needed after reset, if the block pool was used before (i.e.,
  ///   [`ByteBlockPool::next_buffer`](#method.next_buffer) was called
  ///   before).
  pub fn reset(&mut self, zero_fill_buffers: bool, reuse_first: bool) {
    if let Some(buffer_upto) = self.buffer_upto {
      if zero_fill_buffers {
        for i in 0..(buffer_upto + 1) {
          self.buffers[i].fill(0);
        }
      }
      if buffer_upto > 0 || !reuse_first {
        let offset = if reuse_first { 1 } else { 0 };
        self
          .allocator
          .recycle_byte_blocks(&self.buffers, offset, buffer_upto + 1);
        for _i in offset..(buffer_upto + 1) {
          self.buffers.pop();
        }
      }

      if reuse_first {
        self.buffer_upto = Some(0);
        self.byte_upto = 0;
        self.byte_offset = 0;
      } else {
        self.buffer_upto = None;
        self.byte_upto = BYTE_BLOCK_SIZE;
        self.byte_offset = -BYTE_BLOCK_SIZE;
      }
    }
  }
  /// Allocates a new buffer and advances the pool to it. This method should
  /// be called once after the constructor to initialize the pool. In
  /// contrast to the constructor, a [`ByteBlockPool::reset`](#method.
  /// reset) call will advance the pool to its first buffer immediately.
  pub fn next_buffer(&mut self) -> Result<usize> {
    let next_upto = match self.buffer_upto {
      Some(upto) => upto + 1,
      None => 0,
    };

    if next_upto == self.buffers.len() {
      self.buffers.push(self.allocator.get_byte_block());
    }
    // Allocate new buffer and advance the pool to it
    self.buffer_upto = Some(next_upto);
    self.byte_upto = 0;
    match self.byte_offset.checked_add(BYTE_BLOCK_SIZE) {
      Some(val) => self.byte_offset = val,
      None => {
        return Err(LuceneError::number_overflow(
          "Overflow when calculating byte offset.",
        ));
      },
    }
    Ok(next_upto)
  }

  /// Fills the provided [`BytesRef`] with the bytes at the specified offset
  /// and length. # Parameters
  /// - `_builder`: This parameter is currently unused but retained for future
  ///   compatibility.See Note
  /// # Note
  /// In Java, the length of result is adjusted through BytesRefBuilder,
  /// whereas in Rust Lucene, to avoid copying, we operate directly on result.
  ///
  /// However, we still retain the interface definitions from Java Lucene to
  /// maintain consistency with the original implementation as much as
  /// possible.
  pub fn set_bytes_ref<AV>(
    &self,
    _builder: &mut BytesRefBuilder<AV>,
    result: &mut BytesRef<AV>,
    offset: i64,
    length: i32,
  ) -> Result<()>
  where
    AV: SharedAccessVec<u8> + WritableVec<u8>,
  {
    if result.length < length as usize {
      result.bytes = AV::from_vec(vec![0; length as usize]);
    }
    result.length = length as usize;
    let buffer_index = offset >> BYTE_BLOCK_SHIFT;
    let pos = (offset & BYTE_BLOCK_MASK as i64) as i32;
    if pos + length <= BYTE_BLOCK_SIZE {
      // Common case: The slice lives in a single block.
      result.bytes.copy(
        &self.buffers[buffer_index as usize][pos as usize..(pos + length) as usize],
        0,
      );
      result.offset = 0;
    } else {
      // builder.grow_no_copy(length);
      result.offset = 0;
      result.bytes.access_mut(|bytes| {
        self.read_bytes(offset, bytes, 0, length)?;
        // Help the compiler infer types.
        Ok::<(), LuceneError>(())
      })?;
      // builder.get().bytes.clone_from(&result.bytes);
    }
    Ok(())
  }
  /// Appends the bytes in the provided BytesRef at the current position.
  pub fn append_bytes_ref<AV>(&mut self, bytes: &BytesRef<AV>) -> Result<()>
  where
    AV: SharedAccessVec<u8>,
  {
    bytes
      .bytes
      .access(|bytes_ref| self.append_range(bytes_ref, bytes.offset as i32, bytes.length as i32))
  }
  /// Appends the bytes from a source [`ByteBlockPool`] at a given offset and
  /// length.
  ///
  /// # Arguments
  /// * `src_pool` - The source pool to copy from.
  /// * `src_offset` - The source pool offset.
  /// * `length` - The number of bytes to copy.
  pub fn append_from_byte_block_pool(
    &mut self,
    src_pool: &ByteBlockPool,
    mut src_offset: i64,
    length: i32,
  ) -> Result<()> {
    let mut bytes_left = length;
    while bytes_left > 0 {
      let buffer_left = BYTE_BLOCK_SIZE - self.byte_upto;
      if bytes_left < buffer_left {
        // fits within current buffer
        self.append_bytes_single_buffer(src_pool, src_offset, bytes_left)?;
        break;
      } else {
        // fill up this buffer and move to next one
        if buffer_left > 0 {
          self.append_bytes_single_buffer(src_pool, src_offset, buffer_left)?;
          bytes_left -= buffer_left;
          src_offset += buffer_left as i64;
        }
        self.next_buffer()?;
      }
    }
    Ok(())
  }
  fn append_bytes_single_buffer(
    &mut self,
    src_pool: &ByteBlockPool,
    mut src_offset: i64,
    mut length: i32,
  ) -> Result<()> {
    debug_assert!(length <= BYTE_BLOCK_SIZE - self.byte_upto);
    debug_assert!(self.buffer_upto.is_some());
    let buffer_upto = self
      .buffer_upto
      .ok_or_else(|| LuceneError::number_format("buffer not initialized"))?;
    while length > 0 {
      let src_pos = src_offset & BYTE_BLOCK_MASK as i64;
      let bytes_to_copy = std::cmp::min(BYTE_BLOCK_SIZE - src_pos as i32, length);
      self.buffers[buffer_upto].copy_from(
        &src_pool.buffers[(src_offset >> BYTE_BLOCK_SHIFT) as usize]
          [src_pos as usize..(src_pos + bytes_to_copy as i64) as usize],
        self.byte_upto as usize,
      );

      length -= bytes_to_copy;
      src_offset += bytes_to_copy as i64;
      self.byte_upto += bytes_to_copy;
    }
    Ok(())
  }

  /// Appends the provided byte array at the current position.
  ///
  /// # Arguments
  /// * `bytes` - The byte array to write.
  pub fn append(&mut self, bytes: &[u8]) -> Result<()> {
    let length = bytes.len() as i32;
    self.append_range(bytes, 0, length)
  }
  /// Appends the bytes from a source [`ByteBlockPool`] at a given offset and
  /// length.
  ///
  /// # Arguments
  /// * `src_pool` - The source pool to copy from.
  /// * `src_offset` - The source pool offset.
  /// * `length` - The number of bytes to copy.
  pub fn append_range(&mut self, bytes: &[u8], mut offset: i32, length: i32) -> Result<()> {
    let mut bytes_left = length;
    let mut buffer_upto = self.buffer_upto.unwrap_or_default();
    while bytes_left > 0 {
      let buffer_left = BYTE_BLOCK_SIZE - self.byte_upto;
      if bytes_left < buffer_left {
        // fits within current buffer
        self.buffers[buffer_upto].copy_from(
          &bytes[offset as usize..(offset + bytes_left) as usize],
          self.byte_upto as usize,
        );
        self.byte_upto += bytes_left;
        break;
      } else {
        // fill up this buffer and move to next one
        if buffer_left > 0 {
          self.buffers[buffer_upto].copy_from(
            &bytes[offset as usize..(offset + buffer_left) as usize],
            self.byte_upto as usize,
          );
        }
        buffer_upto = self.next_buffer()?;
        bytes_left -= buffer_left;
        offset += buffer_left;
      }
    }
    Ok(())
  }

  /// Reads bytes out of the pool starting at the given offset with the given
  /// length into the given byte array at offset `off`.
  ///
  /// # Note
  /// This method allows copying across block boundaries.
  pub fn read_bytes(
    &self,
    offset: i64,
    bytes: &mut [u8],
    mut bytes_offset: i32,
    bytes_length: i32,
  ) -> Result<()> {
    let mut bytes_left = bytes_length;
    let buffer_index: i32 = (offset >> BYTE_BLOCK_SHIFT).try_convert()?;
    let mut buffer_index = buffer_index as usize;
    let mut pos = (offset & BYTE_BLOCK_MASK as i64) as i32;
    while bytes_left > 0 {
      let chunk = std::cmp::min(BYTE_BLOCK_SIZE - pos, bytes_left);
      bytes.copy_from(
        &self.buffers[buffer_index][pos as usize..(pos + chunk) as usize],
        bytes_offset as usize,
      );

      bytes_offset += chunk;
      bytes_left -= chunk;
      buffer_index += 1;
      pos = 0;
    }
    Ok(())
  }
  /// Reads a single byte at the given offset.
  ///
  /// # Arguments
  /// * `offset` - The offset to read.
  ///
  /// # Returns
  /// The byte at the specified offset.
  pub fn read_byte(&self, offset: usize) -> u8 {
    let buffer_index = offset >> BYTE_BLOCK_SHIFT;
    let pos = offset & BYTE_BLOCK_MASK as usize;
    self.buffers[buffer_index][pos]
  }
  /// the current position (in absolute value) of this byte pool .
  pub fn get_position(&mut self) -> i64 {
    debug_assert!(self.allocator.get_block_size() <= i32::MAX as usize);
    let buffer_upto = match self.buffer_upto {
      Some(upto) => upto as i32,
      None => -1,
    };
    (buffer_upto * self.allocator.get_block_size() as i32 + self.byte_upto) as i64
  }
  pub fn get_buffer_mut(&mut self, buffer_index: usize) -> &mut Vec<u8> {
    &mut self.buffers[buffer_index]
  }
  pub fn get_buffer(&self, buffer_index: usize) -> &Vec<u8> {
    &self.buffers[buffer_index]
  }
  pub fn get_bytes_used(&self) -> i64 {
    self.allocator.get_used()
  }
  /// Get valid buffer_upto
  pub fn buffer_upto(&self) -> Result<usize> {
    match self.buffer_upto {
      Some(upto) => Ok(upto),
      None => Err(LuceneError::illegal_state(
        "buffer_upto not initialized yet, call next_buffer first.",
      )),
    }
  }
}
impl Accountable for ByteBlockPool {
  fn ram_bytes_used(&self) -> Result<i64> {
    todo!()
  }
}

//TODO
const BASE_RAM_BYTES: i64 = 0;
/// Finds the index of the buffer containing a byte, given an offset to that
/// byte.
///
/// The calculation for `buffer_upto` is as follows:
///
/// - `buffer_upto = global_offset >>
///   BYTE_BLOCK_SHIFT`
/// - `buffer_upto = global_offset / BYTE_BLOCK_SIZE`
///
/// # Parameters
/// - `global_offset`: The offset to the target byte.
pub const BYTE_BLOCK_SHIFT: i32 = 15;
/// The size of each buffer in the pool.
pub const BYTE_BLOCK_SIZE: i32 = 1 << BYTE_BLOCK_SHIFT;
/// Use this to find the position of a global offset in a particular buffer.
///
/// # Formula
/// `position_in_current_buffer = global_offset & BYTE_BLOCK_MASK`
///
/// `position_in_current_buffer = global_offset % BYTE_BLOCK_SIZE`
pub(crate) const BYTE_BLOCK_MASK: i32 = BYTE_BLOCK_SIZE - 1;
