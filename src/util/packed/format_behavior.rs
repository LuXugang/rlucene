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
use crate::util::packed::Format;
use crate::util::packed::packed64_single_block::Packed64SingleBlock;

pub(crate) trait FormatBehavior{
    /// Computes how many byte blocks are needed to store `values` values of size `bits_per_value`.
    fn byte_count(&self, packed_ints_version: u32, value_count: u32, bits_per_value: u32) -> u64{
        assert!(
            bits_per_value >= 0 && bits_per_value <= 64,
            "bits_per_value must be between 0 and 64"
        );
        8 * self.long_count(packed_ints_version, value_count, bits_per_value) as u64
    }
    /// * Computes how many long blocks are needed to store `values` values of size `bitsPerValue`.
    fn long_count(&self, packed_ints_version: u32, value_count: u32, bits_per_value: u32) -> u32{
        assert!(
            bits_per_value <= 64,
            "bits_per_value must be between 0 and 64"
        );
        let byte_count = self.byte_count(packed_ints_version, value_count, bits_per_value);
        assert!(
            byte_count < 8 * (u32::MAX as u64),
            "Computed byte count exceeds maximum long block count"
        );
        ((byte_count + 7) >> 3) as u32
    }
    /// Tests whether the provided number of bits per value is supported by the format.
    fn is_supported(&self, bits_per_value: u32) -> bool{
        bits_per_value >= 1 && bits_per_value <= 64
    }
    /// Returns the overhead per value, in bits.
    fn overhead_per_value(&self, bits_per_value: u32) -> f32{
        assert!(
            self.is_supported(bits_per_value),
            "bits_per_value is not supported"
        );
        0.0
    }
    #[allow(unused)]
    fn overhead_ratio(&self, bits_per_value: u32) -> f32 {
        self.overhead_per_value(bits_per_value) / bits_per_value as f32
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate)struct Packed;
impl FormatBehavior for Packed{
    fn byte_count(&self, _packed_ints_version: u32, value_count: u32, bits_per_value: u32) -> u64 {
        ((value_count as f64 * bits_per_value as f64)/ 8f64).ceil() as u64
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PackedSingleBlock;
impl FormatBehavior for PackedSingleBlock{
    fn long_count(&self, _packed_ints_version: u32, value_count: u32, bits_per_value: u32) -> u32 {
        let values_per_block = 64 / bits_per_value;
        (value_count as f64 / values_per_block as f64).ceil() as u32
    }

    fn is_supported(&self, bits_per_value: u32) -> bool {
        Packed64SingleBlock::is_supported(bits_per_value)
    }

    fn overhead_per_value(&self, bits_per_value: u32) -> f32 {
        assert!(self.is_supported(bits_per_value));
        let values_per_block = 64 / bits_per_value;
        let overhead = 64 % bits_per_value;
        overhead as f32 / values_per_block as f32
    }
}
impl FormatBehavior for Format{
    fn byte_count(&self, packed_ints_version: u32, value_count: u32, bits_per_value: u32) -> u64 {
        match self {
            Format::Packed(_) => Packed.byte_count(packed_ints_version, value_count, bits_per_value),
            Format::PackedSingleBlock(_) => PackedSingleBlock.byte_count(packed_ints_version, value_count, bits_per_value),
        }
    }

    fn long_count(&self, packed_ints_version: u32, value_count: u32, bits_per_value: u32) -> u32 {
        match self {
            Format::Packed(_) => Packed.long_count(packed_ints_version, value_count, bits_per_value),
            Format::PackedSingleBlock(_) => PackedSingleBlock.long_count(packed_ints_version, value_count, bits_per_value),
        }
    }

    fn is_supported(&self, bits_per_value: u32) -> bool {
        match self {
            Format::Packed(_) => Packed.is_supported(bits_per_value),
            Format::PackedSingleBlock(_) => PackedSingleBlock.is_supported(bits_per_value),
        }
    }

    fn overhead_per_value(&self, bits_per_value: u32) -> f32 {
        match self {
            Format::Packed(_) => Packed.overhead_per_value(bits_per_value),
            Format::PackedSingleBlock(_) => PackedSingleBlock.overhead_per_value(bits_per_value),
        }
    }

    fn overhead_ratio(&self, bits_per_value: u32) -> f32 {
        match self {
            Format::Packed(_) => Packed.overhead_ratio(bits_per_value),
            Format::PackedSingleBlock(_) => PackedSingleBlock.overhead_ratio(bits_per_value),
        }
    }
}