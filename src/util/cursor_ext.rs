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

use crate::util::error::lucene_error::{LuceneError, Result};
pub trait ReadableCursorExt {
    /// Returns the remaining bytes in the buffer from the current position.
    fn remain(&self) -> u64;

    /// Returns the remaining bytes between a specified position and a limit.
    ///
    /// # Arguments
    /// * `position` - The current position in the buffer.
    /// * `limit` - The effective limit up to which remaining bytes are
    ///   calculated.
    fn remain_between(&self, position: u64, limit: u64) -> u64;

    /// Reads data from the cursor's buffer to the destination slice, starting
    /// at the current position.
    fn read_to(&mut self, dest: &mut [u8], offset: i32, len: i32) -> Result<()>;

    /// Reads data from a specific position in the cursor into the destination
    /// buffer.
    fn read_to_buffer(
        &self,
        dest: &mut [u8],
        offset: usize,
        position: u64,
        len: usize,
    ) -> Result<()>;
}

pub trait WritableCursorExt: ReadableCursorExt {
    /// Writes the entire slice of data into the cursor's buffer.
    fn write_from_slice(&mut self, src: &[u8]) -> Result<()>;

    /// Writes data from the source slice into the cursor's buffer, starting
    /// from the given offset.
    fn write_from(&mut self, src: &[u8], offset: i32, len: i32) -> Result<()>;
}
impl<T> ReadableCursorExt for Cursor<T>
where
    T: AsRef<[u8]>,
{
    fn remain(&self) -> u64 {
        self.remain_between(self.position(), self.get_ref().as_ref().len() as u64)
    }

    fn remain_between(&self, position: u64, limit: u64) -> u64 {
        if limit == 0 {
            return 0;
        }
        debug_assert!(
            position <= limit,
            "Position ({}) exceeds specified limit ({})",
            position,
            limit
        );
        limit.saturating_sub(position)
    }

    fn read_to(&mut self, dest: &mut [u8], offset: i32, len: i32) -> Result<()> {
        let position = self.position();
        perform_read(
            self.get_ref().as_ref(),
            dest,
            offset as usize,
            position,
            len as usize,
        )?;
        self.set_position(position + len as u64);
        Ok(())
    }

    fn read_to_buffer(
        &self,
        dest: &mut [u8],
        offset: usize,
        position: u64,
        len: usize,
    ) -> Result<()> {
        perform_read(self.get_ref().as_ref(), dest, offset, position, len)
    }
}

impl<T> WritableCursorExt for Cursor<T>
where
    T: AsRef<[u8]> + AsMut<[u8]>,
{
    fn write_from_slice(&mut self, src: &[u8]) -> Result<()> {
        let position = self.position() as usize;
        let len = src.len();

        if position + len > self.get_ref().as_ref().len() {
            return Err(LuceneError::illegal_argument(format!(
                "Buffer out of bounds: position={}, len={}, total={}",
                position,
                len,
                self.get_ref().as_ref().len()
            )));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                src.as_ptr(),
                self.get_mut().as_mut().as_mut_ptr().add(position),
                len,
            );
        }
        self.set_position((position + len) as u64);
        Ok(())
    }

    fn write_from(&mut self, src: &[u8], offset: i32, len: i32) -> Result<()> {
        let src_slice = &src[offset as usize..(offset + len) as usize];
        self.write_from_slice(src_slice)
    }
}
fn perform_read(
    source: &[u8],
    dest: &mut [u8],
    offset: usize,
    position: u64,
    len: usize,
) -> Result<()> {
    let total = source.len() as u64;

    if position > total || position + len as u64 > total {
        return Err(LuceneError::illegal_argument(format!(
            "Read out of bounds: position={}, len={}, total={}",
            position, len, total
        )));
    }
    if offset + len > dest.len() {
        return Err(LuceneError::illegal_argument(format!(
            "Destination buffer out of bounds: offset={}, len={}, total={}",
            offset,
            len,
            dest.len()
        )));
    }

    unsafe {
        std::ptr::copy_nonoverlapping(
            source.as_ptr().add(position as usize),
            dest.as_mut_ptr().add(offset),
            len,
        );
    }

    Ok(())
}
