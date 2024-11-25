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

#[derive(Default)]
pub struct ByteArrayDataInput {
    bytes: Vec<u8>,
    pos: i32,
    limit: i32,
}
impl ByteArrayDataInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_bytes(bytes: Vec<u8>) -> Self {
        let len = bytes.len() as i32;
        Self::new_with_range(bytes, 0, len)
    }
    pub fn new_with_range(bytes: Vec<u8>, offset: i32, length: i32) -> Self {
        let mut data_input = Self::new();
        data_input.reset_with_range(bytes, offset, length);
        data_input
    }

    pub fn reset(&mut self, bytes: Vec<u8>) {
        let len = bytes.len() as i32;
        self.reset_with_range(bytes, 0, len);
    }
    pub fn reset_with_range(&mut self, bytes: Vec<u8>, offset: i32, length: i32) {
        self.bytes = bytes;
        self.pos = offset;
        self.limit = offset + length;
    }
    // NOTE: sets pos to 0, which is not right if you had
    // called reset w/ non-zero offset!!
    pub fn rewind(&mut self) {
        self.pos = 0;
    }

    pub fn get_position(&self) -> i32 {
        self.pos
    }
    pub fn set_position(&mut self, pos: i32) {
        self.pos = pos;
    }
    pub fn length(&self) -> i32 {
        self.limit
    }
    pub fn eof(&self) -> bool {
        self.pos == self.limit
    }
}

impl Clone for ByteArrayDataInput {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl DataInput for ByteArrayDataInput {
    fn read_byte(&mut self) -> Result<u8, DataIOError> {
        let value = self.bytes[self.pos as usize];
        self.pos += 1;
        Ok(value)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<(), DataIOError> {
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
        Ok(())
    }

    fn read_short(&mut self) -> Result<i16, DataIOError> {
        let result = BitUtil::get_u16_le(&self.bytes, self.pos as usize) as i16;
        self.pos += 2;
        Ok(result)
    }

    fn read_int(&mut self) -> Result<i32, DataIOError> {
        let value = BitUtil::get_u32_le(&self.bytes, self.pos as usize) as i32;
        self.pos += 4;
        Ok(value)
    }

    fn read_long(&mut self) -> Result<i64, DataIOError> {
        let value = BitUtil::get_u64_le(&self.bytes, self.pos as usize) as i64;
        self.pos += 8;
        Ok(value)
    }

    fn skip_bytes(&mut self, count: i64) -> Result<(), DataIOError> {
        self.pos += count as i32;
        Ok(())
    }
}
