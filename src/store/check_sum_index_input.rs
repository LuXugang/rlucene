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
use crate::store::index_input::IndexInput;
use crate::util::error::lucene_error::{LuceneError, Result};

const SKIP_BUFFER_SIZE: i32 = 1024;
/// An extension of [`IndexInput`] that computes a checksum as it reads data.
/// Callers can retrieve the checksum using the `get_checksum` method from the
/// implemented trait.
pub trait ChecksumIndexInput: IndexInput {
    /// Returns the current checksum value.
    fn get_checksum(&mut self) -> i64;
    /// Inherits documentation from the parent implementation.
    ///
    /// # Note
    /// [`ChecksumIndexInput`] can only seek forward, and seeks are expensive
    /// because they require reading the bytes between the current position
    /// and the target position to update the checksum.
    fn seek(&mut self, pos: i64) -> Result<()> {
        let cur_fp = self.get_file_pointer();
        if pos < cur_fp {
            return Err(LuceneError::illegal_state(format!(
                "cannot seek backwards (pos= {pos}  getFilePointer()= {cur_fp})"
            )));
        }
        self.skip_by_reading(pos - cur_fp)
    }
    /// Skips over `num_bytes` bytes.
    /// The behavior of this method is equivalent to reading the same number of
    /// bytes into a buffer and discarding its content.
    fn skip_by_reading(&mut self, num_bytes: i64) -> Result<()> {
        let mut skip_buffer = [0u8; SKIP_BUFFER_SIZE as usize];
        let mut skipped = 0;
        while skipped < num_bytes {
            debug_assert!((num_bytes - skipped) <= i32::MAX as i64);
            let step = SKIP_BUFFER_SIZE.min((num_bytes - skipped) as i32);
            self.read_bytes_with_buffer(&mut skip_buffer, 0, step, false)?;
            skipped += step as i64;
        }
        Ok(())
    }
}
