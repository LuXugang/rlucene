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
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::packed::packed64::Packed64;
use crate::core::util::packed::packed64_single_block::{
  Packed64SingleBlock, Packed64SingleBlock1, Packed64SingleBlock2, Packed64SingleBlock3,
  Packed64SingleBlock4, Packed64SingleBlock5, Packed64SingleBlock6, Packed64SingleBlock7,
  Packed64SingleBlock8, Packed64SingleBlock9, Packed64SingleBlock10, Packed64SingleBlock12,
  Packed64SingleBlock16, Packed64SingleBlock21, Packed64SingleBlock32,
};
use crate::core::util::packed::{NullReader, Reader};

pub enum PackedIntsReadEnum {
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
  NullReader(NullReader),
}

impl Accountable for PackedIntsReadEnum {
  fn ram_bytes_used(&self) -> Result<i64> {
    match self {
      PackedIntsReadEnum::P64SingleBlock1(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock2(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock3(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock4(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock5(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock6(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock7(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock8(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock9(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock10(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock12(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock16(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock21(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64SingleBlock32(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::P64(op) => op.ram_bytes_used(),
      PackedIntsReadEnum::NullReader(op) => op.ram_bytes_used(),
    }
  }
}

impl Reader for PackedIntsReadEnum {
  fn get(&self, index: usize) -> i64 {
    match self {
      PackedIntsReadEnum::P64SingleBlock1(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock2(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock3(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock4(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock5(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock6(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock7(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock8(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock9(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock10(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock12(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock16(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock21(op) => op.get(index),
      PackedIntsReadEnum::P64SingleBlock32(op) => op.get(index),
      PackedIntsReadEnum::P64(op) => op.get(index),
      PackedIntsReadEnum::NullReader(op) => op.get(index),
    }
  }

  fn get_bulk(&self, index: i32, arr: &mut [i64], off: i32, len: i32) -> Result<i32> {
    match self {
      PackedIntsReadEnum::P64SingleBlock1(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock2(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock3(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock4(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock5(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock6(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock7(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock8(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock9(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock10(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock12(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock16(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock21(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64SingleBlock32(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::P64(op) => op.get_bulk(index, arr, off, len),
      PackedIntsReadEnum::NullReader(op) => op.get_bulk(index, arr, off, len),
    }
  }

  fn size(&self) -> i32 {
    match self {
      PackedIntsReadEnum::P64SingleBlock1(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock2(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock3(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock4(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock5(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock6(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock7(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock8(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock9(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock10(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock12(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock16(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock21(op) => op.size(),
      PackedIntsReadEnum::P64SingleBlock32(op) => op.size(),
      PackedIntsReadEnum::P64(op) => op.size(),
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
