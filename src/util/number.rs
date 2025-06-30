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
use std::fmt;
use std::hash::{Hash, Hasher};

use num_traits::ToPrimitive;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    U8(u8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl Number {
    pub fn to_i8(&self) -> Option<u8> {
        match *self {
            Number::U8(n) => Some(n),
            Number::I16(n) => n.to_u8(),
            Number::I32(n) => n.to_u8(),
            Number::I64(n) => n.to_u8(),
            Number::F32(n) => n.to_u8(),
            Number::F64(n) => n.to_u8(),
        }
    }

    pub fn to_i16(&self) -> Option<i16> {
        match *self {
            Number::U8(n) => Some(n as i16),
            Number::I16(n) => Some(n),
            Number::I32(n) => n.to_i16(),
            Number::I64(n) => n.to_i16(),
            Number::F32(n) => n.to_i16(),
            Number::F64(n) => n.to_i16(),
        }
    }

    pub fn to_i32(&self) -> Option<i32> {
        match *self {
            Number::U8(n) => Some(n as i32),
            Number::I16(n) => Some(n as i32),
            Number::I32(n) => Some(n),
            Number::I64(n) => n.to_i32(),
            Number::F32(n) => n.to_i32(),
            Number::F64(n) => n.to_i32(),
        }
    }

    pub fn to_i64(&self) -> Option<i64> {
        match *self {
            Number::U8(n) => Some(n as i64),
            Number::I16(n) => Some(n as i64),
            Number::I32(n) => Some(n as i64),
            Number::I64(n) => Some(n),
            Number::F32(n) => n.to_i64(),
            Number::F64(n) => n.to_i64(),
        }
    }

    pub fn to_f32(&self) -> Option<f32> {
        match *self {
            Number::U8(n) => (n as i32).to_f32(),
            Number::I16(n) => (n as i32).to_f32(),
            Number::I32(n) => n.to_f32(),
            Number::I64(n) => n.to_f32(),
            Number::F32(n) => Some(n),
            Number::F64(n) => n.to_f32(),
        }
    }

    pub fn to_f64(&self) -> Option<f64> {
        match *self {
            Number::U8(n) => (n as i32).to_f64(),
            Number::I16(n) => (n as i32).to_f64(),
            Number::I32(n) => n.to_f64(),
            Number::I64(n) => n.to_f64(),
            Number::F32(n) => n.to_f64(),
            Number::F64(n) => Some(n),
        }
    }
    pub fn as_string(&self) -> String {
        match *self {
            Number::U8(n) => n.to_string(),
            Number::I16(n) => n.to_string(),
            Number::I32(n) => n.to_string(),
            Number::I64(n) => n.to_string(),
            Number::F32(n) => n.to_string(),
            Number::F64(n) => n.to_string(),
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Number::U8(v) => write!(f, "{}", v),
            Number::I16(v) => write!(f, "{}", v),
            Number::I32(v) => write!(f, "{}", v),
            Number::I64(v) => write!(f, "{}", v),
            Number::F32(v) => write!(f, "{}", v),
            Number::F64(v) => write!(f, "{}", v),
        }
    }
}
impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Number::U8(v) => v.hash(state),
            Number::I16(v) => v.hash(state),
            Number::I32(v) => v.hash(state),
            Number::I64(v) => v.hash(state),
            Number::F32(v) => v.to_bits().hash(state),
            Number::F64(v) => v.to_bits().hash(state),
        }
    }
}
