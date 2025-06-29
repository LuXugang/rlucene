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
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::Result;
use crate::util::packed::mutable_packed64_enum::MutablePacked64Enum;
use crate::util::packed::{NullReader, Reader};

pub enum PackedIntsReadEnum {
    PackedReader(MutablePacked64Enum),
    NullReader(NullReader),
}

impl Accountable for PackedIntsReadEnum {
    fn ram_bytes_used(&self) -> Result<i64> {
        match self {
            PackedIntsReadEnum::PackedReader(op) => op.ram_bytes_used(),
            PackedIntsReadEnum::NullReader(op) => op.ram_bytes_used(),
        }
    }
}

impl Reader for PackedIntsReadEnum {
    fn get(&self, index: i32) -> i64 {
        match self {
            PackedIntsReadEnum::PackedReader(op) => op.get(index),
            PackedIntsReadEnum::NullReader(op) => op.get(index),
        }
    }

    fn get_bulk(&self, index: i32, arr: &mut [i64], off: i32, len: i32) -> i32 {
        match self {
            PackedIntsReadEnum::PackedReader(op) => op.get_bulk(index, arr, off, len),
            PackedIntsReadEnum::NullReader(op) => op.get_bulk(index, arr, off, len),
        }
    }

    fn size(&self) -> i32 {
        match self {
            PackedIntsReadEnum::PackedReader(op) => op.size(),
            PackedIntsReadEnum::NullReader(op) => op.size(),
        }
    }
}
impl Default for PackedIntsReadEnum {
    // used for padding value
    fn default() -> Self {
        PackedIntsReadEnum::NullReader(NullReader::new(0))
    }
}
