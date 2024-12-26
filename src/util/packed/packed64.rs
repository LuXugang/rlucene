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
use crate::util::accountable::Accountable;
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::packed::bulk_operation::of;
use crate::util::packed::format_behavior::{FormatBehavior, PackedImpl};
use crate::util::packed::{Decoder, Encoder, Format, Mutable, PackedInts, Reader};
use std::fmt::{Display, Formatter};

/// Space-optimized random access array of values with a fixed number of bits per value. Values
/// are packed contiguously.
///
/// The implementation strives to achieve maximum performance under the constraint of contiguous
/// bits by avoiding expensive operations. This comes at the cost of code clarity.
///
/// # Technical Details
/// This implementation is a refinement of a non-branching version. The non-branching `get` and
/// `set` methods meant that 2 or 4 atomic accesses in the underlying array were always performed,
/// even for cases where only 1 or 2 accesses were needed. Even with caching, this had a detrimental
/// effect on performance. To address this issue, this implementation avoids using lookup tables
/// for shifts and masks. Instead, shifts and masks are calculated on the fly, which proved to be
/// faster.
///
/// See [LUCENE-4062](https://issues.apache.org/jira/browse/LUCENE-4062) for details.
pub struct Packed64 {
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
    ///
    pub fn new(value_count: u32, bits_per_value: u32) -> Self {
        debug_assert!(
            bits_per_value > 0 && bits_per_value <= 64,
            "bitsPerValue must be > 0 and <= 64"
        );
        let format = Format::Packed(PackedImpl::new(0)); // Corresponds to PackedInts.Format.PACKED in Java
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
    pub fn gcd(mut a: u32, mut b: u32) -> u32 {
        if a < b {
            std::mem::swap(&mut a, &mut b);
        }
        while b != 0 {
            let temp = b;
            b = a % b;
            a = temp;
        }
        a
    }
}

impl Reader for Packed64 {
    fn get(&mut self, index: usize) -> Result<i64, DataIOError> {
        // The abstract index in a bit stream
        let major_bit_pos = (index as u64) * (self.bits_per_value as u64);

        // The index in the backing long-array
        let element_pos = (major_bit_pos >> Self::BLOCK_BITS) as usize;

        // The number of value-bits in the second block
        let end_bits =
            (major_bit_pos & Self::MOD_MASK as u64) as i64 + self.bpv_minus_block_size as i64;

        if end_bits <= 0 {
            // Single block
            // if element_pos == 0{
            //     println!("Single block1:{}",self.blocks[element_pos] as i64);
            //     println!("Single block2:{}",(self.blocks[element_pos] >> -end_bits) as i64 );
            //     println!("Single block3:{}",((self.blocks[element_pos] >> -end_bits) & self.mask_right) as i64 );
            // }

            return Ok(((self.blocks[element_pos] >> -end_bits) & self.mask_right) as i64);
        }
        Ok((((self.blocks[element_pos] << end_bits)
            | (self.blocks[element_pos + 1] >> (Self::BLOCK_SIZE as i64 - end_bits)))
            & self.mask_right) as i64)
    }

    fn get_bulk(
        &mut self,
        mut index: usize,
        arr: &mut [i64],
        mut off: usize,
        mut len: usize,
    ) -> Result<u32, DataIOError> {
        assert!(index < self.value_count as usize, "index out of bounds");
        len = len.min(self.value_count as usize - index);
        assert!(
            off + len <= arr.len(),
            "not enough space in the target array"
        );

        let original_index = index;
        let decoder = of(Format::Packed(PackedImpl::new(0)), self.bits_per_value);

        // Go to the next block where the value does not span across two blocks
        let offset_in_blocks = index % Decoder::long_value_count(decoder) as usize;

        if offset_in_blocks != 0 {
            for _i in offset_in_blocks..Decoder::long_value_count(decoder) as usize {
                if len == 0 {
                    return Ok((index - original_index) as u32);
                }
                arr[off] = self.get(index)?;
                off += 1;
                index += 1;
                len -= 1;
            }
            if len == 0 {
                return Ok((index - original_index) as u32);
            }
        }

        // Bulk get
        assert_eq!(index % Decoder::long_value_count(decoder) as usize, 0);
        let block_index =
            ((index as u64 * self.bits_per_value as u64) >> Self::BLOCK_BITS) as usize;
        assert_eq!(
            ((index as u64 * self.bits_per_value as u64) & Self::MOD_MASK as u64),
            0
        );

        let iterations = len / Decoder::long_value_count(decoder) as usize;
        debug_assert!(iterations <= u32::MAX as usize);
        decoder.decode_u64_to_i64(&self.blocks, block_index, arr, off, iterations as u32);

        let got_values = iterations * Decoder::long_value_count(decoder) as usize;
        index += got_values;
        assert!(len >= got_values, "Remaining length is negative");
        len -= got_values;

        if index > original_index {
            // Stay at the block boundary
            Ok((index - original_index) as u32)
        } else {
            // No progress so far => already at a block boundary but no full block to get
            assert_eq!(index, original_index, "Index mismatch");
            self.default_get_bulk(index, arr, off, len) // This assumes a fallback to another implementation
        }
    }

    fn size(&self) -> u32 {
        self.value_count
    }
}

impl Accountable for Packed64 {
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

impl Display for Packed64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Packed64(bitsPerValue={}, size={}, blocks={})",
            self.bits_per_value,
            self.size(),
            self.blocks.len()
        )
    }
}

impl Mutable for Packed64 {
    fn get_bits_per_value(&self) -> u32 {
        self.bits_per_value
    }

