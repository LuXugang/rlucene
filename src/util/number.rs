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
use num_traits::ToPrimitive;
#[derive(Debug, Clone, Copy)]
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
