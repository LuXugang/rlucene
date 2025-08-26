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
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{ByteBlockPool, CounterEnumLock, byte_block_pool_util};

/// struct that Posting and PostingVector use to write interleaved byte streams
/// into shared fixed-size byte[] arrays. The idea is to allocate slices of
/// increasing lengths. For example, the first slice is 5 bytes, the next slice
/// is 14, etc. We start by writing our bytes into the first 5 bytes. When
/// we hit the end of the slice, we allocate the next slice and then write the
/// address of the new slice into the last 4 bytes of the previous slice (the
/// "forwarding address").
///
/// Each slice is filled with 0's initially, and we mark the end with a non-zero
/// byte. This way the methods that are writing into the slice don't need to
/// record its length and instead allocate a new slice once they hit a non-zero
/// byte.
#[derive(Default)]
pub(crate) struct ByteSlicePool;
impl ByteSlicePool {
    /// An array holding the level sizes for byte slices. The first slice is 5
    /// bytes, the second is 14, and so on.
    pub(crate) const LEVEL_SIZE_ARRAY: [i32; 10] = [5, 14, 20, 30, 40, 40, 80, 80, 120, 200];

    /// An array holding indexes for the LEVEL_SIZE_ARRAY, to quickly navigate
    /// to the next slice level. These are encoded on 4 bits in the slice,
    /// so the values in this array should be less than 16.
    ///
    /// `NEXT_LEVEL_ARRAY[x] == x + 1`, except for the last element, where
    /// `NEXT_LEVEL_ARRAY[x] == x`, pointing at the maximum slice size.
    pub(crate) const NEXT_LEVEL_ARRAY: [i32; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 9];

    /// The first level size for new slices.
    pub(crate) const FIRST_LEVEL_SIZE: i32 = Self::LEVEL_SIZE_ARRAY[0];

    /// Allocates a new slice with the given size and level 0.
    ///
    /// # Returns
    /// The position where the slice starts
    pub fn new_slice(
        &mut self,
        size: i32,
        pool: &mut ByteBlockPool<CounterEnumLock>,
    ) -> Result<i32> {
        if size > byte_block_pool_util::BYTE_BLOCK_SIZE {
            return Err(LuceneError::illegal_argument(format!(
                "Slice size {} should be less than the block size {}",
                size,
                byte_block_pool_util::BYTE_BLOCK_SIZE
            )));
        }

        if pool.byte_upto > byte_block_pool_util::BYTE_BLOCK_SIZE - size {
            pool.next_buffer()?;
        }
        let upto = pool.byte_upto;
        pool.byte_upto += size;
        let buffer_upto = pool.buffer_upto;
        let byte_upto = pool.byte_upto as usize;
        pool.get_buffer_mut(buffer_upto)[byte_upto - 1] = 16; // This codifies level 0.
        Ok(upto)
    }

