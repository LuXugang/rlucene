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
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fst_impl::fst::BytesReader;

/// Static helper methods for `FST::Arc::BitTable`.
///
/// # Experimental
pub(crate) struct BitTableUtil;
impl BitTableUtil {
  /// Returns whether the bit at the given zero-based index is set.
  ///
  /// # Example
  /// A `bit_index` of 10 refers to the third bit on the right of the second
  /// byte.
  ///
  /// # Parameters
  /// - `bit_index`: The zero-based index of the bit. It must be greater than
  ///   or equal to 0 and strictly less than `number of bit-table bytes *
  ///   Byte::SIZE`.
  /// - `reader`: The [`FST::BytesReader`](BytesReader) used for reading. It
  ///   must be positioned at the beginning of the bit-table.
  pub fn is_bit_set(bit_index: i32, reader: &mut impl BytesReader) -> Result<bool> {
    debug_assert!(bit_index >= 0, "bitIndex={bit_index}");
    reader.skip_bytes((bit_index >> 3) as i64)?;
    let b = Self::read_byte(reader)?;
    let mask = 1u64 << (bit_index as u32 & (u8::BITS - 1));
    Ok((b & mask) != 0)
  }
  /// Counts all bits set in the bit-table.
  ///
  /// # Parameters
  /// - `bit_table_bytes`: The number of bytes in the bit-table.
  /// - `reader`: The [`FST::BytesReader`](BytesReader) used for reading. It
  ///   must be positioned at the beginning of the bit-table.
  pub fn count_bits(bit_table_bytes: i32, reader: &mut impl BytesReader) -> Result<i32> {
    debug_assert!(bit_table_bytes >= 0, "bitTableBytes={bit_table_bytes}");
    let mut bit_count = 0;
    let num_long_blocks = bit_table_bytes >> 3;
    for _ in 0..num_long_blocks {
      bit_count += Self::bit_count_8_bytes(reader)?;
    }
    let num_remaining_bytes = bit_table_bytes & (BitUtil::LONG_BYTES - 1) as i32;
    if num_remaining_bytes != 0 {
      bit_count += Self::read_upto_8_bytes(num_remaining_bytes, reader)?.count_ones() as i32;
    }
    Ok(bit_count)
  }
  /// Counts the bits set up to the given bit zero-based index, exclusive.
  ///
  /// In other words, counts how many `1`s there are up to (but excluding) the
  /// given `bit_index`.
  ///
  /// # Example
  /// A `bit_index` of 10 refers to the third bit on the right of the second
  /// byte.
  ///
  /// # Parameters
  /// - `bit_index`: The zero-based index, exclusive. It must be greater than
  ///   or equal to 0 and less than or equal to `number of bit-table bytes *
  ///   Byte::SIZE`.
  /// - `reader`: The [`FST::BytesReader`](BytesReader) used for reading. It
  ///   must be positioned at the beginning of the bit-table.
  pub fn count_bits_upto(bit_index: i32, reader: &mut impl BytesReader) -> Result<i32> {
    debug_assert!(bit_index >= 0, "bitIndex={bit_index}");
    let mut bit_count = 0;
    let num_long_blocks = bit_index >> 6;
    for _ in 0..num_long_blocks {
      // Count the bits set for all plain longs.
      bit_count += Self::bit_count_8_bytes(reader)?;
    }
    let remaining_bits = bit_index & (i64::BITS - 1) as i32;
    if remaining_bits != 0 {
      let num_remaining_bytes = (remaining_bits + (i8::BITS - 1) as i32) >> 3;
      // Prepare a mask with 1s on the right up to bitIndex exclusive.
      let mask = 1u64.wrapping_shl(bit_index as u32).wrapping_sub(1); // Shifts are mod 64.
      // Count the bits set only within the mask part, so up to bitIndex
      // exclusive.
      let l = Self::read_upto_8_bytes(num_remaining_bytes, reader)?;
      bit_count += (l & mask).count_ones() as i32;
    }
    Ok(bit_count)
  }
  /// Returns the index of the next set bit following the given zero-based
  /// index.
  ///
  /// # Example
  /// Given the bit sequence `100011`:
  /// - The next set bit after `index = -1` is at `index = 0`.
  /// - The next set bit after `index = 0` is at `index = 1`.
  /// - The next set bit after `index = 1` is at `index = 5`.
  /// - There is no next set bit after `index = 5`.
  ///
  /// # Parameters
  /// - `bit_index`: The zero-based index of the bit. It must be greater than
  ///   or equal to -1 and strictly less than `number of bit-table bytes *
  ///   Byte::SIZE`.
  /// - `bit_table_bytes`: The number of bytes in the bit-table.
  /// - `reader`: The [`FST::BytesReader`](BytesReader) used for reading. It
  ///   must be positioned at the beginning of the bit-table.
  ///
  /// # Returns
  /// The zero-based index of the next set bit after `bit_index`, or `-1` if
  /// none exist.
  pub fn next_bit_set(
    bit_index: i32,
    bit_table_bytes: i32,
    reader: &mut impl BytesReader,
  ) -> Result<i32> {
    debug_assert!(
      bit_index >= -1 && bit_index < bit_table_bytes * i8::BITS as i32,
      "bitIndex={bit_index} bitTableBytes={bit_table_bytes}"
    );
    let mut byte_index = bit_index / i8::BITS as i32;
    let mask: i32 = -1 << ((bit_index + 1) & (i8::BITS as i32 - 1));
    let mut i: i32;
    if mask == -1 && bit_index != -1 {
      reader.skip_bytes((byte_index + 1) as i64)?;
      i = 0;
    } else {
      reader.skip_bytes(byte_index as i64)?;
      i = (reader.read_byte()? as i32 & 0xFF) & mask;
    }
    while i == 0 {
      byte_index += 1;
      if byte_index == bit_table_bytes {
        return Ok(-1);
      }
      i = reader.read_byte()? as i32 & 0xFF;
    }
    Ok(i.trailing_zeros() as i32 + (byte_index << 3))
  }
  /// Returns the index of the previous set bit preceding the given zero-based
  /// index.
  ///
  /// # Example
  /// Given the bit sequence `100011`:
  /// - There is no previous set bit before `index = 0`.
  /// - The previous set bit before `index = 1` is at `index = 0`.
  /// - The previous set bit before `index = 5` is at `index = 1`.
  /// - The previous set bit before `index = 64` is at `index = 5`.
  ///
  /// # Parameters
  /// - `bit_index`: The zero-based index of the bit. It must be greater than
  ///   or equal to 0 and less than or equal to `number of bit-table bytes *
  ///   Byte::SIZE`.
  /// - `reader`: The [`FST::BytesReader`](BytesReader) used for reading. It
  ///   must be positioned at the beginning of the bit-table.
  ///
  /// # Returns
  /// The zero-based index of the previous set bit before `bit_index`, or `-1`
  /// if none exist.
  pub fn previous_bit_set(bit_index: i32, reader: &mut impl BytesReader) -> Result<i32> {
    debug_assert!(bit_index >= 0, "bitIndex={bit_index}");
    let mut byte_index = bit_index >> 3;
    reader.skip_bytes(byte_index as i64)?;
    let mask: i32 = (1 << (bit_index & (i8::BITS - 1) as i32)) - 1;
    let mut i = reader.read_byte()? as i32 & 0xFF;
    i &= mask;
    while i == 0 {
      if byte_index == 0 {
        return Ok(-1);
      }
      byte_index -= 1;
      // FST.BytesReader implementations support negative skip.
      reader.skip_bytes(-2)?;
      i = reader.read_byte()? as i32 & 0xFF;
    }
    Ok(((i32::BITS - 1) as i32 - i.leading_zeros() as i32) + (byte_index << 3))
  }

  fn read_byte(reader: &mut impl BytesReader) -> Result<u64> {
    let b = reader.read_byte()?;
    Ok((b as u64) & 0xFF)
  }

  fn read_upto_8_bytes(num_bytes: i32, reader: &mut impl BytesReader) -> Result<u64> {
    debug_assert!(num_bytes > 0 && num_bytes <= 8, "numBytes={num_bytes}");
    let mut l = Self::read_byte(reader)?;
    let mut shift = 0;
    let mut remaining = num_bytes - 1;
    while remaining != 0 {
      shift += 8;
      l |= Self::read_byte(reader)? << shift;
      remaining -= 1;
    }
    Ok(l)
  }

  fn bit_count_8_bytes(reader: &mut impl BytesReader) -> Result<i32> {
    let l = reader.read_long()?;
    Ok(l.count_ones() as i32)
  }
}
