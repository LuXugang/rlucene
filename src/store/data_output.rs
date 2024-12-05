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
use crate::index::BytesRef;
use crate::store::data_input::DataInput;
use crate::util::bit_util::BitUtil;
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::group_vint_util::{GroupVIntUtil, MAX_LENGTH_PER_GROUP};
use std::collections::{HashMap, HashSet};

/**
 * Abstract base class for performing write operations of Lucene's low-level data types.
 *
 * `DataOutput` may only be used from one thread, because it is not thread safe (it keeps
 * internal state like file position).
*/
pub trait DataOutput: Sized {
    /**
     * Writes a single byte.
     *
     * The most primitive data type is an eight-bit byte. Files are accessed as sequences of bytes.
     * All other data types are defined as sequences of bytes, so file formats are byte-order
     * independent.
     */
    fn write_byte(&mut self, b: u8) -> Result<(), DataIOError>;

    /**
     * Writes an array of bytes.
     */
    fn write_bytes_with_len(&mut self, b: &[u8], len: usize) -> Result<(), DataIOError> {
        self.write_bytes_range(b, 0, len)
    }
    /**
     * Writes an array of bytes.
     *
     */
    fn write_bytes_range(
        &mut self,
        b: &[u8],
        offset: usize,
        length: usize,
    ) -> Result<(), DataIOError>;

    /**
     * Writes an int as four bytes (LE byte order).
     */
    fn write_int(&mut self, i: i32) -> Result<(), DataIOError> {
        self.write_byte(i as u8)?;
        self.write_byte((i >> 8) as u8)?;
        self.write_byte((i >> 16) as u8)?;
        self.write_byte((i >> 24) as u8)?;
        Ok(())
    }

    /**
     * Writes a i16 as two bytes (LE byte order).
     */
    fn write_short(&mut self, i: i16) -> Result<(), DataIOError> {
        self.write_byte(i as u8)?;
        self.write_byte((i >> 8) as u8)?;
        Ok(())
    }

    /**
     * Writes an int in a variable-length format. Writes between one and five bytes. Smaller values
     * take fewer bytes. Negative numbers are supported, but should be avoided.
     *
     * VByte is a variable-length format for positive i32s is defined where the high-order bit
     * of each byte indicates whether more bytes remain to be read. The low-order seven bits are
     * appended as increasingly more significant bits in the resulting i32 value. Thus values from
     * zero to 127 may be stored in a single byte, values from 128 to 16,383 may be stored in two
     * bytes, and so on.
     *
     * VByte Encoding Example
     *
     * <table class="padding2" style="border-spacing: 0px; border-collapse: separate; border: 0">
     * <caption>variable length encoding examples</caption>
     * <tr style="vertical-align: top">
     *   <th style="text-align:left">Value</th>
     *   <th style="text-align:left">Byte 1</th>
     *   <th style="text-align:left">Byte 2</th>
     *   <th style="text-align:left">Byte 3</th>
     * </tr>
     * <tr style="vertical-align: bottom">
     *   <td>0</td>
     *   <td><code>00000000</code></td>
     *   <td></td>
     *   <td></td>
     * </tr>
     * <tr style="vertical-align: bottom">
     *   <td>1</td>
     *   <td><code>00000001</code></td>
     *   <td></td>
     *   <td></td>
     * </tr>
     * <tr style="vertical-align: bottom">
     *   <td>2</td>
     *   <td><code>00000010</code></td>
     *   <td></td>
     *   <td></td>
     * </tr>
     * <tr>
     *   <td style="vertical-align: top">...</td>
     *   <td></td>
     *   <td></td>
     *   <td></td>
     * </tr>
     * <tr style="vertical-align: bottom">
     *   <td>127</td>
     *   <td><code>01111111</code></td>
     *   <td></td>
     *   <td></td>
     * </tr>
     * <tr style="vertical-align: bottom">
     *   <td>128</td>
     *   <td><code>10000000</code></td>
     *   <td><code>00000001</code></td>
     *   <td></td>
     * </tr>
     * <tr style="vertical-align: bottom">
     *   <td>129</td>
     *   <td><code>10000001</code></td>
     *   <td><code>00000001</code></td>
     *   <td></td>
     * </tr>
     * <tr style="vertical-align: bottom">
     *   <td>130</td>
     *   <td><code>10000010</code></td>
     *   <td><code>00000001</code></td>
     *   <td></td>
     * </tr>
     * <tr>
     *   <td style="vertical-align: top">...</td>
     *   <td></td>
     *   <td></td>
     *   <td></td>
     * </tr>
     * <tr style="vertical-align: bottom">
     *   <td>16,383</td>
     *   <td><code>11111111</code></td>
     *   <td><code>01111111</code></td>
     *   <td></td>
     * </tr>
     * <tr style="vertical-align: bottom">
     *   <td>16,384</td>
     *   <td><code>10000000</code></td>
     *   <td><code>10000000</code></td>
     *   <td><code>00000001</code></td>
     * </tr>
     * <tr style="vertical-align: bottom">
     *   <td>16,385</td>
     *   <td><code>10000001</code></td>
     *   <td><code>10000000</code></td>
     *   <td><code>00000001</code></td>
     * </tr>
     * <tr>
     *   <td style="vertical-align: top">...</td>
     *   <td ></td>
     *   <td ></td>
     *   <td ></td>
     * </tr>
     * </table>
     *
     * <p>This provides compression while still being efficient to decode.
     *
     * @param i Smaller values take fewer bytes. Negative numbers are supported, but should be
     *     avoided.
     * @throws IOException If there is an I/O error writing to the underlying medium.
     * @see DataInput#readVInt()
     */
    fn write_vint(&mut self, i: i32) -> Result<(), DataIOError> {
        let mut i = i as u32;
        while (i & !0x7F) != 0 {
            self.write_byte(((i & 0x7F) | 0x80) as u8)?;
            i >>= 7;
        }
        self.write_byte(i as u8)?;
        Ok(())
    }

