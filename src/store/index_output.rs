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
use crate::store::data_output::DataOutput;
use crate::util::error::lucene_error::LuceneError;
use std::fmt::Display;

/// A `DataOutput` for appending data to a file in a `Directory`.
///
/// # Note
/// Instances of this class are **not** thread-safe.
///
/// # See Also
/// [`Directory`](crate::store::directory::Directory)
///
/// [`IndexInput`](crate::store::index_input::IndexInput)
pub trait IndexOutput: DataOutput + Display {
    /// Returns the current position in this file, where the next write will occur.
    fn get_file_pointer(&self) -> i64;
    /// Returns the current checksum of bytes written so far.
    fn get_checksum(&mut self) -> u64;
    /// Returns the name used to create this `IndexOutput`. This is especially useful when using
    /// [`Directory::create_temp_output`](crate::store::directory::Directory::create_temp_output).
    fn get_name(&self) -> &str;
    /// Aligns the current file pointer to multiples of `alignment_bytes` bytes to improve reads
    /// with mmap. This will write between 0 and `(alignment_bytes - 1)` zero bytes using
    /// [`write_byte`](DataOutput::write_byte).
    ///
    /// # Arguments
    /// * `alignment_bytes` - The alignment to which it should forward the file pointer (must be a power of 2).
    ///
    /// # Returns
    /// The new file pointer after alignment.
    ///
    /// # See Also
    /// [`align_offset`]
    fn align_file_pointer(&mut self, alignment_bytes: i32) -> Result<i64, LuceneError> {
        let offset = self.get_file_pointer();
        let aligned_offset = align_offset(offset, alignment_bytes)?;
        let count = (aligned_offset - offset) as usize;
        for _ in 0..count {
            self.write_byte(0)?;
        }
        Ok(aligned_offset)
    }
}
/// Aligns the given `offset` to multiples of `alignment_bytes` bytes by rounding up.
/// The alignment must be a power of 2.
///
/// # Arguments
/// * `offset` - The offset to be aligned.
/// * `alignment_bytes` - The alignment to which it should be rounded (must be a power of 2).
pub fn align_offset(offset: i64, alignment_bytes: i32) -> Result<i64, LuceneError> {
    if alignment_bytes == 0 || alignment_bytes.count_ones() != 1 {
        return Err(LuceneError::illegal_argument(
            "Alignment must be a power of 2",
        ));
    }
    Ok((offset + alignment_bytes as i64 - 1) & !(alignment_bytes as i64 - 1))
}
