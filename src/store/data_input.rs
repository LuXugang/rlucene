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
use crate::util::bit_util::BitUtil;
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::group_vint_util::GroupVIntUtil;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;

/// Base trait for performing read operations on Lucene's low-level data types.
///
/// # Note
/// [`DataInput`] is not thread-safe as it maintains internal state (e.g., file position).
pub trait DataInput: Sized + Display {
    /// Reads and returns a single byte.
    ///
    /// # See Also
    /// [`DataOutput::write_byte`](crate::store::data_output::DataOutput::write_byte)
    fn read_byte(&mut self) -> Result<u8, DataIOError>;
    /// Reads a specified number of bytes into an array at the specified offset.
    ///
    /// # Arguments
    /// * `b` - The array to read bytes into.
    /// * `offset` - The offset in the array to start storing bytes.
    /// * `len` - The number of bytes to read.
    ///
    /// # See Also
    /// [`DataOutput::write_bytes_range`](crate::store::data_output::DataOutput::write_bytes_range)
    fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<(), DataIOError>;
    /// Reads a specified number of bytes into an array at the specified offset, with control over 
    /// whether the read should be buffered. Callers who have their own buffer should pass `false` 
    /// for `use_buffer`. Currently, only `BufferedIndexInput` respects this parameter.
    ///
    /// # Arguments
    /// * `b` - The array to read bytes into.
    /// * `offset` - The offset in the array to start storing bytes.
    /// * `len` - The number of bytes to read.
    /// * `use_buffer` - Set to `false` if the caller will handle buffering.
    ///
    /// # See Also
    /// [`DataOutput::write_bytes_with_len`](crate::store::data_output::DataOutput::write_bytes_with_len)
    fn read_bytes_with_buffer(
        &mut self,
        b: &mut [u8],
        offset: usize,
        len: usize,
        _use_buffer: bool,
    ) -> Result<(), DataIOError> {
        self.read_bytes(b, offset, len)
    }
    /// # See
    /// [`DataInput::default_read_short`](DataInput::default_read_short)
    fn read_short(&mut self) -> Result<i16, DataIOError> {
        self.default_read_short()
    }
    /// Reads two bytes and returns a `short` (little-endian byte order).
    ///
    /// # See Also
    /// [`DataOutput::write_short`](crate::store::data_output::DataOutput::write_short)
    /// [`BitUtil::get_i16_le`](BitUtil::get_i16_le)
    fn default_read_short(&mut self) -> Result<i16, DataIOError> {
        let b1 = self.read_byte()?;
        let b2 = self.read_byte()?;
        Ok(i16::from_le_bytes([b2, b1]))
    }
    /// # See
    /// [`DataInput::default_read_int`](DataInput::default_read_int)
    fn read_int(&mut self) -> Result<i32, DataIOError> {
        self.default_read_int()
    }
    /// Reads four bytes and returns an `int` (little-endian byte order).
    ///
    /// # See Also
    /// [`DataOutput::write_int`](crate::store::data_output::DataOutput::write_int)
    /// [`BitUtil::get_i32_le`](BitUtil::get_i32_le)
    fn default_read_int(&mut self) -> Result<i32, DataIOError> {
        let b1 = self.read_byte()?;
        let b2 = self.read_byte()?;
        let b3 = self.read_byte()?;
        let b4 = self.read_byte()?;
        Ok(i32::from_le_bytes([b4, b3, b2, b1]))
    }
    
   /// Override if you have an efficient implementation. In general this is when the input supports
     /// random access.
    fn read_group_vint(&mut self, dst: &mut [i64], offset: usize) -> Result<(), DataIOError> {
        self.default_read_group_vint(dst, offset)
    }
    fn default_read_group_vint(
        &mut self,
        dst: &mut [i64],
        offset: usize,
    ) -> Result<(), DataIOError> {
        GroupVIntUtil::read_group_vint(self, dst, offset)
    }
    /// Reads an `int` stored in a variable-length format. Reads between one and five bytes, 
    /// with smaller values taking fewer bytes. Negative numbers are supported but should be avoided.
    ///
    /// # Format
    /// The format is described further in [`DataOutput::write_vint`](crate::store::data_output::DataOutput::write_vint).
    ///
    /// # See Also
    /// [`DataOutput::write_vint`](crate::store::data_output::DataOutput::write_vint)
    fn read_vint(&mut self) -> Result<i32, DataIOError> {
        let mut b = self.read_byte()? as i32;
        let mut i = b & 0x7F;
        let mut shift = 7;

        while (b & 0x80) != 0 {
            b = self.read_byte()? as i32;
            i |= (b & 0x7F) << shift;
            shift += 7;
        }
        Ok(i)
    }
    /// Reads a [`zig-zag`](BitUtil::zig_zag_decode_i32)-encoded 
    /// [`read_vint`](#method.read_vint) variable-length integer.
    ///
    /// # See Also
    /// [`DataOutput::write_zint`](crate::store::data_output::DataOutput::write_zint)
    fn read_zint(&mut self) -> Result<i32, DataIOError> {
        Ok(BitUtil::zig_zag_decode_i32(self.read_vint()? as u32))
    }
    /// # See
    /// [`DataInput::default_read_long`](DataInput::default_read_long)
    fn read_long(&mut self) -> Result<i64, DataIOError> {
        self.default_read_long()
    }
    /// Reads eight bytes and returns a `long` (little-endian byte order).
    ///
    /// # See Also
    /// [`DataOutput::write_long`](crate::store::data_output::DataOutput::write_long)
    /// [`BitUtil::get_i64_le`](BitUtil::get_i64_le)
    fn default_read_long(&mut self) -> Result<i64, DataIOError> {
        let b1 = self.read_int()? as u64 & 0xFFFFFFFF;
        let b2 = (self.read_int()? as u64) << 32;
        Ok((b2 | b1) as i64)
    }
    /// Reads a specified number of `long` values.
    ///
    /// # Note
    /// This is an experimental API.
    fn read_longs(
        &mut self,
        dst: &mut [i64],
        offset: usize,
        len: usize,
    ) -> Result<(), DataIOError> {
        let mut i = 0;
        while i < len {
            dst[i + offset] = self.read_long()?;
            i += 1;
        }
        Ok(())
    }
    /// Reads a specified number of `int` values into an array at the specified offset.
    ///
    /// # Arguments
    /// * `dst` - The array to read values into.
    /// * `offset` - The offset in the array to start storing `int` values.
    /// * `length` - The number of `int` values to read.
    fn read_ints(&mut self, dst: &mut [i32], offset: usize, len: usize) -> Result<(), DataIOError> {
        let mut i = 0;
        while i < len {
            dst[i + offset] = self.read_int()?;
            i += 1;
        }
        Ok(())
    }

