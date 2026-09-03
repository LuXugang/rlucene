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
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::core::util::bit_util::BitUtil;
use crate::core::util::core_helper::CoreHelper;
use crate::impl_from_for_enum;
use num_bigint::BigInt;
use num_traits::ToPrimitive;

#[derive(Debug, Clone)]
pub enum Number {
  U8(u8),
  I16(i16),
  I32(i32),
  I64(i64),
  F32(f32),
  F64(f64),
  BigInt(BigInt),
}

impl Number {
  pub fn to_i8(&self) -> Option<u8> {
    match self {
      Number::U8(n) => Some(*n),
      Number::I16(n) => n.to_u8(),
      Number::I32(n) => n.to_u8(),
      Number::I64(n) => n.to_u8(),
      // Java byteValue first converts to int, then narrows to the low eight bits.
      Number::F32(n) => Some(*n as i32 as u8),
      Number::F64(n) => Some(*n as i32 as u8),
      Number::BigInt(n) => n.to_u8(),
    }
  }

  pub fn to_i16(&self) -> Option<i16> {
    match self {
      Number::U8(n) => n.to_i16(),
      Number::I16(n) => Some(*n),
      Number::I32(n) => n.to_i16(),
      Number::I64(n) => n.to_i16(),
      // Java shortValue first converts to int, then narrows to the low sixteen bits.
      Number::F32(n) => Some(*n as i32 as i16),
      Number::F64(n) => Some(*n as i32 as i16),
      Number::BigInt(n) => n.to_i16(),
    }
  }

  pub fn to_i32(&self) -> Option<i32> {
    match self {
      Number::U8(n) => n.to_i32(),
      Number::I16(n) => n.to_i32(),
      Number::I32(n) => Some(*n),
      Number::I64(n) => n.to_i32(),
      // Like Java intValue, Rust casts saturate overflow and convert NaN to zero.
      Number::F32(n) => Some(*n as i32),
      Number::F64(n) => Some(*n as i32),
      Number::BigInt(n) => n.to_i32(),
    }
  }

  pub fn to_i64(&self) -> Option<i64> {
    match self {
      Number::U8(n) => n.to_i64(),
      Number::I16(n) => n.to_i64(),
      Number::I32(n) => n.to_i64(),
      Number::I64(n) => Some(*n),
      Number::F32(n) => Some(*n as i64),
      Number::F64(n) => Some(*n as i64),
      Number::BigInt(n) => n.to_i64(),
    }
  }

  pub fn to_f32(&self) -> Option<f32> {
    match self {
      Number::U8(n) => n.to_f32(),
      Number::I16(n) => n.to_f32(),
      Number::I32(n) => n.to_f32(),
      Number::I64(n) => n.to_f32(),
      Number::F32(n) => Some(*n),
      Number::F64(n) => n.to_f32(),
      Number::BigInt(n) => n.to_f32(),
    }
  }

  pub fn to_f64(&self) -> Option<f64> {
    match self {
      Number::U8(n) => n.to_f64(),
      Number::I16(n) => n.to_f64(),
      Number::I32(n) => n.to_f64(),
      Number::I64(n) => n.to_f64(),
      Number::F32(n) => n.to_f64(),
      Number::F64(n) => Some(*n),
      Number::BigInt(n) => n.to_f64(),
    }
  }
}

impl PartialEq for Number {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::U8(a), Self::U8(b)) => a == b,
      (Self::I16(a), Self::I16(b)) => a == b,
      (Self::I32(a), Self::I32(b)) => a == b,
      (Self::I64(a), Self::I64(b)) => a == b,
      (Self::F32(a), Self::F32(b)) => CoreHelper::compare_f32(*a, *b).is_eq(),
      (Self::F64(a), Self::F64(b)) => CoreHelper::compare_f64(*a, *b).is_eq(),
      (Self::BigInt(a), Self::BigInt(b)) => a == b,
      _ => false,
    }
  }
}

impl fmt::Display for Number {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Number::U8(v) => write!(f, "{v}"),
      Number::I16(v) => write!(f, "{v}"),
      Number::I32(v) => write!(f, "{v}"),
      Number::I64(v) => write!(f, "{v}"),
      Number::F32(v) => write!(f, "{v}"),
      Number::F64(v) => write!(f, "{v}"),
      Number::BigInt(v) => write!(f, "{v}"),
    }
  }
}
impl Hash for Number {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    match self {
      Number::U8(v) => v.hash(state),
      Number::I16(v) => v.hash(state),
      Number::I32(v) => v.hash(state),
      Number::I64(v) => v.hash(state),
      Number::F32(v) => (BitUtil::float_to_int_bits(*v) as u32).hash(state),
      Number::F64(v) => (BitUtil::double_to_long_bits(*v) as u64).hash(state),
      Number::BigInt(v) => v.hash(state),
    }
  }
}
impl_from_for_enum!(
    Number,
    u8 => U8,
    i16 => I16,
    i32 => I32,
    i64 => I64,
    f32 => F32,
    f64 => F64,
    BigInt => BigInt,
);
