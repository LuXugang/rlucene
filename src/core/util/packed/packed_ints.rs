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
use std::fmt;
use std::fmt::{Display, Formatter};

use crate::core::store::{DataInput, DataOutput};
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::longs_ref::LongsRef;
use crate::core::util::packed::bulk_operation::of;
use crate::core::util::packed::bulk_operation_packed_enum::BulkOperationPackedEnum;
use crate::core::util::packed::format_behavior::{PackedImpl, PackedSingleBlockImpl};
use crate::core::util::packed::mutable_packed64_enum::MutablePacked64Enum;
use crate::core::util::packed::packed_reader_iterator::PackedReaderIterator;
use crate::core::util::packed::packed_writer::PackedWriter;
use crate::core::util::packed::packed64::Packed64;
use crate::core::util::packed::packed64_single_block::create;

pub struct PackedInts;
impl PackedInts {
  /// At most 700% memory overhead, always select a direct implementation.
  pub const FASTEST: f32 = 7.0;

  /// At most 50% memory overhead, always select a reasonably fast
  /// implementation.
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
  ) -> Result<&'static BulkOperationPackedEnum> {
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
  /// A result containing a reference to the encoder, or an error if the
  /// version is invalid.
  pub(crate) fn get_encoder(
    format: Format,
    version: i32,
    bits_per_value: i32,
  ) -> Result<&'static BulkOperationPackedEnum> {
    PackedInts::get_decoder(format, version, bits_per_value)
  }
  /// Expert: Restore a [`ReaderIterator`] from a stream without reading
  /// metadata at the beginning of the stream. This method is useful to
  /// restore data from streams which have been created using
  /// `PackedInts::get_writer_no_header`.
  ///
  /// # Arguments
  /// - `input`: The stream to read data from, positioned at the beginning of
  ///   the packed values.
  /// - `format`: The format used to serialize.
  /// - `version`: The version used to serialize the data.
  /// - `value_count`: How many values the stream holds.
  /// - `bits_per_value`: The number of bits per value.
  /// - `mem`: How much memory the iterator is allowed to use to read-ahead
  ///   (likely to speed up iteration).
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
  ) -> Result<ReaderIteratorImpl<PackedReaderIterator<'_, T>>>
  where
    T: DataInput,
  {
    check_version(version)?;
    let sub_reader =
      PackedReaderIterator::new(format, version, value_count, bits_per_value, input, mem)?;
    Ok(ReaderIteratorImpl::new(
      value_count,
      bits_per_value,
      sub_reader,
    ))
  }
  /// Create a packed integer array with the given amount of values
  /// initialized to 0. The `value_count` and the `bits_per_value` cannot
  /// be changed after creation. All mutables known by this factory
  /// are kept fully in RAM.
  ///
  /// Positive values of `acceptable_overhead_ratio` will trade space for
  /// speed by selecting a faster but potentially less memory-efficient
  /// implementation. An `acceptable_overhead_ratio` of
  /// [`PackedInts::COMPACT`] will make sure that the most memory-efficient
  /// implementation is selected, whereas [`PackedInts::FASTEST`] will
  /// make sure that the fastest implementation is selected.
  ///
  /// # Arguments
  /// - `value_count`: The number of elements.
  /// - `bits_per_value`: The number of bits available for any given value.
  /// - `acceptable_overhead_ratio`: An acceptable overhead ratio per value.
  ///
  /// # Returns
  /// A mutable packed integer array.
  pub(crate) fn get_mutable(
    value_count: i32,
    bits_per_value: i32,
    acceptable_overhead_ratio: f32,
  ) -> MutablePacked64Enum {
    let format_and_bits =
      fastest_format_and_bits(value_count, bits_per_value, acceptable_overhead_ratio);
    PackedInts::get_mutable_impl(
      value_count,
      format_and_bits.bits_per_value,
      format_and_bits.format,
    )
  }

  /// Same as [`get_mutable`](PackedInts::get_mutable) with a pre-computed
  /// number of bits per value and format.
  pub(crate) fn get_mutable_impl(
    value_count: i32,
    bits_per_value: i32,
    format: Format,
  ) -> MutablePacked64Enum {
    debug_assert!(value_count >= 0);
    match format {
      Format::PackedSingleBlock(_) => create(value_count, bits_per_value),
      Format::Packed(_) => {
        MutablePacked64Enum::P64(MutableImpl::new(Packed64::new(value_count, bits_per_value)))
      },
    }
  }
  /// Expert: Create a packed integer array writer for the given output,
  /// format, value count, and number of bits per value.
  ///
  /// The resulting stream will be long-aligned. This means that depending on
  /// the format which is used, up to 63 bits will be wasted. An easy way
  /// to make sure that no space is lost is to always use a `value_count`
  /// that is a multiple of 64.
  ///
  /// This method does not write any metadata to the stream, meaning that it
  /// is your responsibility to store it somewhere else in order to be
  /// able to recover data from the stream later on:
  ///
  /// - `format` (using
  ///   [`Format::get_id`](crate::core::util::packed::FormatBehavior::get_id))
  /// - `value_count`
  /// - `bits_per_value`
  /// - [`PackedInts::VERSION_CURRENT`].
  ///
  /// It is possible to start writing values without knowing how many of them
  /// you are actually going to write. To do this, just pass `-1` as
  /// `value_count`. On the other hand, for any positive
  /// value of `value_count`, the returned writer will make sure that you
  /// don't write more values than expected and pad the end of the stream
  /// with zeros in case you have written less than `value_count`
  /// when calling [`Writer::finish`].
  ///
  /// The `mem` parameter lets you control how much memory can be used to
  /// buffer changes in memory before flushing to disk. High values of
  /// `mem` are likely to improve throughput. On the other hand, if speed
  /// is not that important to you, a value of `0` will use as little memory
  /// as possible and should already offer reasonable throughput.
  ///
  /// # Arguments
  /// - `out`: The data output.
  /// - `format`: The format to use to serialize the values.
  /// - `value_count`: The number of values.
  /// - `bits_per_value`: The number of bits per value.
  /// - `mem`: How much memory (in bytes) can be used to speed up
  ///   serialization.
  ///
  /// # Returns
  /// A `Writer` instance.
  pub(crate) fn get_writer_no_header<T>(
    out: &mut T,
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

  /// Returns how many bits are required to hold values up to and including
  /// `max_value`.
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
  pub fn bits_required(max_value: i64) -> Result<i32> {
    if max_value < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "max_value must be non-negative (got {max_value})"
      )));
    }
    Ok(PackedInts::unsigned_bits_required(max_value))
  }

  /// Returns how many bits are required to store `bits`, interpreted as an
  /// unsigned value. NOTE: This method returns at least 1.
  ///
  /// # Arguments
  /// - `bits`: The unsigned value for which to determine the required bit
  ///   count.
  ///
  /// # Returns
  /// The number of bits required to store `bits`.
  pub fn unsigned_bits_required(bits: i64) -> i32 {
    (64 - bits.leading_zeros() as usize).max(1) as i32
  }

  /// Calculates the maximum unsigned long that can be expressed with the
  /// given number of bits.
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
      !(!0i64 << bits_per_value)
    }
  }
  /// Copy `src[src_pos..src_pos+len]` into `dest[dest_pos..dest_pos+len]`
  /// using at most `mem` bytes.
  pub fn copy(
    src: &mut impl Reader,
    src_pos: i32,
    dest: &mut impl Mutable,
    dest_pos: i32,
    len: i32,
    mem: i32,
  ) {
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
        dest.set(dest_pos + i, src.get((src_pos + i) as usize));
      }
    } else if len > 0 {
      // Use bulk operations
      let buf_size = capacity.min(len);
      let mut buf = vec![0; buf_size as usize];
      PackedInts::copy_with_buffer(src, src_pos, dest, dest_pos, len, &mut buf);
    }
  }
  /// Same as `copy` but uses a pre-allocated buffer.
  pub fn copy_with_buffer(
    src: &impl Reader,
    mut src_pos: i32,
    dest: &mut impl Mutable,
    mut dest_pos: i32,
    mut len: i32,
    buf: &mut [i64],
  ) {
    debug_assert!(!buf.is_empty(), "Buffer length must be greater than 0");

    let mut remaining = 0;

    while len > 0 {
      debug_assert!(buf.len() <= i32::MAX as usize);
      let read = src.get_bulk(
        src_pos,
        buf,
        remaining,
        len.min(buf.len() as i32 - remaining),
      );
      debug_assert!(read > 0, "Read operation failed");
      src_pos += read;
      len -= read;
      remaining += read;

      let written = dest.set_bulk(dest_pos, buf, 0, remaining);
      debug_assert!(written > 0, "Write operation failed");
      dest_pos += written;

      if written < remaining {
        buf.copy_within(written as usize..remaining as usize, 0);
      }
      remaining -= written;
    }

    while remaining > 0 {
      let written = dest.set_bulk(dest_pos, buf, 0, remaining);
      dest_pos += written;
      remaining -= written;
      if remaining > 0 {
        buf.copy_within(written as usize..(written + remaining) as usize, 0);
      }
    }
  }
  /// Check that the block size is a power of 2, within the right bounds, and
  /// return its log in base 2.
  pub fn check_block_size(
    block_size: i32,
    min_block_size: i32,
    max_block_size: i32,
  ) -> Result<i32> {
    if block_size < min_block_size || block_size > max_block_size {
      return Err(LuceneError::illegal_argument(format!(
        "block_size must be >= {min_block_size} and <= {max_block_size}, got {block_size}"
      )));
    }

    if block_size & (block_size - 1) != 0 {
      return Err(LuceneError::illegal_argument(format!(
        "block_size must be a power of two, got {block_size}"
      )));
    }
    let result = block_size.trailing_zeros();
    debug_assert!(result <= i32::MAX as u32);
    Ok(result as i32)
  }
  /// Return the number of blocks required to store `size` values on
  /// `block_size`.
  pub fn num_blocks(size: usize, block_size: i32) -> Result<i32> {
    let num_blocks = (size / block_size as usize)
      + if size.is_multiple_of(block_size as usize) {
        0
      } else {
        1
      };
    let result = num_blocks.checked_mul(block_size as usize);
    match result {
      Some(result) => {
        if result < size {
          return Err(LuceneError::illegal_argument(
            "size is too large for this block size",
          ));
        }
        debug_assert!(num_blocks <= i32::MAX as usize);
        Ok(num_blocks as i32)
      },
      None => Err(LuceneError::illegal_argument(format!(
        "multiply overflow:block_size:{block_size}, num_blocks:{num_blocks} "
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
pub fn check_version(version: i32) -> Result<()> {
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

  /// A format that may insert padding bits to improve encoding and decoding
  /// speed. This format is deprecated; use `Packed` instead.
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
/// Find the fastest [`Format`] and `bits_per_value` that would restore from
/// disk with overhead less than the specified acceptable overhead ratio.
///
/// # Arguments
///
/// * `value_count` - The number of values to write. Use `usize::MAX` if
///   unknown.
/// * `bits_per_value` - The number of bits per value.
/// * `acceptable_overhead_ratio` - The acceptable overhead ratio.
///
/// # Returns
///
/// A `FormatAndBits` struct containing the selected format and bits per value.
// `value_count` is not used in Java Lucene
pub fn fastest_format_and_bits(
  // TODO
  _value_count: i32,
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
  /// The minimum number of long blocks to encode in a single iteration, when
  /// using long encoding.
  fn long_block_count(&self) -> i32 {
    unimplemented!("long_block_count() must be implemented if it needs to be used")
  }

  /// The number of values that can be stored in `long_block_count()` long
  /// blocks.
  fn long_value_count(&self) -> i32 {
    unimplemented!("long_value_count() must be implemented if it needs to be used")
  }

  /// The minimum number of byte blocks to encode in a single iteration, when
  /// using byte encoding.
  fn byte_block_count(&self) -> i32 {
    unimplemented!("byte_block_count() must be implemented if it needs to be used")
  }

  /// The number of values that can be stored in `byte_block_count()` byte
  /// blocks.
  fn byte_value_count(&self) -> i32 {
    unimplemented!("byte_value_count() must be implemented if it needs to be used")
  }

  /// Read `iterations * block_count()` blocks from `blocks`, decode them, and
  /// write `iterations * value_count()` values into `values`.
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
    unimplemented!("decode_long_to_long() must be implemented if it needs to be used")
  }

  /// Read `8 * iterations * block_count()` blocks from `blocks`, decode them,
  /// and write `iterations * value_count()` values into `values`.
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
    unimplemented!("decode_byte_to_long() must be implemented if it needs to be used")
  }

  /// Read `iterations * block_count()` blocks from `blocks`, decode them, and
  /// write `iterations * value_count()` values into `values`.
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
    unimplemented!("decode_long_to_int() must be implemented if it needs to be used")
  }

  /// Read `8 * iterations * block_count()` blocks from `blocks`, decode them,
  /// and write `iterations * value_count()` values into `values`.
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
  /// The minimum number of long blocks to encode in a single iteration, when
  /// using long encoding.
  fn long_block_count(&self) -> i32 {
    unimplemented!("long_block_count() must be implemented if it needs to be used")
  }

  /// The number of values that can be stored in `long_block_count()` long
  /// blocks.
  fn long_value_count(&self) -> i32 {
    unimplemented!("long_value_count() must be implemented if it needs to be used")
  }

  /// The minimum number of byte blocks to encode in a single iteration, when
  /// using byte encoding.
  fn byte_block_count(&self) -> i32 {
    unimplemented!("byte_block_count() must be implemented if it needs to be used")
  }

  /// The number of values that can be stored in `byte_block_count()` byte
  /// blocks.
  fn byte_value_count(&self) -> i32 {
    unimplemented!("byte_value_count() must be implemented if it needs to be used")
  }

  /// Read `iterations * value_count()` values from `values`, encode them, and
  /// write `iterations * block_count()` blocks into `blocks`.
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
    unimplemented!("encode_long_to_long() must be implemented if it needs to be used")
  }

  /// Read `iterations * value_count()` values from `values`, encode them, and
  /// write `8 * iterations * block_count()` blocks into `blocks`.
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
    unimplemented!("encode_long_to_byte() must be implemented if it needs to be used")
  }

  /// Read `iterations * value_count()` values from `values`, encode them, and
  /// write `iterations * block_count()` blocks into `blocks`.
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
    unimplemented!("encode_int_to_long() must be implemented if it needs to be used")
  }

  /// Read `iterations * value_count()` values from `values`, encode them, and
  /// write `8 * iterations * block_count()` blocks into `blocks`.
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
    unimplemented!("encode_int_to_byte() must be implemented if it needs to be used")
  }
}

