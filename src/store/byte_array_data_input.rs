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
use crate::util::error::lucene_error::Result;
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
    pos: i32,
    limit: i32,
}
impl ByteArrayDataInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_bytes(bytes: Vec<u8>) -> Self {
        let len = bytes.len();
        debug_assert!(len <= i32::MAX as usize, "bytes length exceeds u32 range");
        Self::with_range(bytes, 0, len as i32)
    }
    pub fn with_range(bytes: Vec<u8>, offset: i32, length: i32) -> Self {
        let mut data_input = Self::new();
        data_input.reset_with_range(bytes, offset, length);
        data_input
    }

    pub fn reset(&mut self, bytes: Vec<u8>) {
        let len = bytes.len();
        debug_assert!(len <= i32::MAX as usize, "bytes length exceeds u32 range");
        self.reset_with_range(bytes, 0, len as i32);
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
    fn read_byte(&mut self) -> Result<u8> {
        let value = self.bytes[self.pos as usize];
        self.pos += 1;
        Ok(value)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
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

    fn read_short(&mut self) -> Result<i16> {
        let result = BitUtil::get_i16_le(&self.bytes, self.pos as usize);
        self.pos += 2;
        Ok(result)
    }

    fn read_int(&mut self) -> Result<i32> {
        let value = BitUtil::get_i32_le(&self.bytes, self.pos as usize);
        self.pos += 4;
        Ok(value)
    }

    fn read_long(&mut self) -> Result<i64> {
        let value = BitUtil::get_i64_le(&self.bytes, self.pos as usize);
        self.pos += 8;
        Ok(value)
    }

    fn skip_bytes(&mut self, count: i64) -> Result<()> {
        debug_assert!(count <= i32::MAX as i64, "count exceeds usize range");
        self.pos += count as i32;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::Cursor;

    use crate::store::data_input::DataInput;
    use crate::store::data_output::DataOutput;
    use crate::store::{ByteArrayDataInput, ByteArrayDataOutput};
    use crate::util::error::lucene_error::Result;

    #[allow(dead_code)] // for quick search
    struct TestByteArrayDataInput;

    #[test]
    fn test_basic() -> Result<()> {
        let bytes = vec![1, 65];
        let mut data_input = ByteArrayDataInput::with_bytes(bytes);
        assert_eq!(data_input.read_string()?, "A");
        assert!(data_input.eof());
        Ok(())
    }

    #[test]
    fn test_data_types() -> Result<()> {
        // write some primitives using ByteArrayDataOutput:
        let bytes = vec![0u8; 32];
        let mut out = ByteArrayDataOutput::with_bytes(bytes);

        out.write_byte(43)?;
        out.write_short(12345)?;
        out.write_int(1234567890)?;
        out.write_long(1234567890123456789)?;
        let size = out.get_position();
        assert_eq!(size, 15);

        let mut buf: Cursor<&[u8]> = Cursor::new(&out.bytes[..size as usize]);

        assert_eq!(buf.read_u8()?, 43);
        assert_eq!(buf.read_i16::<LittleEndian>()?, 12345);
        assert_eq!(buf.read_i32::<LittleEndian>()?, 1234567890);
        assert_eq!(buf.read_i64::<LittleEndian>()?, 1234567890123456789);
        assert_eq!(buf.position() as usize, size as usize);
        assert_eq!(buf.get_ref().len() - buf.position() as usize, 0);

        // read the primitives using ByteArrayDataInput:
        let mut data_input = ByteArrayDataInput::with_range(out.bytes, 0, size);
        assert_eq!(data_input.read_byte()?, 43);
        assert_eq!(data_input.read_short()?, 12345);
        assert_eq!(data_input.read_int()?, 1234567890);
        assert_eq!(data_input.read_long()?, 1234567890123456789);
        assert!(data_input.eof());
        Ok(())
    }
}
