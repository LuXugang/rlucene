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
use std::io::Cursor;

use crate::store::{BufferedIndexInput, BufferedIndexInputBase};
use crate::util::error::lucene_error::Result;

pub struct DummyBufferedIndexInputBase;

impl crate::util::clone::TryClone for DummyBufferedIndexInputBase {
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl BufferedIndexInputBase for DummyBufferedIndexInputBase {
    fn seek_internal(&mut self, _pos: i64) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn read_internal(
        &mut self,
        _b: &mut Cursor<Vec<u8>>,
        _len: i64,
        _file_pointer: i64,
    ) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type Slice = BufferedIndexInput<DummyBufferedIndexInputBase>;

    fn slice(&self, _slice_description: &str, _offset: i64, _length: i64) -> Result<Self::Slice> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn length(&self) -> i64 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
