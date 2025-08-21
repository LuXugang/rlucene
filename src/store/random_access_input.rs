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
use crate::util::error::lucene_error::Result;

/// Random Access Index API. Unlike [`IndexInput`](crate::store::IndexInput),
/// this has no concept of file position; all reads are absolute. However, like
/// `IndexInput`, it is only intended for use by a single thread.
pub trait RandomAccessInput {
    /// The number of bytes in the file.
    fn length(&self) -> i64;
    /// Reads a byte at the given position in the file
    fn read_byte(&mut self, pos: i64) -> Result<u8>;
    /// Reads a specified number of bytes starting at a given position into an
    /// array at the specified offset.
    fn read_bytes(&mut self, pos: i64, buf: &mut [u8], offset: i32, len: i32) -> Result<()> {
        for i in 0..len {
            buf[(offset + i) as usize] = self.read_byte(pos + i as i64)?;
        }
        Ok(())
    }
    ///  Reads an i16 (LE byte order) at the given position in the file.
    fn read_short(&mut self, pos: i64) -> Result<i16>;
    /// Reads an i32 (LE byte order) at the given position in the file.
    fn read_int(&mut self, pos: i64) -> Result<i32>;
    /// Reads a long (LE byte order) at the given position in the file.
    fn read_long(&mut self, pos: i64) -> Result<i64>;
    ///  Prefetch data in the background.
    fn prefetch(&mut self, pos: i64, len: i64) -> Result<()>;
}

pub enum Either2RandomAccessInput<A, B> {
    A(A),
    B(B),
}
impl<A, B> RandomAccessInput for Either2RandomAccessInput<A, B>
where
    A: RandomAccessInput,
    B: RandomAccessInput,
{
    fn length(&self) -> i64 {
        match self {
            Either2RandomAccessInput::A(f) => f.length(),
            Either2RandomAccessInput::B(s) => s.length(),
        }
    }

    fn read_byte(&mut self, pos: i64) -> Result<u8> {
        match self {
            Either2RandomAccessInput::A(f) => f.read_byte(pos),
            Either2RandomAccessInput::B(s) => s.read_byte(pos),
        }
    }

    fn read_bytes(&mut self, pos: i64, buf: &mut [u8], offset: i32, len: i32) -> Result<()> {
        match self {
            Either2RandomAccessInput::A(f) => f.read_bytes(pos, buf, offset, len),
            Either2RandomAccessInput::B(s) => s.read_bytes(pos, buf, offset, len),
        }
    }

    fn read_short(&mut self, pos: i64) -> Result<i16> {
        match self {
            Either2RandomAccessInput::A(f) => f.read_short(pos),
            Either2RandomAccessInput::B(s) => s.read_short(pos),
        }
    }

    fn read_int(&mut self, pos: i64) -> Result<i32> {
        match self {
            Either2RandomAccessInput::A(f) => f.read_int(pos),
            Either2RandomAccessInput::B(s) => s.read_int(pos),
        }
    }

    fn read_long(&mut self, pos: i64) -> Result<i64> {
        match self {
            Either2RandomAccessInput::A(f) => f.read_long(pos),
            Either2RandomAccessInput::B(s) => s.read_long(pos),
        }
    }

    fn prefetch(&mut self, pos: i64, len: i64) -> Result<()> {
        match self {
            Either2RandomAccessInput::A(f) => f.prefetch(pos, len),
            Either2RandomAccessInput::B(s) => s.prefetch(pos, len),
        }
    }
}
