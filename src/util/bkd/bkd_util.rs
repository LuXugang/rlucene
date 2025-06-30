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
use crate::util::array_util::{
    ByteArrayComparatorEnum, CommonPrefixLength4, CommonPrefixLength8, CommonPrefixLengthN,
};
use crate::util::bit_util::BitUtil;
use crate::util::CoreHelper;

pub(crate) struct BKDUtil;

impl BKDUtil {
    /// Return a comparator that computes the common prefix length across the
    /// next {@code numBytes} of the provided arrays.
    pub fn get_prefix_length_comparator(num_bytes: usize) -> ByteArrayComparatorEnum {
        if num_bytes == BitUtil::LONG_BYTES {
            ByteArrayComparatorEnum::CommonPrefixLength8(CommonPrefixLength8)
        } else if num_bytes == BitUtil::INT_BYTES {
            ByteArrayComparatorEnum::CommonPrefixLength4(CommonPrefixLength4)
        } else {
            ByteArrayComparatorEnum::CommonPrefixLength(CommonPrefixLengthN { num_bytes })
        }
    }
    /// Return the length of the common prefix across the next 8 bytes of both
    /// provided arrays.
    pub fn common_prefix_length8(a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> i32 {
        let a_long = BitUtil::get_i64_le(a, a_offset);
        let b_long = BitUtil::get_i64_le(b, b_offset);
        let common_prefix_in_bits = (a_long ^ b_long).swap_bytes().leading_zeros();
        (common_prefix_in_bits >> 3) as i32
    }

    /// Return the length of the common prefix across the next 4 bytes of both
    /// provided arrays.
    pub fn common_prefix_length4(a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> i32 {
        let a_int = BitUtil::get_i32_le(a, a_offset);
        let b_int = BitUtil::get_i32_le(b, b_offset);
        let common_prefix_in_bits = (a_int ^ b_int).swap_bytes().leading_zeros();
        (common_prefix_in_bits >> 3) as i32
    }
    /// Return a predicate that tells whether the next `numBytes` bytes are
    /// equal.
    pub fn get_equals_predicate(num_bytes: usize) -> ByteArrayPredicateEnum {
        if num_bytes == BitUtil::LONG_BYTES {
            ByteArrayPredicateEnum::Equals8(Equals8)
        } else if num_bytes == BitUtil::INT_BYTES {
            ByteArrayPredicateEnum::Equals4(Equals4)
        } else {
            ByteArrayPredicateEnum::Equals(Equals { num_bytes })
        }
    }
    /// Check whether the next 8 bytes are exactly the same in the provided
    /// arrays.
    pub fn equals8(a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> bool {
        let a_long = BitUtil::get_i64_le(a, a_offset);
        let b_long = BitUtil::get_i64_le(b, b_offset);
        a_long == b_long
    }
    /// Check whether the next 4 bytes are exactly the same in the provided
    /// arrays.
    pub fn equals4(a: &[u8], a_offset: usize, b: &[u8], b_offset: usize) -> bool {
        let a_int = BitUtil::get_i32_le(a, a_offset);
        let b_int = BitUtil::get_i32_le(b, b_offset);
        a_int == b_int
    }

    /// Return the length of the common prefix across the next `num_bytes` of
    /// both provided arrays.
    pub fn common_prefix_length_n(
        a: &[u8],
        a_offset: usize,
        b: &[u8],
        b_offset: usize,
        num_bytes: usize,
    ) -> i32 {
        let slice_a = &a[a_offset..a_offset + num_bytes];
        let slice_b = &b[b_offset..b_offset + num_bytes];
        let cmp = CoreHelper::miss_match(slice_a, slice_b);
        debug_assert!(num_bytes <= i32::MAX as usize);
        if cmp == -1 {
            num_bytes as i32
        } else {
            cmp
        }
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
#[cfg(test)]
mod tests {
    use rand::Rng;

    use crate::test::util::lucene_test_case::random;
    use crate::test::util::test_util::TestUtil;
    use crate::util::bit_util::BitUtil;
    use crate::util::bkd::bkd_util::BKDUtil;
    use crate::util::SliceCopyOps;

    #[allow(dead_code)] // for quick search
    struct TestBKDUtil;

    #[test]
    fn test_equals4() {
        let mut random = random();
        let a_offset = TestUtil::next_int(&mut random, 0, 3) as usize;
        let b_offset = TestUtil::next_int(&mut random, 0, 3) as usize;

        let mut a = vec![0u8; BitUtil::INT_BYTES + a_offset];
        let mut b = vec![0u8; BitUtil::INT_BYTES + b_offset];

        for i in 0..BitUtil::INT_BYTES {
            a[a_offset + i] = random.random();
        }
        b.copy_from(&a[a_offset..a_offset + 4], b_offset);

        assert!(BKDUtil::equals4(&a, a_offset, &b, b_offset));

        for i in 0..BitUtil::INT_BYTES {
            loop {
                let random_byte: u8 = random.random();
                if random_byte != a[a_offset + i] {
                    b[b_offset + i] = random_byte;
                    break;
                }
            }
            assert!(!BKDUtil::equals4(&a, a_offset, &b, b_offset));
            b[b_offset + i] = a[a_offset + i];
        }
    }
    #[test]
    fn test_equals8() {
        let mut random = random();
        let a_offset = TestUtil::next_int(&mut random, 0, 7) as usize;
        let b_offset = TestUtil::next_int(&mut random, 0, 7) as usize;
        let mut a = vec![0u8; BitUtil::LONG_BYTES + a_offset];
        let mut b = vec![0u8; BitUtil::LONG_BYTES + b_offset];

        for i in 0..BitUtil::LONG_BYTES {
            a[a_offset + i] = random.random();
        }
        b.copy_from(&a[a_offset..a_offset + 8], b_offset);

        assert!(BKDUtil::equals8(&a, a_offset, &b, b_offset));

        for i in 0..BitUtil::LONG_BYTES {
            loop {
                let random_byte: u8 = random.random();
                if random_byte != a[a_offset + i] {
                    b[b_offset + i] = random_byte;
                    break;
                }
            }
            assert!(!BKDUtil::equals8(&a, a_offset, &b, b_offset));
            b[b_offset + i] = a[a_offset + i];
        }
    }

    #[test]
    fn test_common_prefix_length4() {
        let mut random = random();
        let a_offset = TestUtil::next_int(&mut random, 0, 3) as usize;
        let b_offset = TestUtil::next_int(&mut random, 0, 3) as usize;
        let mut a = vec![0u8; BitUtil::INT_BYTES + a_offset];
        let mut b = vec![0u8; BitUtil::INT_BYTES + b_offset];

        for i in 0..BitUtil::INT_BYTES {
            a[a_offset + i] = random.random();
            loop {
                let random_byte: u8 = random.random();
                if random_byte != a[a_offset + i] {
                    b[b_offset + i] = random_byte;
                    break;
                }
            }
        }

        for i in 0..BitUtil::INT_BYTES {
            assert_eq!(
                i as i32,
                BKDUtil::common_prefix_length4(&a, a_offset, &b, b_offset)
            );
            b[b_offset + i] = a[a_offset + i];
        }
        assert_eq!(
            4,
            BKDUtil::common_prefix_length4(&a, a_offset, &b, b_offset)
        );
    }

    #[test]
    fn test_common_prefix_length8() {
        let mut random = random();
        let a_offset = TestUtil::next_int(&mut random, 0, 7) as usize;
        let b_offset = TestUtil::next_int(&mut random, 0, 7) as usize;
        let mut a = vec![0u8; BitUtil::LONG_BYTES + a_offset];
        let mut b = vec![0u8; BitUtil::LONG_BYTES + b_offset];

        for i in 0..BitUtil::LONG_BYTES {
            a[a_offset + i] = random.random();
            loop {
                let random_byte: u8 = random.random();
                if random_byte != a[a_offset + i] {
                    b[b_offset + i] = random_byte;
                    break;
                }
            }
        }

        for i in 0..BitUtil::LONG_BYTES {
            assert_eq!(
                i as i32,
                BKDUtil::common_prefix_length8(&a, a_offset, &b, b_offset)
            );
            b[b_offset + i] = a[a_offset + i];
        }
        assert_eq!(
            8,
            BKDUtil::common_prefix_length8(&a, a_offset, &b, b_offset)
        );
    }

    #[test]
    fn test_common_prefix_length_n() {
        let mut random = random();
        let num_bytes = TestUtil::next_int(&mut random, 2, 16) as usize;
        let a_offset = TestUtil::next_int(&mut random, 0, num_bytes as i32 - 1) as usize;
        let b_offset = TestUtil::next_int(&mut random, 0, num_bytes as i32 - 1) as usize;
        let mut a = vec![0u8; num_bytes + a_offset];
        let mut b = vec![0u8; num_bytes + b_offset];

        for i in 0..num_bytes {
            a[a_offset + i] = random.random();
            loop {
                let random_byte: u8 = random.random();
                if random_byte != a[a_offset + i] {
                    b[b_offset + i] = random_byte;
                    break;
                }
            }
        }

        for i in 0..num_bytes {
            assert_eq!(
                i as i32,
                BKDUtil::common_prefix_length_n(&a, a_offset, &b, b_offset, num_bytes)
            );
            b[b_offset + i] = a[a_offset + i];
        }
        assert_eq!(
            num_bytes as i32,
            BKDUtil::common_prefix_length_n(&a, a_offset, &b, b_offset, num_bytes)
        );
    }
}
