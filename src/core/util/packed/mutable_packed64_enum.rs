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
use std::fmt::{Display, Formatter};

use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::packed::packed64::Packed64;
use crate::core::util::packed::packed64_single_block::{
  Packed64SingleBlock, Packed64SingleBlock1, Packed64SingleBlock2, Packed64SingleBlock3,
  Packed64SingleBlock4, Packed64SingleBlock5, Packed64SingleBlock6, Packed64SingleBlock7,
  Packed64SingleBlock8, Packed64SingleBlock9, Packed64SingleBlock10, Packed64SingleBlock12,
  Packed64SingleBlock16, Packed64SingleBlock21, Packed64SingleBlock32,
};
use crate::core::util::packed::{Mutable, Reader};

pub(crate) enum MutablePacked64Enum {
  P64SingleBlock1(Packed64SingleBlock<Packed64SingleBlock1>),
  P64SingleBlock2(Packed64SingleBlock<Packed64SingleBlock2>),
  P64SingleBlock3(Packed64SingleBlock<Packed64SingleBlock3>),
  P64SingleBlock4(Packed64SingleBlock<Packed64SingleBlock4>),
  P64SingleBlock5(Packed64SingleBlock<Packed64SingleBlock5>),
  P64SingleBlock6(Packed64SingleBlock<Packed64SingleBlock6>),
  P64SingleBlock7(Packed64SingleBlock<Packed64SingleBlock7>),
  P64SingleBlock8(Packed64SingleBlock<Packed64SingleBlock8>),
  P64SingleBlock9(Packed64SingleBlock<Packed64SingleBlock9>),
  P64SingleBlock10(Packed64SingleBlock<Packed64SingleBlock10>),
  P64SingleBlock12(Packed64SingleBlock<Packed64SingleBlock12>),
  P64SingleBlock16(Packed64SingleBlock<Packed64SingleBlock16>),
  P64SingleBlock21(Packed64SingleBlock<Packed64SingleBlock21>),
  P64SingleBlock32(Packed64SingleBlock<Packed64SingleBlock32>),
  P64(Packed64),
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
  fn ram_bytes_used(&self) -> Result<i64> {
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
  fn get(&self, index: usize) -> i64 {
    match self {
      MutablePacked64Enum::P64SingleBlock1(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock2(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock3(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock4(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock5(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock6(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock7(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock8(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock9(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock10(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock12(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock16(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock21(op) => op.get(index),
      MutablePacked64Enum::P64SingleBlock32(op) => op.get(index),
      MutablePacked64Enum::P64(op) => op.get(index),
    }
  }

  fn get_bulk(&self, index: i32, arr: &mut [i64], off: i32, len: i32) -> Result<i32> {
    match self {
      MutablePacked64Enum::P64SingleBlock1(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock2(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock3(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock4(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock5(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock6(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock7(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock8(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock9(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock10(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock12(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock16(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock21(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock32(op) => op.get_bulk(index, arr, off, len),
      MutablePacked64Enum::P64(op) => op.get_bulk(index, arr, off, len),
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

  fn set(&mut self, index: i32, value: i64) -> Result<()> {
    match self {
      MutablePacked64Enum::P64SingleBlock1(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock2(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock3(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock4(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock5(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock6(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock7(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock8(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock9(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock10(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock12(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock16(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock21(op) => op.set(index, value),
      MutablePacked64Enum::P64SingleBlock32(op) => op.set(index, value),
      MutablePacked64Enum::P64(op) => op.set(index, value),
    }
  }

  fn set_bulk(&mut self, index: i32, arr: &[i64], off: i32, len: i32) -> Result<i32> {
    match self {
      MutablePacked64Enum::P64SingleBlock1(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock2(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock3(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock4(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock5(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock6(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock7(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock8(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock9(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock10(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock12(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock16(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock21(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64SingleBlock32(op) => op.set_bulk(index, arr, off, len),
      MutablePacked64Enum::P64(op) => op.set_bulk(index, arr, off, len),
    }
  }

  fn fill(&mut self, from_index: i32, to_index: i32, val: i64) -> Result<()> {
    match self {
      MutablePacked64Enum::P64SingleBlock1(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock2(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock3(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock4(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock5(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock6(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock7(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock8(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock9(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock10(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock12(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock16(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock21(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64SingleBlock32(op) => op.fill(from_index, to_index, val),
      MutablePacked64Enum::P64(op) => op.fill(from_index, to_index, val),
    }
  }

  fn clear(&mut self) -> Result<()> {
    match self {
      MutablePacked64Enum::P64SingleBlock1(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock2(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock3(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock4(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock5(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock6(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock7(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock8(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock9(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock10(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock12(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock16(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock21(op) => op.clear(),
      MutablePacked64Enum::P64SingleBlock32(op) => op.clear(),
      MutablePacked64Enum::P64(op) => op.clear(),
    }
  }
}
