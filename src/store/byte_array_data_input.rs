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
use crate::util::error::runtime_error::RuntimeError;
use std::any::type_name;
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
    pos: u32,
    limit: u32,
}
impl ByteArrayDataInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_bytes(bytes: Vec<u8>) -> Self {
        let len = bytes.len();
        debug_assert!(len <= u32::MAX as usize, "bytes length exceeds u32 range");
        Self::new_with_range(bytes, 0, len as u32)
    }
    pub fn new_with_range(bytes: Vec<u8>, offset: u32, length: u32) -> Self {
        let mut data_input = Self::new();
        data_input.reset_with_range(bytes, offset, length);
        data_input
    }

    pub fn reset(&mut self, bytes: Vec<u8>) {
        let len = bytes.len();
        debug_assert!(len <= u32::MAX as usize, "bytes length exceeds u32 range");
        self.reset_with_range(bytes, 0, len as u32);
    }
    pub fn reset_with_range(&mut self, bytes: Vec<u8>, offset: u32, length: u32) {
        self.bytes = bytes;
        self.pos = offset;
        self.limit = offset + length;
    }
    // NOTE: sets pos to 0, which is not right if you had
    // called reset w/ non-zero offset!!
    pub fn rewind(&mut self) {
        self.pos = 0;
    }

    pub fn get_position(&self) -> u32 {
        self.pos
    }
    pub fn set_position(&mut self, pos: u32) {
        self.pos = pos;
    }
    pub fn length(&self) -> u32 {
        self.limit
    }
    pub fn eof(&self) -> bool {
        self.pos == self.limit
    }
    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }
}

impl Display for ByteArrayDataInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let address = self as *const Self as usize;
        write!(f, "{}@{:x}", self.type_name(), address)
    }
}

impl DataInput for ByteArrayDataInput {
    fn read_byte(&mut self) -> Result<u8, RuntimeError> {
        let value = self.bytes[self.pos as usize];
        self.pos += 1;
        Ok(value)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: u32, len: u32) -> Result<(), RuntimeError> {
        debug_assert!(
            (offset + len) as usize <= b.len(),
            "Offset and length exceed the destination buffer size"
        );
        debug_assert!(
            (self.pos + len) as usize <= self.bytes.len(),
            "Read range exceeds the source buffer size"
        );
        unsafe {
            let src = self.bytes.as_ptr().add(self.pos as usize);
            let dst = b.as_mut_ptr().add(offset as usize);
            std::ptr::copy_nonoverlapping(src, dst, len as usize);
        }
        self.pos += len;
        Ok(())
    }

    fn read_short(&mut self) -> Result<i16, RuntimeError> {
        let result = BitUtil::get_i16_le(&self.bytes, self.pos as usize);
        self.pos += 2;
        Ok(result)
    }

    fn read_int(&mut self) -> Result<i32, RuntimeError> {
        let value = BitUtil::get_i32_le(&self.bytes, self.pos as usize);
        self.pos += 4;
        Ok(value)
    }

    fn read_long(&mut self) -> Result<i64, RuntimeError> {
        let value = BitUtil::get_i64_le(&self.bytes, self.pos as usize);
        self.pos += 8;
        Ok(value)
    }

    fn skip_bytes(&mut self, count: u64) -> Result<(), RuntimeError> {
        debug_assert!(count <= u32::MAX as u64, "count exceeds usize range");
        self.pos += count as u32;
        Ok(())
    }
}
