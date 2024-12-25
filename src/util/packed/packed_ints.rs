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
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::longs_ref::LongsRef;
use std::cmp::min;
use crate::util::packed::format_behavior::{FormatBehavior, Packed, PackedSingleBlock};

pub struct PackedInts;
/// At most 700% memory overhead, always select a direct implementation.
pub const FASTEST: f32 = 7.0;

/// At most 50% memory overhead, always select a reasonably fast implementation.
pub const FAST: f32 = 0.5;

/// At most 25% memory overhead.
pub const DEFAULT: f32 = 0.25;

/// No memory overhead at all, but the returned implementation may be slow.
pub const COMPACT: f32 = 0.0;

/// Default amount of memory to use for bulk operations (1KB).
pub const DEFAULT_BUFFER_SIZE: i32 = 1024;

/// Codec name for PackedInts.
pub const CODEC_NAME: &str = "PackedInts";

/// Version constants.
pub const VERSION_MONOTONIC_WITHOUT_ZIGZAG: i32 = 2;
pub const VERSION_START: i32 = VERSION_MONOTONIC_WITHOUT_ZIGZAG;
pub const VERSION_CURRENT: i32 = VERSION_MONOTONIC_WITHOUT_ZIGZAG;

/// Check the validity of a version number.
///
/// # Arguments
///
/// * `version` - The version number to check.
///
/// # Errors
///
/// Returns an `IllegalArgumentError` if the version is out of bounds.
pub fn check_version(version: i32) -> Result<(), DataIOError> {
    if version < VERSION_START {
        return Err(DataIOError::illegal_argument(format!(
            "Version is too old, should be at least {} (got {})",
            VERSION_START, version
        )));
    } else if version > VERSION_CURRENT {
        return Err(DataIOError::illegal_argument(format!(
            "Version is too new, should be at most {} (got {})",
            VERSION_CURRENT, version
        )));
    }
    Ok(())
}
/// A format to write packed integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Compact format, all bits are written contiguously.
    Packed(Packed),

    /// A format that may insert padding bits to improve encoding and decoding speed.
    /// This format is deprecated; use `Packed` instead.
    PackedSingleBlock(PackedSingleBlock),
}
/// Represents a combination of Format and bitsPerValue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatAndBits {
    pub format: Format,
    pub bits_per_value: u32,
}
/// Find the fastest [`Format`] and `bits_per_value` that would restore from disk
/// with overhead less than the specified acceptable overhead ratio.
///
/// # Arguments
///
/// * `value_count` - The number of values to write. Use `usize::MAX` if unknown.
/// * `bits_per_value` - The number of bits per value.
/// * `acceptable_overhead_ratio` - The acceptable overhead ratio.
///
/// # Returns
///
/// A `FormatAndBits` struct containing the selected format and bits per value.
#[allow(unused)] // `value_count` is not used in Java Lucene
pub fn fastest_format_and_bits(
    mut value_count: i32,
    bits_per_value: u32,
    mut acceptable_overhead_ratio: f32,
) -> FormatAndBits {
    // Handle unknown value count
    if value_count == -1 {
        value_count = i32::MAX;
    }

    acceptable_overhead_ratio = acceptable_overhead_ratio.clamp(COMPACT, FASTEST);
    let acceptable_overhead_per_value = acceptable_overhead_ratio * bits_per_value as f32;
    let max_bits_per_value = bits_per_value + acceptable_overhead_per_value as u32;

    let actual_bits_per_value = if bits_per_value <= 8 && max_bits_per_value >= 8 {
        8
    } else if bits_per_value <= 16 && max_bits_per_value >= 16 {
        16
    } else if bits_per_value <= 32 && max_bits_per_value >= 32 {
        32
    } else if bits_per_value <= 64 && max_bits_per_value >= 64 {
        64
    } else {
        bits_per_value
    };

    FormatAndBits {
        format: Format::Packed(Packed),
        bits_per_value: actual_bits_per_value,
    }
}
/// A decoder for packed integers.
pub trait Decoder {
    /// The minimum number of long blocks to encode in a single iteration, when using long encoding.
    fn long_block_count(&self) -> u32 {
        unreachable!("long_block_count() must be implemented if it need to be used")
    }

