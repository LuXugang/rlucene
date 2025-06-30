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
use crate::util::error::lucene_error::Result;

/// Random Access Index API. Unlike [`IndexInput`](crate::store::IndexInput),
/// this has no concept of file position; all reads are absolute. However, like
/// `IndexInput`, it is only intended for use by a single thread.
pub trait RandomAccessInput {
    /// The number of bytes in the file.
    fn length(&self) -> i64;
    /// Reads a byte at the given position in the file
    fn read_byte(&mut self, pos: i64) -> Result<u8>;
    /// Reads a specified number of bytes starting at a given position into an
    /// array at the specified offset.
    fn read_bytes(&mut self, pos: i64, buf: &mut [u8], offset: i32, len: i32) -> Result<()> {
        for i in 0..len {
            buf[(offset + i) as usize] = self.read_byte(pos + i as i64)?;
        }
        Ok(())
    }
    ///  Reads an i16 (LE byte order) at the given position in the file.
    fn read_short(&mut self, pos: i64) -> Result<i16>;
    /// Reads an i32 (LE byte order) at the given position in the file.
    fn read_int(&mut self, pos: i64) -> Result<i32>;
    /// Reads a long (LE byte order) at the given position in the file.
    fn read_long(&mut self, pos: i64) -> Result<i64>;
    ///  Prefetch data in the background.
    fn prefetch(&mut self, pos: i64, len: i64) -> Result<()>;
}
