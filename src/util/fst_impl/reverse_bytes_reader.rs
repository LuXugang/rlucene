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
use std::rc::Rc;

use crate::store::DataInput;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::fst::BytesReader;

/// Reads in reverse from a single byte array.
pub struct ReverseBytesReader {
    bytes: Rc<Vec<u8>>,
    pos: i32,
}

#[allow(unused)]
impl ReverseBytesReader {
    pub fn new(bytes: Rc<Vec<u8>>) -> Self {
        Self { bytes, pos: 0 }
    }
}

impl DataInput for ReverseBytesReader {
    fn read_byte(&mut self) -> Result<u8> {
        let b = self.bytes[self.pos as usize];
        self.pos -= 1;
        Ok(b)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
        let offset = offset as usize;
        for i in 0..len as usize {
            b[offset + i] = self.bytes[self.pos as usize];
            self.pos -= 1;
        }
        Ok(())
    }

    fn skip_bytes(&mut self, count: i64) -> Result<()> {
        self.pos -= count as i32;
        Ok(())
    }
}

impl BytesReader for ReverseBytesReader {
    fn get_position(&self) -> i64 {
        self.pos as i64
    }

    fn set_position(&mut self, pos: i64) {
        self.pos = pos as i32;
    }
}

impl std::fmt::Display for ReverseBytesReader {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ReverseBytesReader")
    }
}
