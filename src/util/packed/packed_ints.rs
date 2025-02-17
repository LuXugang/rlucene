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
use crate::store::{DataInput, DataOutput};
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::LuceneError;

use crate::util::longs_ref::LongsRef;
use crate::util::packed::bulk_operation::of;
use crate::util::packed::bulk_operation_packed_enum::BulkOperationPackedEnum;
use crate::util::packed::format_behavior::{PackedImpl, PackedSingleBlockImpl};
use crate::util::packed::mutable_packed64_enum::MutablePacked64Enum;
use crate::util::packed::packed64::Packed64;
use crate::util::packed::packed64_single_block::create;
use crate::util::packed::packed_reader_iterator::PackedReaderIterator;
use crate::util::packed::packed_writer::PackedWriter;
use std::cmp::min;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::string::ToString;

#[allow(dead_code)]
pub struct PackedInts;
impl PackedInts {
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
    pub const CODEC_NAME: &'static str = "PackedInts";

    /// Version constants.
    pub const VERSION_MONOTONIC_WITHOUT_ZIGZAG: i32 = 2;
    pub const VERSION_START: i32 = Self::VERSION_MONOTONIC_WITHOUT_ZIGZAG;
    pub const VERSION_CURRENT: i32 = Self::VERSION_MONOTONIC_WITHOUT_ZIGZAG;
    /// Get a [`Decoder`].
    ///
    /// # Arguments
    /// - `format`: The format used to store packed integers.
    /// - `version`: The compatibility version.
    /// - `bits_per_value`: The number of bits per value.
    ///
    /// # Returns
    /// A decoder.
    pub(crate) fn get_decoder(
        format: Format,
        version: i32,
        bits_per_value: i32,
    ) -> Result<&'static BulkOperationPackedEnum, LuceneError> {
        check_version(version)?;
        Ok(of(format, bits_per_value))
    }
    /// Get an [`Encoder`].
    ///
    /// # Arguments
    /// - `format`: The format used to store packed integers.
    /// - `version`: The compatibility version.
    /// - `bits_per_value`: The number of bits per value.
    ///
    /// # Returns
    /// A result containing a reference to the encoder, or an error if the version is invalid.
    pub(crate) fn get_encoder(
        format: Format,
        version: i32,
        bits_per_value: i32,
    ) -> Result<&'static BulkOperationPackedEnum, LuceneError> {
        PackedInts::get_decoder(format, version, bits_per_value)
    }
    /// Expert: Restore a [`ReaderIterator`] from a stream without reading metadata at the
    /// beginning of the stream. This method is useful to restore data from streams which have been
    /// created using `PackedInts::get_writer_no_header`.
    ///
    /// # Arguments
    /// - `input`: The stream to read data from, positioned at the beginning of the packed values.
    /// - `format`: The format used to serialize.
    /// - `version`: The version used to serialize the data.
    /// - `value_count`: How many values the stream holds.
    /// - `bits_per_value`: The number of bits per value.
    /// - `mem`: How much memory the iterator is allowed to use to read-ahead (likely to speed up iteration).
    ///
    /// # Returns
    /// A `ReaderIterator`.
    ///
    /// # Errors
    /// Returns an error if the version is invalid.
    pub fn get_reader_iterator_no_header<T>(
        input: &mut T,
        format: Format,
        version: i32,
        value_count: i32,
        bits_per_value: i32,
        mem: i32,
    ) -> Result<ReaderIteratorImpl<impl ReaderIterator + use<'_, T>>, LuceneError>
    where
        T: DataInput,
    {
        check_version(version)?;
        let sub_reader =
            PackedReaderIterator::new(format, version, value_count, bits_per_value, input, mem);
        Ok(ReaderIteratorImpl::new(
            value_count,
            bits_per_value,
            sub_reader,
        ))
    }
    /// Create a packed integer array with the given amount of values initialized to 0. The `value_count`
    /// and the `bits_per_value` cannot be changed after creation. All mutables known by this factory
    /// are kept fully in RAM.
    ///
    /// Positive values of `acceptable_overhead_ratio` will trade space for speed by selecting a faster
    /// but potentially less memory-efficient implementation. An `acceptable_overhead_ratio` of
    /// [`PackedInts::COMPACT`] will make sure that the most memory-efficient implementation is selected,
    /// whereas [`PackedInts::FASTEST`] will make sure that the fastest implementation is selected.
    ///
    /// # Arguments
    /// - `value_count`: The number of elements.
    /// - `bits_per_value`: The number of bits available for any given value.
    /// - `acceptable_overhead_ratio`: An acceptable overhead ratio per value.
    ///
    /// # Returns
    /// A mutable packed integer array.
    ///
    pub(crate) fn get_mutable(
        value_count: i32,
        bits_per_value: i32,
        acceptable_overhead_ratio: f32,
    ) -> Result<MutablePacked64Enum, LuceneError> {
        let format_and_bits =
            fastest_format_and_bits(value_count, bits_per_value, acceptable_overhead_ratio);
        PackedInts::get_mutable_impl(
            value_count,
            format_and_bits.bits_per_value,
            format_and_bits.format,
        )
    }

    /// Same as [`get_mutable`](PackedInts::get_mutable) with a pre-computed number of bits per value and format.
    pub(crate) fn get_mutable_impl(
        value_count: i32,
        bits_per_value: i32,
        format: Format,
    ) -> Result<MutablePacked64Enum, LuceneError> {
        debug_assert!(value_count >= 0);
        match format {
            Format::PackedSingleBlock(_) => Ok(create(value_count, bits_per_value)?),
            Format::Packed(_) => Ok(MutablePacked64Enum::P64(MutableImpl::new(Packed64::new(
                value_count,
                bits_per_value,
            )))),
        }
    }
    /// Expert: Create a packed integer array writer for the given output, format, value count, and
    /// number of bits per value.
    ///
    /// The resulting stream will be long-aligned. This means that depending on the format which is
    /// used, up to 63 bits will be wasted. An easy way to make sure that no space is lost is to always
    /// use a `value_count` that is a multiple of 64.
    ///
    /// This method does not write any metadata to the stream, meaning that it is your responsibility
    /// to store it somewhere else in order to be able to recover data from the stream later on:
    ///
    /// - `format` (using [`Format::get_id`](crate::util::packed::FormatBehavior::get_id))
    /// - `value_count`
    /// - `bits_per_value`
    /// - [`PackedInts::VERSION_CURRENT`].
    ///
    /// It is possible to start writing values without knowing how many of them you are actually
    /// going to write. To do this, just pass `-1` as `value_count`. On the other hand, for any positive
    /// value of `value_count`, the returned writer will make sure that you don't write more values than
    /// expected and pad the end of the stream with zeros in case you have written less than `value_count`
    /// when calling [`Writer::finish`].
    ///
    /// The `mem` parameter lets you control how much memory can be used to buffer changes in memory
    /// before flushing to disk. High values of `mem` are likely to improve throughput. On the other
    /// hand, if speed is not that important to you, a value of `0` will use as little memory as possible
    /// and should already offer reasonable throughput.
    ///
    /// # Arguments
    /// - `out`: The data output.
    /// - `format`: The format to use to serialize the values.
    /// - `value_count`: The number of values.
    /// - `bits_per_value`: The number of bits per value.
    /// - `mem`: How much memory (in bytes) can be used to speed up serialization.
    ///
    /// # Returns
    /// A `Writer` instance.
    ///
    #[allow(unused)]
    pub(crate) fn get_writer_no_header<T>(
        out: &'_ mut T,
        format: Format,
        value_count: i32,
        bits_per_value: i32,
        mem: i32,
    ) -> PackedWriter<'_, T>
    where
        T: DataOutput,
    {
        PackedWriter::new(format, out, value_count, bits_per_value, mem)
    }

    /// Returns how many bits are required to hold values up to and including `max_value`.
    ///
    /// This method will always return at least 1.
    ///
    /// # Arguments
    ///
    /// * `max_value` - The maximum value that should be representable.
    ///
    /// # Returns
    ///
    /// The number of bits needed to represent values from 0 to `max_value`.
    ///
    pub fn bits_required(max_value: i64) -> Result<i32, LuceneError> {
        if max_value < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "max_value must be non-negative (got {})",
                max_value
            )));
        }
        Ok(PackedInts::unsigned_bits_required(max_value))
    }

    /// Returns how many bits are required to store `bits`, interpreted as an unsigned value.
    /// NOTE: This method returns at least 1.
    ///
    /// # Arguments
    /// - `bits`: The unsigned value for which to determine the required bit count.
    ///
    /// # Returns
    /// The number of bits required to store `bits`.
    pub fn unsigned_bits_required(bits: i64) -> i32 {
        (64 - bits.leading_zeros() as usize).max(1) as i32
    }

    /// Calculates the maximum unsigned long that can be expressed with the given number of bits.
    ///
    /// # Arguments
    ///
    /// * `bits_per_value` - The number of bits available for any given value.
    ///
    /// # Returns
    ///
    /// The maximum value for the given number of bits.
    pub fn max_value(bits_per_value: i32) -> i64 {
        if bits_per_value == 64 {
            i64::MAX
        } else {
            (1i64 << bits_per_value) - 1
        }
    }
    /// Copy `src[src_pos..src_pos+len]` into `dest[dest_pos..dest_pos+len]` using at most `mem` bytes.
    ///
    ///
    pub fn copy(
        src: &mut impl Reader,
        src_pos: i32,
        dest: &mut impl Mutable,
        dest_pos: i32,
        len: i32,
        mem: i32,
    ) -> Result<(), LuceneError> {
        debug_assert!(
            src_pos + len <= src.size(),
            "Source position and length out of bounds"
        );
        debug_assert!(
            dest_pos + len <= dest.size(),
            "Destination position and length out of bounds"
        );

        let capacity = mem >> 3;
        if capacity == 0 {
            for i in 0..len {
                dest.set(dest_pos + i, src.get(src_pos + i)?)?;
            }
        } else if len > 0 {
            // Use bulk operations
            let buf_size = capacity.min(len);
            let mut buf = vec![0; buf_size as usize];
            PackedInts::copy_with_buffer(src, src_pos, dest, dest_pos, len, &mut buf)?;
        }
        Ok(())
    }
    /// Same as `copy` but uses a pre-allocated buffer.
    ///
    pub fn copy_with_buffer(
        src: &mut impl Reader,
        mut src_pos: i32,
        dest: &mut impl Mutable,
        mut dest_pos: i32,
        mut len: i32,
        buf: &mut [i64],
    ) -> Result<(), LuceneError> {
        debug_assert!(!buf.is_empty(), "Buffer length must be greater than 0");

        let mut remaining = 0;

        while len > 0 {
            debug_assert!(buf.len() <= i32::MAX as usize);
            let read = src.get_bulk(
                src_pos,
                buf,
                remaining,
                len.min(buf.len() as i32 - remaining),
            )?;
            debug_assert!(read > 0, "Read operation failed");
            src_pos += read;
            len -= read;
            remaining += read;

            let written = dest.set_bulk(dest_pos, buf, 0, remaining)?;
            debug_assert!(written > 0, "Write operation failed");
            dest_pos += written;

            if written < remaining {
                buf.copy_within(written as usize..remaining as usize, 0);
            }
            remaining -= written;
        }

        while remaining > 0 {
            let written = dest.set_bulk(dest_pos, buf, 0, remaining)?;
            dest_pos += written;
            remaining -= written;
            if remaining > 0 {
                buf.copy_within(written as usize..(written + remaining) as usize, 0);
            }
        }
        Ok(())
    }
    /// Check that the block size is a power of 2, within the right bounds, and return its log in base 2.
    pub fn check_block_size(
        block_size: i32,
        min_block_size: i32,
        max_block_size: i32,
    ) -> Result<i32, LuceneError> {
        if block_size < min_block_size || block_size > max_block_size {
            return Err(LuceneError::illegal_argument(format!(
                "block_size must be >= {} and <= {}, got {}",
                min_block_size, max_block_size, block_size
            )));
        }

        if block_size & (block_size - 1) != 0 {
            return Err(LuceneError::illegal_argument(format!(
                "block_size must be a power of two, got {}",
                block_size
            )));
        }
        let result = block_size.trailing_zeros();
        debug_assert!(result <= i32::MAX as u32);
        Ok(result as i32)
    }
    /// Return the number of blocks required to store `size` values on `block_size`.
    pub fn num_blocks(size: i64, block_size: i32) -> Result<i32, LuceneError> {
        let num_blocks =
            (size / block_size as i64) + if size % block_size as i64 == 0 { 0 } else { 1 };
        let result = num_blocks.checked_mul(block_size as i64);
        match result {
            Some(result) => {
                if result < size {
                    return Err(LuceneError::illegal_argument(
                        "size is too large for this block size".to_string(),
                    ));
                }
                debug_assert!(num_blocks <= i32::MAX as i64);
                Ok(num_blocks as i32)
            }
            None => Err(LuceneError::illegal_argument(format!(
                "multiply overflow:block_size:{}, num_blocks:{} ",
                block_size, num_blocks
            ))),
        }
    }
}

