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
use std::any::type_name;
use std::fmt::{Display, Formatter};

use crate::core::store::data_input::DataInput;
use crate::core::util::access::ByteSource;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::group_vint_util::GroupVIntUtil;
use crate::core::util::{SliceCopyOps, TryIntoInt};

/// `DataInput` backed by a byte array.
///
/// # Warning
/// This struct omits all low-level checks.
///
/// # Note
/// This is an experimental API.
#[derive(Default)]
pub struct ByteArrayDataInput<B>
// TODO: 这里可以考虑改成引用bytes或者所有权
where
    B: ByteSource,
{
    pub(crate) bytes: B,
    pos: usize,
    limit: usize,
}
impl<B> ByteArrayDataInput<B>
where
    B: ByteSource,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_bytes(bytes: B) -> Self {
        let len = bytes.as_slice().len();
        Self::with_range(bytes, 0, len)
    }
    pub fn with_range(bytes: B, offset: usize, length: usize) -> Self {
        let mut data_input = Self::new();
        data_input.reset_with_range(bytes, offset, length);
        data_input
    }
    pub fn reset_meta(&mut self, offset: usize, length: usize) {
        self.pos = offset;
        self.limit = offset + length;
    }
    pub fn reset_with_range(&mut self, bytes: B, offset: usize, length: usize) {
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
    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }
}

impl<B> Display for ByteArrayDataInput<B>
where
    B: ByteSource,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let address = self as *const Self as usize;
        write!(f, "{}@{:x}", self.type_name(), address)
    }
}

impl<B> DataInput for ByteArrayDataInput<B>
where
    B: ByteSource,
{
    fn read_byte(&mut self) -> Result<u8> {
        let value = self.bytes.as_slice()[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
        b.copy_from(&self.bytes.as_slice()[self.pos..self.pos + len], offset);
        self.pos += len;
        Ok(())
    }

    fn read_short(&mut self) -> Result<i16> {
        let result = BitUtil::get_i16_le(self.bytes.as_slice(), self.pos);
        self.pos += 2;
        Ok(result)
    }

    fn read_int(&mut self) -> Result<i32> {
        let value = BitUtil::get_i32_le(self.bytes.as_slice(), self.pos);
        self.pos += 4;
        Ok(value)
    }

    fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
        GroupVIntUtil::read_group_vint_i32(self, dst, offset)
    }

    fn read_long(&mut self) -> Result<i64> {
        let value = BitUtil::get_i64_le(self.bytes.as_slice(), self.pos);
        self.pos += 8;
        Ok(value)
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        let num_bytes: usize = num_bytes.try_convert()?;
        self.pos += num_bytes;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use byteorder::{LittleEndian, ReadBytesExt};

    use crate::core::store::data_input::DataInput;
    use crate::core::store::data_output::DataOutput;
    use crate::core::store::{ByteArrayDataInput, ByteArrayDataOutput};
    use crate::core::util::error::lucene_error::Result;

    #[allow(dead_code)] // for quick search
    struct TestByteArrayDataInput;

    #[test]
    fn test_basic() -> Result<()> {
        let bytes = vec![1, 65];
        let mut data_input = ByteArrayDataInput::with_bytes(bytes.as_slice());
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

        let mut buf: Cursor<&[u8]> = Cursor::new(&out.bytes[..size]);

        assert_eq!(buf.read_u8()?, 43);
        assert_eq!(buf.read_i16::<LittleEndian>()?, 12345);
        assert_eq!(buf.read_i32::<LittleEndian>()?, 1234567890);
        assert_eq!(buf.read_i64::<LittleEndian>()?, 1234567890123456789);
        assert_eq!(buf.position() as usize, size);
        assert_eq!(buf.get_ref().len() - buf.position() as usize, 0);

        // read the primitives using ByteArrayDataInput:
        let mut data_input = ByteArrayDataInput::with_range(out.bytes.as_slice(), 0, size);
        assert_eq!(data_input.read_byte()?, 43);
        assert_eq!(data_input.read_short()?, 12345);
        assert_eq!(data_input.read_int()?, 1234567890);
        assert_eq!(data_input.read_long()?, 1234567890123456789);
        assert!(data_input.eof());
        Ok(())
    }
}