    /**
     * Write a `BitUtil#zig_zag_encode_i32(i32)` zig-zag-encoded `#writeVInt(i32)`
     * variable-length i32. This is typically useful to write small signed ints and is equivalent
     * to calling `writeVInt(BitUtil.zig_zag_encode_i32(i))`
     */
    fn write_zint(&mut self, i: i32) -> Result<(), DataIOError> {
        self.write_vint(BitUtil::zig_zag_encode_i32(i))
    }

    /**
     * Writes a long as eight bytes (LE byte order).
     */
    fn write_long(&mut self, i: i64) -> Result<(), DataIOError> {
        self.write_int(i as i32)?;
        self.write_int((i >> 32) as i32)?;
        Ok(())
    }
    // write a potentially negative vLong
    fn write_vlong(&mut self, i: i64) -> Result<(), DataIOError> {
        if i < 0 {
            return Err(DataIOError::illegal_argument(
                "cannot write negative vLong (got: ".to_string() + &i.to_string() + ")",
            ));
        }
        self.write_signed_vlong(i)?;
        Ok(())
    }

    fn write_signed_vlong(&mut self, i: i64) -> Result<(), DataIOError> {
        let mut i = i as u64;
        while (i & !0x7F) != 0 {
            self.write_byte(((i & 0x7F) | 0x80) as u8)?;
            i >>= 7;
        }
        self.write_byte(i as u8)?;
        Ok(())
    }
    /**
     * Write a `BitUtil#zig_zag_encode_i64(i64)` encoded `#writeVLong(i64)`
     * variable-length long. Writes between one and ten bytes. This is typically useful to write
     * small signed ints.
     */
    fn write_zlong(&mut self, i: i64) -> Result<(), DataIOError> {
        self.write_signed_vlong(BitUtil::zig_zag_encode_i64(i))
    }

    /**
     * Writes a string.
     *
     * <p>Writes strings as UTF-8 encoded bytes. First the length, in bytes, is written as a {@link
     * #writeVInt VInt}, followed by the bytes.
     *
     */
    fn write_string(&mut self, s: &str) -> Result<(), DataIOError> {
        let utf8_result = BytesRef::new_from_string(s);
        let len = utf8_result.length as usize;
        let offset = utf8_result.offset as usize;
        self.write_vint(len as i32)?;
        self.write_bytes_range(&utf8_result.bytes, offset, len)
    }

    fn copy_bytes<T: DataInput>(
        &mut self,
        input: &mut T,
        num_bytes: i64,
    ) -> Result<(), DataIOError> {
        debug_assert!(num_bytes >= 0, "num_bytes = {}", num_bytes);
        let mut buffer = vec![0u8; COPY_BUFFER_SIZE];
        let mut left = num_bytes;
        while left > 0 {
            let to_copy = if left > COPY_BUFFER_SIZE as i64 {
                COPY_BUFFER_SIZE as i64
            } else {
                left
            };
            input.read_bytes(&mut buffer, 0, to_copy as usize)?;
            self.write_bytes_with_len(&buffer, to_copy as usize)?;
            left -= to_copy;
        }
        Ok(())
    }
    /**
     * Writes a String map.
     *
     * <p>First the size is written as an {@link #writeVInt(int) vInt}, followed by each key-value
     * pair written as two consecutive {@link #writeString(String) String}s.
     *
     */
    fn write_map_of_strings(&mut self, map: &HashMap<String, String>) -> Result<(), DataIOError> {
        self.write_vint(map.len() as i32)?;
        for (key, value) in map.iter() {
            self.write_string(key)?;
            self.write_string(value)?;
        }
        Ok(())
    }

    /**
     * Writes a String set.
     *
     * <p>First the size is written as an {@link #writeVInt(int) vInt}, followed by each value written
     * as a {@link #writeString(String) String}.
     */
    fn write_set_of_strings(&mut self, set: &HashSet<String>) -> Result<(), DataIOError> {
        self.write_vint(set.len() as i32)?;
        for value in set.iter() {
            self.write_string(value)?;
        }
        Ok(())
    }

    /**
     * Encode i32s using group-varint. It uses `DataOutput#writeVInt` to encode tail
     * values that are not enough for a group. we need a `vec<i64>` because this is what postings are
     * using, all longs are actually required to be i32s.
     */
    fn write_group_vints(&mut self, values: &mut [i64], limit: usize) -> Result<(), DataIOError> {
        let mut group_vint_bytes: Vec<u8> = vec![0; MAX_LENGTH_PER_GROUP];
        GroupVIntUtil::write_group_vints(self, &mut group_vint_bytes, values, limit)?;
        Ok(())
    }
}
const COPY_BUFFER_SIZE: usize = 16384;
