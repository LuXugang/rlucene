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
use crate::store::index_output::IndexOutput;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::{LuceneError, Result};

use byteorder::{LittleEndian, WriteBytesExt};
use crc32fast::Hasher;
use std::fmt::{Display, Formatter};
use std::io::{BufWriter, Write};

/// Implementation struct for buffered [`IndexOutput`] that writes to an [`OutputStream`](Write).
pub struct OutputStreamIndexOutput<W>
where
    W: Write,
{
    os: XBufferedOutputStream<W>,
    bytes_written: i64,
    name: String,
    resource_description: String,
}
impl<W: Write> OutputStreamIndexOutput<W>
where
    W: Write,
{
    /// Creates a new [`OutputStreamIndexOutput`] with the given buffer size.
    ///
    /// # Arguments
    /// * `buffer_size` - The buffer size in bytes used to buffer writes internally.
    ///
    /// # Errors
    /// Returns an `IllegalArgumentError` if the given buffer size is less than [`BitUtil::LONG_BYTES`].
    ///
    pub fn new(
        resource_description: &str,
        name: &str,
        inner: W,
        buffer_size: i32,
    ) -> Result<OutputStreamIndexOutput<W>> {
        if (buffer_size as usize) < BitUtil::LONG_BYTES {
            return Err(LuceneError::illegal_argument(format!(
                "Buffer size too small, need: {}, got: {}",
                BitUtil::LONG_BYTES,
                buffer_size
            )));
        }
        let os = XBufferedOutputStream::new(inner, buffer_size);
        Ok(Self {
            os,
            bytes_written: 0,
            name: name.to_string(),
            resource_description: resource_description.to_string(),
        })
    }
}

impl<W: Write> DataOutput for OutputStreamIndexOutput<W>
where
    W: Write,
{
    fn write_byte(&mut self, b: u8) -> Result<()> {
        self.bytes_written += 1;
        self.os.write_u8(b)
    }

    fn write_bytes_range(
        &mut self,
        b: &[u8],
        offset: i32,
        length: i32,
    ) -> Result<()> {
        let end = offset + length;
        self.bytes_written += length as i64;
        self.os.write_bytes(&b[offset as usize..end as usize])
    }

    fn write_int(&mut self, i: i32) -> Result<()> {
        self.bytes_written += 4;
        self.os.write_i32(i)
    }

    fn write_short(&mut self, i: i16) -> Result<()> {
        self.bytes_written += 2;
        self.os.write_i16(i)
    }

    fn write_long(&mut self, i: i64) -> Result<()> {
        self.bytes_written += 8;
        self.os.write_i64(i)
    }
}

impl<W: Write> Display for OutputStreamIndexOutput<W>
where
    W: Write,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.resource_description)
    }
}

impl<W: Write> IndexOutput for OutputStreamIndexOutput<W>
where
    W: Write,
{
    fn get_file_pointer(&self) -> i64 {
        self.bytes_written
    }

    fn get_checksum(&mut self) -> u64 {
        self.os.checksum = self.os.hasher.clone().finalize();
        self.os.checksum as u64
    }

    fn get_name(&self) -> &str {
        self.name.as_str()
    }
}

pub struct XBufferedOutputStream<W: Write> {
    inner: BufWriter<W>,
    hasher: Hasher,
    checksum: u32,
}

impl<W: Write> XBufferedOutputStream<W> {
    pub fn new(inner: W, buffer_size: i32) -> Self {
        Self {
            inner: BufWriter::with_capacity(buffer_size as usize, inner),
            hasher: Hasher::new(),
            checksum: 0,
        }
    }

    pub fn checksum(&self) -> u32 {
        self.checksum
    }

    //TODO: If frequent checksum calculations become a bottleneck, we might consider
    // caching a batch of data and then calculating the checksum.
    fn update_checksum(&mut self, buf: &[u8]) {
        self.hasher.update(buf);
    }

    pub fn write_u8(&mut self, value: u8) -> Result<()> {
        self.inner.write_u8(value)?;
        self.update_checksum(&[value]);
        Ok(())
    }

    pub fn write_bytes(&mut self, buf: &[u8]) -> Result<()> {
        debug_assert!(buf.len() <= u32::MAX as usize);
        self.inner.write_all(buf)?;
        self.update_checksum(buf);
        Ok(())
    }

    pub fn write_i16(&mut self, value: i16) -> Result<()> {
        self.inner.write_i16::<LittleEndian>(value)?;
        self.update_checksum(&value.to_le_bytes());
        Ok(())
    }

