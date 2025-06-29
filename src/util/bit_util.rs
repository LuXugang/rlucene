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
use std::{mem, ptr};

pub struct BitUtil {}
impl BitUtil {
    pub const SHORT_BYTES: usize = mem::size_of::<i16>();
    pub const INT_BYTES: usize = mem::size_of::<i32>();
    pub const LONG_BYTES: usize = mem::size_of::<i64>();
    pub const FLOAT_BYTES: usize = mem::size_of::<f32>();
    pub const DOUBLE_BYTES: usize = mem::size_of::<f64>();
    pub const USIZE_BYTES: usize = mem::size_of::<usize>();
    pub const FLOAT_NAN_BITS: u32 = 0x7fc00000;
    pub const DOUBLE_NAN_BITS: u64 = 0x7ff8000000000000;
    // i16 big_endian
    pub fn get_i16_be(bytes: &[u8], pos: usize) -> i16 {
        debug_assert!(
            pos + Self::SHORT_BYTES <= bytes.len(),
            "Index out of bounds"
        );

        unsafe {
            let raw_value = std::ptr::read_unaligned(bytes.as_ptr().add(pos) as *const i16);
            i16::from_be(raw_value)
        }
    }

    #[cfg(target_endian = "little")]
    pub fn set_i16_be(bytes: &mut [u8], pos: usize, value: i16) {
        Self::set_i16_be_with_len(bytes, pos, value, Self::SHORT_BYTES);
    }

    #[cfg(target_endian = "little")]
    pub fn set_i16_be_with_len(bytes: &mut [u8], pos: usize, value: i16, len: usize) {
        debug_assert!(
            pos + len <= bytes.len(),
            "Index out of bounds: pos={} len={} bytes.len()={}",
            pos,
            len,
            bytes.len()
        );
        debug_assert!(
            (0..=Self::SHORT_BYTES).contains(&len),
            "Invalid length: len={} (must be <= 2)",
            len
        );

        let value_be = value.to_be();

        unsafe {
            let value_ptr = &value_be as *const i16 as *const u8;
            let dest_ptr = bytes.as_mut_ptr().add(pos);
            std::ptr::copy_nonoverlapping(value_ptr, dest_ptr, len);
        }
    }
    // i16 little_endian
    #[cfg(target_endian = "little")]
    pub fn get_i16_le(bytes: &[u8], pos: usize) -> i16 {
        debug_assert!(
            pos + Self::SHORT_BYTES <= bytes.len(),
            "Index out of bounds"
        );
        unsafe { ptr::read_unaligned(bytes.as_ptr().add(pos) as *const i16) }
    }
    #[cfg(target_endian = "little")]
    pub fn set_i16_le(bytes: &mut [u8], pos: usize, value: i16) {
        // Call the more flexible implementation with len = 2 (write all bytes)
        Self::set_i16_le_with_len(bytes, pos, value, Self::SHORT_BYTES);
    }

    #[cfg(target_endian = "little")]
    pub fn set_i16_le_with_len(bytes: &mut [u8], pos: usize, value: i16, len: usize) {
        debug_assert!(
            pos + len <= bytes.len(),
            "Index out of bounds: pos={} len={} bytes.len()={}",
            pos,
            len,
            bytes.len()
        );
        debug_assert!(
            (0..=Self::SHORT_BYTES).contains(&len),
            "Invalid length: len={} (must be <= {})",
            len,
            Self::SHORT_BYTES
        );

        let value_le = value.to_le();

        unsafe {
            let value_ptr = &value_le as *const i16 as *const u8;
            let dest_ptr = bytes.as_mut_ptr().add(pos);
            std::ptr::copy_nonoverlapping(value_ptr, dest_ptr, len);
        }
    }

    // i32 big_endian
    pub fn get_i32_be(bytes: &[u8], pos: usize) -> i32 {
        debug_assert!(pos + Self::INT_BYTES <= bytes.len(), "Index out of bounds");

        unsafe {
            let raw_value = std::ptr::read_unaligned(bytes.as_ptr().add(pos) as *const i32);
            i32::from_be(raw_value)
        }
    }
    #[cfg(target_endian = "little")]
    pub fn set_i32_be(bytes: &mut [u8], pos: usize, value: i32) {
        Self::set_i32_be_with_len(bytes, pos, value, Self::INT_BYTES);
    }

    #[cfg(target_endian = "little")]
    pub fn set_i32_be_with_len(bytes: &mut [u8], pos: usize, value: i32, len: usize) {
        debug_assert!(
            pos + len <= bytes.len(),
            "Index out of bounds: pos={} len={} bytes.len()={}",
            pos,
            len,
            bytes.len()
        );
        debug_assert!(
            (0..=Self::INT_BYTES).contains(&len),
            "Invalid length: len={} (must be <= 4)",
            len
        );

        let value_be = value.to_be();

        unsafe {
            let value_ptr = &value_be as *const i32 as *const u8;
            let dest_ptr = bytes.as_mut_ptr().add(pos);
            std::ptr::copy_nonoverlapping(value_ptr, dest_ptr, len);
        }
    }
    // i32 little_endian
    #[cfg(target_endian = "little")]
    pub fn get_i32_le(bytes: &[u8], pos: usize) -> i32 {
        debug_assert!(pos + Self::INT_BYTES <= bytes.len(), "Index out of bounds");

        unsafe { ptr::read_unaligned(bytes.as_ptr().add(pos) as *const i32) }
    }

