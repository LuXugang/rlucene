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
use crate::index::BytesRef;
use crate::util::comparator::Comparator;
use crate::util::error::lucene_error::Result;

/// Specialized [`BytesRef`] comparator that `StringSorter` has optimizations
/// for.
///
/// # Note
/// This is an internal API.
pub trait BytesRefComparator {
    /// Returns the unsigned byte to use for comparison at index `i`, or `-1` if
    /// all bytes that are useful for comparisons are exhausted. This may
    /// only be called with a value of `i` between `0` (inclusive) and
    /// `compared_bytes_count` (exclusive).
    fn byte_at(&self, _bytes_ref: &BytesRef<Vec<u8>>, _i: i32) -> i32 {
        unimplemented!("byte_at must be implemented if it need to be used")
    }
    fn compare_with_offset(&self, o1: &BytesRef<Vec<u8>>, o2: &BytesRef<Vec<u8>>, k: i32) -> i32 {
        for i in k..self.compared_bytes_count() {
            let b1 = self.byte_at(o1, i);
            let b2 = self.byte_at(o2, i);
            if b1 != b2 {
                return b1 - b2;
            } else if b1 == -1 {
                break;
            }
        }
        0
    }
    fn compared_bytes_count(&self) -> i32 {
        unimplemented!("compared_bytes_count must be implemented if it need to be used")
    }
}

pub struct Natural {
    compared_bytes_count: i32,
}
impl Default for Natural {
    fn default() -> Self {
        Natural {
            compared_bytes_count: i32::MAX,
        }
    }
}

impl Comparator<BytesRef<Vec<u8>>> for Natural {
    const TYPE: &'static str = BYTES_REF_COMPARATOR_TYPE;

    fn compare(&self, a: &BytesRef<Vec<u8>>, b: &BytesRef<Vec<u8>>) -> Result<i32> {
        Ok(self.compare_with_offset(a, b, 0))
    }
}

impl BytesRefComparator for Natural {
    fn byte_at(&self, bytes_ref: &BytesRef<Vec<u8>>, i: i32) -> i32 {
        if bytes_ref.length <= i as usize {
            -1
        } else {
            bytes_ref.bytes[i as usize + bytes_ref.offset] as i32
        }
    }

    fn compare_with_offset(&self, o1: &BytesRef<Vec<u8>>, o2: &BytesRef<Vec<u8>>, k: i32) -> i32 {
        let start1 = o1.offset + k as usize;
        let start2 = o2.offset + k as usize;

        let slice1 = &o1.bytes[start1..(o1.offset + o1.length)];
        let slice2 = &o2.bytes[start2..(o2.offset + o2.length)];

        for (byte_a, byte_b) in slice1.iter().zip(slice2.iter()) {
            if byte_a != byte_b {
                return *byte_a as i32 - *byte_b as i32;
            }
        }
        (slice1.len() as i32) - (slice2.len() as i32)
    }

    fn compared_bytes_count(&self) -> i32 {
        self.compared_bytes_count
    }
}

pub const BYTES_REF_COMPARATOR_TYPE: &str = "BytesRefComparator";
