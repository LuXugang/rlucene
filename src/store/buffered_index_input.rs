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
use crate::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::store::index_input::IndexInput;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{
    BufferedIndexInputBase, ByteBuffersIndexInput, Context, DataInput, IOContext, ReadableCursorExt,
};
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::error::runtime_error::RuntimeError;
use std::fmt::{format, Display, Formatter};
use std::io::{Cursor, Read};

/// Default buffer size set to `BUFFER_SIZE`.
pub const BUFFER_SIZE: u32 = 1024;
/// Minimum buffer size allowed
pub const MIN_BUFFER_SIZE: u32 = 8;

/// A buffer size for merges set to `MERGE_BUFFER_SIZE`. */
/// The normal read buffer size defaults to 1024, but
/// increasing this during merging seems to yield
/// performance gains.  However, we don't want to increase
/// it too much because there are quite a few
/// BufferedIndexInputs created during merging.  See
/// LUCENE-888 for details.
pub const MERGE_BUFFER_SIZE: u32 = 4096;

/// Base implementation class for buffered [`IndexInput`]. */
pub struct BufferedIndexInput<T>
where
    T: IndexInput + BufferedIndexInputBase,
{
    buffer_size: u32,
    resource_desc: String,
    buffer: Cursor<Vec<u8>>,
    sub_index_input: T,
    buffer_start: u64,
}
impl<T> BufferedIndexInput<T>
where
    T: IndexInput + BufferedIndexInputBase,
{
    pub fn new_with_buffer_size(
        sub_index_input: T,
        resource_desc: &str,
        buffer_size: u32,
    ) -> BufferedIndexInput<T> {
        let buffer = Cursor::new(vec![0u8; buffer_size as usize]);
        BufferedIndexInput {
            buffer_size,
            resource_desc: resource_desc.to_string(),
            buffer,
            sub_index_input,
            buffer_start: 0,
        }
    }
    pub fn new_with_resource_desc(
        sub_index_input: T,
        resource_desc: &str,
    ) -> BufferedIndexInput<T> {
        Self::new_with_buffer_size(sub_index_input, resource_desc, BUFFER_SIZE)
    }

    pub fn new_io_context(
        sub_index_input: T,
        resource_desc: &str,
        context: IOContext,
    ) -> BufferedIndexInput<T> {
        Self::new_with_buffer_size(sub_index_input, resource_desc, Self::buffer_size(context))
    }

    /// Returns default buffer sizes for the given [`IOContext`].
    pub fn buffer_size(io_context: IOContext) -> u32 {
        match io_context.context {
            Context::Merge => MERGE_BUFFER_SIZE,
            Context::Default | Context::Flush => BUFFER_SIZE,
        }
    }

    fn check_buffer_size(buffer_size: u32) -> Result<(), RuntimeError> {
        if buffer_size < MIN_BUFFER_SIZE {
            return Err(RuntimeError::illegal_argument(format!(
                "bufferSize must be at least MIN_BUFFER_SIZE (got {})",
                buffer_size
            )));
        }
        Ok(())
    }

    fn refill(&self) -> Result<(), DataIOError> {
        todo!()
    }
}

