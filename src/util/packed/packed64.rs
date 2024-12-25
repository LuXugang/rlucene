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
use crate::util::packed::format_behavior::{FormatBehavior, Packed};
use crate::util::packed::{Format, PackedInts};

pub(crate) struct Packed64 {
    /// Values are stored contiguously in the blocks array.
    blocks: Vec<u64>,
    /// A right-aligned mask of width `bits_per_value` used by the `get` method.
    mask_right: u64,
    /// Optimization: Saves one lookup in the `get` method.
    bpv_minus_block_size: i32,
    /// The number of elements in the array.
    value_count: u32,
    /// The number of bits available for any given value.
    bits_per_value: u32,
}
impl Packed64 {
    pub const BLOCK_SIZE: u32 = 64; // 32 = int, 64 = long
    pub const BLOCK_BITS: u32 = 6; // The #bits representing BLOCK_SIZE
    pub const MOD_MASK: u32 = Self::BLOCK_SIZE - 1; // x % BLOCK_SIZE
    /// Creates an array with the internal structures adjusted for the given limits and initialized to 0.
    ///
    /// # Arguments
    ///
    /// * `value_count` - The number of elements.
    /// * `bits_per_value` - The number of bits available for any given value.
    ///
    /// # Returns
    ///
    /// A new instance of `Packed64`.
    pub fn new(value_count: u32, bits_per_value: u32) -> Self {
        let format = Format::Packed(Packed); // Corresponds to PackedInts.Format.PACKED in Java
        let long_count =
            format.long_count(PackedInts::VERSION_CURRENT, value_count, bits_per_value);
        let blocks = vec![0; long_count as usize];

        let mask_right =
            (!0u64) << (Self::BLOCK_SIZE - bits_per_value) >> (Self::BLOCK_SIZE - bits_per_value);
        let bpv_minus_block_size = bits_per_value as i32 - Self::BLOCK_SIZE as i32;

        Self {
            blocks,
            mask_right,
            bpv_minus_block_size,
            value_count,
            bits_per_value,
        }
    }
}
