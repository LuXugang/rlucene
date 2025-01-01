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
use crate::store::index_input::{get_full_slice_description, IndexInput};
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{BufferedIndexInput, BufferedIndexInputBase, BUFFER_SIZE};
use crate::util::error::runtime_error::RuntimeError;
use crate::util::ReadableCursorExt;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

const CHUNK_SIZE: usize = 16384;
pub struct NIOFSIndexInput {
    /// the file we will read from
    file: File,
    /// start offset: non-zero in the slice case
    off: u64,
    /// end offset (start+length)
    end: u64,
    resource_desc: String,
    buffer_size: u32,
}

impl NIOFSIndexInput {
    pub fn new(file: File, resource_desc: &str) -> Self {
        let metadata = file.metadata().unwrap();
        let len = metadata.len();
        Self {
            file,
            off: 0,
            end: len,
            resource_desc: resource_desc.to_string(),
            buffer_size: BUFFER_SIZE,
        }
    }
    pub fn new_with_range(
        file: File,
        off: u64,
        length: u64,
        resource_desc: &str,
        buffer_size: u32,
    ) -> Self {
        Self {
            file,
            off,
            end: off + length,
            resource_desc: resource_desc.to_string(),
            buffer_size,
        }
    }
    pub fn get_buffer_size(&self) -> u32 {
        self.buffer_size
    }
}

impl BufferedIndexInputBase for NIOFSIndexInput {
    fn seek_internal(&mut self, pos: u64) -> Result<(), RuntimeError> {
        if pos > self.length() {
            return Err(RuntimeError::eof(format!(
                "read past EOF: pos={} vs length={} in {}",
                pos,
                self.length(),
                self,
            )));
        }
        Ok(())
    }

    /// Reads data from the file into the provided buffer, ensuring that the data is read
    /// in chunks of a configurable size and does not exceed the file's defined bounds.
    ///
    /// # Arguments
    ///
    /// * `buffer` - A mutable reference to a `Cursor<Vec<u8>>`, which acts as the target buffer for
    ///   storing the data. The position of the cursor is updated after each read to reflect
    ///   the amount of data written.
    /// * `len` - The number of bytes to read from the file. This must not exceed the buffer's remaining
    /// * `file_pointer` - The initial position in the file from which to start reading.
    ///
    /// # Errors
    ///
    /// This method returns a `DataIOError` in the following cases:
    ///
    /// * [`RuntimeError::Eof`] - If the requested read range exceeds the file's bounds or if the file
    ///   unexpectedly reaches EOF during a read.
    /// * [`RuntimeError::Io`] - For general I/O errors encountered while reading or seeking the file.
    ///
    /// # Details
    ///
    /// This method reads data from the file in chunks of up to `CHUNK_SIZE` bytes to optimize
    /// performance for large reads. Each chunk is written into the buffer starting at the cursor's
    /// current position, and the cursor's position is incremented accordingly. The method ensures
    /// that:
    ///
    /// 1. The file's read position (`file_pointer`) is correctly advanced for each chunk.
    /// 2. The buffer is not overrun, with proper validation of its capacity before writing.
    /// 3. The read length is fully consumed or an appropriate error is returned.
    ///
    /// The file pointer (`pos`) is adjusted dynamically during the read process, and the method uses
    /// `seek` to position the file pointer correctly for each chunk.
    fn read_internal(
        &mut self,
        buffer: &mut Cursor<Vec<u8>>,
        len: u64,
        file_pointer: u64,
    ) -> Result<(), RuntimeError> {
        debug_assert!(buffer.remain() >= len, "buffer overflow");
        let mut pos = file_pointer + self.off;

        // Check if the requested read exceeds the file's end
        if pos + len > self.end {
            return Err(RuntimeError::eof(format!(
                "read past EOF: position={} len={} end={}",
                pos, len, self.end
            )));
        }

        let mut read_length = len;
        while read_length > 0 {
            // Determine the size of the current chunk to read
            let to_read = CHUNK_SIZE.min(read_length as usize);

            // Seek to the correct position in the file
            self.file
                .seek(SeekFrom::Start(pos))
                .map_err(RuntimeError::io)?;

            // Prepare the buffer slice for writing
            let buffer_start = buffer.position() as usize;
            let buffer_end = buffer_start + to_read;
            let buffer_slice = &mut buffer.get_mut()[buffer_start..buffer_end];

            let bytes_read = self.file.read(buffer_slice).map_err(RuntimeError::io)?;

            if bytes_read == 0 {
                return Err(RuntimeError::eof(format!(
                    "read past EOF during chunk read: position={} chunk size={} end={}",
                    pos, to_read, self.end
                )));
            }

            // Update the position and remaining length
            pos += bytes_read as u64;
            read_length -= bytes_read as u64;
            // Update the buffer cursor position for next read
            buffer.set_position(buffer.position() + bytes_read as u64);
        }

        // Ensure the entire requested length was read
        debug_assert_eq!(
            read_length, 0,
            "Unexpected remaining length after read: {}",
            read_length
        );
        Ok(())
    }
    fn slice(
        &self,
        slice_description: &str,
        offset: u64,
        length: u64,
    ) -> Result<impl IndexInput + RandomAccessInput, RuntimeError> {
        if offset + length > self.length() {
            return Err(RuntimeError::illegal_argument(format!(
                "slice() {} out of bounds: offset={}, length={}, fileLength={}: {}",
                slice_description,
                offset,
                length,
                self.length(),
                self
            )));
        }

        let resource_desc = get_full_slice_description(slice_description);
        let sub_index_input = NIOFSIndexInput::new_with_range(
            // Clone the file handle to create a new `File` instance pointing to the same file resource.
            self.file.try_clone().map_err(RuntimeError::io)?,
            self.off + offset,
            length,
            &resource_desc,
            self.buffer_size,
        );
        BufferedIndexInput::new_with_buffer_size(sub_index_input, &resource_desc, self.buffer_size)
    }
    fn length(&self) -> u64 {
        self.end - self.off
    }
}

impl Display for NIOFSIndexInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.resource_desc)
    }
}

impl Clone for NIOFSIndexInput {
    fn clone(&self) -> Self {
        Self {
            file: self.file.try_clone().unwrap(),
            off: self.off,
            end: self.end,
            resource_desc: self.resource_desc.clone(),
            buffer_size: self.buffer_size,
        }
    }
}
