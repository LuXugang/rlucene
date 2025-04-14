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
use crate::store::DataOutput;
use crate::util::accountable::Accountable;
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::Result;
use crate::util::SliceCopyOps;
// Storing a single contiguous byte[] for the current node of the FST we are writing. The byte[]
// will only grow, never shrink.
// Note: This is only safe for usage that is bounded in the number of bytes written. Do not make
// this public! Public users should instead use ByteBuffersDataOutput
pub(crate) struct GrowableByteArrayDataOutput {
    bytes: Vec<u8>,
    next_write: i32,
}
impl GrowableByteArrayDataOutput {
    const INITIAL_SIZE: usize = 1 << 8;
    pub(crate) fn new() -> Self {
        Self {
            bytes: vec![0u8; Self::INITIAL_SIZE],
            next_write: 0,
        }
    }
    pub(crate) fn get_position(&self) -> i32 {
        self.next_write
    }
    /// Returns the full byte buffer.
    pub(crate) fn get_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Set the position of the byte array, increasing the capacity if needed.
    pub(crate) fn set_position(&mut self, new_len: i32) {
        debug_assert!(new_len >= 0);
        if new_len > self.next_write {
            self.ensure_capacity(new_len - self.next_write);
        }
        self.next_write = new_len;
    }

    /// Ensure we can write additional `capacity_to_write` bytes.
    fn ensure_capacity(&mut self, capacity_to_write: i32) -> Result<()> {
        debug_assert!(capacity_to_write > 0);
        ArrayUtil::grow_with_len(&mut self.bytes, capacity_to_write)
    }
    /// Writes all of our bytes to the target `Write`.
    pub(crate) fn write_to(&self, out: &mut impl DataOutput) -> Result<()> {
        out.write_bytes_range(&self.bytes, 0, self.next_write)
    }

    /// Copies bytes from this store to a target buffer.
    pub(crate) fn write_to_slice(
        &self,
        src_offset: i32,
        dest: &mut [u8],
        dest_offset: i32,
        len: i32,
    ) {
        debug_assert!(src_offset + len <= self.next_write);
        dest.copy_from(
            &self.bytes[src_offset as usize..(src_offset + len) as usize],
            dest_offset as usize,
        );
    }
}

impl DataOutput for GrowableByteArrayDataOutput {
    fn write_byte(&mut self, b: u8) -> Result<()> {
        self.ensure_capacity(1)?;
        self.bytes[self.next_write as usize] = b;
        self.next_write += 1;
        Ok(())
    }

    fn write_bytes_range(&mut self, b: &[u8], offset: i32, len: i32) -> Result<()> {
        if len == 0 {
            return Ok(());
        }
        self.ensure_capacity(len)?;
        let start = offset as usize;
        let end = start + len as usize;
        self.bytes
            .copy_from(&b[start..end], self.next_write as usize);
        self.next_write += len;
        Ok(())
    }
}
impl Accountable for GrowableByteArrayDataOutput {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}
