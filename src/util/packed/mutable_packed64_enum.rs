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
use crate::util::error::lucene_error::LuceneError;
use crate::util::packed::packed64::Packed64;
use crate::util::packed::packed64_single_block::{
    Packed64SingleBlock, Packed64SingleBlock1, Packed64SingleBlock10, Packed64SingleBlock12,
    Packed64SingleBlock16, Packed64SingleBlock2, Packed64SingleBlock21, Packed64SingleBlock3,
    Packed64SingleBlock32, Packed64SingleBlock4, Packed64SingleBlock5, Packed64SingleBlock6,
    Packed64SingleBlock7, Packed64SingleBlock8, Packed64SingleBlock9,
};
use crate::util::packed::{Mutable, MutableImpl, Reader};
use std::fmt::{Display, Formatter};

pub enum MutablePacked64Enum {
    P64SingleBlock1(MutableImpl<Packed64SingleBlock<Packed64SingleBlock1>>),
    P64SingleBlock2(MutableImpl<Packed64SingleBlock<Packed64SingleBlock2>>),
    P64SingleBlock3(MutableImpl<Packed64SingleBlock<Packed64SingleBlock3>>),
    P64SingleBlock4(MutableImpl<Packed64SingleBlock<Packed64SingleBlock4>>),
    P64SingleBlock5(MutableImpl<Packed64SingleBlock<Packed64SingleBlock5>>),
    P64SingleBlock6(MutableImpl<Packed64SingleBlock<Packed64SingleBlock6>>),
    P64SingleBlock7(MutableImpl<Packed64SingleBlock<Packed64SingleBlock7>>),
    P64SingleBlock8(MutableImpl<Packed64SingleBlock<Packed64SingleBlock8>>),
    P64SingleBlock9(MutableImpl<Packed64SingleBlock<Packed64SingleBlock9>>),
    P64SingleBlock10(MutableImpl<Packed64SingleBlock<Packed64SingleBlock10>>),
    P64SingleBlock12(MutableImpl<Packed64SingleBlock<Packed64SingleBlock12>>),
    P64SingleBlock16(MutableImpl<Packed64SingleBlock<Packed64SingleBlock16>>),
    P64SingleBlock21(MutableImpl<Packed64SingleBlock<Packed64SingleBlock21>>),
    P64SingleBlock32(MutableImpl<Packed64SingleBlock<Packed64SingleBlock32>>),
    P64(MutableImpl<Packed64>),
}
impl Display for MutablePacked64Enum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MutablePacked64Enum::P64SingleBlock1(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock2(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock3(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock4(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock5(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock6(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock7(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock8(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock9(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock10(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock12(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock16(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock21(op) => op.fmt(f),
            MutablePacked64Enum::P64SingleBlock32(op) => op.fmt(f),
            MutablePacked64Enum::P64(op) => op.fmt(f),
        }
    }
}

impl Accountable for MutablePacked64Enum {
    fn ram_bytes_used(&self) -> u64 {
        match self {
            MutablePacked64Enum::P64SingleBlock1(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock2(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock3(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock4(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock5(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock6(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock7(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock8(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock9(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock10(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock12(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock16(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock21(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64SingleBlock32(op) => op.ram_bytes_used(),
            MutablePacked64Enum::P64(op) => op.ram_bytes_used(),
        }
    }
}

impl Reader for MutablePacked64Enum {
    fn get(&mut self, index: i32) -> Result<i64, LuceneError> {
        match self {
            MutablePacked64Enum::P64SingleBlock1(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock2(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock3(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock4(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock5(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock6(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock7(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock8(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock9(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock10(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock12(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock16(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock21(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64SingleBlock32(op) => op.sub_reader.get(index),
            MutablePacked64Enum::P64(op) => op.sub_reader.get(index),
        }
    }

    fn get_bulk(
        &mut self,
        index: i32,
        arr: &mut [i64],
        off: i32,
        len: i32,
    ) -> Result<i32, LuceneError> {
        match self {
            MutablePacked64Enum::P64SingleBlock1(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock2(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock3(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock4(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock5(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock6(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock7(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock8(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock9(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock10(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock12(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock16(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock21(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock32(op) => {
                op.sub_reader.get_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64(op) => op.sub_reader.get_bulk(index, arr, off, len),
        }
    }

    fn size(&self) -> i32 {
        match self {
            MutablePacked64Enum::P64SingleBlock1(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock2(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock3(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock4(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock5(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock6(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock7(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock8(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock9(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock10(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock12(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock16(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock21(op) => op.size(),
            MutablePacked64Enum::P64SingleBlock32(op) => op.size(),
            MutablePacked64Enum::P64(op) => op.size(),
        }
    }
}

impl Mutable for MutablePacked64Enum {
    fn get_bits_per_value(&self) -> i32 {
        match self {
            MutablePacked64Enum::P64SingleBlock1(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock2(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock3(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock4(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock5(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock6(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock7(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock8(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock9(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock10(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock12(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock16(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock21(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64SingleBlock32(op) => op.get_bits_per_value(),
            MutablePacked64Enum::P64(op) => op.get_bits_per_value(),
        }
    }

    fn set(&mut self, index: i32, value: i64) -> Result<(), LuceneError> {
        match self {
            MutablePacked64Enum::P64SingleBlock1(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock2(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock3(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock4(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock5(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock6(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock7(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock8(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock9(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock10(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock12(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock16(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock21(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64SingleBlock32(op) => op.sub_reader.set(index, value),
            MutablePacked64Enum::P64(op) => op.sub_reader.set(index, value),
        }
    }

    fn set_bulk(
        &mut self,
        index: i32,
        arr: &[i64],
        off: i32,
        len: i32,
    ) -> Result<i32, LuceneError> {
        match self {
            MutablePacked64Enum::P64SingleBlock1(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock2(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock3(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock4(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock5(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock6(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock7(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock8(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock9(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock10(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock12(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock16(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock21(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64SingleBlock32(op) => {
                op.sub_reader.set_bulk(index, arr, off, len)
            }
            MutablePacked64Enum::P64(op) => op.sub_reader.set_bulk(index, arr, off, len),
        }
    }

    fn fill(&mut self, from_index: i32, to_index: i32, val: i64) -> Result<(), LuceneError> {
        match self {
            MutablePacked64Enum::P64SingleBlock1(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock2(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock3(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock4(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock5(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock6(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock7(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock8(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock9(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock10(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock12(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock16(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock21(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64SingleBlock32(op) => {
                op.sub_reader.fill(from_index, to_index, val)
            }
            MutablePacked64Enum::P64(op) => op.sub_reader.fill(from_index, to_index, val),
        }
    }

    fn clear(&mut self) -> Result<(), LuceneError> {
        match self {
            MutablePacked64Enum::P64SingleBlock1(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock2(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock3(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock4(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock5(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock6(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock7(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock8(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock9(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock10(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock12(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock16(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock21(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64SingleBlock32(op) => op.sub_reader.clear(),
            MutablePacked64Enum::P64(op) => op.sub_reader.clear(),
        }
    }
}
