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
use crate::util::error::runtime_error::RuntimeError;
use crate::util::packed::bulk_operation::of;
use crate::util::packed::format_behavior::PackedSingleBlockImpl;
use crate::util::packed::mutable_packed64_enum::MutablePacked64Enum;
use crate::util::packed::{Decoder, Encoder, Format, Mutable, MutableImpl, PackedInts, Reader};
use std::fmt::{Display, Formatter};

pub struct Packed64SingleBlock<T>
where
    T: Packed64SingleBlockBase,
{
    blocks: Vec<u64>,
    value_count: u32,
    bits_per_value: u32,
    sub_reader: T,
}
/// Checks if the given `bits_per_value` is supported.
const SUPPORTED_BITS_PER_VALUE: [u32; 14] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 16, 21, 32];
pub fn is_supported(bits_per_value: u32) -> bool {
    SUPPORTED_BITS_PER_VALUE
        .binary_search(&bits_per_value)
        .is_ok()
}
pub const MAX_SUPPORTED_BITS_PER_VALUE: u32 = 32;
impl<T> Packed64SingleBlock<T>
where
    T: Packed64SingleBlockBase,
{
    /// Supported bits per value
    fn required_capacity(value_count: u32, values_per_block: u32) -> u32 {
        value_count / values_per_block
            + if value_count % values_per_block == 0 {
                0
            } else {
                1
            }
    }
    pub fn new(bits_per_value: u32, value_count: u32, sub_reader: T) -> Self {
        debug_assert!(
            bits_per_value > 0 && bits_per_value <= 64,
            "bitsPerValue must be > 0 and <= 64"
        );
        assert!(
            is_supported(bits_per_value),
            "Unsupported bits_per_value: {}",
            bits_per_value
        );
        let values_per_block = 64 / bits_per_value;
        let required_capacity = Self::required_capacity(value_count, values_per_block);
        Self {
            blocks: vec![0; required_capacity as usize],
            value_count,
            bits_per_value,
            sub_reader,
        }
    }
}

