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
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

use crate::core::store::DataInput;
use crate::core::store::data_output::DataOutput;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// A `DataOutput` for appending data to a file in a `Directory`.
///
/// # Note
/// Instances of this struct are **not** thread-safe.
///
/// # See Also
/// [`Directory`](crate::core::store::directory::Directory)
///
/// [`IndexInput`](crate::core::store::index_input::IndexInput)
pub trait IndexOutput: DataOutput + Display {
    /// Returns the current position in this file, where the next write will
    /// occur.
    fn get_file_pointer(&self) -> i64;
    /// Returns the current checksum of bytes written so far.
    fn get_checksum(&mut self) -> u64;
    /// Returns the name used to create this `IndexOutput`. This is especially
    /// useful when using
    /// [`Directory::create_temp_output`](crate::core::store::directory::Directory::create_temp_output).
    fn get_name(&self) -> &str;
    /// Aligns the current file pointer to multiples of `alignment_bytes` bytes
    /// to improve reads with mmap. This will write between 0 and
    /// `(alignment_bytes - 1)` zero bytes using
    /// [`write_byte`](DataOutput::write_byte).
    ///
    /// # Arguments
    /// * `alignment_bytes` - The alignment to which it should forward the file
    ///   pointer (must be a power of 2).
    ///
    /// # Returns
    /// The new file pointer after alignment.
    ///
    /// # See Also
    /// [`align_offset`]
    fn align_file_pointer(&mut self, alignment_bytes: i32) -> Result<i64> {
        let offset = self.get_file_pointer();
        let aligned_offset = align_offset(offset, alignment_bytes)?;
        let count = (aligned_offset - offset) as usize;
        for _ in 0..count {
            self.write_byte(0)?;
        }
        Ok(aligned_offset)
    }
}
/// Aligns the given `offset` to multiples of `alignment_bytes` bytes by
/// rounding up. The alignment must be a power of 2.
///
/// # Arguments
/// * `offset` - The offset to be aligned.
/// * `alignment_bytes` - The alignment to which it should be rounded (must be a
///   power of 2).
pub fn align_offset(offset: i64, alignment_bytes: i32) -> Result<i64> {
    if alignment_bytes == 0 || alignment_bytes.count_ones() != 1 {
        return Err(LuceneError::illegal_argument(
            "Alignment must be a power of 2",
        ));
    }
    Ok((offset + alignment_bytes as i64 - 1) & !(alignment_bytes as i64 - 1))
}

pub enum Either2IndexOutput<A, B> {
    A(A),
    B(B),
}

impl<A, B> DataOutput for Either2IndexOutput<A, B>
where
    A: IndexOutput,
    B: IndexOutput,
{
    fn write_byte(&mut self, b: u8) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_byte(b),
            Either2IndexOutput::B(s) => s.write_byte(b),
        }
    }

    fn write_bytes_with_len(&mut self, b: &[u8], len: i32) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_bytes_with_len(b, len),
            Either2IndexOutput::B(s) => s.write_bytes_with_len(b, len),
        }
    }

    fn write_bytes_range(&mut self, b: &[u8], offset: i32, length: i32) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_bytes_range(b, offset, length),
            Either2IndexOutput::B(s) => s.write_bytes_range(b, offset, length),
        }
    }

    fn write_int(&mut self, i: i32) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_int(i),
            Either2IndexOutput::B(s) => s.write_int(i),
        }
    }

    fn write_short(&mut self, i: i16) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_short(i),
            Either2IndexOutput::B(s) => s.write_short(i),
        }
    }

    fn write_vint(&mut self, i: i32) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_vint(i),
            Either2IndexOutput::B(s) => s.write_vint(i),
        }
    }

    fn write_zint(&mut self, i: i32) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_zint(i),
            Either2IndexOutput::B(s) => s.write_zint(i),
        }
    }

    fn write_long(&mut self, i: i64) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_long(i),
            Either2IndexOutput::B(s) => s.write_long(i),
        }
    }

    fn write_vlong(&mut self, i: i64) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_vlong(i),
            Either2IndexOutput::B(s) => s.write_vlong(i),
        }
    }

    fn write_signed_vlong(&mut self, i: i64) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_signed_vlong(i),
            Either2IndexOutput::B(s) => s.write_signed_vlong(i),
        }
    }

    fn write_zlong(&mut self, i: i64) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_zlong(i),
            Either2IndexOutput::B(s) => s.write_zlong(i),
        }
    }

    fn write_string(&mut self, s: &str) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_string(s),
            Either2IndexOutput::B(s1) => s1.write_string(s),
        }
    }

    fn copy_bytes(&mut self, input: &mut impl DataInput, num_bytes: i64) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.copy_bytes(input, num_bytes),
            Either2IndexOutput::B(s) => s.copy_bytes(input, num_bytes),
        }
    }

    fn write_map_of_strings(&mut self, map: &HashMap<String, String>) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_map_of_strings(map),
            Either2IndexOutput::B(s) => s.write_map_of_strings(map),
        }
    }

    fn write_set_of_strings(&mut self, set: &HashSet<String>) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_set_of_strings(set),
            Either2IndexOutput::B(s) => s.write_set_of_strings(set),
        }
    }

    fn write_group_vints_i64(&mut self, values: &mut [i64], limit: i32) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_group_vints_i64(values, limit),
            Either2IndexOutput::B(s) => s.write_group_vints_i64(values, limit),
        }
    }

    fn write_group_vints_i32(&mut self, values: &mut [i32], limit: i32) -> Result<()> {
        match self {
            Either2IndexOutput::A(f) => f.write_group_vints_i32(values, limit),
            Either2IndexOutput::B(s) => s.write_group_vints_i32(values, limit),
        }
    }
}

impl<A, B> Display for Either2IndexOutput<A, B>
where
    A: IndexOutput,
    B: IndexOutput,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Either2IndexOutput::A(t) => t.fmt(f),
            Either2IndexOutput::B(s) => s.fmt(f),
        }
    }
}

impl<A, B> IndexOutput for Either2IndexOutput<A, B>
where
    A: IndexOutput,
    B: IndexOutput,
{
    fn get_file_pointer(&self) -> i64 {
        match self {
            Either2IndexOutput::A(t) => t.get_file_pointer(),
            Either2IndexOutput::B(s) => s.get_file_pointer(),
        }
    }

    fn get_checksum(&mut self) -> u64 {
        match self {
            Either2IndexOutput::A(t) => t.get_checksum(),
            Either2IndexOutput::B(s) => s.get_checksum(),
        }
    }

    fn get_name(&self) -> &str {
        match self {
            Either2IndexOutput::A(t) => t.get_name(),
            Either2IndexOutput::B(s) => s.get_name(),
        }
    }

    fn align_file_pointer(&mut self, alignment_bytes: i32) -> Result<i64> {
        match self {
            Either2IndexOutput::A(t) => t.align_file_pointer(alignment_bytes),
            Either2IndexOutput::B(s) => s.align_file_pointer(alignment_bytes),
        }
    }
}