    pub fn write_i32(&mut self, value: i32) -> Result<()> {
        self.inner.write_i32::<LittleEndian>(value)?;
        self.update_checksum(&value.to_le_bytes());
        Ok(())
    }

    pub fn write_i64(&mut self, value: i64) -> Result<()> {
        self.inner.write_i64::<LittleEndian>(value)?;
        self.update_checksum(&value.to_le_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::store::data_output::DataOutput;
    use crate::store::index_output::IndexOutput;
    use crate::store::output_stream_index_output::OutputStreamIndexOutput;
    use crate::util::error::lucene_error::Result;
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::Cursor;

    #[allow(dead_code)] // for quick search
    struct TestOutputStreamIndexOutput;

    #[test]
    fn test_data_types() -> Result<()> {
        for offset in 0..12 {
            do_test_data_types(offset)?;
        }
        Ok(())
    }

    fn do_test_data_types(offset: usize) -> Result<()> {
        use crc32fast::Hasher;

        let mut buffer = Vec::new();
        {
            let mut out =
                OutputStreamIndexOutput::new("test", "test", &mut buffer, 12)?;
            let mut hasher = Hasher::new();
            for i in 0..offset {
                out.write_byte(i as u8)?;
                hasher.update(&[i as u8]);
            }
            out.write_short(12345)?;
            hasher.update(&12345u16.to_le_bytes());

            out.write_int(1234567890)?;
            hasher.update(&1234567890u32.to_le_bytes());

            out.write_long(1234567890123456789)?;
            hasher.update(&1234567890123456789u64.to_le_bytes());
            assert_eq!(out.get_file_pointer(), (offset + 14) as i64);
            assert_eq!(
                out.get_checksum() as u32,
                hasher.finalize(),
                "Checksum mismatch"
            );
        }

        let mut reader = Cursor::new(buffer);
        for i in 0..offset {
            assert_eq!(reader.read_u8()?, i as u8);
        }

        assert_eq!(reader.read_i16::<LittleEndian>()?, 12345);
        assert_eq!(reader.read_i32::<LittleEndian>()?, 1234567890);
        assert_eq!(reader.read_i64::<LittleEndian>()?, 1234567890123456789);
        assert_eq!(reader.position() as usize, reader.get_ref().len());

        Ok(())
    }

    #[test]
    fn test_write_exceeding_buffer() -> Result<()> {
        use crc32fast::Hasher;

        let buffer_size = 8;
        let large_data: Vec<u8> = (0..16).collect();
        let mut buffer = Vec::new();
        {
            let mut out = OutputStreamIndexOutput::new(
                "test",
                "test",
                &mut buffer,
                buffer_size,
            )?;

            let mut hasher = Hasher::new();

            out.write_bytes_range(&large_data, 0, large_data.len() as i32)?;
            hasher.update(&large_data);

            assert_eq!(out.get_file_pointer(), large_data.len() as i64);
            assert_eq!(
                out.get_checksum(),
                hasher.finalize() as u64,
                "Checksum mismatch"
            );
        }

        assert_eq!(buffer, large_data);

        Ok(())
    }
    #[test]
    fn test_multiple_writes_with_checksum() -> Result<()> {
        use crc32fast::Hasher;

        let mut buffer = Vec::new();
        let combined_data: Vec<u8>;
        {
            let mut out =
                OutputStreamIndexOutput::new("test", "test", &mut buffer, 8)?;

            let data1 = b"Hello";
            let data2 = b"World";
            let mut hasher = Hasher::new();

            out.write_bytes_range(data1, 0, data1.len() as i32)?;
            hasher.update(data1);
            let sum1 = out.get_checksum();
            out.write_bytes_range(data2, 0, data2.len() as i32)?;
            hasher.update(data2);
            let sum2 = out.get_checksum();
            assert_ne!(sum1, sum2, "Checksum mismatch");

            assert_eq!(
                out.get_checksum(),
                hasher.finalize() as u64,
                "Checksum mismatch"
            );
            combined_data = [data1.as_slice(), data2.as_slice()].concat();
        }

        assert_eq!(buffer, combined_data);

        Ok(())
    }

    trait MyTrait {
        fn method_a(&self) {
            println!("Default implementation of method_a");
        }
    }

    struct MyStruct;

    impl MyTrait for MyStruct {
        fn method_a(&self) {}
    }

    #[test]
    fn main() {
        let instance = MyStruct;

        println!("Calling method_a:");
        instance.method_a();
    }
}