    /// The number of values that can be stored in `long_block_count()` long blocks.
    fn long_value_count(&self) -> u32 {
        unreachable!("long_value_count() must be implemented if it need to be used")
    }

    /// The minimum number of byte blocks to encode in a single iteration, when using byte encoding.
    fn byte_block_count(&self) -> u32 {
        unreachable!("byte_block_count() must be implemented if it need to be used")
    }

    /// The number of values that can be stored in `byte_block_count()` byte blocks.
    fn byte_value_count(&self) -> u32 {
        unreachable!("byte_value_count() must be implemented if it need to be used")
    }

    /// Read `iterations * block_count()` blocks from `blocks`, decode them, and write
    /// `iterations * value_count()` values into `values`.
    ///
    /// # Arguments
    ///
    /// * `blocks` - The long blocks that hold packed integer values.
    /// * `blocks_offset` - The offset where to start reading blocks.
    /// * `values` - The buffer to write the decoded values into.
    /// * `values_offset` - The offset where to start writing values.
    /// * `iterations` - Controls how much data to decode.
    fn decode_long_to_long(
        &self,
        _blocks: &[u64],
        _blocks_offset: usize,
        values: &mut [i64],
        _values_offset: usize,
        _iterations: u32,
    ) {
        unreachable!("decode_long_to_long() must be implemented if it need to be used")
    }

    /// Read `8 * iterations * block_count()` blocks from `blocks`, decode them, and write
    /// `iterations * value_count()` values into `values`.
    ///
    /// # Arguments
    ///
    /// * `blocks` - The long blocks that hold packed integer values.
    /// * `blocks_offset` - The offset where to start reading blocks.
    /// * `values` - The buffer to write the decoded values into.
    /// * `values_offset` - The offset where to start writing values.
    /// * `iterations` - Controls how much data to decode.
    fn decode_byte_to_long(
        &self,
        blocks: &[u8],
        _blocks_offset: usize,
        values: &mut [i64],
        _values_offset: usize,
        _iterations: u32,
    ) {
        unreachable!("decode_byte_to_long() must be implemented if it need to be used")
    }

    /// Read `iterations * block_count()` blocks from `blocks`, decode them, and write
    /// `iterations * value_count()` values into `values`.
    ///
    /// # Arguments
    ///
    /// * `blocks` - The long blocks that hold packed integer values.
    /// * `blocks_offset` - The offset where to start reading blocks.
    /// * `values` - The buffer to write the decoded values into.
    /// * `values_offset` - The offset where to start writing values.
    /// * `iterations` - Controls how much data to decode.
    fn decode_long_to_int(
        &self,
        _blocks: &[u64],
        _blocks_offset: usize,
        values: &mut [i32],
        _values_offset: usize,
        _iterations: u32,
    ) {
        unreachable!("decode_long_to_int() must be implemented if it need to be used")
    }

    /// Read `8 * iterations * block_count()` blocks from `blocks`, decode them, and write
    /// `iterations * value_count()` values into `values`.
    ///
    /// # Arguments
    ///
    /// * `blocks` - The long blocks that hold packed integer values.
    /// * `blocks_offset` - The offset where to start reading blocks.
    /// * `values` - The buffer to write the decoded values into.
    /// * `values_offset` - The offset where to start writing values.
    /// * `iterations` - Controls how much data to decode.
    fn decode_byte_to_int(
        &self,
        blocks: &[u8],
        _blocks_offset: usize,
        values: &mut [i32],
        _values_offset: usize,
        _iterations: u32,
    ) {
        unreachable!("decode_byte_to_int() must be implemented")
    }
}
/// An encoder for packed integers.
pub trait Encoder {
    /// The minimum number of long blocks to encode in a single iteration, when using long encoding.
    fn long_block_count(&self) -> u32 {
        unreachable!("long_block_count() must be implemented if it need to be used")
    }