    fn set(&mut self, index: usize, value: i64) -> Result<(), DataIOError> {
        // The abstract index in a contiguous bit stream
        let major_bit_pos = (index as u64) * self.bits_per_value as u64;
        // The index in the backing blocks array
        let element_pos = (major_bit_pos >> Self::BLOCK_BITS) as usize;
        // The number of value-bits in the second block
        let end_bits =
            (major_bit_pos & Self::MOD_MASK as u64) as i64 + self.bpv_minus_block_size as i64;

        if end_bits <= 0 {
            // Single block case
            self.blocks[element_pos] = (self.blocks[element_pos] & !(self.mask_right << -end_bits))
                | (value << -end_bits) as u64;
            return Ok(());
        }

        // Two blocks case
        self.blocks[element_pos] = (self.blocks[element_pos] & !(self.mask_right >> end_bits))
            | ((value as u64) >> end_bits);

        self.blocks[element_pos + 1] = (self.blocks[element_pos + 1] & (!0u64 >> end_bits))
            | (value << (Self::BLOCK_SIZE as i64 - end_bits)) as u64;
        Ok(())
    }

    fn set_bulk(
        &mut self,
        mut index: usize,
        arr: &[i64],
        mut off: usize,
        mut len: usize,
    ) -> Result<u32, DataIOError> {
        assert!(index < self.value_count as usize, "index out of bounds");
        len = len.min(self.value_count as usize - index);
        assert!(
            off + len <= arr.len(),
            "not enough values in the source array"
        );

        let original_index = index;
        let encoder = of(Format::Packed(PackedImpl::new(0)), self.bits_per_value);

        // Go to the next block where the value does not span across two blocks
        let offset_in_blocks = index % Encoder::long_value_count(encoder) as usize;
        if offset_in_blocks != 0 {
            for _ in offset_in_blocks..Encoder::long_value_count(encoder) as usize {
                if len == 0 {
                    debug_assert!(index - original_index <= u32::MAX as usize);
                    return Ok((index - original_index) as u32);
                }
                self.set(index, arr[off])?;
                index += 1;
                off += 1;
                len -= 1;
            }
            if len == 0 {
                debug_assert!(index - original_index <= u32::MAX as usize);
                return Ok((index - original_index) as u32);
            }
        }

        // Bulk set
        assert_eq!(index % Encoder::long_value_count(encoder) as usize, 0);
        let block_index =
            ((index as u64 * self.bits_per_value as u64) >> Self::BLOCK_BITS) as usize;
        assert_eq!(
            ((index as u64 * self.bits_per_value as u64) & Self::MOD_MASK as u64),
            0
        );

        let iterations = len / Encoder::long_value_count(encoder) as usize;
        debug_assert!(iterations <= u32::MAX as usize);
        encoder.encode_i64_to_u64(
            &arr[off..],
            0,
            &mut self.blocks,
            block_index,
            iterations as u32,
        );

        let set_values = iterations * Encoder::long_value_count(encoder) as usize;
        index += set_values;
        len -= set_values;

        if index > original_index {
            // Stay at the block boundary
            debug_assert!(index - original_index <= u32::MAX as usize);
            Ok((index - original_index) as u32)
        } else {
            // No progress so far => already at a block boundary but no full block to set
            assert_eq!(index, original_index);
            Ok(self.default_set_bulk(index, arr, off, len)?)
        }
    }

    fn fill(
        &mut self,
        mut from_index: usize,
        to_index: usize,
        val: i64,
    ) -> Result<(), DataIOError> {
        assert!(
            PackedInts::unsigned_bits_required(val) <= self.bits_per_value,
            "Value requires more bits than allowed by bits_per_value"
        );
        assert!(from_index <= to_index, "from_index must be <= to_index");

        // Minimum number of values that use an exact number of full blocks
        let n_aligned_values = 64 / Packed64::gcd(64, self.bits_per_value);
        let span = to_index - from_index;

        // If the span is too small, fall back to naive filling
        if span <= (3 * n_aligned_values) as usize {
            for _ in from_index..to_index {
                self.default_fill(from_index, to_index, val)?;
            }
            return Ok(());
        }

        // Fill the first values naively until the next block start
        let from_index_mod_n_aligned_values = from_index % n_aligned_values as usize;
        if from_index_mod_n_aligned_values != 0 {
            for _ in from_index_mod_n_aligned_values..n_aligned_values as usize {
                self.set(from_index, val)?;
                from_index += 1;
            }
        }
        assert!(from_index % n_aligned_values as usize == 0);

        // Compute the long[] blocks for nAlignedValues consecutive values and
        // use them to set as many values as possible without applying any mask
        // or shift
        let n_aligned_blocks = (n_aligned_values * self.bits_per_value) >> 6;
        let n_aligned_values_blocks = {
            let mut values = Packed64::new(n_aligned_values, self.bits_per_value);
            for i in 0..n_aligned_values {
                values.set(i as usize, val)?;
            }
            values.blocks
        };
        assert!(n_aligned_blocks as usize <= n_aligned_values_blocks.len());

        // Bulk set values using precomputed blocks
        let start_block = (from_index * self.bits_per_value as usize) >> 6;
        let end_block = (to_index * self.bits_per_value as usize) >> 6;
        for block in start_block..end_block {
            let block_value = n_aligned_values_blocks[block % n_aligned_blocks as usize];
            self.blocks[block] = block_value;
        }

        // Fill the gap
        for i in ((end_block << 6) / self.bits_per_value as usize)..to_index {
            self.set(i, val)?;
        }
        Ok(())
    }

    fn clear(&mut self) -> Result<(), DataIOError> {
        self.blocks.fill(0);
        Ok(())
    }
}
