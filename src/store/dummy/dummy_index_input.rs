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
use crate::store::{DataInput, IndexInput};
use crate::util::error::lucene_error::Result;

pub struct DummyIndexInput;

impl DataInput for DummyIndexInput {
    fn read_byte(&mut self) -> Result<u8> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn read_bytes(&mut self, _b: &mut [u8], _offset: i32, _len: i32) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn skip_bytes(&mut self, _num_bytes: i64) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl Display for DummyIndexInput {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl crate::util::clone::TryClone for DummyIndexInput {
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl IndexInput for DummyIndexInput {
    fn get_file_pointer(&self) -> i64 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn seek(&mut self, _pos: i64) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn length(&self) -> i64 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type Slice = DummyIndexInput;

    fn slice(
        &self,
        _slice_description: &str,
        _offset: i64,
        _length: i64,
    ) -> Result<DummyIndexInput> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type RandomAccessSlice = DummyIndexInput;

    fn random_access_slice(&self, _offset: i64, _length: i64) -> Result<DummyIndexInput> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
impl RandomAccessInput for DummyIndexInput {
    fn length(&self) -> i64 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn read_byte(&mut self, _pos: i64) -> Result<u8> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn read_short(&mut self, _pos: i64) -> Result<i16> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn read_int(&mut self, _pos: i64) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn read_long(&mut self, _pos: i64) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn prefetch(&mut self, _pos: i64, _len: i64) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
