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

use crate::core::index::BytesRef;
use crate::core::util::accountable::Accountable;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::bytes_ref_hash::do_hash;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{
  BYTE_BLOCK_MASK, BYTE_BLOCK_SHIFT, BYTE_BLOCK_SIZE, ByteBlockPool, SliceCopyOps,
};

pub struct BytesRefBlockPool;

impl Default for BytesRefBlockPool {
  fn default() -> Self {
    Self::new()
  }
}

impl BytesRefBlockPool {
  pub fn new() -> Self {
    Self {}
  }
  /// Resets this buffer to the empty state.
  pub fn reset(&mut self, byte_block_pool: &mut ByteBlockPool) {
    byte_block_pool.reset(false, false)
  }

  /// Populates the given [`BytesRef`] with the term starting at `start`.
  pub fn fill_bytes_ref(
    &self,
    term: &mut BytesRef<Vec<u8>>,
    start: i32,
    byte_block_pool: &ByteBlockPool,
  ) -> Result<()> {
    let block = byte_block_pool.get_buffer((start >> BYTE_BLOCK_SHIFT) as usize);
    let pos = (start & BYTE_BLOCK_MASK) as usize;

    let (length, offset) = if (block[pos] & 0x80) == 0 {
      // Length is 1 byte
      (block[pos] as i32, (pos + 1) as i32)
    } else {
      // Length is 2 bytes
      (
        (BitUtil::get_i16_be(block, pos) & 0x7FFF) as i32,
        (pos + 2) as i32,
      )
    };
    let length = length as usize;
    ArrayUtil::grow_no_copy(&mut term.bytes, length)?;
    term
      .bytes
      .copy_from(&block[offset as usize..offset as usize + length], 0);
    term.offset = 0;
    term.length = length;
    Ok(())
  }
  /// Add a term, returning the start position on the underlying
  /// [`ByteBlockPool`]. This can be used to read back the value using
  /// `fill_bytes_ref`.
  ///
  /// # See Also
  /// * `fill_bytes_ref(BytesRef, int)`
  pub fn add_bytes_ref(
    &mut self,
    bytes: &BytesRef<Vec<u8>>,
    pool: &mut ByteBlockPool,
  ) -> Result<i32> {
    let length = bytes.length as i32;
    let len2 = 2 + bytes.length as i32;
    if len2 + pool.byte_upto > BYTE_BLOCK_SIZE {
      if len2 > BYTE_BLOCK_SIZE {
        return Err(LuceneError::max_bytes_length_exceeded(format!(
          "bytes can be at most {} in length; got {}",
          BYTE_BLOCK_SIZE - 2,
          bytes.length
        )));
      }
      pool.next_buffer()?;
    }

    let buffer_upto = pool.byte_upto;
    let text_start = buffer_upto + pool.byte_offset;
    let buffer_index = pool.buffer_upto()?;
    let buffer = pool.get_buffer_mut(buffer_index);

    // We first encode the length, followed by the bytes. Length is
    // encoded as vInt, but will consume 1 or 2 bytes at
    // most (we reject too-long terms, above).
    let new_length = if length < 128 {
      // 1 byte to store length
      buffer[buffer_upto as usize] = length as u8;
      debug_assert!(length >= 0, "Length must be positive: {length}");
      buffer.copy_from(
        &bytes.bytes[bytes.offset..bytes.offset + length as usize],
        buffer_upto as usize + 1,
      );
      length + 1
    } else {
      // 2 byte to store length
      BitUtil::set_i16_be(buffer, buffer_upto as usize, (length | 0x8000) as i16);
      buffer.copy_from(
        &bytes.bytes[bytes.offset..bytes.offset + length as usize],
        buffer_upto as usize + 2,
      );
      length + 2
    };
    pool.byte_upto += new_length;
    Ok(text_start)
  }
  /// Computes the hash of the BytesRef at the given start.
  pub fn hash(&self, start: i32, pool: &ByteBlockPool) -> i32 {
    let offset = (start & BYTE_BLOCK_MASK) as usize;
    let bytes = pool.get_buffer((start >> BYTE_BLOCK_SHIFT) as usize);

    let (len, pos) = if (bytes[offset] & 0x80) == 0 {
      // length is 1 byte
      (bytes[offset] as usize, offset + 1)
    } else {
      // length is 2 bytes (16-bit value, but only using lower 15
      // bits)
      let len = BitUtil::get_i16_be(bytes, offset) & 0x7FFF;
      (len as usize, offset + 2)
    };

    do_hash(bytes, pos, len)
  }
  /// Computes the equality between the BytesRef at the given start position
  /// and the provided BytesRef.
  pub fn equals(&self, start: i32, b: &BytesRef<Vec<u8>>, pool: &ByteBlockPool) -> bool {
    let pos = (start & BYTE_BLOCK_MASK) as usize;
    let bytes = pool.get_buffer((start >> BYTE_BLOCK_SHIFT) as usize);

    let (length, offset) = if (bytes[pos] & 0x80) == 0 {
      // length is 1 byte
      (bytes[pos] as usize, pos + 1)
    } else {
      // length is 2 bytes (16-bit value, but only using lower 15
      // bits)
      let length = BitUtil::get_i16_be(bytes, pos) & 0x7FFF;
      (length as usize, pos + 2)
    };

    // Compare slices of bytes
    bytes[offset..offset + length] == b.bytes[b.offset..(b.offset + b.length)]
  }
}

impl Accountable for BytesRefBlockPool {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(0)
  }
}
