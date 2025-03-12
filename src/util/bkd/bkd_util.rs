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
use crate::util::array_util::ByteArrayComparator;
use crate::util::bit_util::BitUtil;
use crate::util::CommonUtil;

pub(crate) struct BKDUtil;

impl BKDUtil {
    /// Return the length of the common prefix across the next 8 bytes of both provided arrays.
    pub fn common_prefix_length8(a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> i32 {
        let a_long = BitUtil::get_i64_le(a, a_offset);
        let b_long = BitUtil::get_i64_le(b, b_offset);
        let common_prefix_in_bits = (a_long ^ b_long).swap_bytes().leading_zeros();
        (common_prefix_in_bits >> 3) as i32
    }

    /// Return the length of the common prefix across the next 4 bytes of both provided arrays.
    pub fn common_prefix_length4(a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> i32 {
        let a_int = BitUtil::get_i32_le(a, a_offset);
        let b_int = BitUtil::get_i32_le(b, b_offset);
        let common_prefix_in_bits = (a_int ^ b_int).swap_bytes().leading_zeros();
        (common_prefix_in_bits >> 3) as i32
    }
    /// Return a predicate that tells whether the next `numBytes` bytes are equal.
    pub fn get_equals_predicate(num_bytes: usize) -> ByteArrayPredicateEnum {
        if num_bytes == BitUtil::LONG_BYTES {
            ByteArrayPredicateEnum::Equals8(Equals8)
        } else if num_bytes == BitUtil::INT_BYTES {
            ByteArrayPredicateEnum::Equals4(Equals4)
        } else {
            ByteArrayPredicateEnum::Equals(Equals { num_bytes })
        }
    }
    /// Check whether the next 8 bytes are exactly the same in the provided arrays.
    pub fn equals8(a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> bool {
        let a_long = BitUtil::get_i64_le(a, a_offset);
        let b_long = BitUtil::get_i64_le(b, b_offset);
        a_long == b_long
    }
    /// Check whether the next 4 bytes are exactly the same in the provided arrays.
    pub fn equals4(a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> bool {
        let a_int = BitUtil::get_i32_le(a, a_offset);
        let b_int = BitUtil::get_i32_le(b, b_offset);
        a_int == b_int
    }

    /// Return the length of the common prefix across the next `num_bytes` of both provided arrays.
    pub fn common_prefix_length_n(
        a: &[u8],
        a_offset: usize,
        b: &[u8],
        b_offset: usize,
        num_bytes: usize,
    ) -> i32 {
        let slice_a = &a[a_offset..a_offset + num_bytes];
        let slice_b = &b[b_offset..b_offset + num_bytes];
        let cmp = CommonUtil::miss_match(slice_a, slice_b);
        debug_assert!(num_bytes <= i32::MAX as usize);
        if cmp == -1 {
            num_bytes as i32
        } else {
            cmp
        }
    }
}
pub struct CommonPrefixLength8;
impl ByteArrayComparator for CommonPrefixLength8 {
    fn compare(&self, a: &[u8], a_i: usize, b: &[u8], b_i: usize) -> i32 {
        BKDUtil::common_prefix_length8(a, a_i, b, b_i)
    }
}
pub struct CommonPrefixLength4;
impl ByteArrayComparator for CommonPrefixLength4 {
    fn compare(&self, a: &[u8], a_i: usize, b: &[u8], b_i: usize) -> i32 {
        BKDUtil::common_prefix_length4(a, a_i, b, b_i)
    }
}
pub struct CommonPrefixLengthN {
    num_bytes: usize,
}
impl ByteArrayComparator for CommonPrefixLengthN {
    fn compare(&self, a: &[u8], a_i: usize, b: &[u8], b_i: usize) -> i32 {
        BKDUtil::common_prefix_length_n(a, a_i, b, b_i, self.num_bytes)
    }
}

/// Predicate for a fixed number of bytes.
pub trait ByteArrayPredicate {
    /// Test bytes starting from the given offsets.
    fn test(&self, a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> bool;
}
pub struct Equals {
    num_bytes: usize,
}
impl ByteArrayPredicate for Equals {
    fn test(&self, a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> bool {
        a[a_offset..a_offset + self.num_bytes] == b[b_offset..b_offset + self.num_bytes]
    }
}
pub struct Equals8;
impl ByteArrayPredicate for Equals8 {
    fn test(&self, a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> bool {
        BKDUtil::equals8(a, a_offset, b, b_offset)
    }
}
pub struct Equals4;
impl ByteArrayPredicate for Equals4 {
    fn test(&self, a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> bool {
        BKDUtil::equals4(a, a_offset, b, b_offset)
    }
}
pub enum ByteArrayPredicateEnum {
    Equals8(Equals8),
    Equals4(Equals4),
    Equals(Equals),
}
impl ByteArrayPredicate for ByteArrayPredicateEnum {
    fn test(&self, a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> bool {
        match self {
            ByteArrayPredicateEnum::Equals8(e) => e.test(a, a_offset, b, b_offset),
            ByteArrayPredicateEnum::Equals4(e) => e.test(a, a_offset, b, b_offset),
            ByteArrayPredicateEnum::Equals(e) => e.test(a, a_offset, b, b_offset),
        }
    }
}
