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
use std::ptr;

pub struct BitUtil {}
impl BitUtil {
    #[cfg(target_endian = "little")]
    pub fn get_u16_le(bytes: &[u8], pos: usize) -> u16 {
        debug_assert!(pos + 2 <= bytes.len(), "Index out of bounds");
        unsafe { ptr::read_unaligned(bytes.as_ptr().add(pos) as *const u16) }
    }
    #[cfg(target_endian = "little")]
    pub fn set_u16_le(bytes: &mut [u8], pos: usize, value: u16) {
        debug_assert!(pos + 2 <= bytes.len(), "Index out of bounds");
        unsafe { ptr::write_unaligned(bytes.as_mut_ptr().add(pos) as *mut u16, value) }
    }
    #[cfg(target_endian = "little")]
    pub fn get_u32_le(bytes: &[u8], pos: usize) -> u32 {
        debug_assert!(pos + 4 <= bytes.len(), "Index out of bounds");

        unsafe { ptr::read_unaligned(bytes.as_ptr().add(pos) as *const u32) }
    }

    #[cfg(target_endian = "little")]
    pub fn set_u32_le(bytes: &mut [u8], pos: usize, value: u32) {
        debug_assert!(pos + 4 <= bytes.len(), "Index out of bounds");

        let value = value.to_le();
        unsafe { ptr::write_unaligned(bytes.as_mut_ptr().add(pos) as *mut u32, value) }
    }

    #[cfg(target_endian = "little")]
    pub fn get_u64_le(bytes: &[u8], pos: usize) -> u64 {
        debug_assert!(pos + 8 <= bytes.len(), "Index out of bounds");

        unsafe { ptr::read_unaligned(bytes.as_ptr().add(pos) as *const u64) }
    }

    #[cfg(target_endian = "little")]
    pub fn set_u64_le(bytes: &mut [u8], pos: usize, value: u64) {
        debug_assert!(pos + 8 <= bytes.len(), "Index out of bounds");

        let value = value.to_le();
        unsafe { ptr::write_unaligned(bytes.as_mut_ptr().add(pos) as *mut u64, value) }
    }
    pub fn zig_zag_decode_i32(i: i32) -> i32 {
        (i >> 1) ^ -(i & 1)
    }

    pub fn zig_zag_encode_i32(i: i32) -> i32 {
        (i >> 31) ^ (i << 1)
    }

    pub fn zig_zag_decode_i64(l: i64) -> i64 {
        (l >> 1) ^ -(l & 1)
    }

    pub fn zig_zag_encode_i64(l: i64) -> i64 {
        (l >> 1) ^ -(l & 1)
    }
}