/// Check the validity of a version number.
///
/// # Arguments
///
/// * `version` - The version number to check.
///
/// # Errors
///
/// Returns an `IllegalArgumentError` if the version is out of bounds.
pub fn check_version(version: i32) -> Result<(), LuceneError> {
    if version < PackedInts::VERSION_START {
        return Err(LuceneError::illegal_argument(format!(
            "Version is too old, should be at least {} (got {})",
            PackedInts::VERSION_START,
            version
        )));
    } else if version > PackedInts::VERSION_CURRENT {
        return Err(LuceneError::illegal_argument(format!(
            "Version is too new, should be at most {} (got {})",
            PackedInts::VERSION_CURRENT,
            version
        )));
    }
    Ok(())
}
/// A format to write packed integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Compact format, all bits are written contiguously.
    Packed(PackedImpl),

    /// A format that may insert padding bits to improve encoding and decoding speed.
    /// This format is deprecated; use `Packed` instead.
    PackedSingleBlock(PackedSingleBlockImpl),
}
impl Default for Format {
    fn default() -> Self {
        Format::Packed(PackedImpl::new(0))
    }
}
/// Represents a combination of Format and bitsPerValue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatAndBits {
    pub format: Format,
    pub bits_per_value: i32,
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
    bits_per_value: i32,
    mut acceptable_overhead_ratio: f32,
) -> FormatAndBits {
    acceptable_overhead_ratio =
        acceptable_overhead_ratio.clamp(PackedInts::COMPACT, PackedInts::FASTEST);
    let acceptable_overhead_per_value = acceptable_overhead_ratio * bits_per_value as f32;
    let max_bits_per_value = bits_per_value + acceptable_overhead_per_value as i32;

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
        format: Format::Packed(PackedImpl::new(0)),
        bits_per_value: actual_bits_per_value,
    }
}
/// A decoder for packed integers.
pub trait Decoder {
    /// The minimum number of long blocks to encode in a single iteration, when using long encoding.
    fn long_block_count(&self) -> i32 {
        unimplemented!("long_block_count() must be implemented if it need to be used")
    }

    /// The number of values that can be stored in `long_block_count()` long blocks.
    fn long_value_count(&self) -> i32 {
        unimplemented!("long_value_count() must be implemented if it need to be used")
    }

    /// The minimum number of byte blocks to encode in a single iteration, when using byte encoding.
    fn byte_block_count(&self) -> i32 {
        unimplemented!("byte_block_count() must be implemented if it need to be used")
    }