    /// The number of values that can be stored in `long_block_count()` long blocks.
    fn long_value_count(&self) -> u32 {
        unreachable!("long_value_count() must be implemented if it need to be used")
    }

    /// The minimum number of byte blocks to encode in a single iteration, when using byte encoding.
    fn byte_block_count(&self) -> u32 {
        unreachable!("byte_block_count() must be implemented if it need to be used")
    }

    /// The number of values that can be stored in `byte_block_count()` byte blocks.
    fn byte_value_count(&self) -> u32 {
        unreachable!("byte_value_count() must be implemented if it need to be used")
    }

    /// Read `iterations * value_count()` values from `values`, encode them, and write
    /// `iterations * block_count()` blocks into `blocks`.
    ///
    /// # Arguments
    ///
    /// * `values` - The buffer containing values to encode.
    /// * `values_offset` - The offset where to start reading values.
    /// * `blocks` - The buffer to write encoded blocks into.
    /// * `blocks_offset` - The offset where to start writing blocks.
    /// * `iterations` - Controls how much data to encode.
    fn encode_long_to_long(
        &self,
        _values: &[i64],
        _values_offset: usize,
        _blocks: &mut [u64],
        _blocks_offset: usize,
        _iterations: u32,
    ) {
        unreachable!("encode_long_to_long() must be implemented if it need to be used")
    }

    /// Read `iterations * value_count()` values from `values`, encode them, and write
    /// `8 * iterations * block_count()` blocks into `blocks`.
    ///
    /// # Arguments
    ///
    /// * `values` - The buffer containing values to encode.
    /// * `values_offset` - The offset where to start reading values.
    /// * `blocks` - The buffer to write encoded blocks into.
    /// * `blocks_offset` - The offset where to start writing blocks.
    /// * `iterations` - Controls how much data to encode.
    fn encode_long_to_byte(
        &self,
        _values: &[i64],
        _values_offset: usize,
        _blocks: &mut [u8],
        _blocks_offset: usize,
        _iterations: u32,
    ) {
        unreachable!("encode_long_to_byte() must be implemented if it need to be used")
    }

    /// Read `iterations * value_count()` values from `values`, encode them, and write
    /// `iterations * block_count()` blocks into `blocks`.
    ///
    /// # Arguments
    ///
    /// * `values` - The buffer containing values to encode.
    /// * `values_offset` - The offset where to start reading values.
    /// * `blocks` - The buffer to write encoded blocks into.
    /// * `blocks_offset` - The offset where to start writing blocks.
    /// * `iterations` - Controls how much data to encode.
    fn encode_int_to_long(
        &self,
        _values: &[i32],
        _values_offset: usize,
        _blocks: &mut [u64],
        _blocks_offset: usize,
        _iterations: u32,
    ) {
        unreachable!("encode_int_to_long() must be implemented if it need to be used")
    }

    /// Read `iterations * value_count()` values from `values`, encode them, and write
    /// `8 * iterations * block_count()` blocks into `blocks`.
    ///
    /// # Arguments
    ///
    /// * `values` - The buffer containing values to encode.
    /// * `values_offset` - The offset where to start reading values.
    /// * `blocks` - The buffer to write encoded blocks into.
    /// * `blocks_offset` - The offset where to start writing blocks.
    /// * `iterations` - Controls how much data to encode.
    fn encode_int_to_byte(
        &self,
        _values: &[i32],
        _values_offset: usize,
        _blocks: &mut [u8],
        _blocks_offset: usize,
        _iterations: u32,
    ) {
        unreachable!("encode_int_to_byte() must be implemented if it need to be used")
    }
}

/// A read-only random access array of positive integers.
pub trait Reader {
    /// Get the value at the given index.
    fn get(&self, index: usize) -> Result<u64, DataIOError>;

