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
use crate::store::{BufferedIndexInputBase, ByteBuffersIndexInput, Context, DataInput, IOContext};
use crate::util::bit_util::{FLOAT_BYTES, INT_BYTES, LONG_BYTES, SHORT_BYTES};
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::error::runtime_error::RuntimeError;
use crate::util::group_vint_util::GroupVIntUtil;
use crate::util::ReadableCursorExt;
use byteorder::{ByteOrder, LE};
use std::fmt::{Display, Formatter};
use std::io::Cursor;

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
    T: BufferedIndexInputBase,
{
    buffer_size: u32,
    resource_desc: String,
    buffer: Cursor<Vec<u8>>,
    sub_index_input: T,
    buffer_start: u64,
    /// global pos in the file, used for sequential read
    pos: u64,
    /// valid data length in the buffer
    length: u32,
}
impl<T> BufferedIndexInput<T>
where
    T: BufferedIndexInputBase,
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
            pos: 0,
            length: 0,
        }
    }
    pub fn new_with_resource_desc(
        sub_index_input: T,
        resource_desc: &str,
    ) -> BufferedIndexInput<T> {
        Self::new_with_buffer_size(sub_index_input, resource_desc, BUFFER_SIZE)
    }

    pub fn new_with_io_context(
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
    /// Refills the buffer with data from the underlying input, preserving unaligned bytes.
    ///
    /// This method handles cases where a previous read left some unaligned bytes in the buffer.
    /// It copies these unaligned bytes to the start of the buffer and refills the remaining space
    /// with new data from the underlying input.
    ///
    /// # Arguments
    /// * `remain_unaligned_bytes` - The number of unaligned bytes remaining in the buffer from the previous read.
    /// * `start` - The starting position in the underlying input to begin reading data.
    ///
    /// # Returns
    /// * `Ok(())` - If the buffer is successfully refilled.
    /// * `Err(DataIOError)` - If an error occurs during the refill operation, such as reaching EOF.
    ///
    /// # Behavior
    /// 1. Calculates the range `[start, end)` for data to be read from the underlying input.
    /// 2. Ensures that the read operation does not exceed the end of the file (EOF).
    /// 3. Copies the unaligned bytes to the start of the buffer.
    /// 4. Reads new data into the remaining space in the buffer.
    /// 5. Updates the buffer's position and the valid data length (`self.length`).
    ///
    /// # Notes
    /// - The `buffer_start` is adjusted to include the unaligned bytes.
    /// - The new valid data length is the sum of the unaligned bytes and the newly read bytes.
    ///
    /// # Errors
    /// * Returns `DataIOError::eof` if no new data can be read from the underlying input.
    fn refill(&mut self, remain_unaligned_bytes: u32, start: u64) -> Result<(), DataIOError> {
        // After the last read, some unaligned bytes remain in the buffer.
        let mut end = start + (self.buffer_size - remain_unaligned_bytes) as u64;

        // Don't read past EOF
        let length = self.sub_index_input.length();
        if end > length {
            end = length;
        }

        let new_length = end - start;
        if new_length == 0 {
            return Err(DataIOError::eof(format!("read past EOF: {}", self)));
        }

        // valid data length in buffer
        debug_assert!(new_length <= u32::MAX as u64);
        self.length = new_length as u32 + remain_unaligned_bytes;
        // Set the buffer position to the remaining unaligned bytes
        // so that the next write within `read_internal` starts from remaining unaligned bytes
        self.buffer.set_position(remain_unaligned_bytes as u64);
        let file_pointer = self.get_file_pointer();
        self.sub_index_input
            .read_internal(&mut self.buffer, new_length, file_pointer)?;
        // Adjust buffer_start to include unaligned bytes
        self.buffer_start = start - remain_unaligned_bytes as u64;
        Ok(())
    }
    fn read_longs(
        &mut self,
        pos: u64,
        len: u32,
        output: &mut [i64],
        use_buffer: bool,
    ) -> Result<(), DataIOError> {
        self.read_buffer(
            pos,
            len,
            output,
            LONG_BYTES as u32,
            LE::read_i64,
            use_buffer,
        )
    }
    fn read_bytes(
        &mut self,
        pos: u64,
        len: u32,
        output: &mut [u8],
        use_buffer: bool,
    ) -> Result<(), DataIOError> {
        // This closure is not expected to be called under any circumstances.

    self.read_buffer(pos, len, output, 1, |_| unreachable!(), use_buffer)
    }
    fn read_ints(
        &mut self,
        pos: u64,
        len: u32,
        output: &mut [i32],
        use_buffer: bool,
    ) -> Result<(), DataIOError> {
        self.read_buffer(pos, len, output, INT_BYTES as u32, LE::read_i32, use_buffer)
    }
    fn read_shorts(
        &mut self,
        pos: u64,
        len: u32,
        output: &mut [i16],
        use_buffer: bool,
    ) -> Result<(), DataIOError> {
        self.read_buffer(
            pos,
            len,
            output,
            SHORT_BYTES as u32,
            LE::read_i16,
            use_buffer,
        )
    }
    fn read_floats(
        &mut self,
        pos: u64,
        len: u32,
        output: &mut [f32],
        use_buffer: bool,
    ) -> Result<(), DataIOError> {
        self.read_buffer(
            pos,
            len,
            output,
            FLOAT_BYTES as u32,
            LE::read_f32,
            use_buffer,
        )
    }
    /// Reads and converts a specified number of elements starting at a given position into a target buffer.
    ///
    /// This method reads a specified number of bytes, starting at the position (`pos`),
    /// and converts them into elements of type `D` using the provided `converter` function. The converted elements
    /// are written into the `target` buffer.
    ///
    /// The method prioritizes reading from the internal buffer if the data is available. If the data is not in
    /// the buffer, it will either refill the buffer or read directly from the underlying input depending on the
    /// `use_buffer` flag.
    ///
    /// # Arguments
    /// * `pos` - The starting position in the file or stream to read from.
    /// * `target` - The target buffer where the converted data will be written.
    /// * `len` - The number of elements to read and convert.
    /// * `type_size` - The size in bytes of each element to be read.
    /// * `use_buffer` - Whether to use the internal buffer for reading. If `false`, data will be read directly.
    /// * `converter` - A closure that processes a chunk of raw bytes into the target buffer format.
    ///
    /// # Returns
    /// * `Ok(())` - If the requested data is successfully read and converted.
    /// * `Err(DataIOError)` - If an error occurs during reading, such as reaching EOF.
    ///
    /// # Behavior
    /// - If `use_buffer` is `true` and the requested data fits in the buffer, it will be read directly from the buffer.
    /// - If `use_buffer` is `true` but the requested data exceeds the buffer, the method will refill the buffer
    ///   and continue reading.
    /// - If `use_buffer` is `false`, data will be read directly from the underlying input, bypassing the buffer.
    /// - The provided `converter` processes chunks of raw bytes to populate the target buffer.
    ///
    /// # Errors
    /// This method may return the following errors:
    /// * `DataIOError::eof` - If attempting to read beyond the end of the file or stream.
    ///
    /// # Notes
    /// - The method assumes that the buffer's `refill` method ensures enough data is available for reading,
    ///   eliminating the need for additional checks.
    /// - When unaligned data remains in the buffer (e.g., when the available bytes are not a multiple of `type_size`),
    ///   the method copies the remaining bytes to the start of the buffer for further processing. This ensures that
    ///   subsequent reads start with aligned data. The maximum amount of data copied is `type_size - 1` bytes, which
    ///   is minimal. For example, if the largest type being read is `i64` (8 bytes), at most 7 bytes are copied.
    ///   Such small amounts of data copying have negligible performance impact.
    fn read_buffer<D, F>(
        &mut self,
        pos: u64,
        len: u32,
        target: &mut [D],
        type_size: u32,
        converter: F,
        use_buffer: bool,
    ) -> Result<(), DataIOError>
    where
        D: Copy,
        F: Fn(&[u8]) -> D,
    {
        let total_bytes = len * type_size;
        let mut elements_read = 0;
        let mut unaligned_bytes = 0;
        // Check if the position is within the current buffer range
        if pos >= self.buffer_start && pos < self.buffer_start + self.length as u64 {
            let buffer_offset = (pos - self.buffer_start) as u32;
            let available = self
                .buffer
                .remain_between(buffer_offset as u64, self.length as u64);
            // If the buffer contains enough data to satisfy the request
            if available >= total_bytes as u64 {
                let src = &self.buffer.get_ref()
                    [buffer_offset as usize..(buffer_offset as usize + total_bytes as usize)];
                Self::process_data(
                    src,
                    &mut target[0..len as usize],
                    len as usize,
                    type_size,
                    &converter,
                );
                return Ok(());
            }

            let aligned_bytes = (available as u32 / type_size) * type_size;
            let aligned_elements = aligned_bytes / type_size;

            if aligned_elements > 0 {
                let src = &self.buffer.get_ref()
                    [buffer_offset as usize..(buffer_offset + aligned_bytes) as usize];
                Self::process_data(
                    src,
                    &mut target[elements_read as usize
                        ..elements_read as usize + aligned_elements as usize],
                    aligned_elements as usize,
                    type_size,
                    &converter,
                );
                elements_read += aligned_elements;
            }
            // Handle unaligned bytes that can't form a complete element
            unaligned_bytes = available as u32 - aligned_bytes;
            if unaligned_bytes > 0 {
                let buffer = self.buffer.get_mut();
                let unaligned_start = (buffer_offset + aligned_bytes) as usize;
                // Copy unaligned bytes to the start of the buffer, we would read these bytes later when buffer was refilled again
                buffer.copy_within(
                    unaligned_start..unaligned_start + unaligned_bytes as usize,
                    0,
                );
            }
        }

        debug_assert!(self.buffer.position() <= u32::MAX as u64);
        let remaining_len = len - elements_read;
        let remaining_bytes = remaining_len * type_size;

        if use_buffer && remaining_bytes < self.buffer_size {
            let start = self.buffer_start + self.length as u64;
            self.refill(unaligned_bytes, start)?;

            let available = self.length;
            let readable_elements = (available / type_size).min(remaining_len);

            if readable_elements > 0 {
                let src =
                    &self.buffer.get_ref()[0..readable_elements as usize * type_size as usize];
                Self::process_data(
                    src,
                    &mut target[elements_read as usize
                        ..elements_read as usize + readable_elements as usize],
                    readable_elements as usize,
                    type_size,
                    &converter,
                );
            }
            // If there are still remaining elements, report EOF
            let remaining_elements = remaining_len - readable_elements;
            if remaining_elements > 0 {
                return Err(DataIOError::eof(format!(
                    "read past EOF: expected {}, got {}",
                    remaining_len, readable_elements
                )));
            }

            return Ok(());
        }
        // If the buffer is not used or the remaining data exceeds the buffer size,
        // read directly from the underlying input
        let after =
            self.buffer_start + self.length as u64 + remaining_len as u64 * type_size as u64;
        if after > self.sub_index_input.length() {
            return Err(DataIOError::eof(format!("read past EOF: {}", self)));
        }

        let mut temp_cursor = Cursor::new(vec![0; (remaining_len * type_size) as usize]);
        let file_pointer = self.get_file_pointer();
        self.sub_index_input.read_internal(
            &mut temp_cursor,
            (remaining_len * type_size) as u64,
            file_pointer,
        )?;

        let src = temp_cursor.get_ref();
        Self::process_data(
            src,
            &mut target[elements_read as usize..elements_read as usize + remaining_len as usize],
            remaining_len as usize,
            type_size,
            &converter,
        );

        self.buffer_start = after;
        Ok(())
    }
    /// Processes data by converting a source byte slice (`src`) into a destination slice (`dst`) of type `D`.
    ///
    /// # Parameters
    /// - `src`: A byte slice containing the source data to be processed.
    /// - `dst`: A mutable slice of type `D` where the converted data will be stored.
    /// - `len`: The number of elements to process.
    /// - `type_size`: The size of each element in bytes in the source data.
    /// - `converter`: A conversion function that takes a byte slice and converts it into type `D`.
    ///
    /// # Behavior
    /// - If `type_size` is 1, the function directly copies the data from `src` to `dst` as bytes using unsafe code.
    /// - Otherwise, it iterates over `src` in chunks of `type_size`, applies the `converter` function to each chunk,
    ///   and stores the resulting value in `dst`.
    ///
    /// # Safety
    /// - Unsafe code is used to cast `dst` to a mutable byte slice when `type_size == 1`. The caller must ensure
    ///   that `dst` has sufficient capacity and correct alignment to avoid undefined behavior.
    ///
    fn process_data<D, F>(src: &[u8], dst: &mut [D], len: usize, type_size: u32, converter: &F)
    where
        D: Copy,
        F: Fn(&[u8]) -> D,
    {
        if type_size == 1 {
            unsafe {
                let dst_u8 = std::slice::from_raw_parts_mut(dst.as_mut_ptr() as *mut u8, len);
                dst_u8.copy_from_slice(src);
            }
        } else {
            for i in 0..len {
                let chunk_start = i * type_size as usize;
                dst[i] = converter(&src[chunk_start..chunk_start + type_size as usize]);
            }
        }
    }
    /// Resolves the position within the buffer, optimizing for backward random reads.
    ///
    /// When performing random reads at position `pos`, this function checks if the requested position
    /// can be directly resolved within the current buffer. If not, it adjusts the buffer range
    /// dynamically to optimize subsequent backward reads:
    ///
    /// - For backward random reads, instead of always starting the buffer from `pos`, the buffer is
    ///   adjusted to include the range `[pos + width - buffer_size, pos + width]`, if possible. This
    ///   approach minimizes redundant loading of the same data when successive backward reads occur.
    ///
    /// - For forward random reads, the buffer starts directly at `pos` to ensure data availability.
    ///
    /// # Arguments
    /// * `pos` - The target position to resolve.
    /// * `width` - The number of bytes needed for the read operation.
    ///
    /// # Returns
    /// * `Ok(pos)` - If the position is resolved successfully.
    /// * `Err(DataIOError)` - If an error occurs while seeking or refilling the buffer.
    ///
    /// # Behavior
    /// - If the position `pos` is within the current buffer and there is enough data for `width` bytes,
    ///   the function returns `pos` directly.
    /// - If the position is outside the current buffer, it dynamically adjusts `buffer_start`:
    ///   - For backward reads, it calculates a new `buffer_start` such that the buffer includes the
    ///     desired position and minimizes redundant reloading of the same data.
    ///   - For forward reads, it directly sets `buffer_start` to `pos`.
    /// - The buffer is refilled as needed to ensure the requested data is available.
    ///
    /// # Efficiency
    /// This method is particularly efficient for scenarios involving frequent backward random reads,
    /// as it reduces redundant I/O operations by aligning the buffer with anticipated access patterns.
    fn resolve_position_in_buffer(&mut self, pos: u64, width: u32) -> Result<u64, DataIOError> {
        let index: i64 = pos as i64 - self.buffer_start as i64;
        if index >= 0 && index <= (self.length as i64 - width as i64) {
            return Ok(pos);
        }
        if index < 0 {
            // if we're moving backwards, then try and fill up the previous page rather than
            // starting again at the current pos, to avoid successive backwards reads reloading
            // the same data over and over again.  We also check that we can read `width`
            // bytes without going over the end of the buffer
            let temp_buffer_start = (self.buffer_start as i64 - self.buffer_size as i64)
                .max(pos as i64 + width as i64 - self.buffer_size as i64);
            self.buffer_start = temp_buffer_start.max(0) as u64;
            self.buffer_start = self.buffer_start.min(pos);
            self.pos = self.buffer_start;
        } else {
            self.buffer_start = pos;
            self.pos = pos;
        }
        self.length = 0;
        self.sub_index_input.seek_internal(self.buffer_start)?;
        self.refill(0, self.buffer_start)?;
        Ok(pos)
    }
    #[cfg(feature = "test_only")]
    pub fn get_sub_index_input(&self) -> &T {
        &self.sub_index_input
    }
}

