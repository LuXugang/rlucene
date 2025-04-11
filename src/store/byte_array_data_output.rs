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
use crate::store::data_output::DataOutput;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
/// `DataOutput` backed by a byte array.
///
/// # Warning
/// This struct omits most low-level checks, so be sure to test thoroughly with assertions enabled.
///
/// # Note
/// This is an experimental API.
pub struct ByteArrayDataOutput {
    pub bytes: Vec<u8>,
    pos: i32,
    limit: i32,
}

impl Default for ByteArrayDataOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteArrayDataOutput {
    pub fn new() -> Self {
        Self::with_bytes(vec![])
    }

    pub fn with_bytes(bytes: Vec<u8>) -> Self {
        let len = bytes.len();
        debug_assert!(len <= i32::MAX as usize);
        Self::with_range(bytes, 0, len as i32)
    }
    pub fn with_range(bytes: Vec<u8>, offset: i32, length: i32) -> Self {
        Self {
            bytes,
            pos: offset,
            limit: offset + length,
        }
    }
    pub fn reset(&mut self) -> Result<()> {
        let len = self.bytes.len();
        let offset = 0;
        self.reset_with_range(offset, len as i32)
    }
    pub fn reset_with_range(&mut self, offset: i32, length: i32) -> Result<()> {
        if (offset + length) > self.bytes.len() as i32 {
            return Err(LuceneError::array_index_out_of_bounds(format!(
                "offset: {}, length: {} exceeds bytes length: {}",
                offset,
                length,
                self.bytes.len()
            )));
        }
        self.pos = offset;
        self.limit = offset + length;
        Ok(())
    }

    pub fn get_position(&self) -> i32 {
        self.pos
    }
}

impl DataOutput for ByteArrayDataOutput {
    fn write_byte(&mut self, b: u8) -> Result<()> {
        debug_assert!(self.pos < self.limit, "Write exceeds the allowed limit");
        debug_assert!(
            self.pos < self.limit,
            "Write position out of bounds: pos={}, limit={}",
            self.pos,
            self.limit
        );

        unsafe {
            *self.bytes.as_mut_ptr().add(self.pos as usize) = b;
        }
        self.pos += 1;
        Ok(())
    }

    fn write_bytes_range(&mut self, b: &[u8], offset: i32, length: i32) -> Result<()> {
        debug_assert!(
            self.pos + length <= self.limit,
            "Write exceeds the allowed limit: pos={}, length={}, limit={}",
            self.pos,
            length,
            self.limit
        );
        debug_assert!(
            (offset + length) as usize <= b.len(),
            "Source slice out of bounds: offset={}, length={}, source_len={}",
            offset,
            length,
            b.len()
        );
        debug_assert!(
            (self.pos + length) as usize <= self.bytes.len(),
            "Destination slice out of bounds: pos={}, length={}, dest_len={}",
            self.pos,
            length,
            self.bytes.len()
        );

        debug_assert!(
            {
                let dst_start = self.bytes.as_mut_ptr() as usize + self.pos as usize;
                let dst_end = dst_start + length as usize;
                let src_start = b.as_ptr() as usize + offset as usize;
                let src_end = src_start + length as usize;
                dst_start >= src_end || src_start >= dst_end
            },
            "Source and destination memory regions overlap"
        );

        unsafe {
            let dst = self.bytes.as_mut_ptr().add(self.pos as usize);
            let src = b.as_ptr().add(offset as usize);
            std::ptr::copy_nonoverlapping(src, dst, length as usize);
        }

        self.pos += length;
        Ok(())
    }

    fn write_int(&mut self, i: i32) -> Result<()> {
        debug_assert!(
            self.pos + BitUtil::INT_BYTES as i32 <= self.limit,
            "Write exceeds the allowed limit"
        );
        BitUtil::set_i32_le(&mut self.bytes, self.pos as usize, i);
        self.pos += BitUtil::INT_BYTES as i32;
        Ok(())
    }

    fn write_short(&mut self, i: i16) -> Result<()> {
        debug_assert!(
            self.pos + BitUtil::SHORT_BYTES as i32 <= self.limit,
            "Write exceeds the allowed limit"
        );
        BitUtil::set_i16_le(&mut self.bytes, self.pos as usize, i);
        self.pos += BitUtil::SHORT_BYTES as i32;
        Ok(())
    }

    fn write_long(&mut self, i: i64) -> Result<()> {
        debug_assert!(
            self.pos + BitUtil::LONG_BYTES as i32 <= self.limit,
            "Write exceeds the allowed limit"
        );
        BitUtil::set_i64_le(&mut self.bytes, self.pos as usize, i);
        self.pos += BitUtil::LONG_BYTES as i32;
        Ok(())
    }
}