    /// Bulk get: read at least one and at most `len` values starting from `index`
    /// into `arr[off..off+len]` and return the actual number of values that have been read.
    fn bulk_get(
        &self,
        index: usize,
        arr: &mut [u64],
        off: usize,
        len: usize,
    ) -> Result<usize, DataIOError> {
        debug_assert!(len > 0, "len must be > 0");
        debug_assert!(
            index < self.size() as usize,
            "index out of bounds: {}",
            index
        );
        debug_assert!(off + len <= arr.len(), "offset + len exceeds array length");

        let gets = min(self.size() as usize - index, len);
        for (i, o) in (index..index + gets).zip(off..off + gets) {
            arr[o] = self.get(i)?;
        }
        Ok(gets)
    }

    /// Returns the number of values in the reader.
    fn size(&self) -> u32;
}
/// Run-once iterator interface to decode previously saved PackedInts.
pub trait ReaderIterator {
    /// Returns the next value.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue decoding the next value.
    fn next(&mut self) -> Result<i64, DataIOError>{
        unreachable!("next() must be implemented if it need to be used")
    }

    /// Returns at least 1 and at most `count` next values.
    ///
    /// The returned reference MUST NOT be modified.
    ///
    /// # Arguments
    ///
    /// * `count` - The maximum number of values to retrieve.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue decoding the values.
    fn next_batch(&mut self, count: u32) -> Result<LongsRef, DataIOError>;

    /// Returns the number of bits per value.
    fn get_bits_per_value(&self) -> u32{
        unreachable!("get_bits_per_value() must be implemented if it need to be used")
    }

    /// Returns the total number of values.
    fn size(&self) -> u32{
        unreachable!("size() must be implemented if it need to be used")
    }

    /// Returns the current position.
    fn ord(&self) -> i32;
}
/// A base implementation of the `ReaderIterator` trait.
pub(crate) struct ReaderIteratorImpl<C>
where
    C: ReaderIterator,
{
    bits_per_value: u32,
    value_count: u32,
    next_values: Option<LongsRef>,
    sub_reader: C,
}

impl<C> ReaderIteratorImpl<C>
where
    C: ReaderIterator,
{
    /// Creates a new `ReaderIteratorImpl`.
    ///
    /// # Arguments
    ///
    /// * `value_count` - Total number of values.
    /// * `bits_per_value` - Number of bits per value.
    pub fn new(value_count: u32, bits_per_value: u32, sub_reader: C) -> Self {
        Self {
            bits_per_value,
            value_count,
            next_values: None,
            sub_reader,
        }
    }
}

impl<C> ReaderIterator for ReaderIteratorImpl<C>
where
    C: ReaderIterator,
{
    fn next(&mut self) -> Result<i64, DataIOError> {
        let mut next_values = self.next_batch(1)?;
        debug_assert!(next_values.length > 0, "next_values buffer is empty");
        let result = next_values.longs[next_values.offset];
        next_values.offset += 1;
        next_values.length -= 1;
        Ok(result)
    }

    fn next_batch(&mut self, count: u32) -> Result<LongsRef, DataIOError> {
        self.sub_reader.next_batch(count)
    }

    fn get_bits_per_value(&self) -> u32 {
        self.bits_per_value
    }

    fn size(&self) -> u32 {
        self.value_count
    }

    fn ord(&self) -> i32 {
        self.sub_reader.ord()
    }
}

impl PackedInts {
    pub fn max_value(bits_per_value: u32) -> u64 {
        if bits_per_value == 64 {
            u64::MAX
        } else {
            (1u64 << bits_per_value) - 1
        }
    }
    /// Returns how many bits are required to store `bits`, interpreted as an unsigned value.
    /// NOTE: This method returns at least 1.
    ///
    /// # Arguments
    /// - `bits`: The unsigned value for which to determine the required bit count.
    ///
    /// # Returns
    /// The number of bits required to store `bits`.
    pub fn unsigned_bits_required(bits: u64) -> u32 {
        (64 - bits.leading_zeros() as usize).max(1) as u32
    }
}