impl<T> DataInput for BufferedIndexInput<T>
where
    T: IndexInput + BufferedIndexInputBase,
{
    fn read_byte(&mut self) -> Result<u8, DataIOError> {
        todo!()
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: u32, len: u32) -> Result<(), DataIOError> {
        self.read_bytes_with_buffer(b, offset, len, true)
    }

    fn read_bytes_with_buffer(
        &mut self,
        b: &mut [u8],
        offset: u32,
        len: u32,
        use_buffer: bool,
    ) -> Result<(), DataIOError> {
        let mut current_offset = offset;
        let mut current_len = len;

        let mut available = self.buffer.remain();
        debug_assert!(available <= u32::MAX as u64);
        if current_len as u64 <= available {
            // the buffer contains enough data to satisfy this request
            if current_len > 0 {
                self.buffer.read_to(b, current_offset, current_len)?;
            }
        } else {
            // the buffer does not have enough data. First serve all we've got.
            if available > 0 {
                self.buffer.read_to(b, current_offset, available as u32)?;
                debug_assert!(self.buffer.remain() == 0);
                current_offset += available as u32;
                current_len -= available as u32;
            }
            // and now, read the remaining 'len' bytes:
            if use_buffer && current_len < self.buffer_size {
                // If the amount left to read is small enough, and
                // we are allowed to use our buffer, do it in the usual
                // buffered way: fill the buffer and read slice from it:
                self.refill()?;

                available = self.buffer.remain();
                let start = self.buffer.position() as usize;
                if available < current_len as u64 {
                    // Throw an error when refill() could not read len bytes:
                    self.buffer.read_to(b, current_offset, available as u32)?;
                    debug_assert!(self.buffer.remain() == 0);
                    return Err(DataIOError::eof(format!("read past EOF: {}", self)));
                } else {
                    self.buffer.read_to(b, current_offset, current_len)?;
                }
            } else {
                debug_assert!(self.buffer.remain() == 0);
                // The amount left to read is larger than the buffer
                // or we've been asked to not use our buffer -
                // there's no performance reason not to read it all
                // at once. Note that unlike the previous code of
                // this function, there is no need to do a seek
                // here, because there's no need to reread what we
                // had in the buffer.
                let after = self.buffer_start + self.buffer.position() as u64 + current_len as u64;
                if after > self.sub_index_input.length() {
                    return Err(DataIOError::eof(format!("read past EOF: {}", self)));
                }
                let mut temp_cursor = Cursor::new(vec![0; current_len as usize]);
                self.sub_index_input.read_internal(&mut temp_cursor)?;
                temp_cursor.read_to(b, current_offset as u32, current_len as u32)?;
                self.buffer_start = after;
                self.buffer.set_position(0);
                self.buffer.get_mut().clear();
            }
        }
        Ok(())
    }

    fn skip_bytes(&mut self, num_bytes: u64) -> Result<(), DataIOError> {
        todo!()
    }
}

impl<T> Display for BufferedIndexInput<T>
where
    T: IndexInput + BufferedIndexInputBase,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<T> Clone for BufferedIndexInput<T>
where
    T: IndexInput + BufferedIndexInputBase,
{
    fn clone(&self) -> Self {
        todo!()
    }
}

impl<T> IndexInput for BufferedIndexInput<T>
where
    T: IndexInput + BufferedIndexInputBase,
{
    fn get_file_pointer(&self) -> u64 {
        self.buffer_start + self.buffer.position()
    }

    fn seek(&mut self, pos: u64) -> Result<(), DataIOError> {
        todo!()
    }

    fn length(&self) -> u64 {
        self.sub_index_input.length()
    }

    fn slice(
        &self,
        slice_description: &str,
        offset: u64,
        length: u64,
    ) -> Result<ByteBuffersIndexInput, DataIOError> {
        unreachable!()
    }

    fn is_random_access(&self) -> bool {
        todo!()
    }

    fn get_random_access_slice(
        &self,
        offset: u64,
        length: u64,
    ) -> Result<ByteBuffersDataInput, DataIOError> {
        unreachable!()
    }
}

impl<T> RandomAccessInput for BufferedIndexInput<T>
where
    T: IndexInput + BufferedIndexInputBase,
{
    fn length(&self) -> u64 {
        todo!()
    }

    fn read_byte(&mut self, pos: u64) -> Result<u8, DataIOError> {
        todo!()
    }

    fn read_short(&mut self, pos: u64) -> Result<i16, DataIOError> {
        todo!()
    }

    fn read_int(&mut self, pos: u64) -> Result<i32, DataIOError> {
        todo!()
    }

    fn read_long(&mut self, pos: u64) -> Result<i64, DataIOError> {
        todo!()
    }

    fn pre_fetch(&mut self, pos: u64, len: u64) -> Result<(), DataIOError> {
        todo!()
    }
}
impl<T> BufferedIndexInputBase for BufferedIndexInput<T>
where
    T: IndexInput + BufferedIndexInputBase,
{
    fn seek_internal(&self, pos: u64) -> Result<(), DataIOError> {
        self.sub_index_input.seek_internal(pos)
    }

    fn read_internal(&mut self, b: &mut Cursor<Vec<u8>>) -> Result<(), DataIOError> {
        self.sub_index_input.read_internal(b)
    }
}