    /// Reads a specified number of `float` values into an array at the specified offset.
    ///
    /// # Arguments
    /// * `floats` - The array to read values into.
    /// * `offset` - The offset in the array to start storing `float` values.
    /// * `len` - The number of `float` values to read.
    fn read_floats(
        &mut self,
        dst: &mut [f32],
        offset: usize,
        len: usize,
    ) -> Result<(), DataIOError> {
        let mut i = 0;
        while i < len {
            dst[i + offset] = f32::from_bits(self.read_int()? as u32);
            i += 1;
        }
        Ok(())
    }

    /// Reads a `long` stored in a variable-length format. Reads between one and nine bytes, 
    /// with smaller values taking fewer bytes. Negative numbers are not supported.
    ///
    /// # Format
    /// The format is described further in [`DataOutput::write_vint`](crate::store::data_output::DataOutput::write_vint).
    ///
    /// # See Also
    /// [`DataOutput::write_vlong`](crate::store::data_output::DataOutput::write_vlong)
    fn read_vlong(&mut self) -> Result<i64, DataIOError> {
        let mut b = self.read_byte()? as i64;
        let mut i = b & 0x7F;
        let mut shift = 7;
        while (b & 0x80) != 0 {
            b = self.read_byte()? as i64;
            i |= (b & 0x7F) << shift;
            shift += 7;
        }
        Ok(i)
    }
    /// Reads a [`zig-zag`](BitUtil::zig_zag_decode_i64)-encoded 
    /// [`read_vlong`](#method.read_vlong) variable-length integer. Reads between one and ten bytes.
    ///
    /// # See Also
    /// [`DataOutput::write_zlong`](crate::store::data_output::DataOutput::write_zlong)
    fn read_zlong(&mut self) -> Result<i64, DataIOError> {
        Ok(BitUtil::zig_zag_decode_i64(self.read_vlong()? as u64))
    }
    /// Reads a string.
    ///
    /// # See Also
    /// [`DataOutput::write_string`](crate::store::data_output::DataOutput::write_string)
    fn read_string(&mut self) -> Result<String, DataIOError> {
        let length = self.read_vint()?;
        debug_assert!(length >= 0, "Length must be positive: {}", length);
        let mut bytes = vec![0u8; length as usize];
        self.read_bytes(&mut bytes, 0, length as usize)?;
        Ok(String::from_utf8(bytes)?)
    }

    /// Reads a `HashMap<String, String>` previously written with 
    /// [`DataOutput::write_map_of_strings`](crate::store::data_output::DataOutput::write_map_of_strings).
    ///
    /// # Returns
    /// An immutable map containing the written contents.
    fn read_map_of_strings(&mut self) -> Result<HashMap<String, String>, DataIOError> {
        let count = self.read_vint()?;

        if count == 0 {
            Ok(HashMap::new())
        } else if count == 1 {
            let mut map = HashMap::new();
            map.insert(self.read_string()?, self.read_string()?);
            return Ok(map);
        } else {
            let mut map: HashMap<String, String> = HashMap::with_capacity(count as usize);
            for _ in 0..count {
                map.insert(self.read_string()?, self.read_string()?);
            }
            Ok(map)
        }
    }
    /// Reads a `HashSet<String>` previously written with 
    /// [`DataOutput::write_set_of_strings`](crate::store::data_output::DataOutput::write_set_of_strings).
    ///
    /// # Returns
    /// An immutable set containing the written contents.
    fn read_set_of_strings(&mut self) -> Result<HashSet<String>, DataIOError> {
        let count = self.read_vint()?;
        if count == 0 {
            Ok(HashSet::new())
        } else if count == 1 {
            let mut set = HashSet::new();
            set.insert(self.read_string()?);
            Ok(set)
        } else {
            let mut set = HashSet::with_capacity(count as usize);
            for _ in 0..count {
                set.insert(self.read_string()?);
            }
            Ok(set)
        }
    }
    /// Skips over `num_bytes` bytes. This method may skip bytes in whatever way is most optimal, 
    /// and may not behave the same as reading the skipped bytes.
    fn skip_bytes(&mut self, num_bytes: u64) -> Result<(), DataIOError>;
}
