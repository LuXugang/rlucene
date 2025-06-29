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

use crate::store::random_access_input::RandomAccessInput;
use crate::store::DataInput;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::fst::BytesReader;

/// Implements reverse read from a RandomAccessInput.
pub struct ReverseRandomAccessReader<R>
where
    R: RandomAccessInput,
{
    input: R,
    pos: i64,
}
#[allow(unused)]
impl<R> ReverseRandomAccessReader<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: R) -> Self {
        Self { input, pos: 0 }
    }
}

impl<R> DataInput for ReverseRandomAccessReader<R>
where
    R: RandomAccessInput,
{
    fn read_byte(&mut self) -> Result<u8> {
        let b = self.input.read_byte(self.pos)?;
        self.pos -= 1;
        Ok(b)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
        let offset = offset as usize;
        let len = len as usize;
        let mut i = offset;
        let end = offset + len;
        while i < end {
            b[i] = self.input.read_byte(self.pos)?;
            self.pos -= 1;
            i += 1;
        }
        Ok(())
    }

    fn skip_bytes(&mut self, count: i64) -> Result<()> {
        self.pos -= count;
        Ok(())
    }
}

impl<R> BytesReader for ReverseRandomAccessReader<R>
where
    R: RandomAccessInput,
{
    fn get_position(&self) -> i64 {
        self.pos
    }

    fn set_position(&mut self, pos: i64) {
        self.pos = pos;
    }
}

impl<R> Display for ReverseRandomAccessReader<R>
where
    R: RandomAccessInput,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReverseRandomAccessReader")
    }
}