    #[cfg(target_endian = "little")]
    pub fn set_i32_le(bytes: &mut [u8], pos: usize, value: i32) {
        Self::set_i32_le_with_len(bytes, pos, value, Self::INT_BYTES);
    }
    #[cfg(target_endian = "little")]
    pub fn set_i32_le_with_len(bytes: &mut [u8], pos: usize, value: i32, len: usize) {
        debug_assert!(
            pos + len <= bytes.len(),
            "Index out of bounds: pos={} len={} bytes.len()={}",
            pos,
            len,
            bytes.len()
        );
        debug_assert!(
            (0..=Self::INT_BYTES).contains(&len),
            "Invalid length: len={} (must be <= 4)",
            len
        );

        let value_le = value.to_le();

        unsafe {
            let value_ptr = &value_le as *const i32 as *const u8;
            let dest_ptr = bytes.as_mut_ptr().add(pos);
            std::ptr::copy_nonoverlapping(value_ptr, dest_ptr, len);
        }
    }

    // i64 big_endian
    pub fn get_i64_be(bytes: &[u8], pos: usize) -> i64 {
        debug_assert!(pos + Self::LONG_BYTES <= bytes.len(), "Index out of bounds");

        unsafe {
            let raw_value = std::ptr::read_unaligned(bytes.as_ptr().add(pos) as *const i64);
            i64::from_be(raw_value)
        }
    }

    pub fn set_i64_be(bytes: &mut [u8], pos: usize, value: i64) {
        Self::set_i64_be_with_len(bytes, pos, value, Self::LONG_BYTES);
    }

    pub fn set_i64_be_with_len(bytes: &mut [u8], pos: usize, value: i64, len: usize) {
        debug_assert!(
            pos + len <= bytes.len(),
            "Index out of bounds: pos={} len={} bytes.len()={}",
            pos,
            len,
            bytes.len()
        );
        debug_assert!(
            (0..=Self::LONG_BYTES).contains(&len),
            "Invalid length: len={} (must be <= {})",
            len,
            Self::LONG_BYTES
        );

        let value_be = value.to_be();

        unsafe {
            let value_ptr = &value_be as *const i64 as *const u8;
            let dest_ptr = bytes.as_mut_ptr().add(pos);
            std::ptr::copy_nonoverlapping(value_ptr, dest_ptr, len);
        }
    }
    // i64 little_endian
    #[cfg(target_endian = "little")]
    pub fn get_i64_le(bytes: &[u8], pos: usize) -> i64 {
        debug_assert!(pos + Self::LONG_BYTES <= bytes.len(), "Index out of bounds");

        unsafe { ptr::read_unaligned(bytes.as_ptr().add(pos) as *const i64) }
    }

    #[cfg(target_endian = "little")]
    pub fn set_i64_le(bytes: &mut [u8], pos: usize, value: i64) {
        // Call the more flexible implementation with len = 8 (write all bytes)
        Self::set_i64_le_with_len(bytes, pos, value, Self::LONG_BYTES);
    }

    #[cfg(target_endian = "little")]
    pub fn set_i64_le_with_len(bytes: &mut [u8], pos: usize, value: i64, len: usize) {
        debug_assert!(
            pos + len <= bytes.len(),
            "Index out of bounds: pos={} len={} bytes.len()={}",
            pos,
            len,
            bytes.len()
        );
        debug_assert!(
            (0..=Self::LONG_BYTES).contains(&len),
            "Invalid length: len={} (must be <= {})",
            len,
            Self::LONG_BYTES
        );

        let value_le = value.to_le();

        unsafe {
            let value_ptr = &value_le as *const i64 as *const u8;
            let dest_ptr = bytes.as_mut_ptr().add(pos);
            std::ptr::copy_nonoverlapping(value_ptr, dest_ptr, len);
        }
    }

    /// Returns the next highest power of two, or the current value if it's
    /// already a power of two or zero.
    pub fn next_highest_power_of_two_with_i32(mut v: i32) -> i32 {
        v -= 1;
        v |= v >> 1;
        v |= v >> 2;
        v |= v >> 4;
        v |= v >> 8;
        v |= v >> 16;
        v + 1
    }
    /// Returns the next highest power of two, or the current value if it's
    /// already a power of two or zero.
    pub fn next_highest_power_of_two_with_i64(mut v: i64) -> i64 {
        v -= 1;
        v |= v >> 1;
        v |= v >> 2;
        v |= v >> 4;
        v |= v >> 8;
        v |= v >> 16;
        v |= v >> 32;
        v + 1
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
        (((l >> 63) as u64) ^ ((l << 1) as u64)) as i64
    }
    #[cfg(not(target_endian = "little"))]
    compile_error!("This code can only be compiled on little-endian systems.");
}
