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
use crate::util::packed::format_behavior::{Packed, PackedSingleBlock};
use crate::util::packed::{Decoder, Encoder, Format, Mutable, PackedInts, Reader};
use std::fmt::{Display, Formatter};

pub(crate) struct Packed64SingleBlock {
    blocks: Vec<u64>,
    value_count: u32,
    bits_per_value: u32,
}
impl Packed64SingleBlock {
    /// Supported bits per value
    const SUPPORTED_BITS_PER_VALUE: [u32; 14] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 16, 21, 32];
    const MAX_SUPPORTED_BITS_PER_VALUE: u32 = 32;

    /// Checks if the given `bits_per_value` is supported.
    pub fn is_supported(bits_per_value: u32) -> bool {
        Self::SUPPORTED_BITS_PER_VALUE
            .binary_search(&bits_per_value)
            .is_ok()
    }
    fn required_capacity(value_count: u32, values_per_block: u32) -> u32 {
        value_count / values_per_block
            + if value_count % values_per_block == 0 {
                0
            } else {
                1
            }
    }
    pub fn new(bits_per_value: u32, value_count: u32) -> Self {
        assert!(
            Self::is_supported(bits_per_value),
            "Unsupported bits_per_value: {}",
            bits_per_value
        );
        let values_per_block = 64 / bits_per_value;
        let required_capacity = Self::required_capacity(value_count, values_per_block);
        Self {
            blocks: vec![0; required_capacity as usize],
            value_count,
            bits_per_value,
        }
    }
}

impl Reader for Packed64SingleBlock {
    fn get_bulk(
        &self,
        mut index: usize,
        arr: &mut [i64],
        mut off: usize,
        mut len: usize,
    ) -> Result<u32, DataIOError> {
        assert!(index < self.value_count as usize, "index out of bounds");
        len = len.min(self.value_count as usize - index);
        assert!(
            off + len <= arr.len(),
            "not enough space in destination array"
        );

        let original_index = index;

        // Go to the next block boundary
        let values_per_block = 64 / self.bits_per_value;
        let offset_in_block = index % values_per_block as usize;
        if offset_in_block != 0 {
            for i in offset_in_block..values_per_block as usize {
                if len == 0 {
                    debug_assert!((index - original_index) <= u32::MAX as usize);
                    return Ok((index - original_index) as u32);
                }
                arr[off] = self.get(index)? as i64;
                off += 1;
                index += 1;
                len -= 1;
            }
            if len == 0 {
                debug_assert!((index - original_index) <= u32::MAX as usize);
                return Ok((index - original_index) as u32);
            }
        }

        // Bulk get
        assert_eq!(
            index % values_per_block as usize,
            0,
            "index not aligned with block boundary"
        );
        let decoder = of(Format::Packed(Packed), self.bits_per_value);
        assert_eq!(
            Decoder::long_value_count(decoder),
            1,
            "Decoder longBlockCount mismatch"
        );
        assert_eq!(
            Decoder::long_value_count(decoder),
            values_per_block,
            "Decoder longValueCount mismatch"
        );
        let block_index = index / values_per_block as usize;
        let nblocks = (index + len) / values_per_block as usize - block_index;
        decoder.decode_long_to_long(&self.blocks, block_index, arr, off, nblocks as u32);
        let diff = nblocks * values_per_block as usize;
        index += diff;
        len -= diff;

        if index > original_index {
            // Stay at the block boundary
            debug_assert!(index - original_index <= u32::MAX as usize);
            Ok((index - original_index) as u32)
        } else {
            // No progress so far => already at a block boundary but no full block to get
            assert_eq!(index, original_index, "Index mismatch");
            self.default_get_bulk(index, arr, off, len)
        }
    }
}

impl Accountable for Packed64SingleBlock {
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

impl Display for Packed64SingleBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Packed64SingleBlock(bitsPerValue={}, size={}, blocks={})",
            self.bits_per_value,
            self.value_count,
            self.blocks.len()
        )
    }
}

impl Mutable for Packed64SingleBlock {
    fn set_bulk(&mut self, mut index: usize, arr: &[i64], mut off: usize, mut len: usize) -> u32 {
        assert!(index < self.value_count as usize, "index out of bounds");
        len = len.min(self.value_count as usize - index);
        assert!(off + len <= arr.len(), "not enough space in source array");

        let original_index = index;

        // go to the next block boundary
        let values_per_block = 64 / self.bits_per_value;
        let offset_in_block = index % values_per_block as usize;

        if offset_in_block != 0 {
            for i in offset_in_block..values_per_block as usize {
                if len == 0 {
                    debug_assert!((index - original_index) <= u32::MAX as usize);
                    return (index - original_index) as u32;
                }
                self.set(index, arr[off]);
                off += 1;
                index += 1;
                len -= 1;
            }
            if len == 0 {
                debug_assert!((index - original_index) <= u32::MAX as usize);
                return (index - original_index) as u32;
            }
        }

        // Bulk set
        assert_eq!(
            index % values_per_block as usize,
            0,
            "index not aligned with block boundary"
        );

        let op = of(
            Format::PackedSingleBlock(PackedSingleBlock),
            self.bits_per_value,
        );
        assert_eq!(Decoder::long_block_count(op), 1, "longBlockCount mismatch");
        assert_eq!(
            Decoder::long_block_count(op),
            values_per_block,
            "longValueCount mismatch"
        );

        let block_index = index / values_per_block as usize;
        let nblocks = (index + len) / values_per_block as usize - block_index;

        op.encode_long_to_long(arr, off, &mut self.blocks, block_index, nblocks as u32);

        let diff = nblocks * values_per_block as usize;
        index += diff;
        len -= diff;

        if index > original_index {
            // Stay at the block boundary
            debug_assert!(index - original_index <= u32::MAX as usize);
            (index - original_index) as u32
        } else {
            // No progress so far => already at a block boundary but no full block to set
            assert_eq!(index, original_index, "Index mismatch");
            self.default_set_bulk(index, arr, off, len)
        }
    }

    fn fill(&mut self, mut from_index: usize, to_index: usize, val: i64) {
        assert!(from_index <= to_index, "from_index must be <= to_index");
        assert!(
            PackedInts::unsigned_bits_required(val as u64) <= self.bits_per_value,
            "Value requires more bits than allowed by bits_per_value"
        );

        let values_per_block = 64 / self.bits_per_value;

        // If the range is too small, fallback to naive setting
        if to_index - from_index <= (values_per_block * 2) as usize {
            for i in from_index..to_index {
                self.default_fill(from_index, to_index, val);
            }
            return;
        }

        // set values naively until the next block start
        let mut from_offset_in_block = from_index % values_per_block as usize;
        if from_offset_in_block != 0 {
            for _ in from_offset_in_block..values_per_block as usize {
                self.set(from_index, val);
                from_index += 1;
            }
            assert_eq!(from_index % values_per_block as usize, 0);
        }

        // Bulk set of inner blocks
        let from_block = from_index / values_per_block as usize;
        let to_block = to_index / values_per_block as usize;
        assert_eq!(from_block * values_per_block as usize, from_index);

        let mut block_value: u64 = 0;
        for i in 0..values_per_block {
            block_value |= (val as u64) << (i * self.bits_per_value);
        }

        self.blocks[from_block..to_block].fill(block_value);

        // Fill the gap at the end
        for i in (values_per_block as usize * to_block)..to_index {
            self.set(i, val);
        }
    }

    fn clear(&mut self) {
        self.blocks.fill(0);
    }
}
