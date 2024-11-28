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
use crate::util::bit_util::{BitUtil, INT_BYTES};
use crate::util::error::data_io_error_enum::DataIOError;

// the maximum length of a single group-varint is 4 integers + 1 byte flag.
pub const MAX_LENGTH_PER_GROUP: usize = 17;
// we use long array instead of int array to make negative integer to be read as positive long.
const MASKS: [u64; 4] = [0xFF, 0xFFFF, 0xFFFFFF, 0xFFFFFFFF];

pub struct GroupVIntUtil;

impl GroupVIntUtil {
    pub fn read_group_vints<D>(
        mut data_input: D,
        dst: &mut [i64],
        limit: usize,
    ) -> Result<(), DataIOError>
    where
        D: DataInput,
    {
        let mut i = 0;
        while i <= limit - 4 {
            data_input.read_group_vint(dst, i)?;
            i += 4;
        }
        for j in 0..limit {
            dst[j] = data_input.read_vint()? as i64;
        }
        Ok(())
    }
    /**
     * Default implementation of read single group, for optimal performance, you should use {@link
     * `GroupVIntUtil#readGroupVInts(DataInput, vec<i64>, i32)` instead.
     */
    pub fn read_group_vint<D>(
        data_input: &mut D,
        dst: &mut [i64],
        offset: usize,
    ) -> Result<(), DataIOError>
    where
        D: DataInput,
    {
        {
            let flag = data_input.read_byte()? as usize;

            let n1_minus1 = flag >> 6;
            let n2_minus1 = (flag >> 4) & 0x03;
            let n3_minus1 = (flag >> 2) & 0x03;
            let n4_minus1 = flag & 0x03;

            dst[offset] = Self::read_long_in_group(data_input, n1_minus1)?;
            dst[offset + 1] = Self::read_long_in_group(data_input, n2_minus1)?;
            dst[offset + 2] = Self::read_long_in_group(data_input, n3_minus1)?;
            dst[offset + 3] = Self::read_long_in_group(data_input, n4_minus1)?;

            Ok(())
        }
    }
    fn read_long_in_group<D>(
        data_input: &mut D,
        num_bytes_minus1: usize,
    ) -> Result<i64, DataIOError>
    where
        D: DataInput,
    {
        match num_bytes_minus1 {
            0 => {
                let value = data_input.read_byte()? as u64;
                Ok(value as i64)
            }
            1 => {
                let value = data_input.read_short()? as u64;
                Ok(value as i64)
            }
            2 => {
                let lower = data_input.read_short()? as u64;
                let higher = (data_input.read_byte()? as u64) << 16;
                Ok((lower | higher) as i64)
            }
            _ => {
                let value = data_input.read_int()? as u64;
                Ok(value as i64)
            }
        }
    }
    /**
     * Faster implementation of read single group, It read values from the buffer that would not cross
     * boundaries.
     */
    pub fn read_group_vint_with_reader<D>(
        data_input: &mut D,
        remaining: u64,
        mut pos: u64,
        dst: &mut [i64],
        offset: usize,
    ) -> Result<i32, DataIOError>
    where
        D: DataInput + RandomAccessInput,
    {
        if remaining < MAX_LENGTH_PER_GROUP as u64 {
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

        dst[offset] =
            (RandomAccessInput::read_int(data_input, pos)? as u64 & MASKS[n1_minus1]) as i64;
        pos += 1 + n1_minus1 as u64;

        dst[offset + 1] =
            (RandomAccessInput::read_int(data_input, pos)? as u64 & MASKS[n2_minus1]) as i64;
        pos += 1 + n2_minus1 as u64;

        dst[offset + 2] =
            (RandomAccessInput::read_int(data_input, pos)? as u64 & MASKS[n3_minus1]) as i64;
        pos += 1 + n3_minus1 as u64;

        dst[offset + 3] =
            (RandomAccessInput::read_int(data_input, pos)? as u64 & MASKS[n4_minus1]) as i64;
        pos += 1 + n4_minus1 as u64;
        let result = pos - pos_start;
        debug_assert!(
            result <= i32::MAX as u64,
            "result: {} exceeds i32::MAX",
            result
        );
        Ok(result as i32)
    }
    fn num_bytes(v: i32) -> i32 {
        (INT_BYTES - ((v as usize | 1).leading_zeros() >> 3) as usize) as i32
    }
    fn get_int(value: i64) -> Result<i32, DataIOError> {
        if value > u32::MAX as i64 {
            Err(DataIOError::integer_overflow(format!(
                "value: {} is too large to be converted to i32",
                value
            )))
        } else {
            Ok(value as i32)
        }
    }
    pub fn write_group_vints<D>(
        data_output: &mut D,
        scratch: &mut [u8],
        values: &mut [i64],
        limit: usize,
    ) -> Result<(), DataIOError>
    where
        D: DataOutput,
    {
        let mut read_pos = 0;

        // encode each group
        while (limit - read_pos) >= 4 {
            let mut write_pos = 0;
            let n1_minus1 = Self::num_bytes(Self::get_int(values[read_pos])?) - 1;
            let n2_minus1 = Self::num_bytes(Self::get_int(values[read_pos + 1])?) - 1;
            let n3_minus1 = Self::num_bytes(Self::get_int(values[read_pos + 2])?) - 1;
            let n4_minus1 = Self::num_bytes(Self::get_int(values[read_pos + 3])?) - 1;

            let flag = (n1_minus1 << 6) | (n2_minus1 << 4) | (n3_minus1 << 2) | n4_minus1;
            scratch[write_pos] = flag as u8;
            write_pos += 1;

            BitUtil::set_i32_le(
                &mut scratch[write_pos..],
                Self::get_int(values[read_pos])? as usize,
                n1_minus1,
            );
            write_pos += (n1_minus1 + 1) as usize;

            BitUtil::set_i32_le(
                &mut scratch[write_pos..],
                Self::get_int(values[read_pos + 1])? as usize,
                n2_minus1,
            );
            write_pos += (n2_minus1 + 1) as usize;

            BitUtil::set_i32_le(
                &mut scratch[write_pos..],
                Self::get_int(values[read_pos + 2])? as usize,
                n3_minus1,
            );
            write_pos += (n3_minus1 + 1) as usize;

            BitUtil::set_i32_le(
                &mut scratch[write_pos..],
                Self::get_int(values[read_pos + 3])? as usize,
                n4_minus1,
            );
            write_pos += (n4_minus1 + 1) as usize;

            data_output.write_bytes_with_len(scratch, write_pos)?;
            read_pos += 4;
        }

        // tail vints
        while read_pos < limit {
            data_output.write_vint(Self::get_int(values[read_pos])?)?;
            read_pos += 1;
        }

        Ok(())
    }
}
