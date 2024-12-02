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

/**
 * Abstract base class for performing read operations of Lucene's low-level data types.
 *
 * `DataInput` may only be used from one thread, because it is not thread safe (it keeps
 * internal state like file position). To allow multithreaded use, every `DataInput` instance
 * must be cloned before used in another thread. Subclasses must therefore implement
 * `#clone()`, returning a new `DataInput` which operates on the same underlying resource, but
 * positioned independently.
*/
pub trait DataInput: Sized + Display {
    /**
     * Reads a specified number of bytes into an array at the specified offset.
     */
    fn read_byte(&mut self) -> Result<u8, DataIOError>;
    /**
     * Reads a specified number of bytes into an array at the specified offset.
     */
    fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<(), DataIOError>;
    /**
     * Reads a specified number of bytes into an array at the specified offset with control over
     * whether the read should be buffered (callers who have their own buffer should pass in "false"
     * for useBuffer). Currently only `BufferedIndexInput` respects this parameter.
     *
     */
    fn read_bytes_with_buffer(
        &mut self,
        b: &mut [u8],
        offset: usize,
        len: usize,
        _use_buffer: bool,
    ) -> Result<(), DataIOError> {
        self.read_bytes(b, offset, len)
    }
    /**
     * Reads two bytes and returns a i16 (LE byte order).
     */
    fn read_short(&mut self) -> Result<i16, DataIOError> {
        self.default_read_short()
    }
    fn default_read_short(&mut self) -> Result<i16, DataIOError> {
        let b1 = self.read_byte()?;
        let b2 = self.read_byte()?;
        Ok(i16::from_le_bytes([b2, b1]))
    }
    /**
     * Reads four bytes and returns an int (LE byte order).
     */
    fn read_int(&mut self) -> Result<i32, DataIOError> {
        self.default_read_int()
    }
    fn default_read_int(&mut self) -> Result<i32, DataIOError> {
        let b1 = self.read_byte()?;
        let b2 = self.read_byte()?;
        let b3 = self.read_byte()?;
        let b4 = self.read_byte()?;
        Ok(i32::from_le_bytes([b4, b3, b2, b1]))
    }
    /**
     * Override if you have an efficient implementation. In general this is when the input supports
     * random access.
     */
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
    /**
     * Reads an int stored in variable-length format. Reads between one and five bytes. Smaller values
     * take fewer bytes. Negative numbers are supported, but should be avoided.
     */
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
    /**
     * Read a `BitUtil#zig_Zag_Decode_i32(vint)` zig-zag encoded `#readVInt()` variable-length
     * i32.
     */
    fn read_zint(&mut self) -> Result<i32, DataIOError> {
        Ok(BitUtil::zig_zag_decode_i32(self.read_vint()? as u32))
    }

    fn read_long(&mut self) -> Result<i64, DataIOError> {
        self.default_read_long()
    }
    fn default_read_long(&mut self) -> Result<i64, DataIOError> {
        let b1 = self.read_int()? as u64 & 0xFFFFFFFF;
        let b2 = (self.read_int()? as u64) << 32;
        Ok((b2 | b1) as i64)
    }
    /**
     * Read a specified number of longs.
     */
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
    /**
     * Reads a specified number of ints into an array at the specified offset.
     */
    fn read_ints(&mut self, dst: &mut [i32], offset: usize, len: usize) -> Result<(), DataIOError> {
        let mut i = 0;
        while i < len {
            dst[i + offset] = self.read_int()?;
            i += 1;
        }
        Ok(())
    }

    /**
     * Reads a specified number of floats into an array at the specified offset.
     *
     */
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

    /**
     * Reads a long stored in variable-length format. Reads between one and nine bytes. Smaller values
     * take fewer bytes. Negative numbers are not supported.
     *
     * The format is described further in `DataOutput#writeVInt(int)`.
     */
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

    /**
     * Read a `BitUtil#zig_Zag_Decode_i64(vlong)` zig-zag-encoded `#readVLong()` variable-length}
     * i32. Reads between one and ten bytes.
     */
    fn read_zlong(&mut self) -> Result<i64, DataIOError> {
        Ok(BitUtil::zig_zag_decode_i64(self.read_vlong()? as u64))
    }
    /**
     * Reads a string.
     */
    fn read_string(&mut self) -> Result<String, DataIOError> {
        let length = self.read_vint()?;
        debug_assert!(length >= 0, "Length must be positive: {}", length);
        let mut bytes = vec![0u8; length as usize];
        self.read_bytes(&mut bytes, 0, length as usize)?;
        Ok(String::from_utf8(bytes)?)
    }

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
    fn skip_bytes(&mut self, num_bytes: u64) -> Result<(), DataIOError>;
}
