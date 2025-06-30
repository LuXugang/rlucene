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
use crate::store::DataOutput;
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::dummy::dummy_bytes_reader::DummyBytesReader;
use crate::util::fst_impl::fst_reader::FstReader;

pub struct DummyFSTReader;

impl Accountable for DummyFSTReader {
    fn ram_bytes_used(&self) -> Result<i64> {
        Err(LuceneError::unreachable("this method should not be called"))
    }
}

impl FstReader for DummyFSTReader {
    type FstBytesReader = DummyBytesReader;

    fn get_reverse_bytes_reader(&self) -> Result<Self::FstBytesReader> {
        Err(LuceneError::unreachable("this method should not be called"))
    }

    fn write_to(&self, _out: &mut impl DataOutput) -> Result<()> {
        Err(LuceneError::unreachable("this method should not be called"))
    }
}