impl<T> DataInput for BufferedIndexInput<T>
where
    T: BufferedIndexInputBase,
{
    fn read_byte(&mut self) -> Result<u8, DataIOError> {
        let mut bytes = [0; 1];
        self.read_bytes(self.pos, 1, &mut bytes, true)?;
        self.pos += 1;
        Ok(bytes[0])
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
        self.read_bytes(
            self.pos,
            len,
            &mut b[offset as usize..(offset + len) as usize],
            use_buffer,
        )?;
        self.pos += len as u64;
        Ok(())
    }

    fn read_short(&mut self) -> Result<i16, DataIOError> {
        let mut output = [0; 1];
        self.read_shorts(self.pos, 1, &mut output, true)?;
        self.pos += SHORT_BYTES as u64;
        Ok(output[0])
    }

    fn read_int(&mut self) -> Result<i32, DataIOError> {
        let mut output = [0; 1];
        self.read_ints(self.pos, 1, &mut output, true)?;
        self.pos += INT_BYTES as u64;
        Ok(output[0])
    }

    fn read_group_vint(&mut self, dst: &mut [i64], offset: u32) -> Result<(), DataIOError> {
        let remain =
            self.buffer
                .remain_between(self.buffer.position(), self.length as u64) as usize;
        let len = GroupVIntUtil::read_group_vint_with_reader(
            self,
            remain as u64,
            self.buffer.position(),
            dst,
            offset,
        )?;
        self.pos += len as u64;
        Ok(())
    }

    fn read_long(&mut self) -> Result<i64, DataIOError> {
        let mut output = [0; 1];
        self.read_longs(self.pos, 1, &mut output, true)?;
        self.pos += LONG_BYTES as u64;
        Ok(output[0])
    }

    /// Reads multiple `i64` values into the destination buffer.
    ///
    /// This method reads `len` `i64` elements into the `dst` buffer starting at the specified `offset`.
    /// The reading is performed in chunks, where each chunk size is determined by the internal
    /// buffer size (`buffer_size`) divided by the size of an `i64` element.
    ///
    /// # Arguments
    /// * `dst` - The destination buffer to store the `i64` values.
    /// * `offset` - The offset within the destination buffer to start writing the data.
    /// * `len` - The number of `i64` elements to read.
    ///
    /// # Returns
    /// * `Ok(())` - If the requested number of elements is successfully read.
    /// * `Err(DataIOError)` - If an error occurs during reading.
    ///
    /// # Behavior
    /// - The method ensures the data is read in chunks that fit within the buffer size.
    /// - It updates the global position (`self.pos`) after reading all requested elements.
    ///
    fn read_longs(&mut self, dst: &mut [i64], offset: u32, len: u32) -> Result<(), DataIOError> {
        self.read_longs(
            self.pos,
            len,
            &mut dst[offset as usize..(offset + len) as usize],
            true,
        )?;
        self.pos += len as u64 * LONG_BYTES as u64;
        Ok(())
    }
    /// Reads multiple `i32` values into the destination buffer.
    ///
    /// This method reads `len` `i32` elements into the `dst` buffer starting at the specified `offset`.
    /// The reading is performed in chunks, where each chunk size is determined by the internal
    /// buffer size (`buffer_size`) divided by the size of an `i32` element.
    ///
    /// # Arguments
    /// * `dst` - The destination buffer to store the `i32` values.
    /// * `offset` - The offset within the destination buffer to start writing the data.
    /// * `len` - The number of `i32` elements to read.
    ///
    /// # Returns
    /// * `Ok(())` - If the requested number of elements is successfully read.
    /// * `Err(DataIOError)` - If an error occurs during reading.
    ///
    /// # Behavior
    /// - The method ensures the data is read in chunks that fit within the buffer size.
    /// - It updates the global position (`self.pos`) after reading all requested elements.
    ///
    fn read_ints(&mut self, dst: &mut [i32], offset: u32, len: u32) -> Result<(), DataIOError> {
        self.read_ints(
            self.pos,
            len,
            &mut dst[offset as usize..(offset + len) as usize],
            true,
        )?;

        self.pos += len as u64 * INT_BYTES as u64;
        Ok(())
    }
    /// Reads multiple `f32` values into the destination buffer.
    ///
    /// This method reads `len` `f32` elements into the `dst` buffer starting at the specified `offset`.
    /// The reading is performed in chunks, where each chunk size is determined by the internal
    /// buffer size (`buffer_size`) divided by the size of an `f32` element.
    ///
    /// # Arguments
    /// * `dst` - The destination buffer to store the `f32` values.
    /// * `offset` - The offset within the destination buffer to start writing the data.
    /// * `len` - The number of `f32` elements to read.
    ///
    /// # Returns
    /// * `Ok(())` - If the requested number of elements is successfully read.
    /// * `Err(DataIOError)` - If an error occurs during reading.
    ///
    /// # Behavior
    /// - The method ensures the data is read in chunks that fit within the buffer size.
    /// - It updates the global position (`self.pos`) after reading all requested elements.
    ///
    fn read_floats(&mut self, dst: &mut [f32], offset: u32, len: u32) -> Result<(), DataIOError> {
        let mut remaining = len;
        let mut current_offset = offset;

        while remaining > 0 {
            // Calculate the maximum number of elements to read in one iteration
            let chunk_len = (self.buffer_size / FLOAT_BYTES as u32).min(remaining);

            let pos = self.pos + (current_offset as u64 - offset as u64) * FLOAT_BYTES as u64;

            self.read_floats(
                pos,
                chunk_len,
                &mut dst[current_offset as usize..(current_offset + chunk_len) as usize],
                true, // Use buffer
            )?;

            remaining -= chunk_len;
            current_offset += chunk_len;
        }

        self.pos += len as u64 * FLOAT_BYTES as u64;
        Ok(())
    }

    fn skip_bytes(&mut self, num_bytes: u64) -> Result<(), DataIOError> {
        IndexInput::skip_bytes(self, num_bytes)
    }
}

