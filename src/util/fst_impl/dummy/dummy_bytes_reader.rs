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
use crate::util::error::lucene_error;
use crate::util::error::lucene_error::LuceneError;
use crate::util::fst_impl::fst::BytesReader;

pub struct DummyBytesReader;

impl DataInput for DummyBytesReader {
    fn read_byte(&mut self) -> lucene_error::Result<u8> {
        Err(LuceneError::unsupported_operation(
            "DummyBytesReader does not support reading bytes".to_string(),
        ))
    }

    fn read_bytes(&mut self, _b: &mut [u8], _offset: i32, _len: i32) -> lucene_error::Result<()> {
        Err(LuceneError::unsupported_operation(
            "DummyBytesReader does not support reading bytes".to_string(),
        ))
    }

    fn skip_bytes(&mut self, _num_bytes: i64) -> lucene_error::Result<()> {
        Err(LuceneError::unsupported_operation(
            "DummyBytesReader does not support skipping bytes".to_string(),
        ))
    }
}

impl Display for DummyBytesReader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DummyBytesReader")
    }
}

impl BytesReader for DummyBytesReader {
    fn get_position(&self) -> i64 {
        0
    }

    fn set_position(&mut self, _pos: i64) {}
}
