/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::fmt::{Display, Formatter};
use std::rc::Rc;

use crate::store::{ByteBuffersDataOutput, DataInput, DataOutput};
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::fst::BytesReader;
use crate::util::fst_impl::fst_reader::FstReader;
use crate::util::fst_impl::reverse_bytes_reader::ReverseBytesReader;
/// An adapter struct to use [`ByteBuffersDataOutput`] as a
/// [`FSTReader`](FstReader). It allows the FST to be readable immediately after
/// writing
#[derive(Default)]
pub struct ReadWriteDataOutput {
    pub data_output: ByteBuffersDataOutput,
    pub block_bits: i32,
    pub block_size: i32,
    pub block_mask: i32,
    pub byte_buffers: Option<Rc<Vec<Vec<u8>>>>,
    pub byte_buffer: Option<Rc<Vec<u8>>>,
    pub frozen: bool,
    /// Indicates whether the byte_buffer/byte_buffers have been initialized.
    pub finish: bool,
}

impl ReadWriteDataOutput {
    pub(crate) fn new(block_bits: i32) -> Result<Self> {
        let block_size = 1 << block_bits;
        let block_mask = block_size - 1;
        let data_output = ByteBuffersDataOutput::new_with_reuse(block_bits, block_bits, false)?;
        Ok(Self {
            data_output,
            block_bits,
            block_size,
            block_mask,
            byte_buffers: None,
            byte_buffer: None,
            frozen: false,
            finish: false,
        })
    }

    pub fn freeze(&mut self) -> Result<()> {
        self.frozen = true;
        // We only move the ownership of self.data_output when get_reverse_bytes_reader
        // is called, so that the write_to method can still function correctly.
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

    fn get_reverse_bytes_reader(&self) -> Result<Self::FstBytesReader> {
        if !self.finish {
            return Err(LuceneError::illegal_state("Call ReadWriteDataOutput#init_byte_buffer before Call ReadWriteDataOutput#get_reverse_bytes_reader"));
        }
        if self.byte_buffers.is_some() && self.byte_buffer.is_none() {
            let buffers = self.byte_buffers.as_ref().unwrap().clone();
            Ok(BytesReaderEnum::Impl(BytesReaderImpl::new(
                buffers,
                self.block_bits,
                self.block_size,
                self.block_mask,
            )))
        } else if self.byte_buffer.is_some() && self.byte_buffers.is_none() {
            let buffer = self.byte_buffer.as_ref().unwrap().clone();
            Ok(BytesReaderEnum::ReverseBytes(ReverseBytesReader::new(
                buffer,
            )))
        } else {
            Err(LuceneError::illegal_state(
                "Only one buffer is some".to_string(),
            ))
        }
    }

    fn write_to(&self, out: &mut impl DataOutput) -> Result<()> {
        debug_assert!(!self.finish);
        // Note: After calling get_reverse_bytes_reader, the ownership of data_output
        // will be moved.
        self.data_output.copy_to(out)
    }

    fn init_reader(&mut self) {
        self.finish = true;
        if self.byte_buffer.is_none() && self.byte_buffers.is_none() {
            let (_, byte_buffers_raw) = self.data_output.to_buffer_list_owner();
            let mut data: Vec<Vec<u8>> = byte_buffers_raw
                .into_iter()
                .map(|b| b.into_inner())
                .collect();

            if data.len() == 1 {
                self.byte_buffer = Some(Rc::new(data.remove(0)));
            } else {
                self.byte_buffers = Some(Rc::new(data));
            }
        }
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
    byte_buffers: Rc<Vec<Vec<u8>>>,
    next_buffer: i32,
    next_read: i32,
    current: i32,
    block_size: i32,
    block_bits: i32,
    block_mask: i32,
}

impl BytesReaderImpl {
    pub fn new(
        byte_buffers: Rc<Vec<Vec<u8>>>,
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