/// A read-only random access array of positive integers.
pub trait Reader: Accountable {
  /// Get the value at the given index.
  fn get(&self, _index: usize) -> i64 {
    unimplemented!("get() must be implemented if it needs to be used")
  }

  /// Bulk get: read at least one and at most `len` values starting from
  /// `index` into `arr[off.off+len]` and return the actual number of
  /// values that have been read.
  fn get_bulk(&self, index: i32, arr: &mut [i64], off: i32, len: i32) -> i32 {
    self.default_get_bulk(index, arr, off, len)
  }
  fn default_get_bulk(&self, index: i32, arr: &mut [i64], off: i32, len: i32) -> i32 {
    debug_assert!(len > 0, "len must be > 0");
    debug_assert!(index < self.size(), "index out of bounds: {index}");
    debug_assert!(
      (off + len) as usize <= arr.len(),
      "offset + len exceeds array length"
    );

    let gets = std::cmp::min(self.size() - index, len);
    for (i, o) in (index..index + gets).zip(off..off + gets) {
      arr[o as usize] = self.get(i as usize);
    }
    gets
  }

  /// Returns the number of values in the reader.
  fn size(&self) -> i32 {
    unimplemented!("size() must be implemented if it needs to be used")
  }
}
/// Run-once iterator trait for decoding previously saved packed integers.
pub trait ReaderIterator: Display {
  /// Returns the next value.
  ///
  /// # Errors
  ///
  /// Returns an error if there is an issue decoding the next value.
  fn next(&mut self) -> Result<i64> {
    Err(LuceneError::not_implemented(""))
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
  fn next_batch(&mut self, count: i32) -> Result<&mut LongsRef>;

  /// Returns the number of bits per value.
  fn get_bits_per_value(&self) -> Result<i32> {
    Err(LuceneError::not_implemented(""))
  }

  /// Returns the total number of values.
  fn size(&self) -> Result<i32> {
    Err(LuceneError::not_implemented(""))
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
  fn next(&mut self) -> Result<i64> {
    let next_values = self.next_batch(1)?;
    debug_assert!(next_values.length > 0, "next_values buffer is empty");
    let result = next_values.longs[next_values.offset];
    next_values.offset += 1;
    next_values.length -= 1;
    Ok(result)
  }

  fn next_batch(&mut self, count: i32) -> Result<&mut LongsRef> {
    self.sub_reader.next_batch(count)
  }

  fn get_bits_per_value(&self) -> Result<i32> {
    Ok(self.bits_per_value)
  }

  fn size(&self) -> Result<i32> {
    Ok(self.value_count)
  }

  fn ord(&self) -> i32 {
    self.sub_reader.ord()
  }
}
impl<C> Display for ReaderIteratorImpl<C>
where
  C: ReaderIterator,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.sub_reader)
  }
}

