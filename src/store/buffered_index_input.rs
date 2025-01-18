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
use crate::store::dummy::dummy_buffered_index_input_base::DummyBufferedIndexInputBase;
use crate::store::index_input::IndexInput;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{BufferedIndexInputBase, Context, DataInput, IOContext};
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::LuceneError;
use crate::util::group_vint_util::GroupVIntUtil;
use crate::util::{ReadableCursorExt, VecCopyOps};
use byteorder::{ByteOrder, LE};
use std::fmt::{Display, Formatter};
use std::io::Cursor;
/// Base implementation class for buffered [`IndexInput`]. */
pub struct BufferedIndexInput<T>
where
    T: BufferedIndexInputBase<Slice = BufferedIndexInput<T>>,
{
    buffer_size: i32,
    resource_desc: String,
    buffer: Cursor<Vec<u8>>,
    sub_index_input: T,
    buffer_start: i64,
    /// global pos in the file, used for sequential read
    pos: i64,
    /// valid data length in the buffer
    length: i32,
}
impl BufferedIndexInput<DummyBufferedIndexInputBase> {
    /// Default buffer size set to `BUFFER_SIZE`.
    pub const BUFFER_SIZE: i32 = 1024;
    /// Minimum buffer size allowed
    pub const MIN_BUFFER_SIZE: i32 = 8;

