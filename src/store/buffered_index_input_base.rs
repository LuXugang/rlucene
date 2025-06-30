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
use std::io::Cursor;

use crate::store::random_access_input::RandomAccessInput;
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;

pub trait BufferedIndexInputBase: crate::util::clone::TryClone {
    /// Expert: Implements seek functionality. Sets the current position in this
    /// file, where the next call to
    /// [`read_internal`](BufferedIndexInputBase::read_internal) will occur.
    ///
    /// # See Also
    /// [`read_internal`](BufferedIndexInputBase::read_internal)
    fn seek_internal(&mut self, pos: i64) -> Result<()>;
    /// Expert: Implements buffer refill. Reads bytes from the current position
    /// in the input.
    ///
    /// # Arguments
    /// * `b` - The buffer to read bytes into.
    fn read_internal(&mut self, b: &mut Cursor<Vec<u8>>, len: i64, file_pointer: i64)
        -> Result<()>;

    /// Creates a slice of this index input, with the given description, offset,
    /// and length. The slice is positioned at the beginning.
    type Slice: IndexInput + RandomAccessInput;
    fn slice(&self, slice_description: &str, offset: i64, length: i64) -> Result<Self::Slice>;

    /// The number of bytes in the file.
    fn length(&self) -> i64;
}
