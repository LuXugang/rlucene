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
use crate::store::{ByteBuffersDataOutput, DataInput, DataOutput};
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::fst::BytesReader;
use crate::util::fst_impl::fst_reader::FstReader;
use crate::util::fst_impl::reverse_bytes_reader::ReverseBytesReader;
use std::fmt::{Display, Formatter};
use std::io::Cursor;
/// An adapter class to use [`ByteBuffersDataOutput`] as a [`FSTReader`](FstReader). It allows the FST
/// to be readable immediately after writing
pub struct ReadWriteDataOutput {
    pub data_output: ByteBuffersDataOutput,
    pub block_bits: i32,
    pub block_size: i32,
    pub block_mask: i32,
    pub byte_buffers: Option<Vec<Cursor<Vec<u8>>>>,
    /// Whether the data output is frozen
    pub frozen: bool,
}
impl ReadWriteDataOutput {
    pub(crate) fn new(block_bits: i32) -> Self {
        let block_size = 1 << block_bits;
        let block_mask = block_size - 1;
        let data_output = ByteBuffersDataOutput::new();
        Self {
            data_output,
            block_bits,
            block_size,
            block_mask,
            byte_buffers: None,
            frozen: false,
        }
    }
    pub fn freeze(&mut self) -> Result<()> {
        self.frozen = true;

        // this operation is costly, so we want to compute it once and cache
        let (_, byte_buffers) = self.data_output.to_buffer_list_owner();
        self.byte_buffers = Some(byte_buffers);
        Ok(())
    }
}

impl Accountable for ReadWriteDataOutput {
    fn ram_bytes_used(&self) -> Result<i64> {
        self.data_output.ram_bytes_used()
    }
}

impl FstReader for ReadWriteDataOutput {
    type FstBytesReader = BytesReaderEnum;

    fn get_reverse_bytes_reader(&mut self) -> Result<Self::FstBytesReader> {
        debug_assert!(self.byte_buffers.is_some());
        if self.byte_buffers.is_none() {
            return Err(LuceneError::illegal_state(
                "byte_buffers is None".to_string(),
            ));
        }
        let byte_buffers = self.byte_buffers.take().unwrap();
        let mut data: Vec<Vec<u8>> = byte_buffers.into_iter().map(|b| b.into_inner()).collect();
        if data.len() == 1 {
            Ok(BytesReaderEnum::ReverseBytes(ReverseBytesReader::new(
                std::mem::take(&mut data[0]),
            )))
        } else {
            Ok(BytesReaderEnum::Impl(BytesReaderImpl::new(
                data,
                self.block_bits,
                self.block_size,
                self.block_mask,
            )))
        }
    }

    fn write_to(&mut self, out: &mut impl DataOutput) -> Result<()> {
        self.data_output.copy_to(out)
    }
}
impl DataOutput for ReadWriteDataOutput {
    fn write_byte(&mut self, b: u8) -> Result<()> {
        debug_assert!(!self.frozen);
        DataOutput::write_byte(&mut self.data_output, b)
    }

    fn write_bytes_range(&mut self, b: &[u8], offset: i32, length: i32) -> Result<()> {
        debug_assert!(!self.frozen);
        self.data_output.write_bytes_range(b, offset, length)
    }
}

pub struct BytesReaderImpl {
    byte_buffers: Vec<Vec<u8>>,
    next_buffer: i32,
    next_read: i32,
    current: i32,
    block_size: i32,
    block_bits: i32,
    block_mask: i32,
}
impl BytesReaderImpl {
    pub fn new(
        byte_buffers: Vec<Vec<u8>>,
        block_bits: i32,
        block_size: i32,
        block_mask: i32,
    ) -> Self {
        Self {
            byte_buffers,
            next_buffer: -1,
            next_read: 0,
            current: 0,
            block_size,
            block_bits,
            block_mask,
        }
    }
}

impl DataInput for BytesReaderImpl {
    fn read_byte(&mut self) -> Result<u8> {
        if self.next_read == -1 {
            self.current = self.next_buffer;
            self.next_buffer -= 1;
            self.next_read = self.block_size - 1;
        }
        let byte = &self.byte_buffers[self.current as usize][self.next_read as usize];
        self.next_read -= 1;
        Ok(*byte)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
        for i in 0..len {
            b[(offset + i) as usize] = self.read_byte()?;
        }
        Ok(())
    }

    fn skip_bytes(&mut self, count: i64) -> Result<()> {
        self.set_position(self.get_position() - count);
        Ok(())
    }
}

impl Display for BytesReaderImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReadWriteDataOutput#BytesReaderImpl({})",
            self.block_bits
        )
    }
}

impl BytesReader for BytesReaderImpl {
    fn get_position(&self) -> i64 {
        (((self.next_buffer + 1) * self.block_size) + self.next_read) as i64
    }

    fn set_position(&mut self, pos: i64) {
        let buffer_index = (pos >> self.block_bits) as i32;
        if self.next_buffer != buffer_index - 1 {
            self.next_buffer = buffer_index - 1;
            self.current = buffer_index;
        }
        self.next_read = (pos & self.block_mask as i64) as i32;
        debug_assert_eq!(
            self.get_position(),
            pos,
            "pos={} get_pos={}",
            pos,
            self.get_position()
        );
    }
}

pub enum BytesReaderEnum {
    Impl(BytesReaderImpl),
    ReverseBytes(ReverseBytesReader),
}

impl DataInput for BytesReaderEnum {
    fn read_byte(&mut self) -> Result<u8> {
        match self {
            BytesReaderEnum::Impl(reader) => reader.read_byte(),
            BytesReaderEnum::ReverseBytes(reader) => reader.read_byte(),
        }
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
        match self {
            BytesReaderEnum::Impl(reader) => reader.read_bytes(b, offset, len),
            BytesReaderEnum::ReverseBytes(reader) => reader.read_bytes(b, offset, len),
        }
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        match self {
            BytesReaderEnum::Impl(reader) => reader.skip_bytes(num_bytes),
            BytesReaderEnum::ReverseBytes(reader) => reader.skip_bytes(num_bytes),
        }
    }
}

impl Display for BytesReaderEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BytesReaderEnum::Impl(inner) => write!(f, "{}", inner),
            BytesReaderEnum::ReverseBytes(inner) => write!(f, "{}", inner),
        }
    }
}

impl BytesReader for BytesReaderEnum {
    fn get_position(&self) -> i64 {
        match self {
            BytesReaderEnum::Impl(reader) => reader.get_position(),
            BytesReaderEnum::ReverseBytes(reader) => reader.get_position(),
        }
    }

    fn set_position(&mut self, pos: i64) {
        match self {
            BytesReaderEnum::Impl(reader) => reader.set_position(pos),
            BytesReaderEnum::ReverseBytes(reader) => reader.set_position(pos),
        }
    }
}
