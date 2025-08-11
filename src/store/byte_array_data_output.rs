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
use crate::util::SliceCopyOps;
use crate::util::access::AccessVec;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::Result;

/// `DataOutput` backed by a byte array.
///
/// # Warning
/// This struct omits most low-level checks, so be sure to test thoroughly with
/// assertions enabled.
///
/// # Note
/// This is an experimental API.
#[derive(Default)]
pub struct ByteArrayDataOutput<AV>
where
    AV: AccessVec<u8>,
{
    pub bytes: AV,
    pos: usize,
    limit: usize,
}

impl<AV> ByteArrayDataOutput<AV>
where
    AV: AccessVec<u8>,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_bytes(bytes: AV) -> Self {
        let len = bytes.access(|bytes| bytes.len());
        Self::with_range(bytes, 0, len)
    }
    pub fn with_range(bytes: AV, offset: usize, length: usize) -> Self {
        Self {
            bytes,
            pos: offset,
            limit: offset + length,
        }
    }
    pub fn reset(&mut self) -> Result<()> {
        let len = self.bytes.access(|bytes| bytes.len());
        let offset = 0;
        self.reset_with_range(offset, len)
    }
    pub fn reset_with_range(&mut self, offset: usize, length: usize) -> Result<()> {
        self.pos = offset;
        self.limit = offset + length;
        Ok(())
    }

    pub fn get_position(&self) -> usize {
        self.pos
    }
}

impl<AV> DataOutput for ByteArrayDataOutput<AV>
where
    AV: AccessVec<u8>,
{
    fn write_byte(&mut self, b: u8) -> Result<()> {
        debug_assert!(self.pos < self.limit);
        self.bytes.access_mut(|bytes| {
            bytes[self.pos] = b;
        });
        self.pos += 1;
        Ok(())
    }

    fn write_bytes_range(&mut self, b: &[u8], offset: i32, length: i32) -> Result<()> {
        debug_assert!(
            self.pos + length as usize <= self.limit,
            "Write exceeds the allowed limit: pos={}, length={}, limit={}",
            self.pos,
            length,
            self.limit
        );
        self.bytes.access_mut(|bytes| {
            bytes.copy_from(&b[offset as usize..(offset + length) as usize], self.pos);
        });
        self.pos += length as usize;
        Ok(())
    }

    fn write_int(&mut self, i: i32) -> Result<()> {
        debug_assert!(
            self.pos + BitUtil::INT_BYTES <= self.limit,
            "Write exceeds the allowed limit"
        );
        self.bytes.access_mut(|bytes| {
            BitUtil::set_i32_le(bytes, self.pos, i);
        });
        self.pos += BitUtil::INT_BYTES;
        Ok(())
    }

    fn write_short(&mut self, i: i16) -> Result<()> {
        debug_assert!(
            self.pos + BitUtil::SHORT_BYTES <= self.limit,
            "Write exceeds the allowed limit"
        );
        self.bytes.access_mut(|bytes| {
            BitUtil::set_i16_le(bytes, self.pos, i);
        });
        self.pos += BitUtil::SHORT_BYTES;
        Ok(())
    }

    fn write_long(&mut self, i: i64) -> Result<()> {
        debug_assert!(
            self.pos + BitUtil::LONG_BYTES <= self.limit,
            "Write exceeds the allowed limit"
        );
        self.bytes.access_mut(|bytes| {
            BitUtil::set_i64_le(bytes, self.pos, i);
        });
        self.pos += BitUtil::LONG_BYTES;
        Ok(())
    }
}