    /// Creates a new byte slice in continuation of the provided slice and
    /// returns its offset into the pool.
    ///
    /// # Parameters
    /// - `slice`: The current byte slice.
    /// - `upto`: The offset into the current slice, which is expected to point
    ///   to the last byte of the slice.
    ///
    /// # Returns
    /// The offset of the new slice in the pool.
    pub fn alloc_slice(
        &self,
        slice_index: i32,
        upto: i32,
        pool: &mut ByteBlockPool<CounterEnumLock>,
    ) -> Result<i32> {
        Ok(self.alloc_known_size_slice(slice_index, upto, pool)? >> 8)
    }
    /// Creates a new byte slice in continuation of the provided slice and
    /// returns its length and offset into the pool.
    ///
    /// # Parameters
    /// - `slice`: The current byte slice.
    /// - `upto`: The offset into the current slice, which is expected to point
    ///   to the last byte of the slice.
    ///
    /// # Returns
    /// A value where the lower 8 bits represent the new slice's length, and the
    /// other 24 bits represent the offset into the pool.
    pub fn alloc_known_size_slice(
        &self,
        slice_index: i32,
        upto: i32,
        pool: &mut ByteBlockPool<CounterEnumLock>,
    ) -> Result<i32> {
        let upto = upto as usize;
        let level;
        {
            let slice = pool.get_buffer(slice_index);
            level = slice[upto] & 15; // The last 4 bits codify the level.
        }
        let new_level = Self::NEXT_LEVEL_ARRAY[level as usize];
        let new_size = Self::LEVEL_SIZE_ARRAY[new_level as usize];
        // Maybe allocate another block
        if pool.byte_upto > byte_block_pool_util::BYTE_BLOCK_SIZE - new_size {
            pool.next_buffer()?;
        }

        let new_upto = pool.byte_upto;
        let offset = new_upto + pool.byte_offset;
        pool.byte_upto += new_size;

        // Copy forward the past 3 bytes (which we are about to overwrite with
        // the forwarding address). We actually copy 4 bytes at once
        // since VarHandles make it cheap.
        let past3_bytes;
        {
            let slice = pool.get_buffer(slice_index);
            past3_bytes = (BitUtil::get_i32_le(slice, upto - 3)) & 0xFFFFFF;
        }
        // Ensure we're not changing the content of `buffer` by setting 4 bytes
        // instead of 3. This should never happen since the next
        // `new_size` bytes must be equal to 0.
        let buffer_upto = pool.buffer_upto;
        {
            let current_buffer = pool.get_buffer_mut(buffer_upto);
            debug_assert!(current_buffer[new_upto as usize + 3] == 0);
            BitUtil::set_i32_le(current_buffer, new_upto as usize, past3_bytes);
        }

        // Write forwarding address at end of last slice:
        {
            let slice = pool.get_buffer_mut(slice_index);
            BitUtil::set_i32_le(slice, upto - 3, offset);
        }

        // Write new level:
        let byte_upto = pool.byte_upto as usize;
        let current_buffer = pool.get_buffer_mut(buffer_upto);
        current_buffer[byte_upto - 1] = (16 | new_level) as u8;

        Ok(((new_upto + 3) << 8) | (new_size - 3))
    }
}

#[cfg(test)]
mod tests {

    use parking_lot::Mutex;
    use rand::Rng;
    use std::sync::Arc;

    use crate::index::byte_slice_pool::ByteSlicePool;
    use crate::test::util::lucene_test_case::lucene_test_case_util::random;
    use crate::test::util::test_util::TestUtil;
    use crate::util::allocator_byte::{
        AllocatorByteEnum, DirectAllocatorByte, DirectTrackingAllocatorByte,
    };
    use crate::util::bit_util::BitUtil;
    use crate::util::error::lucene_error::{LuceneError, Result};
    use crate::util::{
        ByteBlockPool, ByteBlockPoolLock, CounterEnum, SliceCopyOps, byte_block_pool_util,
    };

