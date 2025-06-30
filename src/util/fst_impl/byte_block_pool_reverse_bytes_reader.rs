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

use crate::store::DataInput;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::fst::BytesReader;
use crate::util::{ByteBlockPool, CounterEnumBorrow};

/// Reads in reverse from a ByteBlockPool.
pub struct ByteBlockPoolReverseBytesReader {
    pub(crate) buf: ByteBlockPool<CounterEnumBorrow>,
    // the difference between the FST node address and the hash table copied
    // node address
    pos_delta: i64,
    pos: i64,
}
impl<'a> ByteBlockPoolReverseBytesReader {
    pub fn new(buf: ByteBlockPool<CounterEnumBorrow>) -> Self {
        Self {
            buf,
            pos_delta: 0,
            pos: 0,
        }
    }
    pub fn set_pos_delta(&mut self, pos_delta: i64) {
        self.pos_delta = pos_delta;
    }
}

impl DataInput for ByteBlockPoolReverseBytesReader {
    fn read_byte(&mut self) -> Result<u8> {
        let b = self.buf.read_byte(self.pos);
        self.pos -= 1;
        Ok(b)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
        let offset = offset as usize;
        let len = len as usize;
        for i in 0..len {
            b[offset + i] = self.buf.read_byte(self.pos);
            self.pos -= 1;
        }
        Ok(())
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        self.pos -= num_bytes;
        Ok(())
    }
}

impl Display for ByteBlockPoolReverseBytesReader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ByteBlockPoolReverseBytesReader")
    }
}

impl BytesReader for ByteBlockPoolReverseBytesReader {
    fn get_position(&self) -> i64 {
        self.pos + self.pos_delta
    }

    fn set_position(&mut self, pos: i64) {
        self.pos = pos - self.pos_delta;
    }
}