impl<T> Display for BufferedIndexInput<T>
where
    T: BufferedIndexInputBase,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BufferedIndexInput({})", self.resource_desc)
    }
}

impl<T> Clone for BufferedIndexInput<T>
where
    T: BufferedIndexInputBase,
{
    fn clone(&self) -> Self {
        todo!()
    }
}

impl<T> IndexInput for BufferedIndexInput<T>
where
    T: BufferedIndexInputBase,
{
    fn get_file_pointer(&self) -> u64 {
        self.pos
    }

    fn seek(&mut self, pos: u64) -> Result<(), DataIOError> {
        if pos >= self.buffer_start && pos < (self.buffer_start + self.length as u64) {
            self.pos = pos;
        } else {
            self.pos = pos;
            self.buffer_start = pos;
            self.length = 0;
            self.sub_index_input.seek_internal(pos)?;
        }
        Ok(())
    }

    fn length(&self) -> u64 {
        self.sub_index_input.length()
    }

    fn slice(
        &self,
        slice_description: &str,
        offset: u64,
        length: u64,
    ) -> Result<impl IndexInput, DataIOError> {
        self.sub_index_input.slice(slice_description, offset, length)
    }

    fn is_random_access(&self) -> bool {
        false
    }

    #[allow(refining_impl_trait)]
    fn get_random_access_slice(
        &self,
        _offset: u64,
        _length: u64,
    ) -> Result<ByteBuffersDataInput, DataIOError> {
        unreachable!()
    }
}

