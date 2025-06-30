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
use std::cell::RefCell;
use std::rc::Rc;

use crate::store::{DataOutput, IndexInput};
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::fst_reader::FstReader;
use crate::util::fst_impl::reverse_random_access_reader::ReverseRandomAccessReader;
/// Provides off heap storage of finite state machine (FST), using underlying
/// index input instead of  byte store on heap
pub struct OffHeapFSTStore<I>
where
    I: IndexInput,
{
    input: Rc<RefCell<I>>,
    offset: i64,
    num_bytes: i64,
}
impl<I> OffHeapFSTStore<I>
where
    I: IndexInput,
{
    pub fn new(input: Rc<RefCell<I>>, offset: i64, num_bytes: i64) -> Self {
        Self {
            input,
            offset,
            num_bytes,
        }
    }
    pub fn size(&self) -> i64 {
        self.num_bytes
    }
}

impl<I> Accountable for OffHeapFSTStore<I>
where
    I: IndexInput,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

impl<I> FstReader for OffHeapFSTStore<I>
where
    I: IndexInput,
{
    type FstBytesReader = ReverseRandomAccessReader<I::RandomAccessSlice>;

    fn get_reverse_bytes_reader(&self) -> Result<Self::FstBytesReader> {
        let input = self.input.borrow_mut();
        let slice = input.random_access_slice(self.offset, self.num_bytes)?;
        Ok(ReverseRandomAccessReader::new(slice))
    }

    fn write_to(&self, _out: &mut impl DataOutput) -> Result<()> {
        Err(LuceneError::unsupported_operation(
            "write_to is not supported for OffHeapFSTStore",
        ))
    }
}