    #[test]
    fn test_alloc_known_size_slice() -> Result<()> {
        let mut random = random();
        let byte_used = Arc::new(Mutex::new(CounterEnum::new_counter(false)));
        let allocator = AllocatorByteEnum::DTA(DirectTrackingAllocatorByte::new(byte_used));
        let mut block_pool = ByteBlockPool::new_sync(allocator);
        block_pool.next_buffer()?;
        let mut slice_pool = ByteSlicePool;

        for _ in 0..100 {
            let size: i32 = if rand::random::<bool>() {
                rand::rng().random_range(100..1000)
            } else {
                rand::rng().random_range(50000..100000)
            };

            let mut random_data = vec![0u8; size as usize];
            random.fill(&mut random_data[..]);

            let mut upto =
                slice_pool.new_slice(ByteSlicePool::FIRST_LEVEL_SIZE, &mut block_pool)?;

            let mut offset = 0;
            while offset < size as usize {
                let mut buffer_upto = block_pool.buffer_upto;
                if block_pool.get_buffer(buffer_upto)[upto as usize] & 16 == 0 {
                    block_pool.get_buffer_mut(buffer_upto)[upto as usize] = random_data[offset];
                    offset += 1;
                    upto += 1;
                } else {
                    let offset_and_length =
                        slice_pool.alloc_known_size_slice(buffer_upto, upto, &mut block_pool)?;
                    let slice_length = offset_and_length & 0xff;
                    upto = offset_and_length >> 8;
                    buffer_upto = block_pool.buffer_upto;
                    assert_ne!(
                        0,
                        block_pool.get_buffer(buffer_upto)[(upto + slice_length - 1) as usize]
                    );
                    assert_eq!(0, block_pool.get_buffer(buffer_upto)[upto as usize]);
                    let write_length =
                        std::cmp::min(slice_length as usize - 1, size as usize - offset);
                    let buffer = block_pool.get_buffer_mut(buffer_upto);
                    buffer.copy_from(&random_data[offset..offset + write_length], upto as usize);
                    offset += write_length;
                    debug_assert!(write_length <= i32::MAX as usize);
                    upto += write_length as i32;
                }
            }
        }
        Ok(())
    }
    #[test]
    fn test_alloc_large_slice() -> Result<()> {
        let allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
        let mut block_pool = ByteBlockPool::new_sync(allocator);
        let mut slice_pool = ByteSlicePool;
        assert_eq!(
            0,
            slice_pool.new_slice(byte_block_pool_util::BYTE_BLOCK_SIZE, &mut block_pool)?
        );
        {
            let buffer_upto = block_pool.buffer_upto;
            let buffer = block_pool.get_buffer_mut(buffer_upto).clone();
            let buffer_0 = block_pool.get_buffer_mut(0).clone();
            assert_eq!(buffer, buffer_0);
            block_pool.next_buffer()?;
        }
        let result =
            slice_pool.new_slice(byte_block_pool_util::BYTE_BLOCK_SIZE + 1, &mut block_pool);
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
        Ok(())
    }
    /// Creates a random byte array and writes it to a [`ByteSlicePool`] one
    /// slice at a time.
    struct SliceWriter {
        has_started: bool,

        block_pool: ByteBlockPoolLock,
        slice_pool: Arc<Mutex<ByteSlicePool>>,

        size: i32,
        random_data: Vec<u8>,
        data_offset: i32,

        slice: i32,
        slice_length: i32,
        slice_offset: i32,

        first_slice_offset: i32,
        first_slice: i32,
    }

    impl SliceWriter {
        /// Creates a new `SliceWriter` instance.
        pub fn new<R: Rng + ?Sized>(
            random: &mut R,
            slice_pool: Arc<Mutex<ByteSlicePool>>,
            block_pool: ByteBlockPoolLock,
        ) -> Self {
            let size: i32 = if random.random_bool(0.5) {
                // size < ByteBlockPool.BYTE_BLOCK_SIZE
                random.random_range(100..1000)
            } else {
                // size > ByteBlockPool.BYTE_BLOCK_SIZE
                random.random_range(50000..100000)
            };

            let mut random_data = vec![0u8; size as usize];
            random.fill(&mut random_data[..]);

            SliceWriter {
                has_started: false,
                block_pool,
                slice_pool,
                size,
                random_data,
                data_offset: 0,
                slice: 0,
                slice_length: 0,
                slice_offset: 0,
                first_slice_offset: 0,
                first_slice: 0,
            }
        }