    /// A buffer size for merges set to `MERGE_BUFFER_SIZE`. */
    /// The normal read buffer size defaults to 1024, but
    /// increasing this during merging seems to yield
    /// performance gains.  However, we don't want to increase
    /// it too much because there are quite a few
    /// BufferedIndexInputs created during merging.  See
    /// LUCENE-888 for details.
    pub const MERGE_BUFFER_SIZE: i32 = 4096;
}
impl<T> BufferedIndexInput<T>
where
    T: BufferedIndexInputBase<Slice = BufferedIndexInput<T>>,
{
    pub fn new_with_buffer_size(
        sub_index_input: T,
        resource_desc: &str,
        buffer_size: i32,
    ) -> Result<BufferedIndexInput<T>, LuceneError> {
        let buffer = Cursor::new(vec![0u8; buffer_size as usize]);
        Self::check_buffer_size(buffer_size)?;
        Ok(BufferedIndexInput {
            buffer_size,
            resource_desc: resource_desc.to_string(),
            buffer,
            sub_index_input,
            buffer_start: 0,
            pos: 0,
            length: 0,
        })
    }
    pub fn new_with_resource_desc(
        sub_index_input: T,
        resource_desc: &str,
    ) -> Result<BufferedIndexInput<T>, LuceneError> {
        Self::new_with_buffer_size(
            sub_index_input,
            resource_desc,
            BufferedIndexInput::BUFFER_SIZE,
        )
    }

    pub fn new_with_io_context(
        sub_index_input: T,
        resource_desc: &str,
        context: &IOContext,
    ) -> Result<BufferedIndexInput<T>, LuceneError> {
        Self::new_with_buffer_size(sub_index_input, resource_desc, Self::buffer_size(context))
    }

    /// Returns default buffer sizes for the given [`IOContext`].
    pub fn buffer_size(io_context: &IOContext) -> i32 {
        match io_context.context {
            Context::Merge => BufferedIndexInput::MERGE_BUFFER_SIZE,
            Context::Default | Context::Flush => BufferedIndexInput::BUFFER_SIZE,
        }
    }

    fn check_buffer_size(buffer_size: i32) -> Result<(), LuceneError> {
        if buffer_size < BufferedIndexInput::MIN_BUFFER_SIZE {
            return Err(LuceneError::illegal_argument(format!(
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
    /// * `Err(LuceneError)` - If an error occurs during the refill operation, such as reaching EOF.
    ///
    /// # Behavior
    /// 1. Calculate the range `[start, end)` for data to be read from the underlying input.
    /// 2. Ensures that the read operation does not exceed the end of the file (EOF).
    /// 3. Copies the unaligned bytes to the start of the buffer.
    /// 4. Reads new data into the remaining space in the buffer.
    /// 5. Updates the buffer's position and the valid data length (`self.length`).
    ///
    /// # Note
    /// - The `buffer_start` is adjusted to include the unaligned bytes.
    /// - The new valid data length is the sum of the unaligned bytes and the newly read bytes.
    ///
    /// # Errors
    /// * Returns `LuceneError::eof` if no new data can be read from the underlying input.
    fn refill(&mut self, remain_unaligned_bytes: i32, start: i64) -> Result<(), LuceneError> {
        // After the last read, some unaligned bytes remain in the buffer.
        let mut end = start + (self.buffer_size - remain_unaligned_bytes) as i64;

        // Don't read past EOF
        let length = self.sub_index_input.length();
        if end > length {
            end = length;
        }

        let new_length = end - start;
        if new_length == 0 {
            return Err(LuceneError::eof(format!("read past EOF: {}", self)));
        }

        // valid data length in buffer
        debug_assert!(new_length <= i32::MAX as i64);
        self.length = new_length as i32 + remain_unaligned_bytes;
        // Set the buffer position to the remaining unaligned bytes
        // so that the next write within `read_internal` starts from remaining unaligned bytes
        self.buffer.set_position(remain_unaligned_bytes as u64);
        self.sub_index_input
            .read_internal(&mut self.buffer, new_length, start)?;
        // Adjust buffer_start to include unaligned bytes
        self.buffer_start = start - remain_unaligned_bytes as i64;
        Ok(())
    }
    fn read_longs(
        &mut self,
        pos: i64,
        len: i32,
        output: &mut [i64],
        use_buffer: bool,
    ) -> Result<(), LuceneError> {
        self.read_buffer(
            pos,
            len,
            output,
            BitUtil::LONG_BYTES as i32,
            LE::read_i64,
            use_buffer,
        )
    }
    fn read_bytes(
        &mut self,
        pos: i64,
        len: i32,
        output: &mut [u8],
        use_buffer: bool,
    ) -> Result<(), LuceneError> {
        // This closure is not expected to be called under any circumstances.

        self.read_buffer(pos, len, output, 1, |_| unreachable!(), use_buffer)
    }
    fn read_ints(
        &mut self,
        pos: i64,
        len: i32,
        output: &mut [i32],
        use_buffer: bool,
    ) -> Result<(), LuceneError> {
        self.read_buffer(
            pos,
            len,
            output,
            BitUtil::INT_BYTES as i32,
            LE::read_i32,
            use_buffer,
        )
    }
    fn read_shorts(
        &mut self,
        pos: i64,
        len: i32,
        output: &mut [i16],
        use_buffer: bool,
    ) -> Result<(), LuceneError> {
        self.read_buffer(
            pos,
            len,
            output,
            BitUtil::SHORT_BYTES as i32,
            LE::read_i16,
            use_buffer,
        )
    }
    fn read_floats(
        &mut self,
        pos: i64,
        len: i32,
        output: &mut [f32],
        use_buffer: bool,
    ) -> Result<(), LuceneError> {
        self.read_buffer(
            pos,
            len,
            output,
            BitUtil::FLOAT_BYTES as i32,
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
    /// * `Err(LuceneError)` - If an error occurs during reading, such as reaching EOF.
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
    /// * `LuceneError::eof` - If attempting to read beyond the end of the file or stream.
    ///
    /// # Note
    /// - The method assumes that the buffer's `refill` method ensures enough data is available for reading,
    ///   eliminating the need for additional checks.
    /// - When unaligned data remains in the buffer (e.g., when the available bytes are not a multiple of `type_size`),
    ///   the method copies the remaining bytes to the start of the buffer for further processing. This ensures that
    ///   subsequent reads start with aligned data. The maximum amount of data copied is `type_size - 1` bytes, which
    ///   is minimal. For example, if the largest type being read is `i64` (8 bytes), at most 7 bytes are copied.
    ///   Such small amounts of data copying have negligible performance impact.
    fn read_buffer<D, F>(
        &mut self,
        pos: i64,
        len: i32,
        target: &mut [D],
        type_size: i32,
        converter: F,
        use_buffer: bool,
    ) -> Result<(), LuceneError>
    where
        D: Copy,
        F: Fn(&[u8]) -> D,
    {
        // Calculate the total bytes to read based on the number of elements and the type size.
        let total_bytes = len * type_size;
        let mut elements_read = 0; // Tracks the number of elements read so far.
        let mut unaligned_bytes = 0; // Tracks bytes that cannot form a complete element.
                                     // Check if the position is within the current buffer range
        if pos >= self.buffer_start && pos < self.buffer_start + self.length as i64 {
            debug_assert!((pos - self.buffer_start) <= i32::MAX as i64);
            let buffer_offset = (pos - self.buffer_start) as i32;
            // Determine the number of bytes available in the buffer from the requested position.
            let available = self
                .buffer
                .remain_between(buffer_offset as u64, self.length as u64);
            // If the buffer contains all the data required for the request:
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
            // Calculate the number of aligned bytes and elements that can be fully read.
            debug_assert!(available <= i32::MAX as u64);
            let aligned_bytes = (available as i32 / type_size) * type_size;
            let aligned_elements = aligned_bytes / type_size;
            // Process aligned elements from the buffer.
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
            // Handle unaligned bytes that cannot form a complete element.
            unaligned_bytes = available as i32 - aligned_bytes;
            if unaligned_bytes > 0 {
                let buffer = self.buffer.get_mut();
                let unaligned_start = (buffer_offset + aligned_bytes) as usize;
                // Copy unaligned bytes to the start of the buffer, we would read these bytes later when buffer/temp_buffer was refilled again
                buffer.copy_within(
                    unaligned_start..unaligned_start + unaligned_bytes as usize,
                    0,
                );
            }
        }

        debug_assert!(self.buffer.position() <= u32::MAX as u64);
        // Calculate the remaining elements and bytes to read.
        let remaining_len = len - elements_read;
        let remaining_bytes = remaining_len * type_size;
        // If the buffer is used and the remaining bytes are less than the buffer size, refill the buffer.
        if use_buffer && remaining_bytes < self.buffer_size {
            let start = self.buffer_start + self.length as i64;
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
                return Err(LuceneError::eof(format!(
                    "read past EOF: expected {}, got {}",
                    remaining_len, readable_elements
                )));
            }

            return Ok(());
        }
        // If the buffer is not used or the remaining data exceeds the buffer size,
        // read directly from the underlying input
        let after = self.buffer_start + (self.length + remaining_bytes) as i64;
        if after > self.sub_index_input.length() {
            return Err(LuceneError::eof(format!("read past EOF: {}", self)));
        }
        // Prepare a temporary buffer to handle unaligned and remaining bytes.
        let mut temp_vec = vec![0; (remaining_bytes + unaligned_bytes) as usize];
        if unaligned_bytes > 0 {
            // If there are unaligned bytes left from the previous buffer,
            // copy them into the beginning of the temporary vector (`temp_vec`).
            //
            // These unaligned bytes are those that could not form a complete element
            // (e.g., a full integer or floating-point value) in the previous buffer.
            // They were left unprocessed and must be handled before reading new data
            // from the underlying input.
            //
            // The unaligned bytes are located at the end of the current buffer.
            // We copy them into the start of the temporary vector to ensure they
            // are preserved when the buffer is refilled with new data.
            temp_vec.copy_from(
                &self.buffer.get_ref()[(self.buffer_size - unaligned_bytes) as usize..],
                0,
            );
        }
        let mut temp_buffer = Cursor::new(temp_vec);
        temp_buffer.set_position(unaligned_bytes as u64);
        self.sub_index_input.read_internal(
            &mut temp_buffer,
            (remaining_bytes - unaligned_bytes) as i64,
            self.buffer_start + self.length as i64,
        )?;

        debug_assert!(temp_buffer.position() == remaining_bytes as u64);
        let src = temp_buffer.get_ref();
        Self::process_data(
            src,
            &mut target[elements_read as usize..elements_read as usize + remaining_len as usize],
            remaining_len as usize,
            type_size,
            &converter,
        );

        self.buffer_start = after;
        // we use temp_buffer to read underling data, so self.buffer is empty
        self.length = 0;
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
    fn process_data<D, F>(src: &[u8], dst: &mut [D], len: usize, type_size: i32, converter: &F)
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
            for (i, dst_item) in dst.iter_mut().enumerate().take(len) {
                let chunk_start = i * type_size as usize;
                *dst_item = converter(&src[chunk_start..chunk_start + type_size as usize]);
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
    /// * `Err(LuceneError)` - If an error occurs while seeking or refilling the buffer.
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
    fn resolve_position_in_buffer(&mut self, pos: i64, width: i32) -> Result<(), LuceneError> {
        let index: i64 = pos - self.buffer_start;
        if index >= 0 && index <= (self.length as i64 - width as i64) {
            return Ok(());
        }
        if index < 0 {
            // If we're moving backwards, then try and fill up the previous page rather than
            // starting again at the current pos, to avoid successive backwards reads reloading
            // the same data over and over again.  We also check that we can read `width`
            // bytes without going over the end of the buffer
            let temp_buffer_start = (self.buffer_start - self.buffer_size as i64)
                .max(pos + width as i64 - self.buffer_size as i64);
            self.buffer_start = temp_buffer_start.max(0);
            self.buffer_start = self.buffer_start.min(pos);
            self.pos = self.buffer_start;
        } else {
            self.buffer_start = pos;
            self.pos = pos;
        }
        self.length = 0;
        self.sub_index_input.seek_internal(self.buffer_start)?;
        self.refill(0, self.buffer_start)?;
        Ok(())
    }
    #[cfg(feature = "test_only")]
    pub fn get_sub_index_input(&self) -> &T {
        &self.sub_index_input
    }
}

impl<T> DataInput for BufferedIndexInput<T>
where
    T: BufferedIndexInputBase<Slice = BufferedIndexInput<T>>,
{
    fn read_byte(&mut self) -> Result<u8, LuceneError> {
        let mut bytes = [0; 1];
        self.read_bytes(self.pos, 1, &mut bytes, true)?;
        self.pos += 1;
        Ok(bytes[0])
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<(), LuceneError> {
        self.read_bytes_with_buffer(b, offset, len, true)
    }

    fn read_bytes_with_buffer(
        &mut self,
        b: &mut [u8],
        offset: i32,
        len: i32,
        use_buffer: bool,
    ) -> Result<(), LuceneError> {
        self.read_bytes(
            self.pos,
            len,
            &mut b[offset as usize..(offset + len) as usize],
            use_buffer,
        )?;
        self.pos += len as i64;
        Ok(())
    }

    fn read_short(&mut self) -> Result<i16, LuceneError> {
        let mut output = [0; 1];
        self.read_shorts(self.pos, 1, &mut output, true)?;
        self.pos += BitUtil::SHORT_BYTES as i64;
        Ok(output[0])
    }

    fn read_int(&mut self) -> Result<i32, LuceneError> {
        let mut output = [0; 1];
        self.read_ints(self.pos, 1, &mut output, true)?;
        self.pos += BitUtil::INT_BYTES as i64;
        Ok(output[0])
    }

    fn read_group_vint(&mut self, dst: &mut [i64], offset: i32) -> Result<(), LuceneError> {
        let remain =
            self.buffer
                .remain_between(self.buffer.position(), self.length as u64) as usize;
        debug_assert!(self.buffer.position() <= i64::MAX as u64);
        let len = GroupVIntUtil::read_group_vint_with_reader(
            self,
            remain as u64,
            self.buffer.position() as i64,
            dst,
            offset,
        )?;
        self.pos += len as i64;
        Ok(())
    }

    fn read_long(&mut self) -> Result<i64, LuceneError> {
        let mut output = [0; 1];
        self.read_longs(self.pos, 1, &mut output, true)?;
        self.pos += BitUtil::LONG_BYTES as i64;
        Ok(output[0])
    }
    fn read_longs(&mut self, dst: &mut [i64], offset: i32, len: i32) -> Result<(), LuceneError> {
        self.read_longs(
            self.pos,
            len,
            &mut dst[offset as usize..(offset + len) as usize],
            true,
        )?;
        self.pos += len as i64 * BitUtil::LONG_BYTES as i64;
        Ok(())
    }
    fn read_ints(&mut self, dst: &mut [i32], offset: i32, len: i32) -> Result<(), LuceneError> {
        self.read_ints(
            self.pos,
            len,
            &mut dst[offset as usize..(offset + len) as usize],
            true,
        )?;

        self.pos += len as i64 * BitUtil::INT_BYTES as i64;
        Ok(())
    }
    fn read_floats(&mut self, dst: &mut [f32], offset: i32, len: i32) -> Result<(), LuceneError> {
        self.read_floats(
            self.pos,
            len,
            &mut dst[offset as usize..(offset + len) as usize],
            true,
        )?;
        self.pos += len as i64 * BitUtil::FLOAT_BYTES as i64;
        Ok(())
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<(), LuceneError> {
        IndexInput::skip_bytes(self, num_bytes)
    }

    fn is_index_input(&self) -> bool {
        true
    }

    fn seek_in_data_input(&mut self, pos: i64) -> Result<(), LuceneError> {
        debug_assert!(self.is_index_input());
        IndexInput::seek(self, pos)
    }

    fn get_file_pointer_in_data_input(&self) -> i64 {
        debug_assert!(self.is_index_input());
        IndexInput::get_file_pointer(self)
    }
}

impl<T> Display for BufferedIndexInput<T>
where
    T: BufferedIndexInputBase<Slice = BufferedIndexInput<T>>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BufferedIndexInput({})", self.resource_desc)
    }
}

impl<T> Clone for BufferedIndexInput<T>
where
    T: BufferedIndexInputBase<Slice = BufferedIndexInput<T>>,
{
    fn clone(&self) -> Self {
        Self {
            buffer_size: self.buffer_size,
            resource_desc: self.resource_desc.clone(),
            buffer: self.buffer.clone(),
            sub_index_input: self.sub_index_input.clone(),
            buffer_start: self.buffer_start,
            pos: self.pos,
            length: self.length,
        }
    }
}

impl<T> IndexInput for BufferedIndexInput<T>
where
    T: BufferedIndexInputBase<Slice = BufferedIndexInput<T>>,
{
    fn get_file_pointer(&self) -> i64 {
        self.pos
    }

    fn seek(&mut self, pos: i64) -> Result<(), LuceneError> {
        if pos >= self.buffer_start && pos < (self.buffer_start + self.length as i64) {
            self.pos = pos;
        } else {
            self.pos = pos;
            self.buffer_start = pos;
            self.length = 0;
            self.sub_index_input.seek_internal(pos)?;
        }
        Ok(())
    }

    fn length(&self) -> i64 {
        self.sub_index_input.length()
    }

    type Slice = BufferedIndexInput<T>;

    fn slice(
        &self,
        slice_description: &str,
        offset: i64,
        length: i64,
    ) -> Result<Self::Slice, LuceneError> {
        self.sub_index_input
            .slice(slice_description, offset, length)
    }

    fn random_access_slice(&self, offset: i64, length: i64) -> Result<Self::Slice, LuceneError> {
        self.slice("random_access_slice", offset, length)
    }
}

impl<T> RandomAccessInput for BufferedIndexInput<T>
where
    T: BufferedIndexInputBase<Slice = BufferedIndexInput<T>>,
{
    fn length(&self) -> i64 {
        self.sub_index_input.length()
    }

    fn read_byte(&mut self, pos: i64) -> Result<u8, LuceneError> {
        let mut bytes = [0; 1];
        self.resolve_position_in_buffer(pos, 1)?;
        self.read_bytes(pos, 1, &mut bytes, true)?;
        Ok(bytes[0])
    }

    fn read_bytes(
        &mut self,
        pos: i64,
        b: &mut [u8],
        offset: i32,
        len: i32,
    ) -> Result<(), LuceneError> {
        self.resolve_position_in_buffer(pos, len)?;
        self.read_bytes(
            pos,
            len,
            &mut b[offset as usize..(offset + len) as usize],
            true,
        )?;
        Ok(())
    }

    fn read_short(&mut self, pos: i64) -> Result<i16, LuceneError> {
        let mut bytes = [0; BitUtil::SHORT_BYTES];
        self.resolve_position_in_buffer(pos, BitUtil::SHORT_BYTES as i32)?;
        self.read_shorts(pos, 1, &mut bytes, true)?;
        Ok(bytes[0])
    }

    fn read_int(&mut self, pos: i64) -> Result<i32, LuceneError> {
        let mut bytes = [0; BitUtil::INT_BYTES];
        self.resolve_position_in_buffer(pos, BitUtil::INT_BYTES as i32)?;
        self.read_ints(pos, 1, &mut bytes, true)?;
        Ok(bytes[0])
    }

    fn read_long(&mut self, pos: i64) -> Result<i64, LuceneError> {
        let mut bytes = [0; BitUtil::LONG_BYTES];
        self.resolve_position_in_buffer(pos, BitUtil::LONG_BYTES as i32)?;
        self.read_longs(pos, 1, &mut bytes, true)?;
        Ok(bytes[0])
    }

    fn pre_fetch(&mut self, _pos: i64, _len: i64) -> Result<(), LuceneError> {
        Ok(())
    }
}

#[allow(unused)]
struct SlicedIndexInput {}
