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

use crate::store::{DataInput, DataOutput};
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::fst::BytesReader;
use crate::util::fst_impl::fst_compiler::fst_compiler_util;
use crate::util::fst_impl::fst_reader::FstReader;
use crate::util::fst_impl::read_write_data_output::{BytesReaderEnum, ReadWriteDataOutput};
use crate::util::fst_impl::reverse_bytes_reader::ReverseBytesReader;
/// Provides storage of finite state machine (FST), using byte array or byte
/// store allocated on heap.
pub struct OnHeapFSTStore {
    /// A [`ReadWriteDataOutput`], used during reading when the FST is very
    /// large (more than 1 GB). If the FST is less than 1 GB then
    /// bytesArray is set instead.
    data_output: Option<ReadWriteDataOutput>,
    ///  Used at read time when the FST fits into a single byte array.
    bytes_array: Option<Rc<Vec<u8>>>,
}

impl OnHeapFSTStore {
    pub fn new(max_block_bits: i32, input: &mut impl DataInput, num_bytes: i64) -> Result<Self> {
        if !(1..=30).contains(&max_block_bits) {
            return Err(LuceneError::illegal_argument(format!(
                "max_block_bits should be in 1..=30; got {}",
                max_block_bits
            )));
        }

        if num_bytes > (1_i64 << max_block_bits) {
            let mut data_output = fst_compiler_util::get_on_heap_reader_writer(max_block_bits)?;
            data_output.copy_bytes(input, num_bytes)?;
            data_output.freeze()?;
            Ok(Self {
                data_output: Some(data_output),
                bytes_array: None,
            })
        } else {
            let mut bytes_array = vec![0u8; num_bytes as usize];
            let len = bytes_array.len() as i32;
            input.read_bytes(&mut bytes_array, 0, len)?;
            Ok(Self {
                data_output: None,
                bytes_array: Some(Rc::new(bytes_array)),
            })
        }
    }
}
impl Accountable for OnHeapFSTStore {
    fn ram_bytes_used(&self) -> Result<i64> {
        Ok(0)
    }
}

impl FstReader for OnHeapFSTStore {
    type FstBytesReader = FstBytesReaderEnum;

    fn get_reverse_bytes_reader(&self) -> Result<Self::FstBytesReader> {
        if let Some(bytes_array) = &self.bytes_array {
            return Ok(FstBytesReaderEnum::Reverse(ReverseBytesReader::new(
                bytes_array.clone(),
            )));
        }

        if let Some(data_output) = &self.data_output {
            Ok(FstBytesReaderEnum::Bytes(
                data_output.get_reverse_bytes_reader()?,
            ))
        } else {
            Err(LuceneError::illegal_state(
                "OnHeapFSTStore has neither bytes_array nor data_output".to_string(),
            ))
        }
    }
    // Note: After calling get_reverse_bytes_reader, the ownership of data_output
    // will be moved.
    fn write_to(&self, out: &mut impl DataOutput) -> Result<()> {
        if let Some(data_output) = &self.data_output {
            data_output.write_to(out)?;
        } else if let Some(bytes_array) = &self.bytes_array {
            let len = bytes_array.len();
            debug_assert!(len <= i32::MAX as usize);
            out.write_bytes_range(bytes_array, 0, len as i32)?;
        } else {
            return Err(LuceneError::illegal_state(
                "OnHeapFSTStore is empty".to_string(),
            ));
        }
        Ok(())
    }

    fn init_reader(&mut self) {
        if let Some(data_output) = &mut self.data_output {
            data_output.init_reader();
        }
    }
}
pub enum FstBytesReaderEnum {
    Reverse(ReverseBytesReader),
    Bytes(BytesReaderEnum),
}

impl DataInput for FstBytesReaderEnum {
    fn read_byte(&mut self) -> Result<u8> {
        match self {
            FstBytesReaderEnum::Reverse(reader) => reader.read_byte(),
            FstBytesReaderEnum::Bytes(reader) => reader.read_byte(),
        }
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
        match self {
            FstBytesReaderEnum::Reverse(reader) => reader.read_bytes(b, offset, len),
            FstBytesReaderEnum::Bytes(reader) => reader.read_bytes(b, offset, len),
        }
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        match self {
            FstBytesReaderEnum::Reverse(reader) => reader.skip_bytes(num_bytes),
            FstBytesReaderEnum::Bytes(reader) => reader.skip_bytes(num_bytes),
        }
    }
}

impl Display for FstBytesReaderEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FstBytesReaderEnum::Reverse(reader) => write!(f, "{}", reader),
            FstBytesReaderEnum::Bytes(reader) => write!(f, "{}", reader),
        }
    }
}

impl BytesReader for FstBytesReaderEnum {
    fn get_position(&self) -> i64 {
        match self {
            FstBytesReaderEnum::Reverse(reader) => reader.get_position(),
            FstBytesReaderEnum::Bytes(reader) => reader.get_position(),
        }
    }

    fn set_position(&mut self, pos: i64) {
        match self {
            FstBytesReaderEnum::Reverse(reader) => reader.set_position(pos),
            FstBytesReaderEnum::Bytes(reader) => reader.set_position(pos),
        }
    }
}