impl<T> RandomAccessInput for BufferedIndexInput<T>
where
    T: BufferedIndexInputBase,
{
    fn length(&self) -> u64 {
        todo!()
    }

    fn read_byte(&mut self, pos: u64) -> Result<u8, DataIOError> {
        let mut bytes = [0; 1];
        let pos = self.resolve_position_in_buffer(pos, 1)?;
        self.read_bytes(pos, 1, &mut bytes, true)?;
        Ok(bytes[0])
    }

    fn read_bytes(
        &mut self,
        pos: u64,
        b: &mut [u8],
        offset: u32,
        len: u32,
    ) -> Result<(), DataIOError> {
        let pos = self.resolve_position_in_buffer(pos, len)?;
        self.read_bytes(
            pos,
            len,
            &mut b[offset as usize..(offset + len) as usize],
            true,
        )?;
        Ok(())
    }

    fn read_short(&mut self, pos: u64) -> Result<i16, DataIOError> {
        let mut bytes = [0; SHORT_BYTES];
        let pos = self.resolve_position_in_buffer(pos, SHORT_BYTES as u32)?;
        self.read_shorts(pos, 1, &mut bytes, true)?;
        Ok(bytes[0])
    }

    fn read_int(&mut self, pos: u64) -> Result<i32, DataIOError> {
        let mut bytes = [0; INT_BYTES];
        let pos = self.resolve_position_in_buffer(pos, INT_BYTES as u32)?;
        self.read_ints(pos, 1, &mut bytes, true)?;
        Ok(bytes[0])
    }

    fn read_long(&mut self, pos: u64) -> Result<i64, DataIOError> {
        let mut bytes = [0; LONG_BYTES];
        let pos = self.resolve_position_in_buffer(pos, LONG_BYTES as u32)?;
        self.read_longs(pos, 1, &mut bytes, true)?;
        Ok(bytes[0])
    }

    fn pre_fetch(&mut self, pos: u64, len: u64) -> Result<(), DataIOError> {
        Ok(())
    }
}

struct SlicedIndexInput {

}