        /// Writes the next slice of data.
        ///
        /// # Returns
        /// `true` if a slice is written and `false` if we're out of data to
        /// write.
        pub fn write_slice(&mut self) -> Result<bool> {
            // The first slice is special
            let mut slice_pool = self.slice_pool.lock();
            if !self.has_started {
                self.data_offset = 0;
                self.slice_length = ByteSlicePool::FIRST_LEVEL_SIZE;
                self.slice_offset =
                    slice_pool.new_slice(self.slice_length, &mut self.block_pool.lock())?;
                self.first_slice_offset = self.slice_offset;
                self.first_slice = self.block_pool.lock().buffer_upto;
                self.slice = self.first_slice;

                let write_length = std::cmp::min(self.size, self.slice_length - 1);
                let mut pool = self.block_pool.lock();
                let buffer = pool.get_buffer_mut(self.first_slice);
                buffer.copy_from(
                    &self.random_data
                        [self.data_offset as usize..(self.data_offset + write_length) as usize],
                    self.slice_offset as usize,
                );
                self.data_offset += write_length;
                self.has_started = true;
                return Ok(true);
            }

            // Have we written everything?
            if self.data_offset == self.size {
                return Ok(false);
            }

            let offset_and_length = slice_pool.alloc_known_size_slice(
                self.slice,
                self.slice_offset + self.slice_length - 1,
                &mut self.block_pool.lock(),
            )?;

            // No, write more
            #[allow(unused_assignments)]
            let mut current_pool_buffer = self.block_pool.lock().get_buffer_mut(self.slice);
            self.slice = self.block_pool.lock().buffer_upto;
            self.slice_length = offset_and_length & 0xff;
            self.slice_offset = offset_and_length >> 8;
            let write_length = std::cmp::min(self.size - self.data_offset, self.slice_length - 1);
            let mut pool = self.block_pool.lock();
            current_pool_buffer = pool.get_buffer_mut(self.slice);
            current_pool_buffer.copy_from(
                &self.random_data
                    [self.data_offset as usize..(self.data_offset + write_length) as usize],
                self.slice_offset as usize,
            );
            self.data_offset += write_length;
            Ok(true)
        }
    }
    /// Reads a sequence of slices into a byte array.
    struct SliceReader {
        has_started: bool,
        block_pool: ByteBlockPoolLock,
        slice_pool: Arc<Mutex<ByteSlicePool>>,

        size: i32,
        read_data: Vec<u8>,
        data_offset: i32,

        slice_length: i32,
        slice_offset: i32,

        slice: i32,
        slice_size_idx: usize,
    }

    impl SliceReader {
        /// Creates a new `SliceReader` instance.
        pub fn new(
            slice_pool: Arc<Mutex<ByteSlicePool>>,
            block_pool: ByteBlockPoolLock,
            size: i32,
            first_slice_offset: i32,
            first_slice: i32,
        ) -> Self {
            SliceReader {
                has_started: false,
                block_pool,
                slice_pool,
                size,
                read_data: vec![0u8; size as usize],
                data_offset: 0,
                slice_length: 0,
                slice_offset: first_slice_offset,
                slice: first_slice,
                slice_size_idx: 0,
            }
        }

