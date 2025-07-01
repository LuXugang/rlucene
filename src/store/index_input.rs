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
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{DataInput, ReadAdvice};
use crate::util::error::lucene_error::{LuceneError, Result};

/// Provides random-access input operations for files within a
/// [`Directory`](crate::store::directory::Directory).
///
/// `IndexInput` supports reading data from a file and maintains its own
/// internal state, such as the current file position.
///
/// # Thread Safety
///
/// `IndexInput` is **not thread-safe**. If you need to use it in multiple
/// threads, you must **clone** the `IndexInput` instance. Each clone operates
/// on the same underlying resource but maintains an independent position.
///
///
/// # See Also
/// - [`Directory`](crate::store::directory::Directory) for file-based
///   operations.
pub trait IndexInput: DataInput + crate::util::clone::TryClone {
    /// Returns the current position in this file, where the next read will
    /// occur.
    ///
    /// # See Also
    /// [`seek`](IndexInput::seek)
    fn get_file_pointer(&self) -> i64;

    /// Sets the current position in this file, where the next read will occur.
    /// If this position is beyond the end of the file, it will return an
    /// `EOFError`, and the stream will be in an undetermined state.
    ///
    /// # See Also
    /// [`get_file_pointer`](IndexInput::get_file_pointer)
    fn seek(&mut self, pos: i64) -> Result<()>;
    /// Inherits documentation from the parent implementation.
    ///
    /// # Behavior
    /// This is functionally equivalent to seeking to `get_file_pointer() +
    /// num_bytes`.
    ///
    /// # See Also
    /// [`get_file_pointer`](IndexInput::get_file_pointer)
    ///
    /// [`seek`](IndexInput::seek)
    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        if num_bytes < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "num_bytes must be >= 0, got {num_bytes}"
            )));
        }
        let skip_to = self.get_file_pointer() + num_bytes;
        self.seek(skip_to)?;
        Ok(())
    }
    /// The number of bytes in the file.
    fn length(&self) -> i64;

    /// Creates a slice of this index input, with the given description, offset,
    /// and length. The slice is positioned at the beginning.
    type Slice: IndexInput;
    fn slice(&self, slice_description: &str, offset: i64, length: i64) -> Result<Self::Slice>;
    /// Creates a slice with a specific [`ReadAdvice`]. This is typically used
    /// by [`CompoundFormat`](crate::codecs::compound_format)
    /// implementations to honor the [`ReadAdvice`] of each file within the
    /// compound file.
    ///
    /// # Note
    /// It is only legal to call this method if this `IndexInput` has been
    /// opened with `ReadAdvice::NORMAL`. However, this method accepts any
    /// `ReadAdvice` value except `None` for the slice.
    ///
    /// The default implementation delegates to [`slice`](IndexInput::slice) and
    /// ignores the `ReadAdvice`.
    fn slice_with_read_advice(
        &self,
        description: &str,
        offset: i64,
        length: i64,
        read_advice: &ReadAdvice,
    ) -> Result<Self::Slice> {
        self.default_slice_with_read_advice(description, offset, length, read_advice)
    }
    fn default_slice_with_read_advice(
        &self,
        description: &str,
        offset: i64,
        length: i64,
        _read_advice: &ReadAdvice,
    ) -> Result<Self::Slice> {
        self.slice(description, offset, length)
    }
    type RandomAccessSlice: RandomAccessInput;
    /// Creates a random-access slice of this index input, with the given offset
    /// and length.
    ///
    /// # Note
    /// The default implementation calls [`slice`](IndexInput::slice), and it
    /// doesn't support random access. It implements absolute reads as
    /// seek+read.
    fn random_access_slice(&self, offset: i64, length: i64) -> Result<Self::RandomAccessSlice>;

    /// Optional method: Gives a hint to this input that some bytes will be read
    /// soon. `IndexInput` implementations may take advantage of this hint
    /// to start fetching pages of data immediately from storage.
    ///
    /// # Arguments
    /// * `offset` - The starting offset.
    /// * `length` - The number of bytes to prefetch.
    ///
    /// # Note
    /// The default implementation is a no-op.
    fn prefetch(&mut self, pos: i64, len: i64) -> Result<()> {
        self.default_prefetch(pos, len)
    }

    fn default_prefetch(&mut self, _pos: i64, _len: i64) -> Result<()> {
        Ok(())
    }
}
/// SubStruct call this to get the String for resourceDescription of a slice of
/// this `IndexInput`.
pub fn get_full_slice_description(slice_description: &str) -> String {
    format!(" [slice={slice_description}] ")
}
