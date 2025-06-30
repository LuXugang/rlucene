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

use crate::store::{DataOutput, IndexOutput};
use crate::util::error::lucene_error::Result;

pub struct DummyIndexOutput;

impl DataOutput for DummyIndexOutput {
    fn write_byte(&mut self, _b: u8) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn write_bytes_range(&mut self, _b: &[u8], _offset: i32, _length: i32) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl Display for DummyIndexOutput {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl IndexOutput for DummyIndexOutput {
    fn get_file_pointer(&self) -> i64 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_checksum(&mut self) -> u64 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_name(&self) -> &str {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
