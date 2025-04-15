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
use crate::store::{DataInput, DataOutput};
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::fst::BytesReader;
use crate::util::fst_impl::fst_compiler::fst_compiler_util;
use crate::util::fst_impl::fst_reader::FstReader;
use crate::util::fst_impl::read_write_data_output::{BytesReaderEnum, ReadWriteDataOutput};
use crate::util::fst_impl::reverse_bytes_reader::ReverseBytesReader;
use std::fmt::{Display, Formatter};

/// Provides storage of finite state machine (FST), using byte array or byte store allocated on heap.
pub struct OnHeapFSTStore {
    /// A [`ReadWriteDataOutput`], used during reading when the FST is very large (more than 1 GB).
    /// If the FST is less than 1 GB then bytesArray is set instead.
    data_output: Option<ReadWriteDataOutput>,
    ///  Used at read time when the FST fits into a single byte array.
    bytes_array: Option<Vec<u8>>,
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
            // FST is big: we need multiple pages
            let mut data_output = fst_compiler_util::get_on_heap_reader_writer(max_block_bits)?;
            data_output.copy_bytes(input, num_bytes)?;
            data_output.freeze()?;
            Ok(Self {
                data_output: Some(data_output),
                bytes_array: None,
            })
        } else {
            // FST fits into a single block: use Vec<u8> directly
            let mut bytes_array = vec![0u8; num_bytes as usize];
            debug_assert!(bytes_array.len() <= i32::MAX as usize);
            let len = bytes_array.len() as i32;
            input.read_bytes(&mut bytes_array, 0, len)?;
            Ok(Self {
                data_output: None, // or an empty fallback
                bytes_array: Some(bytes_array),
            })
        }
    }
}

impl Accountable for OnHeapFSTStore {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

impl FstReader for OnHeapFSTStore {
    type FstBytesReader = FstBytesReaderEnum;

    fn get_reverse_bytes_reader(&mut self) -> Result<Self::FstBytesReader> {
        if self.bytes_array.is_some() {
            Ok(FstBytesReaderEnum::Reverse(ReverseBytesReader::new(
                self.bytes_array.take().unwrap(),
            )))
        } else {
            Ok(FstBytesReaderEnum::Bytes(
                self.data_output
                    .as_mut()
                    .unwrap()
                    .get_reverse_bytes_reader()?,
            ))
        }
    }

    fn write_to(&mut self, out: &mut impl DataOutput) -> Result<()> {
        if self.data_output.is_some() {
            self.data_output.as_mut().unwrap().write_to(out)?;
        } else {
            match &mut self.bytes_array {
                Some(bytes_array) => {
                    let len = bytes_array.len();
                    debug_assert!(len <= i32::MAX as usize);
                    out.write_bytes_range(bytes_array, 0, len as i32)?;
                }
                None => {
                    return Err(LuceneError::illegal_state(
                        "data_output is None".to_string(),
                    ))
                }
            }
        }
        Ok(())
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
