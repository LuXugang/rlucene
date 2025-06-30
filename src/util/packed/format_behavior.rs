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
use crate::util::packed::{p64sb_util, Format};

pub trait FormatBehavior {
    fn get_id(&self) -> u32;
    /// Computes how many byte blocks are needed to store `values` values of
    /// size `bits_per_value`.
    fn byte_count(&self, packed_ints_version: i32, value_count: i32, bits_per_value: i32) -> i64 {
        debug_assert!(
            (0..=64).contains(&bits_per_value),
            "bits_per_value must be between 0 and 64"
        );
        self.long_count(packed_ints_version, value_count, bits_per_value) as i64 * 8
    }
    /// * Computes how many long blocks are needed to store `values` values of
    ///   size `bitsPerValue`.
    fn long_count(&self, packed_ints_version: i32, value_count: i32, bits_per_value: i32) -> i32 {
        debug_assert!(
            (0..=64).contains(&bits_per_value),
            "bits_per_value must be between 0 and 64"
        );
        let byte_count = self.byte_count(packed_ints_version, value_count, bits_per_value);
        debug_assert!(
            byte_count < 8 * (i32::MAX as i64),
            "Computed byte count exceeds maximum long block count"
        );
        ((byte_count + 7) >> 3) as i32
    }
    /// Tests whether the provided number of bits per value is supported by the
    /// format.
    fn is_supported(&self, bits_per_value: i32) -> bool {
        (1..=64).contains(&bits_per_value)
    }
    /// Returns the overhead per value, in bits.
    fn overhead_per_value(&self, bits_per_value: i32) -> f32 {
        debug_assert!(
            self.is_supported(bits_per_value),
            "bits_per_value is not supported"
        );
        0.0
    }
    #[allow(unused)]
    fn overhead_ratio(&self, bits_per_value: i32) -> f32 {
        debug_assert!(
            self.is_supported(bits_per_value),
            "bits_per_value is not supported"
        );
        self.overhead_per_value(bits_per_value) / bits_per_value as f32
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PackedImpl {
    id: u32,
}
impl PackedImpl {
    pub fn new(id: u32) -> Self {
        PackedImpl { id }
    }
}
impl FormatBehavior for PackedImpl {
    fn get_id(&self) -> u32 {
        self.id
    }

    fn byte_count(&self, _packed_ints_version: i32, value_count: i32, bits_per_value: i32) -> i64 {
        ((value_count as f64 * bits_per_value as f64) / 8f64).ceil() as i64
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PackedSingleBlockImpl {
    id: u32,
}
impl PackedSingleBlockImpl {
    pub fn new(id: u32) -> Self {
        PackedSingleBlockImpl { id }
    }
}
impl FormatBehavior for PackedSingleBlockImpl {
    fn get_id(&self) -> u32 {
        self.id
    }

    fn long_count(&self, _packed_ints_version: i32, value_count: i32, bits_per_value: i32) -> i32 {
        let values_per_block = 64 / bits_per_value;
        (value_count as f64 / values_per_block as f64).ceil() as i32
    }

    fn is_supported(&self, bits_per_value: i32) -> bool {
        p64sb_util::is_supported(bits_per_value)
    }

    fn overhead_per_value(&self, bits_per_value: i32) -> f32 {
        debug_assert!(self.is_supported(bits_per_value));
        let values_per_block = 64 / bits_per_value;
        let overhead = 64 % bits_per_value;
        overhead as f32 / values_per_block as f32
    }
}
impl FormatBehavior for Format {
    fn get_id(&self) -> u32 {
        match self {
            Format::Packed(p) => p.get_id(),
            Format::PackedSingleBlock(p) => p.get_id(),
        }
    }

    fn byte_count(&self, packed_ints_version: i32, value_count: i32, bits_per_value: i32) -> i64 {
        match self {
            Format::Packed(p) => p.byte_count(packed_ints_version, value_count, bits_per_value),
            Format::PackedSingleBlock(p) => {
                p.byte_count(packed_ints_version, value_count, bits_per_value)
            },
        }
    }

    fn long_count(&self, packed_ints_version: i32, value_count: i32, bits_per_value: i32) -> i32 {
        match self {
            Format::Packed(p) => p.long_count(packed_ints_version, value_count, bits_per_value),
            Format::PackedSingleBlock(p) => {
                p.long_count(packed_ints_version, value_count, bits_per_value)
            },
        }
    }

    fn is_supported(&self, bits_per_value: i32) -> bool {
        match self {
            Format::Packed(p) => p.is_supported(bits_per_value),
            Format::PackedSingleBlock(p) => p.is_supported(bits_per_value),
        }
    }

    fn overhead_per_value(&self, bits_per_value: i32) -> f32 {
        match self {
            Format::Packed(p) => p.overhead_per_value(bits_per_value),
            Format::PackedSingleBlock(p) => p.overhead_per_value(bits_per_value),
        }
    }

    fn overhead_ratio(&self, bits_per_value: i32) -> f32 {
        match self {
            Format::Packed(p) => p.overhead_ratio(bits_per_value),
            Format::PackedSingleBlock(p) => p.overhead_ratio(bits_per_value),
        }
    }
}
