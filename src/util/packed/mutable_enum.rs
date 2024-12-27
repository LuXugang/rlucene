/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::util::accountable::Accountable;
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::packed::growable_writer::GrowableWriter;
use crate::util::packed::{Mutable, MutablePacked64Enum, Reader};
use std::fmt::{Display, Pointer};

pub enum MutableEnum {
    Packed(MutablePacked64Enum),
    GrowableW(GrowableWriter),
}
impl Display for MutableEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutableEnum::Packed(op) => op.fmt(f),
            MutableEnum::GrowableW(op) => op.fmt(f),
        }
    }
}
impl Accountable for MutableEnum {
    fn ram_bytes_used(&self) -> i64 {
        match self {
            MutableEnum::Packed(op) => op.ram_bytes_used(),
            MutableEnum::GrowableW(op) => op.ram_bytes_used(),
        }
    }
}
impl Reader for MutableEnum {
    fn get(&mut self, index: usize) -> Result<i64, DataIOError> {
        match self {
            MutableEnum::Packed(op) => op.get(index),
            MutableEnum::GrowableW(op) => op.get(index),
        }
    }

    fn get_bulk(
        &mut self,
        index: usize,
        arr: &mut [i64],
        off: usize,
        len: usize,
    ) -> Result<u32, DataIOError> {
        match self {
            MutableEnum::Packed(op) => op.get_bulk(index, arr, off, len),
            MutableEnum::GrowableW(op) => op.get_bulk(index, arr, off, len),
        }
    }

    fn size(&self) -> u32 {
        match self {
            MutableEnum::Packed(op) => op.size(),
            MutableEnum::GrowableW(op) => op.size(),
        }
    }
}
impl Mutable for MutableEnum {
    fn get_bits_per_value(&self) -> u32 {
        match self {
            MutableEnum::Packed(op) => op.get_bits_per_value(),
            MutableEnum::GrowableW(op) => op.get_bits_per_value(),
        }
    }

    fn set(&mut self, index: usize, value: i64) -> Result<(), DataIOError> {
        match self {
            MutableEnum::Packed(op) => op.set(index, value),
            MutableEnum::GrowableW(op) => op.set(index, value),
        }
    }

    fn set_bulk(
        &mut self,
        index: usize,
        arr: &[i64],
        off: usize,
        len: usize,
    ) -> Result<u32, DataIOError> {
        match self {
            MutableEnum::Packed(op) => op.set_bulk(index, arr, off, len),
            MutableEnum::GrowableW(op) => op.set_bulk(index, arr, off, len),
        }
    }

    fn fill(&mut self, from_index: usize, to_index: usize, val: i64) -> Result<(), DataIOError> {
        match self {
            MutableEnum::Packed(op) => op.fill(from_index, to_index, val),
            MutableEnum::GrowableW(op) => op.fill(from_index, to_index, val),
        }
    }

    fn clear(&mut self) -> Result<(), DataIOError> {
        match self {
            MutableEnum::Packed(op) => op.clear(),
            MutableEnum::GrowableW(op) => op.clear(),
        }
    }
}
