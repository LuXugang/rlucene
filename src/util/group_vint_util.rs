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
use crate::store::data_input::DataInput;
use crate::store::data_output::DataOutput;
use crate::store::random_access_input::RandomAccessInput;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::{LuceneError, Result};

// we use long array instead of int array to make negative integer to be read as positive long.
const MASKS: [u64; 4] = [0xFF, 0xFFFF, 0xFFFFFF, 0xFFFFFFFF];

pub struct GroupVIntUtil;

impl GroupVIntUtil {
    // the maximum length of a single group-varint is 4 integers + 1 byte flag.
    pub const MAX_LENGTH_PER_GROUP: usize = 17;
    /// Reads all the group varints, including the tail vints. We need a `Vec<i64>` because this is what
    /// postings are using, and all longs are actually required to be integers.
    ///
    /// # Arguments
    /// * `dst` - The array to read `i64` values into.
    /// * `limit` - The number of `i64` values to read.
    ///
    /// # Note
    /// This is an experimental API.
    pub fn read_group_vints<D>(data_input: &mut D, dst: &mut [i64], limit: i32) -> Result<()>
    where
        D: DataInput,
    {
        debug_assert!(limit >= 0);
        let mut i = 0;
        while i <= limit - 4 {
            data_input.read_group_vint(dst, i)?;
            i += 4;
        }
        while i < limit {
            dst[i as usize] = data_input.read_vint()? as i64 & 0xFFFFFFFF;
            i += 1;
        }
        Ok(())
    }
    /// Default implementation of reading a single group. For optimal performance, you should use
    /// [`GroupVIntUtil::read_group_vints`](GroupVIntUtil::read_group_vints) instead.
    ///
    /// # Arguments
    /// * `in` - The input to use to read data.
    /// * `dst` - The array to read `i64` values into.
    /// * `offset` - The offset in the array to start storing `i64` values.
    pub fn read_group_vint<D>(data_input: &mut D, dst: &mut [i64], offset: i32) -> Result<()>
    where
        D: DataInput,
    {
        {
            let flag = data_input.read_byte()? as usize;

            let n1_minus1 = flag >> 6;
            let n2_minus1 = (flag >> 4) & 0x03;
            let n3_minus1 = (flag >> 2) & 0x03;
            let n4_minus1 = flag & 0x03;

            dst[offset as usize] = Self::read_long_in_group(data_input, n1_minus1)?;
            dst[offset as usize + 1] = Self::read_long_in_group(data_input, n2_minus1)?;
            dst[offset as usize + 2] = Self::read_long_in_group(data_input, n3_minus1)?;
            dst[offset as usize + 3] = Self::read_long_in_group(data_input, n4_minus1)?;

            Ok(())
        }
    }
    fn read_long_in_group<D>(data_input: &mut D, num_bytes_minus1: usize) -> Result<i64>
    where
        D: DataInput,
    {
        match num_bytes_minus1 {
            0 => {
                let value = data_input.read_byte()? as u64 & 0xFF;
                Ok(value as i64)
            }
            1 => {
                let value = data_input.read_short()? as u64 & 0xFFFF;
                Ok(value as i64)
            }
            2 => {
                let short_part = data_input.read_short()? as u64 & 0xFFFF;
                let byte_part = (data_input.read_byte()? as u64 & 0xFF) << 16;
                Ok((short_part | byte_part) as i64)
            }
            _ => {
                let value = data_input.read_int()? as u64 & 0xFFFFFFFF;
                Ok(value as i64)
            }
        }
    }
    /// Faster implementation of reading a single group. It reads values from the buffer that would not cross
    /// boundaries.
    ///
    /// # Arguments
    /// * `in` - The input to use to read data.
    /// * `remaining` - The number of remaining bytes allowed to read for the current block/segment.
    /// * `pos` - The start position to read from the reader.
    /// * `dst` - The array to read `i64` values into.
    /// * `offset` - The offset in the array to start storing `i64` values.
    ///
    /// # Returns
    /// The number of bytes read excluding the flag. This indicates the number of positions that should be
    /// increased for the caller. It is a non-negative number less than `MAX_LENGTH_PER_GROUP`.
    pub fn read_group_vint_with_reader<D>(
        data_input: &mut D,
        remaining: u64,
        mut pos: i64,
        dst: &mut [i64],
        offset: i32,
    ) -> Result<i32>
    where
        D: DataInput + RandomAccessInput,
    {
        if remaining < Self::MAX_LENGTH_PER_GROUP as u64 {
            Self::read_group_vint(data_input, dst, offset)?;
            return Ok(0);
        }

        let flag = DataInput::read_byte(data_input)? as usize;
        let pos_start = pos + 1; // exclude the flag bytes, the position has updated via read_byte().

        let n1_minus1 = flag >> 6;
        let n2_minus1 = (flag >> 4) & 0x03;
        let n3_minus1 = (flag >> 2) & 0x03;
        let n4_minus1 = flag & 0x03;
        // This code path has fewer conditionals and tends to be significantly faster in benchmarks

        dst[offset as usize] =
            (RandomAccessInput::read_int(data_input, pos)? as u64 & MASKS[n1_minus1]) as i64;
        pos += 1 + n1_minus1 as i64;

        dst[offset as usize + 1] =
            (RandomAccessInput::read_int(data_input, pos)? as u64 & MASKS[n2_minus1]) as i64;
        pos += 1 + n2_minus1 as i64;

        dst[offset as usize + 2] =
            (RandomAccessInput::read_int(data_input, pos)? as u64 & MASKS[n3_minus1]) as i64;
        pos += 1 + n3_minus1 as i64;

        dst[offset as usize + 3] =
            (RandomAccessInput::read_int(data_input, pos)? as u64 & MASKS[n4_minus1]) as i64;
        pos += 1 + n4_minus1 as i64;
        let result = pos - pos_start;
        debug_assert!(
            result <= i32::MAX as i64,
            "result: {} exceeds i32::MAX",
            result
        );
        Ok(result as i32)
    }
    fn num_bytes(v: i32) -> u32 {
        // | 1 ensures it returns 1 when v = 0
        BitUtil::INT_BYTES as u32 - ((v | 1).leading_zeros() / 8)
    }
    /// Converts an i64 value to an i32, ensuring it fits within the valid range.
    /// Throws an error if the value is not within 0 to 0xFFFFFFFF.
    fn get_int(value: i64) -> Result<i32> {
        if value > 0xFFFFFFFF {
            Err(LuceneError::integer_overflow(format!(
                "value: {} is too large to be converted to i32",
                value
            )))
        } else {
            Ok(value as i32)
        }
    }
    /// The implementation for group-varint encoding. It uses a maximum of [`MAX_LENGTH_PER_GROUP`](GroupVIntUtil::MAX_LENGTH_PER_GROUP) bytes scratch buffer.
    pub fn write_group_vints<D>(
        data_output: &mut D,
        scratch: &mut [u8],
        values: &mut [i64],
        limit: i32,
    ) -> Result<()>
    where
        D: DataOutput,
    {
        let mut read_pos: usize = 0;

        // encode each group
        while (limit as usize - read_pos) >= 4 {
            let mut write_pos: usize = 0;
            let n1_minus1 = Self::num_bytes(Self::get_int(values[read_pos])?);
            let n2_minus1 = Self::num_bytes(Self::get_int(values[read_pos + 1])?);
            let n3_minus1 = Self::num_bytes(Self::get_int(values[read_pos + 2])?);
            let n4_minus1 = Self::num_bytes(Self::get_int(values[read_pos + 3])?);

            let flag = ((n1_minus1 - 1) << 6)
                | ((n2_minus1 - 1) << 4)
                | ((n3_minus1 - 1) << 2)
                | (n4_minus1 - 1);
            scratch[write_pos] = flag as u8;
            write_pos += 1;

            BitUtil::set_i32_le_with_len(
                &mut scratch[write_pos..],
                0,
                Self::get_int(values[read_pos])?,
                n1_minus1 as usize,
            );
            write_pos += (n1_minus1) as usize;

            BitUtil::set_i32_le_with_len(
                &mut scratch[write_pos..],
                0,
                Self::get_int(values[read_pos + 1])?,
                n2_minus1 as usize,
            );
            write_pos += (n2_minus1) as usize;

            BitUtil::set_i32_le_with_len(
                &mut scratch[write_pos..],
                0,
                Self::get_int(values[read_pos + 2])?,
                n3_minus1 as usize,
            );
            write_pos += (n3_minus1) as usize;

            BitUtil::set_i32_le_with_len(
                &mut scratch[write_pos..],
                0,
                Self::get_int(values[read_pos + 3])?,
                n4_minus1 as usize,
            );
            write_pos += (n4_minus1) as usize;

            debug_assert!(write_pos <= i32::MAX as usize, "write_pos exceeds u32::MAX");
            data_output.write_bytes_with_len(scratch, write_pos as i32)?;
            read_pos += 4;
        }

        // tail vints
        while read_pos < limit as usize {
            data_output.write_vint(Self::get_int(values[read_pos])?)?;
            read_pos += 1;
        }

        Ok(())
    }
}