    /// The number of values that can be stored in `byte_block_count()` byte blocks.
    fn byte_value_count(&self) -> i32 {
        unimplemented!("byte_value_count() must be implemented if it need to be used")
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
    fn decode_u64_to_i64(
        &self,
        _blocks: &[u64],
        _blocks_offset: usize,
        _values: &mut [i64],
        _values_offset: usize,
        _iterations: i32,
    ) {
        unimplemented!("decode_long_to_long() must be implemented if it need to be used")
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
    fn decode_u8_to_i64(
        &self,
        _blocks: &[u8],
        _blocks_offset: usize,
        _values: &mut [i64],
        _values_offset: usize,
        _iterations: i32,
    ) {
        unimplemented!("decode_byte_to_long() must be implemented if it need to be used")
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
    fn decode_u64_to_i32(
        &self,
        _blocks: &[u64],
        _blocks_offset: usize,
        _values: &mut [i32],
        _values_offset: usize,
        _iterations: i32,
    ) {
        unimplemented!("decode_long_to_int() must be implemented if it need to be used")
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
    fn decode_u8_to_i32(
        &self,
        _blocks: &[u8],
        _blocks_offset: usize,
        _values: &mut [i32],
        _values_offset: usize,
        _iterations: i32,
    ) {
        unimplemented!("decode_byte_to_int() must be implemented")
    }
}
/// An encoder for packed integers.
pub trait Encoder {
    /// The minimum number of long blocks to encode in a single iteration, when using long encoding.
    fn long_block_count(&self) -> i32 {
        unimplemented!("long_block_count() must be implemented if it need to be used")
    }

    /// The number of values that can be stored in `long_block_count()` long blocks.
    fn long_value_count(&self) -> i32 {
        unimplemented!("long_value_count() must be implemented if it need to be used")
    }

    /// The minimum number of byte blocks to encode in a single iteration, when using byte encoding.
    fn byte_block_count(&self) -> i32 {
        unimplemented!("byte_block_count() must be implemented if it need to be used")
    }

    /// The number of values that can be stored in `byte_block_count()` byte blocks.
    fn byte_value_count(&self) -> i32 {
        unimplemented!("byte_value_count() must be implemented if it need to be used")
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
    fn encode_i64_to_u64(
        &self,
        _values: &[i64],
        _values_offset: usize,
        _blocks: &mut [u64],
        _blocks_offset: usize,
        _iterations: i32,
    ) {
        unimplemented!("encode_long_to_long() must be implemented if it need to be used")
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
    fn encode_i64_to_u8(
        &self,
        _values: &[i64],
        _values_offset: usize,
        _blocks: &mut [u8],
        _blocks_offset: usize,
        _iterations: i32,
    ) {
        unimplemented!("encode_long_to_byte() must be implemented if it need to be used")
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
    fn encode_i32_to_u64(
        &self,
        _values: &[i32],
        _values_offset: usize,
        _blocks: &mut [u64],
        _blocks_offset: usize,
        _iterations: i32,
    ) {
        unimplemented!("encode_int_to_long() must be implemented if it need to be used")
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
    fn encode_i32_to_u8(
        &self,
        _values: &[i32],
        _values_offset: usize,
        _blocks: &mut [u8],
        _blocks_offset: usize,
        _iterations: i32,
    ) {
        unimplemented!("encode_int_to_byte() must be implemented if it need to be used")
    }
}

/// A read-only random access array of positive integers.
pub trait Reader: Accountable {
    /// Get the value at the given index.
    fn get(&mut self, _index: i32) -> Result<i64, LuceneError> {
        unimplemented!("get() must be implemented if it need to be used")
    }

    /// Bulk get: read at least one and at most `len` values starting from `index`
    /// into `arr[off.off+len]` and return the actual number of values that have been read.
    fn get_bulk(
        &mut self,
        index: i32,
        arr: &mut [i64],
        off: i32,
        len: i32,
    ) -> Result<i32, LuceneError> {
        self.default_get_bulk(index, arr, off, len)
    }
    fn default_get_bulk(
        &mut self,
        index: i32,
        arr: &mut [i64],
        off: i32,
        len: i32,
    ) -> Result<i32, LuceneError> {
        debug_assert!(len > 0, "len must be > 0");
        debug_assert!(index < self.size(), "index out of bounds: {}", index);
        debug_assert!(
            (off + len) as usize <= arr.len(),
            "offset + len exceeds array length"
        );

        let gets = min(self.size() - index, len);
        for (i, o) in (index..index + gets).zip(off..off + gets) {
            arr[o as usize] = self.get(i)?;
        }
        Ok(gets)
    }

    /// Returns the number of values in the reader.
    fn size(&self) -> i32 {
        unimplemented!("size() must be implemented if it need to be used")
    }
}
/// Run-once iterator interface to decode previously saved PackedInts.
pub trait ReaderIterator: Display {
    /// Returns the next value.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue decoding the next value.
    fn next(&mut self) -> Result<i64, LuceneError> {
        unimplemented!("next() must be implemented if it need to be used")
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
    fn next_batch(&mut self, count: i32) -> Result<&mut LongsRef, LuceneError>;

    /// Returns the number of bits per value.
    fn get_bits_per_value(&self) -> i32 {
        unimplemented!("get_bits_per_value() must be implemented if it need to be used")
    }

    /// Returns the total number of values.
    fn size(&self) -> i32 {
        unimplemented!("size() must be implemented if it need to be used")
    }

    /// Returns the current position.
    fn ord(&self) -> i32;
}
/// A base implementation of the `ReaderIterator` trait.
pub struct ReaderIteratorImpl<C>
where
    C: ReaderIterator,
{
    bits_per_value: i32,
    value_count: i32,
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
    pub fn new(value_count: i32, bits_per_value: i32, sub_reader: C) -> Self {
        Self {
            bits_per_value,
            value_count,
            sub_reader,
        }
    }
}

impl<C> ReaderIterator for ReaderIteratorImpl<C>
where
    C: ReaderIterator,
{
    fn next(&mut self) -> Result<i64, LuceneError> {
        let next_values = self.next_batch(1)?;
        debug_assert!(next_values.length > 0, "next_values buffer is empty");
        let result = next_values.longs[next_values.offset as usize];
        next_values.offset += 1;
        next_values.length -= 1;
        Ok(result)
    }

    fn next_batch(&mut self, count: i32) -> Result<&mut LongsRef, LuceneError> {
        self.sub_reader.next_batch(count)
    }

    fn get_bits_per_value(&self) -> i32 {
        self.bits_per_value
    }

    fn size(&self) -> i32 {
        self.value_count
    }

    fn ord(&self) -> i32 {
        self.sub_reader.ord()
    }
}
impl<C> Display for ReaderIteratorImpl<C>
where
    C: ReaderIterator,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.sub_reader)
    }
}

pub trait Mutable: Reader + Display {
    /// Returns the number of bits used to store any given value.
    ///
    /// Note: This does not imply that memory usage is `bits_per_value * #values` as implementations
    /// are free to use non-space-optimal packing of bits.
    fn get_bits_per_value(&self) -> i32 {
        unimplemented!("get_bits_per_value() must be implemented if it need to be used")
    }

    /// Sets the value at the given index in the array.
    ///
    /// # Arguments
    ///
    /// * `index` - The position where the value should be set.
    /// * `value` - The value to be stored, which must conform to the constraints of the array.
    ///
    fn set(&mut self, _index: i32, _value: i64) -> Result<(), LuceneError> {
        unimplemented!("set() must be implemented if it need to be used")
    }
    /// Sets a range of values in the array.
    ///
    /// # Arguments
    ///
    /// * `index` - The starting index in the packed array where values will be set.
    /// * `arr` - The source array of values to set.
    /// * `off` - The offset in the source array to start reading values from.
    /// * `len` - The maximum number of values to set.
    ///
    /// # Returns
    ///
    /// The actual number of values that have been set.
    ///
    fn set_bulk(
        &mut self,
        index: i32,
        arr: &[i64],
        off: i32,
        len: i32,
    ) -> Result<i32, LuceneError> {
        self.default_set_bulk(index, arr, off, len)
    }
    fn default_set_bulk(
        &mut self,
        index: i32,
        arr: &[i64],
        off: i32,
        len: i32,
    ) -> Result<i32, LuceneError> {
        debug_assert!(len > 0, "len must be > 0 (got {})", len);
        debug_assert!(
            index >= 0 && index < self.size(),
            "Index out of bounds: {}",
            index
        );

        let len = len.min(self.size() - index);
        debug_assert!(
            (off + len) as usize <= arr.len(),
            "Array offset and length out of bounds"
        );

        for (i, o) in (index..index + len).zip(off..off + len) {
            self.set(i, arr[o as usize])?;
        }
        Ok(len)
    }

    /// Fills a range in the packed array with a specific value.
    ///
    /// # Arguments
    ///
    /// * `from_index` - The start index of the range to fill (inclusive).
    /// * `to_index` - The end index of the range to fill (exclusive).
    /// * `val` - The value to fill with.
    fn fill(&mut self, from_index: i32, to_index: i32, val: i64) -> Result<(), LuceneError> {
        self.default_fill(from_index, to_index, val)
    }
    fn default_fill(
        &mut self,
        from_index: i32,
        to_index: i32,
        val: i64,
    ) -> Result<(), LuceneError> {
        debug_assert!(val <= PackedInts::max_value(self.get_bits_per_value()));
        debug_assert!(
            from_index <= to_index,
            "from_index must be <= to_index: {} > {}",
            from_index,
            to_index
        );
        for i in from_index..to_index {
            self.set(i, val)?;
        }
        Ok(())
    }

    /// Sets all values in the packed array to 0.
    fn clear(&mut self) -> Result<(), LuceneError> {
        self.fill(0, self.size(), 0)
    }
}
pub struct MutableImpl<T>
where
    T: Mutable + Display,
{
    pub sub_reader: T,
}
impl<T> MutableImpl<T>
where
    T: Mutable + Display,
{
    pub fn new(sub_reader: T) -> Self {
        Self { sub_reader }
    }
}

impl<T> Accountable for MutableImpl<T>
where
    T: Display + Mutable,
{
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

impl<T> Reader for MutableImpl<T>
where
    T: Mutable + Display,
{
    fn size(&self) -> i32 {
        self.sub_reader.size()
    }
}

impl<T> Mutable for MutableImpl<T>
where
    T: Mutable + Display,
{
    fn get_bits_per_value(&self) -> i32 {
        self.sub_reader.get_bits_per_value()
    }
}
impl<T> Display for MutableImpl<T>
where
    T: Mutable + Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.sub_reader)
    }
}

pub struct NullReader {
    value_count: i32,
}
impl NullReader {
    pub fn for_count(value_count: i32) -> Self {
        Self { value_count }
    }
    pub fn new(value_count: i32) -> Self {
        Self::for_count(value_count)
    }
}
impl Accountable for NullReader {
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}
impl Reader for NullReader {
    fn get(&mut self, _index: i32) -> Result<i64, LuceneError> {
        Ok(0)
    }

    fn get_bulk(
        &mut self,
        index: i32,
        arr: &mut [i64],
        off: i32,
        mut len: i32,
    ) -> Result<i32, LuceneError> {
        debug_assert!(len > 0, "len must be > 0 (got {})", len);
        debug_assert!(
            index < self.value_count,
            "index out of bounds (index={}, valueCount={})",
            index,
            self.value_count
        );

        len = len.min(self.value_count - index);
        debug_assert!(
            (off + len) as usize <= arr.len(),
            "not enough space in destination array"
        );

        arr[off as usize..(off + len) as usize].fill(0);
        Ok(len)
    }

    fn size(&self) -> i32 {
        self.value_count
    }
}
pub trait Writer {
    /// The format used to serialize values.
    fn get_format(&self) -> &Format;
    ///  Add a value to the stream.
    fn add(&mut self, v: i64) -> Result<(), LuceneError>;
    /// The number of bits per value.
    fn bits_per_values(&self) -> i32;
    /// Perform end-of-stream operations.
    fn finish(&mut self) -> Result<(), LuceneError>;
    /// Returns the current ord in the stream (number of values that have been written so far minus one).
    fn ord(&self) -> i32;
}

#[derive(Debug, Clone)]
pub struct DummyMutable;
impl Reader for DummyMutable {}
impl Accountable for DummyMutable {
    fn ram_bytes_used(&self) -> i64 {
        unreachable!("DummyMutable should not be used")
    }
}
impl Display for DummyMutable {
    fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        unreachable!("DummyMutable should not be displayed")
    }
}
impl Mutable for DummyMutable {}

#[cfg(test)]
mod tests {
    use crate::store::directory::Directory;
    use crate::store::{
        ByteArrayDataInput, DataInput, DataOutput, IndexInput, IndexOutput, IO_CONTEXT_DEFAULT,
    };
    use crate::test::util::lucene_test_case::{at_least, is_night_mode};
    use crate::test::util::lucene_test_case::{new_directory, new_io_context, random, rarely};

    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::LuceneError;
    use crate::util::long_values::LongValues;
    use crate::util::packed::abstract_block_packed_writer::AbstractBlockPackedWriter;
    use crate::util::packed::abstract_paged_mutable::AbstractPagedMutable;
    use crate::util::packed::block_packed_reader_iterator::BlockPackedReaderIterator;
    use crate::util::packed::block_packed_writer::BlockPackedWriter;
    use crate::util::packed::growable_writer::GrowableWriter;
    use crate::util::packed::monotonic_block_packed_reader::MonotonicBlockPackedReader;
    use crate::util::packed::monotonic_block_packed_writer::MonotonicBlockPackedWriter;
    use crate::util::packed::mutable_packed64_enum::MutablePacked64Enum;
    use crate::util::packed::packed64::Packed64;
    use crate::util::packed::packed_long_values::{PackedLongValues, PackedLongValuesBuilder};
    use crate::util::packed::paged_growable_writer::PagedGrowableWriter;
    use crate::util::packed::paged_mutable::PagedMutable;
    use crate::util::packed::Format::{Packed, PackedSingleBlock};
    use crate::util::packed::{
        create, is_supported, Decoder, Encoder, FormatBehavior, Mutable, MutableImpl, NullReader,
        PackedImpl, PackedInts, PackedSingleBlockImpl, Reader, ReaderIterator, Writer,
        MAX_SUPPORTED_BITS_PER_VALUE,
    };
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[allow(dead_code)] // for quick search
    struct TestPackedInts;
    #[test]
    fn test_byte_count() {
        let mut random = random();
        const ITERATIONS: usize = 3;

        for _ in 0..ITERATIONS {
            let value_count = random.gen_range(1..i32::MAX);

            for format in &[
                Packed(PackedImpl::new(0)),
                PackedSingleBlock(PackedSingleBlockImpl::new(1)),
            ] {
                for bpv in 1..=64 {
                    let byte_count =
                        format.byte_count(PackedInts::VERSION_CURRENT, value_count, bpv);
                    let msg = format!(
                        "format={:?}, byteCount={}, valueCount={}, bpv={}",
                        format, byte_count, value_count, bpv
                    );
                    assert!(
                        byte_count * 8 >= (value_count as i64) * (bpv as i64),
                        "{}",
                        msg
                    );
                    if let Packed(_) = format {
                        assert!(
                            (byte_count - 1) * 8 < (value_count as i64) * (bpv as i64),
                            "{}",
                            msg
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn test_bits_required() -> Result<(), LuceneError> {
        assert_eq!(PackedInts::bits_required((2u64.pow(61) - 1) as i64)?, 61);
        assert_eq!(PackedInts::bits_required(0x1FFFFFFFFFFFFFFF)?, 61);
        assert_eq!(PackedInts::bits_required(0x3FFFFFFFFFFFFFFF)?, 62);
        assert_eq!(PackedInts::bits_required(0x7FFFFFFFFFFFFFFF)?, 63);
        assert_eq!(PackedInts::unsigned_bits_required(-1), 64);
        assert_eq!(PackedInts::unsigned_bits_required(i64::MIN), 64);
        assert_eq!(PackedInts::bits_required(0)?, 1);
        Ok(())
    }
    #[test]
    fn test_max_values() {
        assert_eq!(PackedInts::max_value(1), 1, "1 bit -> max == 1");
        assert_eq!(PackedInts::max_value(2), 3, "2 bit -> max == 3");
        assert_eq!(PackedInts::max_value(8), 255, "8 bit -> max == 255");
        assert_eq!(
            PackedInts::max_value(63),
            i64::MAX,
            "63 bit -> max == i64::MAX"
        );
        assert_eq!(
            PackedInts::max_value(64),
            i64::MAX,
            "64 bit -> max == i64::MAX (same as for 63 bit)"
        );
    }
    #[test]
    fn test_packed_ints() -> Result<(), LuceneError> {
        let mut random = random();
        let num = at_least(&mut random, 3);
        let io_context = new_io_context(&mut random)?;
        for _ in 0..num {
            for nbits in 1..=64 {
                let max_value = PackedInts::max_value(nbits);
                let value_count = TestUtil::next_int(&mut random, 1, 600) as usize;
                let buffer_size = if random.gen_bool(0.5) {
                    TestUtil::next_int(&mut random, 0, 48)
                } else {
                    TestUtil::next_int(&mut random, 0, 4096)
                };
                let mut directory = new_directory(&mut random)?;
                let mut values = vec![0i64; value_count];
                let fp: i64;
                {
                    let mut out = directory.create_output("out.bin", &io_context)?;
                    let mem = random.gen_range(0..2 * PackedInts::DEFAULT_BUFFER_SIZE);
                    let start_fp = out.get_file_pointer();
                    let mut writer = PackedInts::get_writer_no_header(
                        &mut out,
                        Packed(PackedImpl::new(0)),
                        value_count as i32,
                        nbits,
                        mem,
                    );

                    let actual_value_count = if random.gen_bool(0.5) {
                        value_count
                    } else {
                        TestUtil::next_int(&mut random, 0, value_count as i32) as usize
                    };

                    values
                        .iter_mut()
                        .take(actual_value_count)
                        .enumerate()
                        .try_for_each(|(_, value)| {
                            let val = if nbits == 64 {
                                random.gen()
                            } else {
                                TestUtil::next_long(&mut random, 0, max_value)
                            };
                            *value = val;
                            writer.add(val)
                        })?;
                    writer.finish()?;

                    // Ensure that finish() added the missing values
                    let bytes = writer.get_format().byte_count(
                        PackedInts::VERSION_CURRENT,
                        value_count as i32,
                        writer.bits_per_value,
                    );
                    fp = out.get_file_pointer();
                    assert_eq!(bytes, fp - start_fp);
                }

                // Test reader iterator `next`
                {
                    let mut input = directory.open_input("out.bin", &io_context)?;
                    {
                        let mut reader = PackedInts::get_reader_iterator_no_header(
                            &mut input,
                            Packed(PackedImpl::new(0)),
                            PackedInts::VERSION_CURRENT,
                            value_count as i32,
                            nbits,
                            buffer_size,
                        )?;
                        for (i, &expected_value) in values.iter().enumerate().take(value_count) {
                            let next_value = reader.next()?;
                            assert_eq!(
                                expected_value, next_value,
                                "index={}, value_count={}, nbits={}, for reader {}",
                                i, value_count, nbits, reader
                            );
                            assert_eq!(i, reader.ord() as usize);
                        }
                    }
                    assert_eq!(fp, input.get_file_pointer());
                }

                // Test reader iterator bulk `next`
                {
                    let mut input = directory.open_input("out.bin", &io_context)?;
                    {
                        let mut reader = PackedInts::get_reader_iterator_no_header(
                            &mut input,
                            Packed(PackedImpl::new(0)),
                            PackedInts::VERSION_CURRENT,
                            value_count as i32,
                            nbits,
                            buffer_size,
                        )?;
                        let mut i = 0;
                        while i < value_count {
                            let count = TestUtil::next_int(&mut random, 1, 95);
                            let next = reader.next_batch(count)?;
                            for k in 0..next.length {
                                assert_eq!(
                                    values[i + k as usize],
                                    next.longs[(next.offset + k) as usize],
                                    "index={}, value_count={}, nbits={}",
                                    i,
                                    value_count,
                                    nbits
                                );
                            }
                            i += next.length as usize;
                        }
                    }
                    assert_eq!(fp, input.get_file_pointer());
                }
            }
        }
        Ok(())
    }
    #[test]
    fn test_end_pointer() -> Result<(), LuceneError> {
        let mut random = random();

        let mut directory = new_directory(&mut random)?;
        let value_count = random.gen_range(1..=1000);
        let io_context = new_io_context(&mut random)?;

        {
            let mut out = directory.create_output("tests.bin", &io_context)?;
            for _ in 0..value_count {
                out.write_long(0)?
            }
        }

        let mut input = directory.open_input("tests.bin", &io_context)?;

        for version in PackedInts::VERSION_START..=PackedInts::VERSION_CURRENT {
            for bpv in 1..=64 {
                for format in &[
                    Packed(PackedImpl::new(0)),
                    PackedSingleBlock(PackedSingleBlockImpl::new(1)),
                ] {
                    if !format.is_supported(bpv) {
                        continue;
                    }

                    let byte_count = format.byte_count(version, value_count as i32, bpv);

                    let msg = format!(
                        "format={:?}, version={}, value_count={}, bpv={}",
                        format, version, value_count, bpv
                    );

                    // 测试迭代器
                    input.seek(0)?;
                    {
                        let mut iterator = PackedInts::get_reader_iterator_no_header(
                            &mut input,
                            *format,
                            version,
                            value_count as i32,
                            bpv,
                            random.gen_range(1..=65536), // 缓冲区大小随机
                        )?;

                        for _ in 0..value_count {
                            iterator.next()?;
                        }
                    }

                    assert_eq!(
                        byte_count,
                        input.get_file_pointer(),
                        "{}: File pointer mismatch",
                        msg
                    );
                }
            }
        }
        Ok(())
    }
    #[test]
    fn test_controlled_equality() -> Result<(), LuceneError> {
        const VALUE_COUNT: i32 = 255;
        const BITS_PER_VALUE: i32 = 8;

        let packed_ints = create_packed_ints(VALUE_COUNT, BITS_PER_VALUE)?;

        for mut packed_int in packed_ints {
            for i in 0..packed_int.size() {
                packed_int.set(i, (i + 1) as i64)?;
            }
        }
        let mut packed_ints = create_packed_ints(VALUE_COUNT, BITS_PER_VALUE)?;

        assert_list_equality(&mut packed_ints)?;

        Ok(())
    }
    #[test]
    fn test_random_bulk_copy() -> Result<(), LuceneError> {
        let mut random = random();
        let num_iters = at_least(&mut random, 3);

        for j in 0..num_iters {
            let value_count = if is_night_mode() {
                at_least(&mut random, 100000)
            } else {
                at_least(&mut random, 10000)
            };

            let mut bits1 = TestUtil::next_int(&mut random, 1, 64);
            let mut bits2 = TestUtil::next_int(&mut random, 1, 64);

            if bits1 > bits2 {
                std::mem::swap(&mut bits1, &mut bits2);
            }

            let mut packed1 =
                PackedInts::get_mutable(value_count as i32, bits1, PackedInts::COMPACT)?;
            let mut packed2 =
                PackedInts::get_mutable(value_count as i32, bits2, PackedInts::COMPACT)?;

            let max_value = PackedInts::max_value(bits1);
            for i in 0..value_count {
                let val = TestUtil::next_long(&mut random, 0, max_value);
                packed1.set(i, val)?;
                packed2.set(i, val)?;
            }

            let mut buffer = vec![0i64; value_count as usize];

            // Copy random slices over 20 times:
            for _ in 0..20 {
                let start = random.gen_range(0..value_count - 1);
                let len = TestUtil::next_int(&mut random, 1, value_count - start);
                let offset = if len == value_count {
                    0
                } else {
                    random.gen_range(0..(value_count - len))
                };

                if random.gen_bool(0.5) {
                    let got = packed1.get_bulk(start, &mut buffer, offset, len)?;
                    assert!(got as i32 <= len);
                    let sot = packed2.set_bulk(start, &buffer, offset, got as i32)?;
                    assert!(sot <= got);
                } else {
                    PackedInts::copy(
                        &mut packed1,
                        offset,
                        &mut packed2,
                        offset,
                        len,
                        random.gen_range(1..=(10 * len)) as i32,
                    )?;
                }
            }

            for i in 0..value_count {
                assert_eq!(
                    packed1.get(i)?,
                    packed2.get(i)?,
                    "Values at index {} differ, iter:{}",
                    i,
                    j
                );
            }
        }
        Ok(())
    }
    #[test]
    fn test_random_equality() -> Result<(), LuceneError> {
        let mut random = random();
        let num_iters = if is_night_mode() {
            at_least(&mut random, 10)
        } else {
            1
        };
        for _ in 0..num_iters {
            let value_count = TestUtil::next_int(&mut random, 1, 300);
            for bits_per_value in 1..=64 {
                assert_random_equality(value_count, bits_per_value, random.gen::<u64>())?;
            }
        }
        Ok(())
    }
    fn assert_random_equality(
        value_count: i32,
        bits_per_value: i32,
        random: u64,
    ) -> Result<(), LuceneError> {
        let mut packed_ints = create_packed_ints(value_count, bits_per_value)?;

        for packed_int in &mut packed_ints {
            fill(packed_int, bits_per_value, random)?;
        }

        assert_list_equality(&mut packed_ints)?;

        Ok(())
    }

    fn create_packed_ints(
        value_count: i32,
        bits_per_value: i32,
    ) -> Result<Vec<MutablePacked64Enum>, LuceneError> {
        let mut packed_ints: Vec<MutablePacked64Enum> = Vec::new();
        let packed64 = Packed64::new(value_count, bits_per_value);
        let mutable_impl = MutableImpl::new(packed64);
        packed_ints.push(MutablePacked64Enum::P64(mutable_impl));

        for bpv in bits_per_value..=MAX_SUPPORTED_BITS_PER_VALUE {
            if is_supported(bpv) {
                packed_ints.push(create(value_count, bpv)?);
            }
        }

        Ok(packed_ints)
    }

    fn fill(
        packed_int: &mut MutablePacked64Enum,
        bits_per_value: i32,
        seed: u64,
    ) -> Result<(), LuceneError> {
        let max_value = if bits_per_value == 64 {
            i64::MAX
        } else {
            (1 << bits_per_value) - 1
        };
        let mut random = StdRng::seed_from_u64(seed);
        for i in 0..packed_int.size() {
            let value: i64 = if bits_per_value == 64 {
                random.gen()
            } else {
                TestUtil::next_long(&mut random, 0, max_value)
            };

            packed_int.set(i, value)?;
            let retrieved_value = packed_int.get(i)?;

            if value != retrieved_value {
                assert_eq!(
                    value, retrieved_value,
                    "The set/get of the value at index {} should match for {}",
                    i, packed_int
                );
            }
        }

        Ok(())
    }

    fn assert_list_equality(packed_ints: &mut [MutablePacked64Enum]) -> Result<(), LuceneError> {
        assert_list_equality_impl("", packed_ints)
    }
    fn assert_list_equality_impl(
        message: &str,
        packed_ints: &mut [MutablePacked64Enum],
    ) -> Result<(), LuceneError> {
        if packed_ints.is_empty() {
            return Ok(());
        }
        let length: usize;
        let value_count: i32;
        {
            length = packed_ints.len();
            let base = &mut packed_ints[0];
            value_count = base.size();
            for packed_int in packed_ints.iter() {
                assert_eq!(
                    value_count,
                    packed_int.size(),
                    "{}. The number of values should be the same",
                    message
                );
            }
        }

        for i in 0..value_count {
            for j in 1..length {
                assert_eq!(
                    packed_ints[0].get(i)?,
                    packed_ints[j].get(i)?,
                    "{}. The value at index {} should be the same ",
                    message,
                    i
                );
            }
        }

        Ok(())
    }
    #[test]
    fn test_secondary_block_change() -> Result<(), LuceneError> {
        let mut mutable = MutablePacked64Enum::P64(MutableImpl::new(Packed64::new(26, 5)));
        mutable.set(24, 31)?;
        assert_eq!(mutable.get(24)?, 31, "The value #24 should be correct");
        mutable.set(4, 16)?;
        assert_eq!(
            mutable.get(24)?,
            31,
            "The value #24 should remain unchanged"
        );

        Ok(())
    }
    #[test]
    fn test_int_overflow() -> Result<(), LuceneError> {
        Ok(())
    }
    #[test]
    fn test_fill() -> Result<(), LuceneError> {
        let mut random = random();
        let value_count = 1111;
        let from = random.gen_range(0..value_count + 1);
        let to = from + random.gen_range(0..value_count + 1 - from);

        for bpv in 1..=64 {
            let val = TestUtil::next_long(&mut random, 0, PackedInts::max_value(bpv));
            let mut packed_ints = create_packed_ints(value_count, bpv)?;

            for packed in &mut packed_ints {
                let msg = format!(
                    "{} bpv={}, from={}, to={}, val={}",
                    packed, bpv, from, to, val
                );

                packed.fill(0, packed.size(), 1)?;
                packed.fill(from, to, val)?;

                for i in 0..packed.size() {
                    let expected_value: i64 = if i >= from && i < to { val } else { 1 };

                    assert_eq!(packed.get(i)?, expected_value, "{}: i={}", msg, i);
                }
            }
        }

        Ok(())
    }
    #[test]
    fn test_packed_ints_null() -> Result<(), LuceneError> {
        let mut random = random();
        // must be > 10 for the bulk reads below
        let size = TestUtil::next_int(&mut random, 11, 256);
        let mut packed_ints = NullReader::for_count(size);
        let random_index = TestUtil::next_int(&mut random, 0, size - 1);
        assert_eq!(
            packed_ints.get(random_index)?,
            0,
            "The value at random index {} should be 0",
            random_index
        );
        let mut arr = vec![1i64; (size + 10) as usize];
        let r = packed_ints.get_bulk(0, &mut arr, 0, size - 1)?;
        assert_eq!(
            r,
            (size - 1),
            "The number of values read should match size - 1"
        );
        for (i, &value) in arr.iter().take(r as usize).enumerate() {
            assert_eq!(value, 0, "The value at position {} should be 0", i);
        }
        arr.fill(1);
        let r = packed_ints.get_bulk(10, &mut arr, 0, size + 10)?;
        assert_eq!(
            r,
            (size - 10),
            "The number of values read should match size - 10"
        );
        for i in 0..(size - 10) {
            assert_eq!(
                arr[i as usize], 0,
                "The value at position {} should be 0",
                i
            );
        }

        Ok(())
    }
    #[test]
    fn test_bulk_get() -> Result<(), LuceneError> {
        let mut random = random();
        let value_count = 1111;
        let index = random.gen_range(0..value_count) as usize;
        let len = TestUtil::next_int(&mut random, 1, value_count * 2) as usize;
        let off = random.gen_range(0..77);

        for bpv in 1..=64 {
            let mask = PackedInts::max_value(bpv);
            let mut packed_ints = create_packed_ints(value_count, bpv)?;
            for ints in &mut packed_ints {
                for i in 0..ints.size() {
                    ints.set(i, (31 * i as i64 - 1099) & mask)?;
                }
                let mut arr = vec![0i64; off + len];
                let msg = format!(
                    "{} valueCount={}, index={}, len={}, off={}",
                    ints, value_count, index, len, off
                );
                let gets = ints.get_bulk(index as i32, &mut arr, off as i32, len as i32)?;
                assert!(gets > 0, "{}: gets should be greater than 0", msg);
                assert!(
                    gets <= len as i32,
                    "{}: gets should be less than or equal to len",
                    msg
                );
                assert!(
                    gets <= (ints.size() - index as i32),
                    "{}: gets should be less than or equal to remaining values",
                    msg
                );
                for (i, &item) in arr.iter().enumerate() {
                    let m = format!("{}, i={}", msg, i);
                    if i >= off && i < off + gets as usize {
                        assert_eq!(
                            ints.get((i - off + index) as i32)?,
                            item,
                            "{}: value mismatch at index {}",
                            m,
                            i
                        );
                    } else {
                        assert_eq!(item, 0, "{}: array values outside range should be 0", m);
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn test_bulk_set() -> Result<(), LuceneError> {
        let mut random = random();
        let value_count = 1111;
        let index = random.gen_range(0..value_count);
        let len = random.gen_range(1..=(value_count * 2));
        let off = random.gen_range(0..77);

        for bpv in 1..=64 {
            let mask = PackedInts::max_value(bpv);
            let mut packed_ints = create_packed_ints(value_count as i32, bpv)?;
            let mut arr = vec![0i64; off + len];
            let length = arr.len();
            for (i, item) in arr.iter_mut().enumerate().take(length) {
                *item = (31 * i as i64 + 19) & mask;
            }
            for ints in &mut packed_ints {
                let msg = format!(
                    "{} valueCount={}, index={}, len={}, off={}",
                    ints, value_count, index, len, off
                );
                let sets = ints.set_bulk(index as i32, &arr, off as i32, len as i32)?;
                assert!(sets > 0, "{}: gets should be greater than 0", msg);
                assert!(
                    sets <= len as i32,
                    "{}: gets should be less than or equal to len",
                    msg
                );
                for i in 0..ints.size() {
                    let m = format!("{}, i={}", msg, i);
                    if i as usize >= index && (i as usize) < index + sets as usize {
                        assert_eq!(
                            ints.get(i)?,
                            arr[off - index + i as usize],
                            "{}: value mismatch at index {}",
                            m,
                            i
                        );
                    } else {
                        assert_eq!(
                            ints.get(i)?,
                            0,
                            "{}: array values outside range should be 0",
                            m
                        );
                    }
                }
            }
        }
        Ok(())
    }
    #[test]
    fn test_copy() -> Result<(), LuceneError> {
        let mut random = random();
        let value_count = TestUtil::next_int(&mut random, 5, 600);
        let off1 = random.gen_range(0..value_count);
        let off2 = random.gen_range(0..value_count);
        let len = random.gen_range(0..(value_count - off1).min(value_count - off2));
        let mem = random.gen_range(0..1024);
        for bpv in 1..=64 {
            let mask = PackedInts::max_value(bpv);
            for mut r1 in create_packed_ints(value_count as i32, bpv)? {
                for i in 0..r1.size() {
                    r1.set(i, (31 * i as i64 - 1023) & mask)?;
                }
                for mut r2 in create_packed_ints(value_count as i32, bpv)? {
                    let msg = format!(
                        "src={}, dest={}, srcPos={}, destPos={}, len={}, mem={}",
                        r1, r2, off1, off2, len, mem
                    );
                    PackedInts::copy(&mut r1, off1, &mut r2, off2, len, mem)?;
                    for i in 0..r2.size() {
                        let m = format!("{}, i={}", msg, i);
                        if i >= off2 && i < off2 + len {
                            assert_eq!(
                                r1.get(i - off2 + off1)?,
                                r2.get(i)?,
                                "{}: Values mismatch at index {}",
                                m,
                                i
                            );
                        } else {
                            assert_eq!(
                                r2.get(i)?,
                                0,
                                "{}: Unexpected non-zero value at index {}",
                                m,
                                i
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn test_growable_writer() -> Result<(), LuceneError> {
        let mut random = random();
        let value_count = 113 + random.gen_range(0..1112);

        let mut wrt = GrowableWriter::new(1, value_count as i32, PackedInts::DEFAULT)?;

        wrt.set(4, 2)?;
        wrt.set(7, 10)?;
        wrt.set(value_count - 10, 99)?;
        wrt.set(99, 999)?;
        wrt.set(value_count - 1, 1 << 10)?;
        assert_eq!(wrt.get(value_count - 1)?, 1 << 10);

        wrt.set(99, (1 << 23) - 1)?;
        assert_eq!(wrt.get(value_count - 1)?, 1 << 10);

        wrt.set(1, i64::MAX)?;
        wrt.set(2, -3)?;
        assert_eq!(wrt.get_bits_per_value(), 64);
        assert_eq!(wrt.get(value_count - 1)?, 1 << 10);
        assert_eq!(wrt.get(1)?, i64::MAX);
        assert_eq!(wrt.get(2)?, -3);
        assert_eq!(wrt.get(4)?, 2);
        assert_eq!(wrt.get(99)?, (1 << 23) - 1);
        assert_eq!(wrt.get(7)?, 10);
        assert_eq!(wrt.get(value_count - 10)?, 99);
        assert_eq!(wrt.get(value_count - 1)?, 1 << 10);

        // TODO:
        // Check memory usage
        // let ram_used = wrt.ram_bytes_used();
        // assert_eq!(ram_used, ram_usage(&wrt));

        Ok(())
    }
    #[test]
    fn test_paged_growable_writer() -> Result<(), LuceneError> {
        let mut random = random();

        let page_size = 1 << TestUtil::next_int(&mut random, 6, 30);
        let acceptable_overhead_ratio = random.gen::<f32>();
        let initial_bit_width = TestUtil::next_int(&mut random, 1, 64);

        let sub_reader =
            PagedGrowableWriter::with_fill_page(initial_bit_width, acceptable_overhead_ratio);

        let mut writer = AbstractPagedMutable::new(initial_bit_width, 0, page_size, sub_reader)?;
        assert_eq!(writer.size(), 0);

        let mut buf =
            PackedLongValues::delta_packed_long_values_builder_default(random.gen::<f32>())?;
        let size = if is_night_mode() {
            random.gen_range(0..1_000_000)
        } else {
            random.gen_range(0..100_000)
        };

        let mut max = 5;
        for _ in 0..size {
            buf.add(TestUtil::next_long(&mut random, 0, max))?;
            if rarely(&mut random) {
                max = PackedInts::max_value(if rarely(&mut random) {
                    TestUtil::next_int(&mut random, 0, 63)
                } else {
                    TestUtil::next_int(&mut random, 0, 31)
                });
            }
        }
        let bits_per_value = random.gen_range(1..=64);
        writer = AbstractPagedMutable::new(
            bits_per_value,
            size,
            page_size,
            PagedGrowableWriter::with_fill_page(bits_per_value, random.gen::<f32>()),
        )?;
        assert_eq!(writer.size(), size);

        let mut values = buf.build()?;
        for i in (0..size).rev() {
            writer.set(i, values.get(i)?)?;
        }
        for i in 0..size {
            assert_eq!(values.get(i)?, writer.get(i)?);
        }

        // TODO
        // assert!(
        //     (RamUsageTester::ram_used(&writer) as f64 - writer.ram_bytes_used() as f64).abs() < 8.0
        // );

        let new_size = TestUtil::next_long(&mut random, writer.size() / 2, writer.size() * 3 / 2);
        let mut copy = writer.resize(new_size)?;
        for i in 0..copy.size() {
            if i < writer.size() {
                assert_eq!(writer.get(i)?, copy.get(i)?);
            } else {
                assert_eq!(copy.get(i)?, 0);
            }
        }

        let grow_size = TestUtil::next_long(&mut random, writer.size() / 2, writer.size() * 3 / 2);
        let grow = writer.grow_with_size(grow_size)?;
        let grow_len;
        if grow.is_some() {
            let mut new_writer = grow.unwrap();
            grow_len = new_writer.size();
            for i in 0..grow_len {
                if i < writer.size() {
                    assert_eq!(new_writer.get(i)?, writer.get(i)?);
                } else {
                    assert_eq!(new_writer.get(i)?, 0);
                }
            }
        }

        Ok(())
    }
    #[test]
    fn test_paged_mutable() -> Result<(), LuceneError> {
        let mut random = random();
        let bits_per_value = TestUtil::next_int(&mut random, 1, 64);
        let max = PackedInts::max_value(bits_per_value);
        let page_size = 1 << TestUtil::next_int(&mut random, 6, 30);
        let acceptable_overhead_ratio = random.gen::<f32>() / 2.0;

        let mut sub_mutable =
            PagedMutable::with_overhead_ratio(page_size, bits_per_value, acceptable_overhead_ratio);
        let mut writer = AbstractPagedMutable::new(bits_per_value, 0, page_size, sub_mutable)?;
        assert_eq!(writer.size(), 0);

        let mut buf =
            PackedLongValues::delta_packed_long_values_builder_default(random.gen::<f32>())?;
        let size = if is_night_mode() {
            random.gen_range(0..1_000_000)
        } else {
            random.gen_range(0..100_000)
        };

        for _ in 0..size {
            let value = if bits_per_value == 64 {
                random.gen_range(i64::MIN..=i64::MAX)
            } else {
                TestUtil::next_long(&mut random, 0, max)
            };
            buf.add(value)?;
        }

        let acceptable_overhead_ratio = random.gen::<f32>();
        sub_mutable =
            PagedMutable::with_overhead_ratio(page_size, bits_per_value, acceptable_overhead_ratio);
        writer = AbstractPagedMutable::new(bits_per_value, size, page_size, sub_mutable)?;

        assert_eq!(writer.size(), size);

        let mut values = buf.build()?;
        for i in (0..size).rev() {
            writer.set(i, values.get(i)?)?;
        }
        for i in 0..size {
            assert_eq!(values.get(i)?, writer.get(i)?);
        }

        // TODO
        // assert!(
        //     (RamUsageTester::ram_used(&writer) as f64 - RamUsageTester::ram_used(&writer.format) as f64 - writer.ram_bytes_used() as f64).abs() < 8.0
        // );

        let new_size = TestUtil::next_long(&mut random, writer.size() / 2, writer.size() * 3 / 2);
        let mut copy = writer.resize(new_size)?;
        for i in 0..copy.size() {
            if i < writer.size() {
                assert_eq!(writer.get(i)?, copy.get(i)?);
            } else {
                assert_eq!(copy.get(i)?, 0);
            }
        }

        let grow_size = TestUtil::next_long(&mut random, writer.size() / 2, writer.size() * 3 / 2);
        let grow_wrapper = writer.grow_with_size(grow_size)?;
        let grow_len;
        if let Some(g) = grow_wrapper {
            let mut grow = g;
            grow_len = grow.size();
            for i in 0..grow_len {
                if i < writer.size() {
                    assert_eq!(grow.get(i)?, writer.get(i)?);
                } else {
                    assert_eq!(grow.get(i)?, 0);
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_encode_decode() -> Result<(), LuceneError> {
        let mut random = random();

        for format in &[
            Packed(PackedImpl::new(0)),
            PackedSingleBlock(PackedSingleBlockImpl::new(1)),
        ] {
            for bpv in 1..=64 {
                if !format.is_supported(bpv) {
                    continue;
                }

                // let msg = format!("{} {}", format, bpv);
                let msg = format!("{}", bpv);

                let encoder = PackedInts::get_encoder(*format, PackedInts::VERSION_CURRENT, bpv)?;
                let decoder = PackedInts::get_decoder(*format, PackedInts::VERSION_CURRENT, bpv)?;

                let long_block_count = Encoder::long_block_count(encoder);
                let long_value_count = Encoder::long_value_count(encoder);
                let byte_block_count = Encoder::byte_block_count(encoder);
                let byte_value_count = Encoder::byte_value_count(encoder);

                assert_eq!(long_block_count, Encoder::long_block_count(decoder));
                assert_eq!(long_value_count, Encoder::long_value_count(decoder));
                assert_eq!(byte_block_count, Encoder::byte_block_count(decoder));
                assert_eq!(byte_value_count, Encoder::byte_value_count(decoder));

                // let long_iterations = random.gen_range(0..100);
                let long_iterations = 3;
                let byte_iterations = long_iterations * long_value_count / byte_value_count;
                assert_eq!(
                    long_iterations * long_value_count,
                    byte_iterations * byte_value_count
                );

                let blocks_offset = random.gen_range(0..100) as usize;
                let values_offset = random.gen_range(0..100) as usize;
                let blocks_offset2 = random.gen_range(0..100) as usize;
                let blocks_len = (long_iterations * long_block_count) as usize;

                // 1. generate random inputs
                let mut blocks: Vec<u64> = vec![0; blocks_offset + blocks_len];
                for block in blocks.iter_mut() {
                    *block = random.gen::<u64>();

                    if matches!(format, PackedSingleBlock(_)) && 64 % bpv != 0 {
                        let to_clear = 64 % bpv;
                        *block = (*block << to_clear) >> to_clear;
                    }
                }

                // 2. decode
                let mut values =
                    vec![0i64; values_offset + (long_iterations * long_value_count) as usize];
                decoder.decode_u64_to_i64(
                    &blocks,
                    blocks_offset,
                    &mut values,
                    values_offset,
                    long_iterations,
                );
                for &value in &values {
                    assert!(
                        value <= PackedInts::max_value(bpv),
                        "{}: value exceeds maxValue for bpv={}",
                        msg,
                        bpv
                    );
                }
                let mut int_values = vec![0i32; values.len()];
                if bpv <= 32 {
                    decoder.decode_u64_to_i32(
                        &blocks,
                        blocks_offset,
                        &mut int_values,
                        values_offset,
                        long_iterations,
                    );
                    assert!(equals(&int_values, &values), "{}", msg);
                }

                // 3. re-encode
                let mut blocks2 = vec![0u64; blocks_offset2 + blocks_len];
                encoder.encode_i64_to_u64(
                    &values,
                    values_offset,
                    &mut blocks2,
                    blocks_offset2,
                    long_iterations,
                );
                assert_eq!(
                    &blocks[blocks_offset..],
                    &blocks2[blocks_offset2..],
                    "{}: Blocks mismatch after encoding",
                    msg
                );
                if bpv <= 32 {
                    let mut blocks3 = vec![0u64; blocks2.len()];
                    encoder.encode_i32_to_u64(
                        &int_values,
                        values_offset,
                        &mut blocks3,
                        blocks_offset2,
                        long_iterations,
                    );
                    assert_eq!(blocks2, blocks3, "{}", msg);
                }

                // 4. byte[] decoding
                let mut byte_blocks = vec![0u8; 8 * blocks.len()];
                let mut values2 =
                    vec![0i64; values_offset + (long_iterations * long_value_count) as usize];
                byte_blocks
                    .chunks_exact_mut(8)
                    .zip(blocks.iter())
                    .for_each(|(chunk, &block)| chunk.copy_from_slice(&block.to_be_bytes()));

                decoder.decode_u8_to_i64(
                    &byte_blocks,
                    blocks_offset * 8,
                    &mut values2,
                    values_offset,
                    byte_iterations,
                );
                for &value in &values2 {
                    assert!(
                        value <= PackedInts::max_value(bpv),
                        "{}: Byte-decoded value exceeds maxValue for bpv={}",
                        msg,
                        bpv
                    );
                }
                assert_eq!(values, values2, "{}", msg);
                if bpv <= 32 {
                    let mut int_values2 = vec![0i32; values2.len()];
                    decoder.decode_u8_to_i32(
                        &byte_blocks,
                        blocks_offset * 8,
                        &mut int_values2,
                        values_offset,
                        byte_iterations,
                    );
                    assert!(equals(&int_values2, &values2), "{}", msg);
                }
                // 5. byte[] encoding
                let mut blocks3 = vec![0u8; 8 * (blocks_offset2 + blocks_len)];
                encoder.encode_i64_to_u8(
                    &values,
                    values_offset,
                    &mut blocks3,
                    8 * blocks_offset2,
                    byte_iterations,
                );
                assert_eq!(
                    blocks2,
                    blocks3
                        .chunks_exact(8)
                        .map(|chunk| u64::from_be_bytes(chunk.try_into().unwrap()))
                        .collect::<Vec<_>>(),
                    "{}: Byte-encoded blocks mismatch original blocks",
                    msg
                );
                if bpv <= 32 {
                    let mut blocks4 = vec![0u8; blocks3.len()];
                    encoder.encode_i32_to_u8(
                        &int_values,
                        values_offset,
                        &mut blocks4,
                        8 * blocks_offset2,
                        byte_iterations,
                    );
                    assert_eq!(blocks3, blocks4, "{}", msg);
                }
            }
        }

        Ok(())
    }
    fn equals(ints: &[i32], longs: &[i64]) -> bool {
        if ints.len() != longs.len() {
            return false;
        }
        for i in 0..ints.len() {
            if (ints[i] as i64 & 0xFFFFFFFF) != longs[i] {
                return false;
            }
        }
        true
    }
    #[test]
    fn test_packed_long_values_on_zeros() {
        // TOOD
    }
    #[allow(dead_code)]
    enum DataType {
        Packed,
        DeltaPacked,
        Monotonic,
    }
    #[test]
    fn test_packed_long_values() -> Result<(), LuceneError> {
        let mut random = random();

        let arr_size = if is_night_mode() {
            random.gen_range(1..=1_000_000)
        } else {
            random.gen_range(1..=10_000)
        };
        let mut arr = vec![0i64; arr_size];

        let ratio_options = [PackedInts::DEFAULT, PackedInts::COMPACT, PackedInts::FAST];

        for bpv in [0, 1, 63, 64, random.gen_range(2..=62)].iter() {
            for data_type in [DataType::DeltaPacked, DataType::Monotonic, DataType::Packed].iter() {
                // for data_type in [DataType::Packed].iter() {
                let page_size = 1 << TestUtil::next_int(&mut random, 6, 20);
                let acceptable_overhead_ratio = ratio_options
                    [TestUtil::next_int(&mut random, 0, ratio_options.len() as i32 - 1) as usize];

                let mut buf: PackedLongValuesBuilder;
                let inc: i64;

                match data_type {
                    DataType::Packed => {
                        buf = PackedLongValues::packed_long_values_builder(
                            page_size,
                            acceptable_overhead_ratio,
                        )?;
                        inc = 0;
                    }
                    DataType::DeltaPacked => {
                        buf = PackedLongValues::delta_packed_long_values_builder(
                            page_size,
                            acceptable_overhead_ratio,
                        )?;
                        inc = 0;
                    }
                    DataType::Monotonic => {
                        buf = PackedLongValues::monotonic_long_values_builder(
                            page_size,
                            acceptable_overhead_ratio,
                        )?;
                        inc = TestUtil::next_int(&mut random, -1000, 1000) as i64;
                    }
                }

                if *bpv == 0 {
                    arr[0] = random.gen::<i64>();
                    for i in 1..arr.len() {
                        arr[i] = arr[i - 1] + inc;
                    }
                } else if *bpv == 64 {
                    arr.iter_mut().for_each(|item| {
                        *item = random.gen::<i64>();
                    });
                } else {
                    let min_value = TestUtil::next_long(
                        &mut random,
                        i64::MIN,
                        i64::MAX - PackedInts::max_value(*bpv),
                    );
                    arr.iter_mut().enumerate().for_each(|(i, item)| {
                        *item = min_value
                            + inc * i as i64
                            + (random.gen::<i64>() & PackedInts::max_value(*bpv));
                    });
                }

                for &value in &arr {
                    buf.add(value)?;
                    if rarely(&mut random) && !is_night_mode() {
                        // TODO
                        // let expected_bytes_used = ram_usage_tester_ram_used(&buf)?;
                        // let computed_bytes_used = buf.ram_bytes_used();
                        // assert_eq!(expected_bytes_used, computed_bytes_used);
                    }
                }

                assert_eq!(arr.len(), buf.size() as usize);

                let mut values = buf.build()?;
                assert_eq!(arr.len(), values.size() as usize);

                for (i, &value) in arr.iter().enumerate() {
                    assert_eq!(value, values.get(i as i64)?);
                }

                let mut it = values.iterator()?;
                for &value in arr.iter() {
                    if random.gen_bool(0.5) {
                        assert!(it.has_next());
                    }
                    assert_eq!(value, it.next_value()?);
                }
                assert!(!it.has_next());

                // TODO
                // let expected_bytes_used = ram_usage_tester_ram_used(&values)?;
                // let computed_bytes_used = values.ram_bytes_used();
                // assert_eq!(expected_bytes_used, computed_bytes_used);
            }
        }

        Ok(())
    }
    #[test]
    fn test_packed_input_output() {
        // PackedDataOutput is only used for tests, so we don't need to test it
    }
    #[test]
    fn test_block_packed_reader_writer() -> Result<(), LuceneError> {
        let mut random = random();
        let iters = at_least(&mut random, 2);
        for _ in 0..iters {
            let block_size = 1 << TestUtil::next_int(&mut random, 6, 18);
            let value_count: i32 = if is_night_mode() {
                random.gen_range(0..(1 << 18))
            } else {
                random.gen_range(0..(1 << 15))
            };

            let mut values = vec![0i64; value_count as usize];
            let mut min_value = 0;
            let mut bpv = 0;

            for i in 0..value_count {
                if i % block_size == 0 {
                    min_value = if rarely(&mut random) {
                        random.gen_range(0..256) as i64
                    } else if rarely(&mut random) {
                        -5
                    } else {
                        random.gen()
                    };
                    bpv = random.gen_range(0..=64);
                }
                values[i as usize] = if bpv == 0 {
                    min_value
                } else if bpv == 64 {
                    random.gen()
                } else {
                    min_value + TestUtil::next_int(&mut random, 0, (1 << bpv) - 1) as i64
                };
            }
            let fp;
            let mut dir = new_directory(&mut random)?;
            {
                let mut out = dir.create_output("out.bin", &IO_CONTEXT_DEFAULT)?;
                let mut writer =
                    AbstractBlockPackedWriter::new(block_size as i32, BlockPackedWriter, &mut out)?;
                for (i, &value) in values.iter().enumerate() {
                    assert_eq!(i, writer.ord() as usize);
                    writer.add(value)?;
                }
                assert_eq!(value_count as i64, writer.ord());
                writer.finish()?;
                assert_eq!(value_count as i64, writer.ord());
                fp = out.get_file_pointer();
            }

            let mut buf = vec![0u8; fp as usize];
            // test in1
            {
                let mut in1 = dir.open_input("out.bin", &IO_CONTEXT_DEFAULT)?;
                DataInput::read_bytes(&mut in1, &mut buf, 0, fp as i32)?;
                in1.seek(0)?;
                let mut in_ref = in1;
                let mut it = BlockPackedReaderIterator::new(
                    &mut in_ref,
                    PackedInts::VERSION_CURRENT,
                    block_size,
                    value_count as i64,
                )?;

                let mut i: i32 = 0;
                while i < value_count {
                    if random.gen_bool(0.5) {
                        assert_eq!(values[i as usize], it.next_value()?);
                        i += 1;
                    } else {
                        let next_values =
                            it.next_batch(TestUtil::next_int(&mut random, 1, 1024))?;
                        for j in 0..next_values.length as usize {
                            assert_eq!(
                                values[i as usize + j],
                                next_values.longs[j + next_values.offset as usize]
                            );
                        }
                        i += next_values.length as i32;
                    }
                    assert_eq!(i as i64, it.ord());
                }
                let result = it.next_value();
                assert!(matches!(result, Err(LuceneError::Eof(_))));
                assert_eq!(fp, in_ref.get_file_pointer());
                in_ref.seek(0)?;
                let mut it2 = BlockPackedReaderIterator::new(
                    &mut in_ref,
                    PackedInts::VERSION_CURRENT,
                    block_size,
                    value_count as i64,
                )?;
                i = 0;
                loop {
                    let skip = TestUtil::next_int(&mut random, 0, value_count - i);
                    it2.skip(skip as i64)?;
                    i += skip;
                    assert_eq!(i as i64, it2.ord());
                    if i == value_count {
                        break;
                    } else {
                        assert_eq!(values[i as usize], it2.next_value()?);
                        i += 1;
                    }
                }
                assert!(it2.skip(1).is_err());
                assert_eq!(fp, in_ref.get_file_pointer());
            }
            // test in2
            {
                let in2 = ByteArrayDataInput::with_bytes(buf);
                let mut in_ref = in2;
                let mut it = BlockPackedReaderIterator::new(
                    &mut in_ref,
                    PackedInts::VERSION_CURRENT,
                    block_size,
                    value_count as i64,
                )?;

                let mut i = 0;
                while i < value_count {
                    if random.gen_bool(0.5) {
                        assert_eq!(values[i as usize], it.next_value()?);
                        i += 1;
                    } else {
                        let next_values = it.next_batch(random.gen_range(1..=1024))?;
                        for j in 0..next_values.length {
                            assert_eq!(
                                values[i as usize + j as usize],
                                next_values.longs[(j + next_values.offset) as usize]
                            );
                        }
                        i += next_values.length as i32;
                    }
                    assert_eq!(i as i64, it.ord());
                }
                let result = it.next_value();
                assert!(matches!(result, Err(LuceneError::Eof(_))));
                assert_eq!(fp, in_ref.get_position() as i64);

                in_ref.set_position(0);
                let mut it2 = BlockPackedReaderIterator::new(
                    &mut in_ref,
                    PackedInts::VERSION_CURRENT,
                    block_size,
                    value_count as i64,
                )?;
                i = 0;
                loop {
                    let skip = random.gen_range(0..=value_count - i);
                    it2.skip(skip as i64)?;
                    i += skip;
                    assert_eq!(i as i64, it2.ord());
                    if i == value_count {
                        break;
                    } else {
                        assert_eq!(values[i as usize], it2.next_value()?);
                        i += 1;
                    }
                }
                assert!(it2.skip(1).is_err());
                assert_eq!(fp, in_ref.get_position() as i64);
            }
        }
        Ok(())
    }
    #[test]
    fn test_monotonic_block_packed_reader_writer() -> Result<(), LuceneError> {
        let mut random = random();
        let iters = at_least(&mut random, 2);
        for _ in 0..iters {
            let block_size = 1 << TestUtil::next_int(&mut random, 6, 18);
            let value_count = random.gen_range(0..(1 << 18));
            let mut values = vec![0i64; value_count];

            if value_count > 0 {
                values[0] = if random.gen_bool(0.5) {
                    random.gen_range(0..10) as i64
                } else {
                    random.gen_range(0..i32::MAX) as i64
                };

                let mut max_delta = random.gen_range(0..64);
                for i in 1..value_count {
                    if random.gen_bool(0.1) {
                        max_delta = random.gen_range(0..64);
                    }
                    values[i] = std::cmp::max(
                        0,
                        values[i - 1] + TestUtil::next_int(&mut random, -16, max_delta) as i64,
                    );
                }
            }
            let mut dir = new_directory(&mut random)?;
            let file_pointer;
            {
                let mut out = dir.create_output("out.bin", &IO_CONTEXT_DEFAULT)?;
                let mut writer = AbstractBlockPackedWriter::new(
                    block_size as i32,
                    MonotonicBlockPackedWriter,
                    &mut out,
                )?;
                for (i, &value) in values.iter().enumerate().take(value_count) {
                    assert_eq!(i as i64, writer.ord());
                    writer.add(value)?;
                }
                assert_eq!(value_count as i64, writer.ord());
                writer.finish()?;
                assert_eq!(value_count as i64, writer.ord());
                file_pointer = out.get_file_pointer();
            }
            let mut input = dir.open_input("out.bin", &IO_CONTEXT_DEFAULT)?;
            let mut reader = MonotonicBlockPackedReader::of(
                &mut input,
                PackedInts::VERSION_CURRENT,
                block_size,
                value_count as i64,
            )?;
            assert_eq!(file_pointer, input.get_file_pointer());
            for (i, &value) in values.iter().enumerate().take(value_count) {
                assert_eq!(value, reader.get(i as i64)?);
            }
        }

        Ok(())
    }
    #[test]
    fn test_block_reader_overflow() -> Result<(), LuceneError> {
        if !is_night_mode() {
            return Ok(());
        }
        let mut random = random();
        let value_count = TestUtil::next_long(
            &mut random,
            1 + i64::from(i32::MAX),
            i64::from(i32::MAX) * 2,
        );
        let block_size = 1 << TestUtil::next_long(&mut random, 20, 22);
        let mut dir = new_directory(&mut random)?;
        let value_offset = TestUtil::next_long(&mut random, 0, value_count - 1);
        let value = random.gen::<i64>() & 0xFFFFFFFF;

        {
            let mut out = dir.create_output("out.bin", &IO_CONTEXT_DEFAULT)?;
            let mut writer =
                AbstractBlockPackedWriter::new(block_size as i32, BlockPackedWriter, &mut out)?;

            let mut i = 0;
            while i < value_count {
                assert_eq!({ i }, writer.ord());
                if (i & (block_size - 1)) == 0
                    && (i + block_size < value_offset
                        || (i > value_offset && i + block_size < value_count))
                {
                    writer.add_block_of_zeros()?;
                    i += block_size;
                } else if i == value_offset {
                    writer.add(value)?;
                    i += 1;
                } else {
                    writer.add(0)?;
                    i += 1;
                }
            }
        }

        let mut input = dir.open_input("out.bin", &IO_CONTEXT_DEFAULT)?;
        debug_assert!(block_size <= u32::MAX as i64);
        let mut reader = BlockPackedReaderIterator::new(
            &mut input,
            PackedInts::VERSION_CURRENT,
            block_size as i32,
            value_count as i64,
        )?;

        reader.skip(value_offset as i64)?;
        assert_eq!(value, reader.next_value()?);

        Ok(())
    }
}
