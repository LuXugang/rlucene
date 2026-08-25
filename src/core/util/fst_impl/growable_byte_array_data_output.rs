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
use crate::core::store::DataOutput;
use crate::core::util::SliceCopyOps;
use crate::core::util::accountable::Accountable;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::ram_usage_estimator::size_of_vec;
// Store one contiguous byte buffer for the current FST node. The buffer only
// grows; it never shrinks.
// Note: This is only safe for usage that is bounded in the number of bytes
// written. Do not make this public! Public users should instead use
// ByteBuffersDataOutput
pub(crate) struct GrowableByteArrayDataOutput {
  bytes: Vec<u8>,
  next_write: usize,
}
impl GrowableByteArrayDataOutput {
  const INITIAL_SIZE: usize = 1 << 8;
  pub fn new() -> Self {
    Self {
      bytes: vec![0u8; Self::INITIAL_SIZE],
      next_write: 0,
    }
  }
  pub fn get_position(&self) -> usize {
    self.next_write
  }
  /// Returns the full byte buffer.
  pub fn get_bytes(&mut self) -> &mut [u8] {
    &mut self.bytes
  }

  /// Set the position of the byte array, increasing the capacity if needed.
  pub fn set_position(&mut self, new_len: usize) -> Result<()> {
    if new_len > self.next_write {
      self.ensure_capacity(new_len - self.next_write)?;
    }
    self.next_write = new_len;
    Ok(())
  }

  /// Ensure we can write additional `capacity_to_write` bytes.
  fn ensure_capacity(&mut self, capacity_to_write: usize) -> Result<()> {
    ArrayUtil::grow_with_len(&mut self.bytes, self.next_write + capacity_to_write)
  }
  /// Writes all of our bytes to the target `Write`.
  pub fn write_to_data_output(&self, out: &mut impl DataOutput) -> Result<()> {
    out.write_bytes_range(&self.bytes, 0, self.next_write)
  }

  /// Copies bytes from this store to a target buffer.
  pub fn write_to(&self, src_offset: usize, dest: &mut [u8], dest_offset: i32, len: usize) {
    debug_assert!(src_offset + len <= self.next_write);
    dest.copy_from(
      &self.bytes[src_offset..(src_offset + len)],
      dest_offset as usize,
    );
  }
}

impl DataOutput for GrowableByteArrayDataOutput {
  fn write_byte(&mut self, b: u8) -> Result<()> {
    self.ensure_capacity(1)?;
    self.bytes[self.next_write] = b;
    self.next_write += 1;
    Ok(())
  }

  fn write_bytes_range(&mut self, b: &[u8], offset: usize, length: usize) -> Result<()> {
    if length == 0 {
      return Ok(());
    }
    self.ensure_capacity(length)?;
    let start = offset;
    let end = start + length;
    self.bytes.copy_from(&b[start..end], self.next_write);
    self.next_write += length;
    Ok(())
  }
}
impl Accountable for GrowableByteArrayDataOutput {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(size_of_vec(&self.bytes))
  }
}
