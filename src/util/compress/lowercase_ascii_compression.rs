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
use crate::store::{DataInput, DataOutput};
use crate::util::error::lucene_error::{LuceneError, Result};

/// Utility that efficiently compresses arrays mostly containing characters in
/// the `[0x1F, 0x3F)` or `[0x5F, 0x7F)` ranges,
/// which notably include all digits, lowercase letters, `.`, `-`, and `_`.
pub struct LowercaseAsciiCompression;
impl LowercaseAsciiCompression {
    fn is_compressible(b: i32) -> bool {
        let high3_bits = (b.wrapping_add(1)) & !0x1F;
        high3_bits == 0x20 || high3_bits == 0x60
    }
    /// Compresses `input[0..len]` into `out`.
    ///
    /// Returns `false` if the content cannot be compressed.
    /// If compression succeeds, the number of bytes written is guaranteed to be
    /// less than `len`.
    pub fn compress(
        input: &[u8],
        len: usize,
        tmp: &mut [u8],
        out: &mut impl DataOutput,
    ) -> Result<bool> {
        if len < 8 {
            return Ok(false);
        }

        // 1. Count exceptions and fail compression if there are too many of them.
        let max_exceptions = len >> 5;
        let mut previous_exception_index = 0;
        let mut num_exceptions = 0;

        for i in 0..len {
            let b = input[i] as i32;
            if !Self::is_compressible(b) {
                while i - previous_exception_index > 0xFF {
                    num_exceptions += 1;
                    previous_exception_index += 0xFF;
                }
                num_exceptions += 1;
                if num_exceptions > max_exceptions {
                    return Ok(false);
                }
                previous_exception_index = i;
            }
        }

        debug_assert!(num_exceptions <= max_exceptions);

        // 2. Move to 6-bit space
        let compressed_len = len - (len >> 2);
        debug_assert!(compressed_len < len);
        for i in 0..len {
            let b = (input[i] as i32) + 1;
            tmp[i] = ((b & 0x1F) | ((b as u32 & 0x40) >> 1) as i32) as u8;
        }

        // 3. Pack exception bits into tmp[0..compressed_len]
        let mut o = 0usize;
        for i in compressed_len..len {
            tmp[o] |= (tmp[i] & 0x30) << 2; // bits 4-5
            o += 1;
        }
        for i in compressed_len..len {
            tmp[o] |= (tmp[i] & 0x0C) << 4; // bits 2-3
            o += 1;
        }
        for i in compressed_len..len {
            tmp[o] |= (tmp[i] & 0x03) << 6; // bits 0-1
            o += 1;
        }

        debug_assert!(o <= compressed_len);
        debug_assert!(compressed_len <= i32::MAX as usize);
        out.write_bytes_range(tmp, 0, compressed_len as i32)?;

        // 4. Write exception deltas
        debug_assert!(num_exceptions <= i32::MAX as usize);
        out.write_vint(num_exceptions as i32)?;
        if num_exceptions > 0 {
            previous_exception_index = 0;
            let mut num_exceptions2 = 0;

            for i in 0..len {
                let b = input[i] as i32;
                if !Self::is_compressible(b) {
                    while i - previous_exception_index > 0xFF {
                        // We record deltas between exceptions as bytes, so we need to create
                        // "artificial" exceptions if the delta between two of them is greater
                        // than the maximum unsigned byte value.
                        out.write_byte(0xFF)?;
                        previous_exception_index += 0xFF;
                        out.write_byte(input[previous_exception_index])?;
                        num_exceptions2 += 1;
                    }

                    out.write_byte((i - previous_exception_index) as u8)?;
                    previous_exception_index = i;
                    out.write_byte(input[i])?;
                    num_exceptions2 += 1;
                }
            }

            if num_exceptions != num_exceptions2 {
                return Err(LuceneError::illegal_state(format!(
                    "{} <> {} {}",
                    num_exceptions,
                    num_exceptions2,
                    BytesRef::from_slice(input.to_vec(), 0, len),
                )));
            }
        }

        Ok(true)
    }
    /// Decompresses data that was previously compressed using
    /// [`Self::compress`].
    ///
    /// `len` must be the original (uncompressed) length, not the compressed
    /// length.
    pub fn decompress(input: &mut impl DataInput, out: &mut [u8], len: usize) -> Result<()> {
        let saved = len >> 2;
        let compressed_len = len - saved;

        // 1. Copy the packed bytes
        debug_assert!(compressed_len <= i32::MAX as usize);
        input.read_bytes(out, 0, compressed_len as i32)?;

        // 2. Restore the leading 2 bits into whole bytes
        for i in 0..saved {
            let a = (out[i] as u32 & 0xC0) >> 2;
            let b = (out[saved + i] as u32 & 0xC0) >> 4;
            let c = (out[(saved << 1) + i] as u32 & 0xC0) >> 6;
            out[compressed_len + i] = (a | b | c) as u8;
        }

        // 3. Move back to original range
        for i in 0..len {
            let b = out[i];
            out[i] = (((b as u32 & 0x1F) | 0x20 | ((b as u32 & 0x20) << 1)) - 1) as u8;
        }

        // 4. Restore exceptions
        let num_exceptions = input.read_vint()? as usize;
        let mut i = 0usize;

        for _ in 0..num_exceptions {
            i += input.read_byte()? as usize;
            out[i] = input.read_byte()?;
        }

        Ok(())
    }
}
