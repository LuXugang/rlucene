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
use crate::index::BytesRef;
use crate::store::DataInput;
use crate::util::error::lucene_error::Result;

/// A decompressor.
pub trait Decompressor: crate::util::clone::TryClone {
    /// Decompress bytes that were stored between offsets `offset` and `offset +
    /// length` in the original stream from the compressed stream `in` to
    /// `bytes`. After returning, the length of `bytes` must be equal to
    /// `length`. Implementations of this method are free to resize `bytes`
    /// depending on their needs.
    ///
    /// # Parameters
    /// - `in`: The input that stores the compressed stream.
    /// - `original_length`: The length of the original data (before
    ///   compression).
    /// - `offset`: Bytes before this offset do not need to be decompressed.
    /// - `length`: Bytes after `offset + length` do not need to be
    ///   decompressed.
    /// - `bytes`: A reference to a `BytesRef` where to store the decompressed
    ///   data.
    fn decompress(
        &mut self,
        input: &mut impl DataInput,
        original_length: i32,
        offset: i32,
        length: i32,
        bytes: &mut BytesRef<Vec<u8>>,
    ) -> Result<()>;
}
