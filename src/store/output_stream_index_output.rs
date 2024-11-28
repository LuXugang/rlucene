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
use crate::util::error::data_io_error_enum::DataIOError;
use byteorder::{LittleEndian, WriteBytesExt};
use crc32fast::Hasher;
use std::fmt::{Display, Formatter};
use std::io::{BufWriter, Write};

/** Implementation class for buffered `IndexOutput` that writes to an `OutputStream` */
pub struct OutputStreamIndexOutput<W: Write> {
    os: XBufferedOutputStream<W>,
    bytes_written: i64,
    name: String,
    resource_description: String,
}
impl<W: Write> OutputStreamIndexOutput<W> {
    /**
     * Creates a new `OutputStreamIndexOutput` with the given buffer size.
     *
     * bufferSize :recommend value： 8kb
     */
    pub fn new(resource_description: &str, name: &str, inner: W, buffer_size: usize) -> Self {
        let os = XBufferedOutputStream::new(inner, buffer_size);
        Self {
            os,
            bytes_written: 0,
            name: name.to_string(),
            resource_description: resource_description.to_string(),
        }
    }
    pub fn close() {}
}

impl<W: Write> DataOutput for OutputStreamIndexOutput<W> {
    fn write_byte(&mut self, b: u8) -> Result<(), DataIOError> {
        self.bytes_written += 1;
        self.os.write_u8(b)
    }

    fn write_bytes_range(
        &mut self,
        b: &[u8],
        offset: usize,
        length: usize,
    ) -> Result<(), DataIOError> {
        let end = offset + length;
        self.bytes_written += length as i64;
        self.os.write_bytes(&b[offset..end])
    }

    fn write_int(&mut self, i: i32) -> Result<(), DataIOError> {
        self.bytes_written += 4;
        self.os.write_i32(i)
    }

    fn write_short(&mut self, i: i16) -> Result<(), DataIOError> {
        self.bytes_written += 2;
        self.os.write_i16(i)
    }

    fn write_long(&mut self, i: i64) -> Result<(), DataIOError> {
        self.bytes_written += 8;
        self.os.write_i64(i)
    }
}

impl<W: Write> Display for OutputStreamIndexOutput<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.resource_description)
    }
}

impl<W: Write> IndexOutput for OutputStreamIndexOutput<W> {
    fn get_file_pointer(&self) -> i64 {
        self.bytes_written
    }

    fn get_check_sum(&mut self) -> i64 {
        self.os.checksum = self.os.hasher.clone().finalize();
        self.os.checksum as i64
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
    pub fn new(inner: W, buffer_size: usize) -> Self {
        Self {
            inner: BufWriter::with_capacity(buffer_size, inner),
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

    pub fn write_u8(&mut self, value: u8) -> Result<(), DataIOError> {
        self.inner.write_u8(value)?;
        self.update_checksum(&[value]);
        Ok(())
    }

    pub fn write_bytes(&mut self, buf: &[u8]) -> Result<(), DataIOError> {
        self.flush_if_needed(buf.len())?;
        if buf.len() > self.inner.capacity() {
            self.inner.get_mut().write_all(buf)?;
        } else {
            self.inner.write_all(buf)?;
        }
        self.update_checksum(buf);
        Ok(())
    }

    pub fn write_i16(&mut self, value: i16) -> Result<(), DataIOError> {
        self.inner.write_i16::<LittleEndian>(value)?;
        self.update_checksum(&value.to_le_bytes());
        Ok(())
    }

    pub fn write_i32(&mut self, value: i32) -> Result<(), DataIOError> {
        self.inner.write_i32::<LittleEndian>(value)?;
        self.update_checksum(&value.to_le_bytes());
        Ok(())
    }

    pub fn write_i64(&mut self, value: i64) -> Result<(), DataIOError> {
        self.inner.write_i64::<LittleEndian>(value)?;
        self.update_checksum(&value.to_le_bytes());
        Ok(())
    }

    pub fn flush_if_needed(&mut self, len: usize) -> Result<(), DataIOError> {
        if len + self.inner.buffer().len() > self.inner.capacity() {
            self.inner.flush()?;
        }
        Ok(())
    }
}
