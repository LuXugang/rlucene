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
use crate::store::index_input::IndexInput;
use crate::util::error::data_io_error_enum::DataIOError;
use std::fmt::{Display, Formatter};

/**
 * Random Access Index API. Unlike `IndexInput`, this has no concept of file position, all
 * reads are absolute. However, like `IndexInput`, it is only intended for use by a single thread.
 */
pub trait RandomAccessInput {
    /** The number of bytes in the file. */
    fn length(&self) -> u64;
    /**
     * Reads a byte at the given position in the file
     */
    fn read_byte(&mut self, pos: u64) -> Result<u8, DataIOError>;
    /**
     * Reads a specified number of bytes starting at a given position into an array at the specified
     * offset.
     */
    fn read_bytes(
        &mut self,
        pos: u64,
        buf: &mut [u8],
        offset: usize,
        len: usize,
    ) -> Result<(), DataIOError> {
        for i in 0..len {
            buf[offset + i] = self.read_byte(pos + i as u64)?;
        }
        Ok(())
    }
    /**
     * Reads a i16 (LE byte order) at the given position in the file
     */
    fn read_short(&mut self, pos: u64) -> Result<i16, DataIOError>;
    /**
     * Reads an i32 (LE byte order) at the given position in the file
     */
    fn read_int(&mut self, pos: u64) -> Result<i32, DataIOError>;
    /**
     * Reads a long (LE byte order) at the given position in the file
     */
    fn read_long(&mut self, pos: u64) -> Result<i64, DataIOError>;
    /**
     * Prefetch data in the background.
     *
     */
    fn pre_fetch(&mut self, pos: u64, len: u64) -> Result<(), DataIOError>;
}
pub struct DefaultRandomAccessInput<T: IndexInput> {
    slice: T,
    length: u64,
}
impl<T: IndexInput> DefaultRandomAccessInput<T> {
    fn new(slice: T, length: u64) -> Self {
        Self { slice, length }
    }
}
impl<T: IndexInput> RandomAccessInput for DefaultRandomAccessInput<T> {
    fn length(&self) -> u64 {
        debug_assert!(self.slice.length() == self.length);
        self.slice.length()
    }

    fn read_byte(&mut self, pos: u64) -> Result<u8, DataIOError> {
        self.slice.seek(pos)?;
        self.slice.read_byte()
    }

    fn read_bytes(
        &mut self,
        pos: u64,
        buf: &mut [u8],
        offset: usize,
        len: usize,
    ) -> Result<(), DataIOError> {
        self.slice.seek(pos)?;
        self.slice.read_bytes(buf, offset, len)
    }

    fn read_short(&mut self, pos: u64) -> Result<i16, DataIOError> {
        self.slice.seek(pos)?;
        self.slice.read_short()
    }

    fn read_int(&mut self, pos: u64) -> Result<i32, DataIOError> {
        self.slice.seek(pos)?;
        self.slice.read_int()
    }

    fn read_long(&mut self, pos: u64) -> Result<i64, DataIOError> {
        self.slice.seek(pos)?;
        self.slice.read_long()
    }

    fn pre_fetch(&mut self, pos: u64, len: u64) -> Result<(), DataIOError> {
        self.slice.prefetch(pos, len)
    }
}
impl<T: IndexInput> Display for DefaultRandomAccessInput<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