pub trait Mutable: Reader + Display {
  /// Returns the number of bits used to store any given value.
  ///
  /// Note: This does not imply that memory usage is `bits_per_value *
  /// values, as implementations may use non-space-optimal
  /// packing of bits.
  fn get_bits_per_value(&self) -> i32 {
    unimplemented!("get_bits_per_value() must be implemented if it needs to be used")
  }

  /// Sets the value at the given index in the array.
  ///
  /// # Arguments
  ///
  /// * `index` - The position where the value should be set.
  /// * `value` - The value to be stored, which must conform to the
  ///   constraints of the array.
  fn set(&mut self, _index: i32, _value: i64) {
    unimplemented!("set() must be implemented if it needs to be used")
  }
  /// Sets a range of values in the array.
  ///
  /// # Arguments
  ///
  /// * `index` - The starting index in the packed array where values will be
  ///   set.
  /// * `arr` - The source array of values to set.
  /// * `off` - The offset in the source array to start reading values from.
  /// * `len` - The maximum number of values to set.
  ///
  /// # Returns
  ///
  /// The actual number of values that have been set.
  fn set_bulk(&mut self, index: i32, arr: &[i64], off: i32, len: i32) -> i32 {
    self.default_set_bulk(index, arr, off, len)
  }
  fn default_set_bulk(&mut self, index: i32, arr: &[i64], off: i32, len: i32) -> i32 {
    debug_assert!(len > 0, "len must be > 0 (got {len})");
    debug_assert!(
      index >= 0 && index < self.size(),
      "Index out of bounds: {index}"
    );

    let len = len.min(self.size() - index);
    debug_assert!(
      (off + len) as usize <= arr.len(),
      "Array offset and length out of bounds"
    );

    for (i, o) in (index..index + len).zip(off..off + len) {
      self.set(i, arr[o as usize]);
    }
    len
  }

