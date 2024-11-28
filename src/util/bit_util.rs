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
use std::{mem, ptr};

pub struct BitUtil {}
impl BitUtil {
    #[cfg(target_endian = "little")]
    pub fn get_i16_le(bytes: &[u8], pos: usize) -> i16 {
        debug_assert!(pos + 2 <= bytes.len(), "Index out of bounds");
        unsafe { ptr::read_unaligned(bytes.as_ptr().add(pos) as *const i16) }
    }
    #[cfg(target_endian = "little")]
    pub fn set_i16_le(bytes: &mut [u8], pos: usize, value: i16) {
        debug_assert!(pos + 2 <= bytes.len(), "Index out of bounds");
        unsafe { ptr::write_unaligned(bytes.as_mut_ptr().add(pos) as *mut i16, value) }
    }
    #[cfg(target_endian = "little")]
    pub fn get_i32_le(bytes: &[u8], pos: usize) -> i32 {
        debug_assert!(pos + 4 <= bytes.len(), "Index out of bounds");

        unsafe { ptr::read_unaligned(bytes.as_ptr().add(pos) as *const i32) }
    }

    #[cfg(target_endian = "little")]
    pub fn set_i32_le(bytes: &mut [u8], pos: usize, value: i32) {
        debug_assert!(pos + 4 <= bytes.len(), "Index out of bounds");

        let value = value.to_le();
        unsafe { ptr::write_unaligned(bytes.as_mut_ptr().add(pos) as *mut i32, value) }
    }

    #[cfg(target_endian = "little")]
    pub fn get_i64_le(bytes: &[u8], pos: usize) -> i64 {
        debug_assert!(pos + 8 <= bytes.len(), "Index out of bounds");

        unsafe { ptr::read_unaligned(bytes.as_ptr().add(pos) as *const i64) }
    }

    #[cfg(target_endian = "little")]
    pub fn set_i64_le(bytes: &mut [u8], pos: usize, value: i64) {
        debug_assert!(pos + 8 <= bytes.len(), "Index out of bounds");

        let value = value.to_le();
        unsafe { ptr::write_unaligned(bytes.as_mut_ptr().add(pos) as *mut i64, value) }
    }
    pub fn zig_zag_decode_i32(i: u32) -> i32 {
        ((i >> 1) as i32) ^ -((i & 1) as i32)
    }

    pub fn zig_zag_encode_i32(i: i32) -> i32 {
        (i >> 31) ^ (i << 1)
    }

    pub fn zig_zag_decode_i64(l: u64) -> i64 {
        ((l >> 1) as i64) ^ -((l & 1) as i64)
    }

    pub fn zig_zag_encode_i64(l: i64) -> i64 {
        (l >> 1) ^ -(l & 1)
    }
}

pub const SHORT_BYTES: usize = mem::size_of::<i16>();
pub const INT_BYTES: usize = mem::size_of::<i32>();
pub const LONG_BYTES: usize = mem::size_of::<i64>();
pub const FLOAT_BYTES: usize = mem::size_of::<f32>();
pub const USIZE_BYTES: usize = mem::size_of::<usize>();
