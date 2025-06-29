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
use std::fmt::Display;

use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::Result;
use crate::util::packed::growable_writer::GrowableWriter;
use crate::util::packed::mutable_packed64_enum::MutablePacked64Enum;
use crate::util::packed::{DummyMutable, Mutable, Reader};

pub(crate) enum MutableEnum {
    Packed(MutablePacked64Enum),
    GrowableW(GrowableWriter),
    Dummy(DummyMutable),
}
impl Display for MutableEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutableEnum::Packed(op) => op.fmt(f),
            MutableEnum::GrowableW(op) => op.fmt(f),
            MutableEnum::Dummy(op) => op.fmt(f),
        }
    }
}
impl Accountable for MutableEnum {
    fn ram_bytes_used(&self) -> Result<i64> {
        match self {
            MutableEnum::Packed(op) => op.ram_bytes_used(),
            MutableEnum::GrowableW(op) => op.ram_bytes_used(),
            MutableEnum::Dummy(op) => op.ram_bytes_used(),
        }
    }
}
impl Reader for MutableEnum {
    fn get(&self, index: i32) -> i64 {
        match self {
            MutableEnum::Packed(op) => op.get(index),
            MutableEnum::GrowableW(op) => op.get(index),
            MutableEnum::Dummy(op) => op.get(index),
        }
    }

    fn get_bulk(&self, index: i32, arr: &mut [i64], off: i32, len: i32) -> i32 {
        match self {
            MutableEnum::Packed(op) => op.get_bulk(index, arr, off, len),
            MutableEnum::GrowableW(op) => op.get_bulk(index, arr, off, len),
            MutableEnum::Dummy(op) => op.get_bulk(index, arr, off, len),
        }
    }

    fn size(&self) -> i32 {
        match self {
            MutableEnum::Packed(op) => op.size(),
            MutableEnum::GrowableW(op) => op.size(),
            MutableEnum::Dummy(op) => op.size(),
        }
    }
}
impl Mutable for MutableEnum {
    fn get_bits_per_value(&self) -> i32 {
        match self {
            MutableEnum::Packed(op) => op.get_bits_per_value(),
            MutableEnum::GrowableW(op) => op.get_bits_per_value(),
            MutableEnum::Dummy(op) => op.get_bits_per_value(),
        }
    }

    fn set(&mut self, index: i32, value: i64) {
        match self {
            MutableEnum::Packed(op) => op.set(index, value),
            MutableEnum::GrowableW(op) => op.set(index, value),
            MutableEnum::Dummy(op) => op.set(index, value),
        }
    }

    fn set_bulk(&mut self, index: i32, arr: &[i64], off: i32, len: i32) -> i32 {
        match self {
            MutableEnum::Packed(op) => op.set_bulk(index, arr, off, len),
            MutableEnum::GrowableW(op) => op.set_bulk(index, arr, off, len),
            MutableEnum::Dummy(op) => op.set_bulk(index, arr, off, len),
        }
    }

    fn fill(&mut self, from_index: i32, to_index: i32, val: i64) {
        match self {
            MutableEnum::Packed(op) => op.fill(from_index, to_index, val),
            MutableEnum::GrowableW(op) => op.fill(from_index, to_index, val),
            MutableEnum::Dummy(op) => op.fill(from_index, to_index, val),
        }
    }

    fn clear(&mut self) {
        match self {
            MutableEnum::Packed(op) => op.clear(),
            MutableEnum::GrowableW(op) => op.clear(),
            MutableEnum::Dummy(op) => op.clear(),
        }
    }
}
