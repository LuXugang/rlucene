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
use crate::store::data_input::DataInput;
use crate::util::bit_util::BitUtil;
use crate::util::error::data_io_error_enum::DataIOError;
use std::fmt::{Display, Formatter};

#[derive(Default)]
/// `DataInput` backed by a byte array.
///
/// # Warning
/// This class omits all low-level checks.
///
/// # Note
/// This is an experimental API.
pub struct ByteArrayDataInput {
    bytes: Vec<u8>,
    pos: usize,
    limit: usize,
}
impl ByteArrayDataInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_bytes(bytes: Vec<u8>) -> Self {
        let len = bytes.len();
        Self::new_with_range(bytes, 0, len)
    }
    pub fn new_with_range(bytes: Vec<u8>, offset: usize, length: usize) -> Self {
        let mut data_input = Self::new();
        data_input.reset_with_range(bytes, offset, length);
        data_input
    }

    pub fn reset(&mut self, bytes: Vec<u8>) {
        let len = bytes.len();
        self.reset_with_range(bytes, 0, len);
    }
    pub fn reset_with_range(&mut self, bytes: Vec<u8>, offset: usize, length: usize) {
        self.bytes = bytes;
        self.pos = offset;
        self.limit = offset + length;
    }
    // NOTE: sets pos to 0, which is not right if you had
    // called reset w/ non-zero offset!!
    pub fn rewind(&mut self) {
        self.pos = 0;
    }

    pub fn get_position(&self) -> usize {
        self.pos
    }
    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }
    pub fn length(&self) -> usize {
        self.limit
    }
    pub fn eof(&self) -> bool {
        self.pos == self.limit
    }
}

impl Display for ByteArrayDataInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl DataInput for ByteArrayDataInput {
    fn read_byte(&mut self) -> Result<u8, DataIOError> {
        let value = self.bytes[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<(), DataIOError> {
        debug_assert!(
            (offset + len) <= b.len(),
            "Offset and length exceed the destination buffer size"
        );
        debug_assert!(
            (self.pos + len) <= self.bytes.len(),
            "Read range exceeds the source buffer size"
        );
        unsafe {
            let src = self.bytes.as_ptr().add(self.pos);
            let dst = b.as_mut_ptr().add(offset);
            std::ptr::copy_nonoverlapping(src, dst, len);
        }
        self.pos += len;
        Ok(())
    }

    fn read_short(&mut self) -> Result<i16, DataIOError> {
        let result = BitUtil::get_i16_le(&self.bytes, self.pos);
        self.pos += 2;
        Ok(result)
    }

    fn read_int(&mut self) -> Result<i32, DataIOError> {
        let value = BitUtil::get_i32_le(&self.bytes, self.pos);
        self.pos += 4;
        Ok(value)
    }

    fn read_long(&mut self) -> Result<i64, DataIOError> {
        let value = BitUtil::get_i64_le(&self.bytes, self.pos);
        self.pos += 8;
        Ok(value)
    }

    fn skip_bytes(&mut self, count: u64) -> Result<(), DataIOError> {
        debug_assert!(count <= usize::MAX as u64, "count exceeds usize range");
        self.pos += count as usize;
        Ok(())
    }
}
