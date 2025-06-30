/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::fmt::Display;

use crate::store::data_output::DataOutput;
use crate::util::error::lucene_error::{LuceneError, Result};

/// A `DataOutput` for appending data to a file in a `Directory`.
///
/// # Note
/// Instances of this struct are **not** thread-safe.
///
/// # See Also
/// [`Directory`](crate::store::directory::Directory)
///
/// [`IndexInput`](crate::store::index_input::IndexInput)
pub trait IndexOutput: DataOutput + Display {
    /// Returns the current position in this file, where the next write will
    /// occur.
    fn get_file_pointer(&self) -> i64;
    /// Returns the current checksum of bytes written so far.
    fn get_checksum(&mut self) -> u64;
    /// Returns the name used to create this `IndexOutput`. This is especially
    /// useful when using
    /// [`Directory::create_temp_output`](crate::store::directory::Directory::create_temp_output).
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