impl<T> Reader for Packed64SingleBlock<T>
where
    T: Packed64SingleBlockBase,
{
    fn get(&mut self, _index: usize) -> Result<i64, DataIOError> {
        Ok(self.sub_reader.get(_index, &mut self.blocks))
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
            "not enough space in destination array"
        );

        let original_index = index;

        // Go to the next block boundary
        let values_per_block = 64 / self.bits_per_value;
        let offset_in_block = index % values_per_block as usize;
        if offset_in_block != 0 {
            for _ in offset_in_block..values_per_block as usize {
                if len == 0 {
                    debug_assert!((index - original_index) <= u32::MAX as usize);
                    return Ok((index - original_index) as u32);
                }
                arr[off] = self.sub_reader.get(index, &mut self.blocks);
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
        let decoder = of(
            Format::PackedSingleBlock(PackedSingleBlockImpl::new(1)),
            self.bits_per_value,
        );
        assert_eq!(
            Decoder::long_block_count(decoder),
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
        decoder.decode_u64_to_i64(&self.blocks, block_index, arr, off, nblocks as u32);
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

    fn size(&self) -> u32 {
        self.value_count
    }
}
pub fn create(value_count: u32, bits_per_value: u32) -> Result<MutablePacked64Enum, RuntimeError> {
    match bits_per_value {
        1 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock1 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock1(reader))
        }
        2 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock2 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock2(reader))
        }
        3 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock3 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock3(reader))
        }
        4 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock4 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock4(reader))
        }
        5 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock5 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock5(reader))
        }
        6 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock6 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock6(reader))
        }
        7 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock7 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock7(reader))
        }
        8 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock8 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock8(reader))
        }
        9 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock9 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock9(reader))
        }
        10 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock10 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock10(reader))
        }
        12 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock12 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock12(reader))
        }
        16 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock16 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock16(reader))
        }
        21 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock21 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock21(reader))
        }
        32 => {
            let sub_reader =
                Packed64SingleBlock::new(bits_per_value, value_count, Packed64SingleBlock32 {});
            let reader = MutableImpl::new(sub_reader);
            Ok(MutablePacked64Enum::P64SingleBlock32(reader))
        }
        _ => Err(RuntimeError::illegal_argument(format!(
            "Unsupported number of bits per value:  {}",
            bits_per_value
        ))),
    }
}
impl<T> Accountable for Packed64SingleBlock<T>
where
    T: Packed64SingleBlockBase,
{
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

impl<T> Display for Packed64SingleBlock<T>
where
    T: Packed64SingleBlockBase,
{
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

impl<T> Mutable for Packed64SingleBlock<T>
where
    T: Packed64SingleBlockBase,
{
    fn get_bits_per_value(&self) -> u32 {
        self.bits_per_value
    }

    fn set(&mut self, index: usize, value: i64) -> Result<(), DataIOError> {
        self.sub_reader.set(index, value, &mut self.blocks);
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
        assert!(off + len <= arr.len(), "not enough space in source array");

        let original_index = index;

        // go to the next block boundary
        let values_per_block = 64 / self.bits_per_value;
        let offset_in_block = index % values_per_block as usize;

        if offset_in_block != 0 {
            for _ in offset_in_block..values_per_block as usize {
                if len == 0 {
                    debug_assert!((index - original_index) <= u32::MAX as usize);
                    return Ok((index - original_index) as u32);
                }
                self.sub_reader.set(index, arr[off], &mut self.blocks);
                off += 1;
                index += 1;
                len -= 1;
            }
            if len == 0 {
                debug_assert!((index - original_index) <= u32::MAX as usize);
                return Ok((index - original_index) as u32);
            }
        }

        // Bulk set
        assert_eq!(
            index % values_per_block as usize,
            0,
            "index not aligned with block boundary"
        );

        let op = of(
            Format::PackedSingleBlock(PackedSingleBlockImpl::new(1)),
            self.bits_per_value,
        );
        assert_eq!(Decoder::long_block_count(op), 1, "longBlockCount mismatch");
        assert_eq!(
            Decoder::long_value_count(op),
            values_per_block,
            "longValueCount mismatch"
        );

        let block_index = index / values_per_block as usize;
        let nblocks = (index + len) / values_per_block as usize - block_index;

        op.encode_i64_to_u64(
            &arr[off..],
            0,
            &mut self.blocks,
            block_index,
            nblocks as u32,
        );

        let diff = nblocks * values_per_block as usize;
        index += diff;
        len -= diff;

        if index > original_index {
            // Stay at the block boundary
            debug_assert!(index - original_index <= u32::MAX as usize);
            Ok((index - original_index) as u32)
        } else {
            // No progress so far => already at a block boundary but no full block to set
            assert_eq!(index, original_index, "Index mismatch");
            self.default_set_bulk(index, arr, off, len)
        }
    }

    fn fill(
        &mut self,
        mut from_index: usize,
        to_index: usize,
        val: i64,
    ) -> Result<(), DataIOError> {
        assert!(from_index <= to_index, "from_index must be <= to_index");
        assert!(
            PackedInts::unsigned_bits_required(val) <= self.bits_per_value,
            "Value requires more bits than allowed by bits_per_value"
        );

        let values_per_block = 64 / self.bits_per_value;

        // If the range is too small, fallback to naive setting
        if to_index - from_index <= (values_per_block * 2) as usize {
            for _ in from_index..to_index {
                self.default_fill(from_index, to_index, val)?;
            }
            return Ok(());
        }

        // set values naively until the next block start
        let from_offset_in_block = from_index % values_per_block as usize;
        if from_offset_in_block != 0 {
            for _ in from_offset_in_block..values_per_block as usize {
                self.set(from_index, val)?;
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
            self.set(i, val)?;
        }
        Ok(())
    }

    fn clear(&mut self) -> Result<(), DataIOError> {
        self.blocks.fill(0);
        Ok(())
    }
}

#[allow(dead_code)]
pub struct Packed64SingleBlock1 {}
impl Packed64SingleBlockBase for Packed64SingleBlock1 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index >> 6;
        let b = index & 63;
        let shift = b;
        ((blocks[o] >> shift) & 1) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index >> 6;
        let b = index & 63;
        let shift = b;
        blocks[o] = (blocks[o] & !(1 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock2 {}
impl Packed64SingleBlockBase for Packed64SingleBlock2 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index >> 5;
        let b = index & 31;
        let shift = b << 1;
        ((blocks[o] >> shift) & 3) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index >> 5;
        let b = index & 31;
        let shift = b << 1;
        blocks[o] = (blocks[o] & !(3 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock3 {}
impl Packed64SingleBlockBase for Packed64SingleBlock3 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index / 21;
        let b = index % 21;
        let shift = b * 3;
        ((blocks[o] >> shift) & 7) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index / 21;
        let b = index % 21;
        let shift = b * 3;
        blocks[o] = (blocks[o] & !(7 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock4 {}
impl Packed64SingleBlockBase for Packed64SingleBlock4 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index >> 4;
        let b = index & 15;
        let shift = b << 2;
        ((blocks[o] >> shift) & 15) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index >> 4;
        let b = index & 15;
        let shift = b << 2;
        blocks[o] = (blocks[o] & !(15 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock5 {}
impl Packed64SingleBlockBase for Packed64SingleBlock5 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index / 12;
        let b = index % 12;
        let shift = b * 5;
        ((blocks[o] >> shift) & 31) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index / 12;
        let b = index % 12;
        let shift = b * 5;
        blocks[o] = (blocks[o] & !(31 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock6 {}
impl Packed64SingleBlockBase for Packed64SingleBlock6 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index / 10;
        let b = index % 10;
        let shift = b * 6;
        ((blocks[o] >> shift) & 63) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index / 10;
        let b = index % 10;
        let shift = b * 6;
        blocks[o] = (blocks[o] & !(63 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock7 {}
impl Packed64SingleBlockBase for Packed64SingleBlock7 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index / 9;
        let b = index % 9;
        let shift = b * 7;
        ((blocks[o] >> shift) & 127) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index / 9;
        let b = index % 9;
        let shift = b * 7;
        blocks[o] = (blocks[o] & !(127 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock8 {}
impl Packed64SingleBlockBase for Packed64SingleBlock8 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index >> 3;
        let b = index & 7;
        let shift = b << 3;
        ((blocks[o] >> shift) & 255) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index >> 3;
        let b = index & 7;
        let shift = b << 3;
        blocks[o] = (blocks[o] & !(255 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock9 {}
impl Packed64SingleBlockBase for Packed64SingleBlock9 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index / 7;
        let b = index % 7;
        let shift = b * 9;
        ((blocks[o] >> shift) & 511) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index / 7;
        let b = index % 7;
        let shift = b * 9;
        blocks[o] = (blocks[o] & !(511 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock10 {}
impl Packed64SingleBlockBase for Packed64SingleBlock10 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index / 6;
        let b = index % 6;
        let shift = b * 10;
        ((blocks[o] >> shift) & 1023) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index / 6;
        let b = index % 6;
        let shift = b * 10;
        blocks[o] = (blocks[o] & !(1023 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock12 {}
impl Packed64SingleBlockBase for Packed64SingleBlock12 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index / 5;
        let b = index % 5;
        let shift = b * 12;
        ((blocks[o] >> shift) & 4095) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index / 5;
        let b = index % 5;
        let shift = b * 12;
        blocks[o] = (blocks[o] & !(4095 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock16 {}
impl Packed64SingleBlockBase for Packed64SingleBlock16 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index >> 2;
        let b = index & 3;
        let shift = b << 4;
        ((blocks[o] >> shift) & 65535) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index >> 2;
        let b = index & 3;
        let shift = b << 4;
        blocks[o] = (blocks[o] & !(65535 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock21 {}
impl Packed64SingleBlockBase for Packed64SingleBlock21 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index / 3;
        let b = index % 3;
        let shift = b * 21;
        ((blocks[o] >> shift) & 2097151) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index / 3;
        let b = index % 3;
        let shift = b * 21;
        blocks[o] = (blocks[o] & !(2097151 << shift)) | ((value as u64) << shift);
    }
}
#[allow(dead_code)]
pub struct Packed64SingleBlock32 {}
impl Packed64SingleBlockBase for Packed64SingleBlock32 {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64 {
        let o = index >> 1;
        let b = index & 1;
        let shift = b << 5;
        ((blocks[o] >> shift) & 4294967295) as i64
    }

    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]) {
        let o = index >> 1;
        let b = index & 1;
        let shift = b << 5;
        blocks[o] = (blocks[o] & !(4294967295 << shift)) | ((value as u64) << shift);
    }
}

pub trait Packed64SingleBlockBase {
    fn get(&self, index: usize, blocks: &mut [u64]) -> i64;
    fn set(&mut self, index: usize, value: i64, blocks: &mut [u64]);
}
