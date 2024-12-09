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
use crate::util::error::data_io_error_enum::DataIOError;
use std::io::Cursor;
pub trait ReadableCursorExt {
    /// Returns the remaining bytes in the buffer from the current position.
    fn remain(&self) -> u64;

    /// Returns the remaining bytes in the buffer from a specific position.
    fn remain_with_pos(&self, position: u64) -> u64;

    /// Reads data from the cursor's buffer to the destination slice, starting at the current position.
    fn read_to(&mut self, dest: &mut [u8], offset: u32, len: u32) -> Result<(), DataIOError>;

    /// Reads data from a specific position in the cursor into the destination buffer.
    fn read_to_buffer(
        &self,
        dest: &mut [u8],
        offset: usize,
        position: u64,
        len: usize,
    ) -> Result<(), DataIOError>;
}

pub trait WritableCursorExt: ReadableCursorExt {
    /// Writes the entire slice of data into the cursor's buffer.
    fn write_from_slice(&mut self, src: &[u8]) -> Result<(), DataIOError>;

    /// Writes data from the source slice into the cursor's buffer, starting from the given offset.
    fn write_from(&mut self, src: &[u8], offset: u32, len: u32) -> Result<(), DataIOError>;
}
impl<T> ReadableCursorExt for Cursor<T>
where
    T: AsRef<[u8]>,
{
    fn remain(&self) -> u64 {
        self.remain_with_pos(self.position())
    }

    fn remain_with_pos(&self, position: u64) -> u64 {
        let total = self.get_ref().as_ref().len() as u64;
        debug_assert!(
            position <= total,
            "Position ({}) exceeds total ({})",
            position,
            total
        );
        total.saturating_sub(position)
    }

    fn read_to(&mut self, dest: &mut [u8], offset: u32, len: u32) -> Result<(), DataIOError> {
        let position = self.position();
        perform_read(
            self.get_ref().as_ref(),
            dest,
            offset as usize,
            position,
            len as usize,
        )?;
        self.set_position((position + len as u64) as u64);
        Ok(())
    }

    fn read_to_buffer(
        &self,
        dest: &mut [u8],
        offset: usize,
        position: u64,
        len: usize,
    ) -> Result<(), DataIOError> {
        perform_read(self.get_ref().as_ref(), dest, offset, position, len)
    }
}

impl<T> WritableCursorExt for Cursor<T>
where
    T: AsRef<[u8]> + AsMut<[u8]>,
{
    fn write_from_slice(&mut self, src: &[u8]) -> Result<(), DataIOError> {
        let position = self.position() as usize;
        let len = src.len();

        if position + len > self.get_ref().as_ref().len() {
            return Err(DataIOError::illegal_argument(format!(
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

    fn write_from(&mut self, src: &[u8], offset: u32, len: u32) -> Result<(), DataIOError> {
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
) -> Result<(), DataIOError> {
    let total = source.len() as u64;

    if position > total || position + len as u64 > total {
        return Err(DataIOError::illegal_argument(format!(
            "Read out of bounds: position={}, len={}, total={}",
            position, len, total
        )));
    }
    if offset + len > dest.len() {
        return Err(DataIOError::illegal_argument(format!(
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