        /// Reads the next slice of data.
        ///
        /// # Returns
        /// `true` if a slice is read and `false` if the entire sequence has
        /// been read.
        pub fn read_slice(&mut self) -> bool {
            // The first slice is special
            let mut block_pool = self.block_pool.lock();
            if !self.has_started {
                self.data_offset = 0;
                // Index into LEVEL_SIZE_ARRAY, allowing us to find the size of
                // the current slice
                self.slice_size_idx = 0;
                // 4 bytes are for the offset to the next slice, we can't use
                // them for data
                self.slice_length = ByteSlicePool::LEVEL_SIZE_ARRAY[self.slice_size_idx] - 4;
                let read_length = if self.data_offset + self.slice_length + 3 >= self.size {
                    // We are reading the last slice, no more offset, just a
                    // byte for the level
                    self.size - self.data_offset
                } else {
                    self.slice_length
                };

                let current_buffer = block_pool.get_buffer(self.slice);
                self.read_data.copy_from(
                    &current_buffer
                        [self.slice_offset as usize..(self.slice_offset + read_length) as usize],
                    self.data_offset as usize,
                );
                self.data_offset += read_length;
                self.slice_size_idx = std::cmp::min(
                    self.slice_size_idx + 1,
                    ByteSlicePool::LEVEL_SIZE_ARRAY.len() - 1,
                );
                self.has_started = true;
                return true;
            }

            // Have we read everything?
            if self.data_offset == self.size {
                return false;
            }

            // No, read more
            let mut slice_buffer = block_pool.get_buffer(self.slice);
            let global_slice_offset = BitUtil::get_i32_le(
                slice_buffer,
                (self.slice_offset + self.slice_length) as usize,
            ) & 0xFFFFFF;
            self.slice = global_slice_offset / byte_block_pool_util::BYTE_BLOCK_SIZE;
            self.slice_offset = global_slice_offset % byte_block_pool_util::BYTE_BLOCK_SIZE;
            self.slice_length = ByteSlicePool::LEVEL_SIZE_ARRAY[self.slice_size_idx] - 4;
            let read_length = if self.data_offset + self.slice_length + 3 >= self.size {
                // Reading the last slice
                self.size - self.data_offset
            } else {
                self.slice_length
            };

            slice_buffer = block_pool.get_buffer(self.slice);
            self.read_data.copy_from(
                &slice_buffer
                    [self.slice_offset as usize..(self.slice_offset + read_length) as usize],
                self.data_offset as usize,
            );
            self.data_offset += read_length;
            self.slice_size_idx = std::cmp::min(
                self.slice_size_idx + 1,
                ByteSlicePool::LEVEL_SIZE_ARRAY.len() - 1,
            );
            true
        }
    }
    #[test]
    fn test_random_interleaved_slices() -> Result<()> {
        let mut random = random();
        let byte_used = Arc::new(Mutex::new(CounterEnum::new_counter(false)));
        let allocator = AllocatorByteEnum::DTA(DirectTrackingAllocatorByte::new(byte_used));
        let pool = Arc::new(Mutex::new(ByteBlockPool::new_sync(allocator)));
        let slice_pool = Arc::new(Mutex::new(ByteSlicePool));

        let n_iterations = random.random_range(1..=3); // 1-3 iterations with buffer resets
        for _ in 0..n_iterations {
            let n = TestUtil::next_int(&mut random, 2, 3) as usize;
            let mut slice_writers: Vec<SliceWriter> = Vec::with_capacity(n);
            let mut slice_readers: Vec<SliceReader> = Vec::with_capacity(n);

            // Init slice writers
            for _ in 0..n {
                slice_writers.push(SliceWriter::new(
                    &mut random,
                    slice_pool.clone(),
                    pool.clone(),
                ));
            }
            // Write slices
            loop {
                let i = random.random_range(0..n);
                let succeeded = slice_writers[i].write_slice()?;
                if !succeeded {
                    slice_writers
                        .iter_mut()
                        .take(n)
                        .for_each(|writer| while writer.write_slice().unwrap_or(false) {});
                    break;
                }
            }

            // Init slice readers
            slice_writers.iter().take(n).for_each(|writer| {
                slice_readers.push(SliceReader::new(
                    slice_pool.clone(),
                    pool.clone(),
                    writer.size,
                    writer.first_slice_offset,
                    writer.first_slice,
                ));
            });

            // Read slices
            loop {
                let i = rand::rng().random_range(0..n);
                let succeeded = slice_readers[i].read_slice();
                if !succeeded {
                    for j in slice_readers.iter_mut().take(n) {
                        while j.read_slice() {}
                    }
                    break;
                }
            }

            // Compare written data with read data
            for i in 0..n {
                assert_eq!(slice_writers[i].random_data, slice_readers[i].read_data);
            }

            // We don't rely on the buffers being filled with zeros because the
            // SliceWriter keeps the slice length as state, but
            // ByteSlicePool.allocKnownSizeSlice asserts on zeros in the
            // buffer.
            pool.lock().reset(true, rand::rng().random_bool(0.5));
        }

        Ok(())
    }
}