  /// Fills a range in the packed array with a specific value.
  ///
  /// # Arguments
  ///
  /// * `from_index` - The start index of the range to fill (inclusive).
  /// * `to_index` - The end index of the range to fill (exclusive).
  /// * `val` - The value to fill with.
  fn fill(&mut self, from_index: i32, to_index: i32, val: i64) {
    self.default_fill(from_index, to_index, val)
  }
  fn default_fill(&mut self, from_index: i32, to_index: i32, val: i64) {
    debug_assert!(val <= PackedInts::max_value(self.get_bits_per_value()));
    debug_assert!(
      from_index <= to_index,
      "from_index must be <= to_index: {from_index} > {to_index}"
    );
    for i in from_index..to_index {
      self.set(i, val);
    }
  }

  /// Sets all values in the packed array to 0.
  fn clear(&mut self) {
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
  fn ram_bytes_used(&self) -> Result<i64> {
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
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
  fn ram_bytes_used(&self) -> Result<i64> {
    todo!()
  }
}
impl Reader for NullReader {
  fn get(&self, _index: usize) -> i64 {
    0
  }

  fn get_bulk(&self, index: i32, arr: &mut [i64], off: i32, mut len: i32) -> i32 {
    debug_assert!(len > 0, "len must be > 0 (got {len})");
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
    len
  }

  fn size(&self) -> i32 {
    self.value_count
  }
}
pub trait Writer {
  /// The format used to serialize values.
  fn get_format(&self) -> &Format;
  ///  Add a value to the stream.
  fn add(&mut self, v: i64) -> Result<()>;
  /// The number of bits per value.
  fn bits_per_values(&self) -> i32;
  /// Perform end-of-stream operations.
  fn finish(&mut self) -> Result<()>;
  /// Returns the current ord in the stream (number of values that have been
  /// written so far minus one).
  fn ord(&self) -> i32;
}

#[derive(Debug, Clone)]
pub struct DummyMutable;
impl Reader for DummyMutable {}
impl Accountable for DummyMutable {
  fn ram_bytes_used(&self) -> Result<i64> {
    unreachable!("DummyMutable should not be used")
  }
}
impl Display for DummyMutable {
  fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
    unreachable!("{} should not be displayed", std::any::type_name::<Self>())
  }
}
impl Mutable for DummyMutable {}
