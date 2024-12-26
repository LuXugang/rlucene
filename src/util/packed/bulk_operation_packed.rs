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
use crate::util::packed::{Decoder, Encoder, PackedInts};
use std::cmp::Ordering;

pub struct BulkOperationPacked<T>
where
    T: Decoder + Encoder,
{
    sub_operation: Option<T>,
    bits_per_value: u32,
    long_block_count: u32,
    long_value_count: u32,
    byte_block_count: u32,
    byte_value_count: u32,
    mask: u64,
    int_mask: u32,
}
impl<T> BulkOperationPacked<T>
where
    T: Decoder + Encoder,
{
    pub const fn new(bits_per_value: u32, sub_operation: Option<T>) -> Self {
        debug_assert!(
            bits_per_value > 0 && bits_per_value <= 64,
            "bitsPerValue must be > 0 and <= 64"
        );

        let mut blocks = bits_per_value;
        while blocks & 1 == 0 {
            blocks >>= 1;
        }
        let long_block_count = blocks;
        let long_value_count = 64 * long_block_count / bits_per_value;

        let mut byte_block_count = 8 * long_block_count;
        let mut byte_value_count = long_value_count;
        while byte_block_count & 1 == 0 && byte_value_count & 1 == 0 {
            byte_block_count >>= 1;
            byte_value_count >>= 1;
        }

        let mask = if bits_per_value == 64 {
            !0u64
        } else {
            (1u64 << bits_per_value) - 1
        };

        let int_mask = mask as u32;

        debug_assert!(
            long_value_count * bits_per_value == 64 * long_block_count,
            "longValueCount * bitsPerValue must equal 64 * longBlockCount"
        );
        BulkOperationPacked {
            sub_operation,
            bits_per_value,
            long_block_count,
            long_value_count,
            byte_block_count,
            byte_value_count,
            mask,
            int_mask,
        }
    }
}
impl<T> Decoder for BulkOperationPacked<T>
where
    T: Decoder + Encoder,
{
    fn long_block_count(&self) -> u32 {
        self.long_block_count
    }

    fn long_value_count(&self) -> u32 {
        self.long_value_count
    }

    fn byte_block_count(&self) -> u32 {
        self.byte_block_count
    }

    fn byte_value_count(&self) -> u32 {
        self.byte_value_count
    }

    fn decode_u64_to_i64(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: u32,
    ) {
        if self.sub_operation.is_some() {
            self.sub_operation.as_ref().unwrap().decode_u64_to_i64(
                blocks,
                blocks_offset,
                values,
                values_offset,
                iterations,
            );
            return;
        }
        let mut bits_left: i32 = 64;
        for _ in 0..(self.long_value_count * iterations) {
            bits_left -= self.bits_per_value as i32;

            if bits_left < 0 {
                let lower_part = (blocks[blocks_offset]
                    & ((1u64 << (self.bits_per_value as i32 + bits_left)) - 1))
                    << -bits_left;

                blocks_offset += 1;
                let upper_part = blocks[blocks_offset] >> (64 + bits_left);

                values[values_offset] = (lower_part | upper_part) as i64;
                bits_left += 64;
            } else {
                values[values_offset] = ((blocks[blocks_offset] >> bits_left) & self.mask) as i64;
            }
            values_offset += 1;
        }
    }

    fn decode_u8_to_i64(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i64],
        mut values_offset: usize,
        iterations: u32,
    ) {
        if self.sub_operation.is_some() {
            self.sub_operation.as_ref().unwrap().decode_u8_to_i64(
                blocks,
                blocks_offset,
                values,
                values_offset,
                iterations,
            );
            return;
        }
        let mut next_value: i64 = 0;
        let mut bits_left: i32 = self.bits_per_value as i32;

        for _ in 0..(iterations * self.byte_block_count) {
            let bytes = blocks[blocks_offset] as i64;
            blocks_offset += 1;

            if bits_left > 8 {
                // Buffer the value
                bits_left -= 8;
                next_value |= bytes << bits_left;
            } else {
                // Flush the value
                let mut bits = 8 - bits_left;
                values[values_offset] = next_value | (bytes as u64 >> bits) as i64;
                values_offset += 1;

                while bits >= self.bits_per_value as i32 {
                    bits -= self.bits_per_value as i32;
                    values[values_offset] = ((bytes as u64 >> bits) & self.mask) as i64;
                    values_offset += 1;
                }

                bits_left = self.bits_per_value as i32 - bits;
                next_value = (bytes & ((1 << bits) - 1) as i64) << bits_left;
            }
        }

        assert_eq!(bits_left, self.bits_per_value as i32);
    }

    fn decode_u64_to_i32(
        &self,
        blocks: &[u64],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: u32,
    ) {
        if self.sub_operation.is_some() {
            self.sub_operation.as_ref().unwrap().decode_u64_to_i32(
                blocks,
                blocks_offset,
                values,
                values_offset,
                iterations,
            );
            return;
        }
        debug_assert!(
            self.bits_per_value <= 32,
            "Cannot decode {}-bits values into an int[]",
            self.bits_per_value
        );

        let mut bits_left: i32 = 64;

        for _ in 0..(self.long_value_count * iterations) {
            bits_left -= self.bits_per_value as i32;

            if bits_left < 0 {
                // Handle case where bits_left is negative
                let lower_part = (blocks[blocks_offset]
                    & ((1u64 << (self.bits_per_value as i32 + bits_left)) - 1))
                    << -bits_left;

                blocks_offset += 1;

                let upper_part = blocks[blocks_offset] >> (64 + bits_left);

                values[values_offset] = ((lower_part | upper_part) as i64) as i32;
                bits_left += 64;
            } else {
                values[values_offset] = ((blocks[blocks_offset] >> bits_left) & self.mask) as i32;
            }

            values_offset += 1;
        }
    }

    fn decode_u8_to_i32(
        &self,
        blocks: &[u8],
        mut blocks_offset: usize,
        values: &mut [i32],
        mut values_offset: usize,
        iterations: u32,
    ) {
        if self.sub_operation.is_some() {
            self.sub_operation.as_ref().unwrap().decode_u8_to_i32(
                blocks,
                blocks_offset,
                values,
                values_offset,
                iterations,
            );
            return;
        }
        let mut next_value: i32 = 0;
        let mut bits_left: i32 = self.bits_per_value as i32;

        for _ in 0..(iterations * self.byte_block_count) {
            let bytes = blocks[blocks_offset] as i32;
            blocks_offset += 1;

            if bits_left > 8 {
                // Just buffer the value
                bits_left -= 8;
                next_value |= bytes << bits_left;
            } else {
                // Flush the value
                let mut bits = 8 - bits_left;
                values[values_offset] = next_value | (bytes as u32 >> bits) as i32;
                values_offset += 1;

                while bits >= self.bits_per_value as i32 {
                    bits -= self.bits_per_value as i32;
                    values[values_offset] = ((bytes as u32 >> bits) & self.int_mask) as i32;
                    values_offset += 1;
                }

                // Then buffer the remaining bits
                bits_left = self.bits_per_value as i32 - bits;
                next_value = (bytes & ((1 << bits) - 1)) << bits_left;
            }
        }

        debug_assert!(bits_left == self.bits_per_value as i32);
    }
}
impl<T> Encoder for BulkOperationPacked<T>
where
    T: Decoder + Encoder,
{
    fn long_block_count(&self) -> u32 {
        Decoder::long_block_count(self)
    }

    fn long_value_count(&self) -> u32 {
        Decoder::long_value_count(self)
    }

    fn byte_block_count(&self) -> u32 {
        Decoder::byte_block_count(self)
    }

    fn byte_value_count(&self) -> u32 {
        Decoder::byte_value_count(self)
    }

    fn encode_i64_to_u64(
        &self,
        values: &[i64],
        mut values_offset: usize,
        blocks: &mut [u64],
        mut blocks_offset: usize,
        iterations: u32,
    ) {
        let mut next_block: u64 = 0;
        let mut bits_left: i32 = 64;

        for _ in 0..(self.long_value_count * iterations) {
            bits_left -= self.bits_per_value as i32;

            match bits_left.cmp(&0) {
                std::cmp::Ordering::Greater => {
                    // Buffer the value
                    next_block |= (values[values_offset] << bits_left) as u64;
                    values_offset += 1;
                }
                std::cmp::Ordering::Equal => {
                    next_block |= values[values_offset] as u64;
                    values_offset += 1;
                    blocks[blocks_offset] = next_block;
                    blocks_offset += 1;
                    next_block = 0;
                    bits_left = 64;
                }
                std::cmp::Ordering::Less => {
                    // Handle case where bits_left < 0
                    next_block |= (values[values_offset] as u64) >> -bits_left;

                    blocks[blocks_offset] = next_block;
                    blocks_offset += 1;

                    next_block = ((values[values_offset] & ((1u64 << -bits_left) - 1) as i64)
                        << (64 + bits_left)) as u64;
                    values_offset += 1;
                    bits_left += 64;
                }
            }
        }
    }

    fn encode_i64_to_u8(
        &self,
        values: &[i64],
        mut values_offset: usize,
        blocks: &mut [u8],
        mut blocks_offset: usize,
        iterations: u32,
    ) {
        let mut next_block: i32 = 0;
        let mut bits_left: i32 = 8;

        for _ in 0..(self.byte_value_count * iterations) {
            let v = values[values_offset];
            values_offset += 1;
            debug_assert!(
                self.bits_per_value >= PackedInts::unsigned_bits_required(v),
                "Value requires more bits than allowed by bits_per_value"
            );
            if (self.bits_per_value as i32) < bits_left {
                // Just buffer the value
                debug_assert!(v << (bits_left - self.bits_per_value as i32) <= i32::MAX as i64);
                next_block |= (v << (bits_left - self.bits_per_value as i32)) as i32;
                bits_left -= self.bits_per_value as i32;
            } else {
                let mut bits = self.bits_per_value as i32 - bits_left;
                debug_assert!(bits >= 0);
                blocks[blocks_offset] = (next_block as u64 | ((v as u64) >> bits as u64)) as u8;
                blocks_offset += 1;

                while bits >= 8 {
                    bits -= 8;
                    blocks[blocks_offset] = (v as u64 >> bits) as u8;
                    blocks_offset += 1;
                }
                bits_left = 8 - bits;
                debug_assert!(bits_left >= 0);
                next_block = ((v as u64 & ((1u64 << bits) - 1)) << bits_left as u64) as i32;
            }
        }

        debug_assert!(
            bits_left == 8,
            "bits_left must be reset to 8, but was {}",
            bits_left
        );
    }

    fn encode_i32_to_u64(
        &self,
        values: &[i32],
        mut values_offset: usize,
        blocks: &mut [u64],
        mut blocks_offset: usize,
        iterations: u32,
    ) {
        let mut next_block: u64 = 0;
        let mut bits_left: i32 = 64;

        for _ in 0..(self.long_value_count * iterations) {
            bits_left -= self.bits_per_value as i32;
            match bits_left.cmp(&0) {
                Ordering::Greater => {
                    next_block |= (values[values_offset] as u64 & 0xFFFFFFFF) << bits_left;
                    values_offset += 1;
                }
                Ordering::Equal => {
                    next_block |= values[values_offset] as u64 & 0xFFFFFFFF;
                    values_offset += 1;
                    blocks[blocks_offset] = next_block;
                    blocks_offset += 1;
                    next_block = 0;
                    bits_left = 64;
                }
                Ordering::Less => {
                    next_block |= (values[values_offset] as u64 & 0xFFFFFFFF) >> -bits_left;
                    blocks[blocks_offset] = next_block;
                    blocks_offset += 1;
                    next_block = ((values[values_offset] as u64 & 0xFFFFFFFF)
                        & ((1u64 << -bits_left) - 1))
                        << (64 + bits_left);
                    values_offset += 1;
                    bits_left += 64;
                }
            }
        }
    }

    fn encode_i32_to_u8(
        &self,
        values: &[i32],
        mut values_offset: usize,
        blocks: &mut [u8],
        mut blocks_offset: usize,
        iterations: u32,
    ) {
        let mut next_block: i32 = 0;
        let mut bits_left: i32 = 8;

        for _ in 0..(self.byte_value_count * iterations) {
            let v = values[values_offset];
            values_offset += 1;
            debug_assert!(
                PackedInts::unsigned_bits_required(v as i64) <= self.bits_per_value,
                "Value requires more bits than allowed by bits_per_value"
            );
            if (self.bits_per_value as i32) < bits_left {
                next_block |= v << (bits_left - self.bits_per_value as i32);
                bits_left -= self.bits_per_value as i32;
            } else {
                let mut bits = self.bits_per_value as i32 - bits_left;
                blocks[blocks_offset] = (next_block as u32 | (v as u32 >> bits)) as u8;
                blocks_offset += 1;

                while bits >= 8 {
                    bits -= 8;
                    blocks[blocks_offset] = (v as u32 >> bits) as u8;
                    blocks_offset += 1;
                }
                bits_left = 8 - bits;
                next_block = (v & ((1 << bits) - 1)) << bits_left;
            }
        }
        debug_assert!(
            bits_left == 8,
            "bits_left must be reset to 8, but was {}",
            bits_left
        );
    }
}
