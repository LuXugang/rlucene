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
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::error::runtime_error::RuntimeError;
use crate::util::longs_ref::LongsRef;
use crate::util::packed::bulk_operation::of;
use crate::util::packed::bulk_operation_packed_enum::BulkOperationPackedEnum;
use crate::util::packed::format_behavior::{FormatBehavior, Packed, PackedSingleBlock};
use crate::util::packed::packed64::Packed64;
use crate::util::packed::packed64_single_block::{create, Packed64SingleBlock};
use crate::util::packed::packed64_single_block_enum::MutablePacked64Enum;
use crate::util::packed::packed_long_values::DEFAULT_PAGE_SIZE;
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
    pub const VERSION_MONOTONIC_WITHOUT_ZIGZAG: u32 = 2;
    pub const VERSION_START: u32 = Self::VERSION_MONOTONIC_WITHOUT_ZIGZAG;
    pub const VERSION_CURRENT: u32 = Self::VERSION_MONOTONIC_WITHOUT_ZIGZAG;
    /// Get a [`Decoder`].
    ///
    /// # Arguments
    /// - `format`: The format used to store packed integers.
    /// - `version`: The compatibility version.
    /// - `bits_per_value`: The number of bits per value.
    ///
    /// # Returns
    /// A decoder.
    pub fn get_decoder(
        format: Format,
        version: u32,
        bits_per_value: u32,
    ) -> Result<&'static BulkOperationPackedEnum, DataIOError> {
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
    pub fn get_encoder(
        format: Format,
        version: u32,
        bits_per_value: u32,
    ) -> Result<&'static BulkOperationPackedEnum, DataIOError> {
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
        version: u32,
        value_count: u32,
        bits_per_value: u32,
        mem: u32,
    ) -> Result<PackedReaderIterator<T>, DataIOError>
    where
        T: DataInput,
    {
        check_version(version)?;
        Ok(PackedReaderIterator::new(
            format,
            version,
            value_count,
            bits_per_value,
            input,
            mem,
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
    pub fn get_mutable(
        value_count: u32,
        bits_per_value: u32,
        acceptable_overhead_ratio: f32,
    ) -> Result<MutablePacked64Enum, RuntimeError> {
        let format_and_bits =
            fastest_format_and_bits(value_count, bits_per_value, acceptable_overhead_ratio);
        PackedInts::get_mutable_impl(
            value_count,
            format_and_bits.bits_per_value,
            format_and_bits.format,
        )
    }

    /// Same as [`get_mutable`](get_mutable) with a pre-computed number of bits per value and format.
    pub fn get_mutable_impl(
        value_count: u32,
        bits_per_value: u32,
        format: Format,
    ) -> Result<MutablePacked64Enum, RuntimeError> {
        match format {
            Format::PackedSingleBlock(_) => Ok(create(value_count, bits_per_value)?),
            Format::Packed(_) => Ok(MutablePacked64Enum::P64(MutableImpl::new(
                Packed64::new(value_count, bits_per_value),
                value_count,
                bits_per_value,
            ))),
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
    /// - `format` (using [`Format::get_id`])
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
    pub fn get_writer_no_header<T>(
        out: &'_ mut T,
        format: Format,
        value_count: i32,
        bits_per_value: u32,
        mem: u32,
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
    pub fn bits_required(max_value: i64) -> Result<u32, RuntimeError> {
        if max_value < 0 {
            return Err(RuntimeError::illegal_argument(format!(
                "max_value must be non-negative (got {})",
                max_value
            )));
        }
        Ok(PackedInts::unsigned_bits_required(max_value as u64))
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

    /// Calculates the maximum unsigned long that can be expressed with the given number of bits.
    ///
    /// # Arguments
    ///
    /// * `bits_per_value` - The number of bits available for any given value.
    ///
    /// # Returns
    ///
    /// The maximum value for the given number of bits.
    pub fn max_value(bits_per_value: u32) -> u64 {
        if bits_per_value == 64 {
            u64::MAX
        } else {
            (1u64 << bits_per_value) - 1
        }
    }
    /// Copy `src[src_pos..src_pos+len]` into `dest[dest_pos..dest_pos+len]` using at most `mem` bytes.
    ///
    ///
    pub fn copy(
        src: &mut impl Reader,
        src_pos: usize,
        dest: &mut impl Mutable,
        dest_pos: usize,
        len: usize,
        mem: u32,
    ) -> Result<(), DataIOError> {
        assert!(
            src_pos + len <= src.size() as usize,
            "Source position and length out of bounds"
        );
        assert!(
            dest_pos + len <= dest.size() as usize,
            "Destination position and length out of bounds"
        );

        let capacity = mem >> 3; // Convert memory to the number of 64-bit elements
        if capacity == 0 {
            for i in 0..len {
                dest.set(dest_pos + i, src.get(src_pos + i)?);
            }
        } else if len > 0 {
            // Use bulk operations
            let buf_size = (capacity as usize).min(len);
            let mut buf = vec![0; buf_size];
            PackedInts::copy_with_buffer(src, src_pos, dest, dest_pos, len, &mut buf);
        }
        Ok(())
    }
    /// Same as `copy` but uses a pre-allocated buffer.
    ///
    pub fn copy_with_buffer(
        src: &mut impl Reader,
        mut src_pos: usize,
        dest: &mut impl Mutable,
        mut dest_pos: usize,
        mut len: usize,
        buf: &mut [i64],
    ) -> Result<(), DataIOError> {
        assert!(!buf.is_empty(), "Buffer length must be greater than 0");

        let mut remaining = 0;

        while len > 0 {
            let read = src.get_bulk(src_pos, buf, remaining, len.min(buf.len() - remaining))?;
            assert!(read > 0, "Read operation failed");
            src_pos += read as usize;
            len -= read as usize;
            remaining += read as usize;

            let written = dest.set_bulk(dest_pos, buf, 0, remaining) as usize;
            assert!(written > 0, "Write operation failed");
            dest_pos += written;

            if written < remaining {
                buf.copy_within(written..remaining, 0);
            }
            remaining -= written;
        }

        while remaining > 0 {
            let written = dest.set_bulk(dest_pos, buf, 0, remaining) as usize;
            dest_pos += written;
            remaining -= written;
            if remaining > 0 {
                buf.copy_within(written..(written + remaining), 0);
            }
        }
        Ok(())
    }
    /// Check that the block size is a power of 2, within the right bounds, and return its log in base 2.
    pub fn check_block_size(
        block_size: u32,
        min_block_size: u32,
        max_block_size: u32,
    ) -> Result<u32, RuntimeError> {
        if block_size < min_block_size || block_size > max_block_size {
            panic!(
                "block_size must be >= {} and <= {}, got {}",
                min_block_size, max_block_size, block_size
            );
        }

        if block_size & (block_size - 1) != 0 {
            return Err(RuntimeError::illegal_argument(format!(
                "block_size must be a power of two, got {}",
                block_size
            )));
        }

        Ok(block_size.trailing_zeros())
    }
    /// Return the number of blocks required to store `size` values on `block_size`.
    pub fn num_blocks(size: u64, block_size: u32) -> Result<u32, RuntimeError> {
        let num_blocks =
            (size / block_size as u64) + if size % block_size as u64 == 0 { 0 } else { 1 };
        let result = num_blocks.checked_mul(block_size as u64);
        match result {
            Some(result) => {
                if result < size {
                    return Err(RuntimeError::illegal_argument(
                        "size is too large for this block size".to_string(),
                    ));
                }
                debug_assert!(num_blocks <= u32::MAX as u64);
                Ok(num_blocks as u32)
            }
            None => Err(RuntimeError::illegal_argument(format!(
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
pub fn check_version(version: u32) -> Result<(), DataIOError> {
    if version < PackedInts::VERSION_START {
        return Err(DataIOError::illegal_argument(format!(
            "Version is too old, should be at least {} (got {})",
            PackedInts::VERSION_START,
            version
        )));
    } else if version > PackedInts::VERSION_CURRENT {
        return Err(DataIOError::illegal_argument(format!(
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
    mut value_count: u32,
    bits_per_value: u32,
    mut acceptable_overhead_ratio: f32,
) -> FormatAndBits {
    acceptable_overhead_ratio =
        acceptable_overhead_ratio.clamp(PackedInts::COMPACT, PackedInts::FASTEST);
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
        format: Format::Packed(Packed::new(0)),
        bits_per_value: actual_bits_per_value,
    }
}
/// A decoder for packed integers.
pub trait Decoder {
    /// The minimum number of long blocks to encode in a single iteration, when using long encoding.
    fn long_block_count(&self) -> u32 {
        unimplemented!("long_block_count() must be implemented if it need to be used")
    }

    /// The number of values that can be stored in `long_block_count()` long blocks.
    fn long_value_count(&self) -> u32 {
        unimplemented!("long_value_count() must be implemented if it need to be used")
    }

    /// The minimum number of byte blocks to encode in a single iteration, when using byte encoding.
    fn byte_block_count(&self) -> u32 {
        unimplemented!("byte_block_count() must be implemented if it need to be used")
    }

    /// The number of values that can be stored in `byte_block_count()` byte blocks.
    fn byte_value_count(&self) -> u32 {
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
    fn decode_long_to_long(
        &self,
        _blocks: &[u64],
        _blocks_offset: usize,
        _values: &mut [i64],
        _values_offset: usize,
        _iterations: u32,
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
    fn decode_byte_to_long(
        &self,
        _blocks: &[u8],
        _blocks_offset: usize,
        _values: &mut [i64],
        _values_offset: usize,
        _iterations: u32,
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
    fn decode_long_to_int(
        &self,
        _blocks: &[u64],
        _blocks_offset: usize,
        _values: &mut [i32],
        _values_offset: usize,
        _iterations: u32,
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
    fn decode_byte_to_int(
        &self,
        _blocks: &[u8],
        _blocks_offset: usize,
        _values: &mut [i32],
        _values_offset: usize,
        _iterations: u32,
    ) {
        unimplemented!("decode_byte_to_int() must be implemented")
    }
}
/// An encoder for packed integers.
pub trait Encoder {
    /// The minimum number of long blocks to encode in a single iteration, when using long encoding.
    fn long_block_count(&self) -> u32 {
        unimplemented!("long_block_count() must be implemented if it need to be used")
    }

    /// The number of values that can be stored in `long_block_count()` long blocks.
    fn long_value_count(&self) -> u32 {
        unimplemented!("long_value_count() must be implemented if it need to be used")
    }

    /// The minimum number of byte blocks to encode in a single iteration, when using byte encoding.
    fn byte_block_count(&self) -> u32 {
        unimplemented!("byte_block_count() must be implemented if it need to be used")
    }

    /// The number of values that can be stored in `byte_block_count()` byte blocks.
    fn byte_value_count(&self) -> u32 {
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
    fn encode_long_to_long(
        &self,
        _values: &[i64],
        _values_offset: usize,
        _blocks: &mut [u64],
        _blocks_offset: usize,
        _iterations: u32,
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
    fn encode_long_to_byte(
        &self,
        _values: &[i64],
        _values_offset: usize,
        _blocks: &mut [u8],
        _blocks_offset: usize,
        _iterations: u32,
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
    fn encode_int_to_long(
        &self,
        _values: &[i32],
        _values_offset: usize,
        _blocks: &mut [u64],
        _blocks_offset: usize,
        _iterations: u32,
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
    fn encode_int_to_byte(
        &self,
        _values: &[i32],
        _values_offset: usize,
        _blocks: &mut [u8],
        _blocks_offset: usize,
        _iterations: u32,
    ) {
        unimplemented!("encode_int_to_byte() must be implemented if it need to be used")
    }
}

/// A read-only random access array of positive integers.
pub(crate) trait Reader: Accountable {
    /// Get the value at the given index.
    fn get(&mut self, _index: usize) -> Result<i64, DataIOError> {
        unimplemented!("get() must be implemented if it need to be used")
    }

    /// Bulk get: read at least one and at most `len` values starting from `index`
    /// into `arr[off..off+len]` and return the actual number of values that have been read.
    fn get_bulk(
        &mut self,
        index: usize,
        arr: &mut [i64],
        off: usize,
        len: usize,
    ) -> Result<u32, DataIOError> {
        self.default_get_bulk(index, arr, off, len)
    }
    fn default_get_bulk(
        &mut self,
        index: usize,
        arr: &mut [i64],
        off: usize,
        len: usize,
    ) -> Result<u32, DataIOError> {
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
        debug_assert!(gets <= u32::MAX as usize);
        Ok(gets as u32)
    }

    /// Returns the number of values in the reader.
    fn size(&self) -> u32 {
        unimplemented!("size() must be implemented if it need to be used")
    }
}
/// Run-once iterator interface to decode previously saved PackedInts.
pub(crate) trait ReaderIterator {
    /// Returns the next value.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue decoding the next value.
    fn next(&mut self) -> Result<i64, DataIOError> {
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
    fn next_batch(&mut self, count: u32) -> Result<LongsRef, DataIOError>;

    /// Returns the number of bits per value.
    fn get_bits_per_value(&self) -> u32 {
        unimplemented!("get_bits_per_value() must be implemented if it need to be used")
    }

    /// Returns the total number of values.
    fn size(&self) -> u32 {
        unimplemented!("size() must be implemented if it need to be used")
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

pub trait Mutable: Reader + Display {
    /// Returns the number of bits used to store any given value.
    ///
    /// Note: This does not imply that memory usage is `bits_per_value * #values` as implementations
    /// are free to use non-space-optimal packing of bits.
    fn get_bits_per_value(&self) -> u32 {
        unimplemented!("get_bits_per_value() must be implemented if it need to be used")
    }

    /// Sets the value at the given index in the array.
    ///
    /// # Arguments
    ///
    /// * `index` - The position where the value should be set.
    /// * `value` - The value to be stored, which must conform to the constraints of the array.
    ///
    fn set(&mut self, _index: usize, _value: i64) {
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
    fn set_bulk(&mut self, index: usize, arr: &[i64], off: usize, len: usize) -> u32 {
        self.default_set_bulk(index, arr, off, len)
    }
    fn default_set_bulk(&mut self, index: usize, arr: &[i64], off: usize, len: usize) -> u32 {
        assert!(len > 0, "len must be > 0 (got {})", len);
        assert!(
            index < self.size() as usize,
            "Index out of bounds: {}",
            index
        );

        let len = len.min(self.size() as usize - index);
        assert!(
            off + len <= arr.len(),
            "Array offset and length out of bounds"
        );

        for (i, o) in (index..index + len).zip(off..off + len) {
            self.set(i, arr[o]);
        }
        debug_assert!(len <= u32::MAX as usize);
        len as u32
    }

    /// Fills a range in the packed array with a specific value.
    ///
    /// # Arguments
    ///
    /// * `from_index` - The start index of the range to fill (inclusive).
    /// * `to_index` - The end index of the range to fill (exclusive).
    /// * `val` - The value to fill with.
    fn fill(&mut self, from_index: usize, to_index: usize, val: i64) {
        self.default_fill(from_index, to_index, val)
    }
    fn default_fill(&mut self, from_index: usize, to_index: usize, val: i64) {
        assert!(val as u64 <= PackedInts::max_value(self.get_bits_per_value()));
        assert!(
            from_index <= to_index,
            "from_index must be <= to_index: {} > {}",
            from_index,
            to_index
        );
        for i in from_index..to_index {
            self.set(i, val);
        }
    }

    /// Sets all values in the packed array to 0.
    fn clear(&mut self) {
        self.fill(0, self.size() as usize, 0);
    }
}
pub(crate) struct MutableImpl<T>
where
    T: Mutable + Display,
{
    sub_reader: T,
    value_count: u32,
    bits_per_value: u32,
}
impl<T> MutableImpl<T>
where
    T: Mutable + Display,
{
    pub fn new(sub_reader: T, value_count: u32, bits_per_value: u32) -> Self {
        Self {
            sub_reader,
            value_count,
            bits_per_value,
        }
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
    fn size(&self) -> u32 {
        self.value_count
    }
}

impl<T> Mutable for MutableImpl<T>
where
    T: Mutable + Display,
{
    fn get_bits_per_value(&self) -> u32 {
        self.bits_per_value
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
    value_count: u32,
}
impl NullReader {
    pub fn for_count(value_count: u32) -> Self {
        Self { value_count }
    }
    pub fn new(value_count: u32) -> Self {
        Self::for_count(value_count)
    }
}
impl Accountable for NullReader {
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}
impl Reader for NullReader {
    fn get(&mut self, _index: usize) -> Result<i64, DataIOError> {
        Ok(0)
    }

    fn get_bulk(
        &mut self,
        index: usize,
        arr: &mut [i64],
        off: usize,
        mut len: usize,
    ) -> Result<u32, DataIOError> {
        assert!(
            index < self.value_count as usize,
            "index out of bounds (index={}, valueCount={})",
            index,
            self.value_count
        );

        len = len.min(self.value_count as usize - index);
        assert!(
            off + len <= arr.len(),
            "not enough space in destination array"
        );

        arr[off..off + len].fill(0);
        Ok(len as u32)
    }

    fn size(&self) -> u32 {
        self.value_count
    }
}
pub trait Writer {
    /// The format used to serialize values.
    fn get_format(&self) -> &Format;
    ///  Add a value to the stream.
    fn add(&mut self, v: i64) -> Result<(), DataIOError>;
    /// The number of bits per value.
    fn bits_per_values(&self) -> u32;
    /// Perform end-of-stream operations.
    fn finish(&mut self) -> Result<(), DataIOError>;
    /// Returns the current ord in the stream (number of values that have been written so far minus one).
    fn ord(&self) -> i32;
}
