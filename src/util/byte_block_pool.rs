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
use std::cmp::min;

//todo
const BASE_RAM_BYTES: i64 = 0;
/**
 * Use this to find the index of the buffer containing a byte, given an offset to that byte.
 *
*/
const BYTE_BLOCK_SHIFT: i32 = 15;
/** The size of each buffer in the pool. */
const BYTE_BLOCK_SIZE: i32 = 1 << BYTE_BLOCK_SHIFT;
/** Use this to find the position of a global offset in a particular buffer.*/
const BYTE_BLOCK_MASK: i32 = BYTE_BLOCK_SIZE - 1;
/**
 * This class enables the allocation of fixed-size buffers and their management as part of a buffer
 * array. Allocation is done through the use of an `Allocator` which can be customized, e.g.
 * to allow recycling old buffers. There are methods for writing #append(BytesRef) and
 * reading from the buffers (e.g. `read_bytes(i64, vec<i16>, i32, i32)`, which handle
 * read/write operations across buffer boundaries.
 *
 * @lucene.internal
 */
struct ByteBlockPool {
    buffers: Vec<Vec<u8>>,
    // Current head buffer's index
    buffer_upto: usize,
    allocator: AllocatorEnum,
    buffer: i32,
    byte_offset: i32,
}
impl ByteBlockPool {
    fn new(allocator: AllocatorEnum) -> Self {
        ByteBlockPool {
            buffers: Vec::with_capacity(10),
            buffer_upto: 0,
            allocator,
            buffer: 0,
            byte_offset: -(BYTE_BLOCK_SIZE),
        }
    }
    /**
     * Expert: Resets the pool to its initial state, while optionally reusing the first buffer.
     * Buffers that are not reused are reclaimed by Allocator#recycleByteBlocks(vec<vec<u8>>, i32,
     * i32). Buffers can be filled with zeros before recycling them. This is useful if a slice pool
     * works on top of this byte pool and relies on the buffers being filled with zeros to find the
     * non-zero end of slices.
     *
     * @param zeroFillBuffers if true the buffers are filled with 0. This should be
     *     set to true if this pool is used with slices.
     * @param reuseFirst if true the first buffer will be reused and calling
     *     ByteBlockPool#next_buffer() is not needed after reset iff the block pool was used before
     *     ie. ByteBlockPool#next_Buffer() was called before.
     */
    fn reset(&mut self, zero_fill_buffers: bool, reuse_first: bool) {
        if !self.buffers.is_empty() {
            if zero_fill_buffers {
                for buffer in self.buffers.iter_mut() {
                    buffer.clear();
                }
            }
            if self.buffer_upto > 0 || !reuse_first {
                let offset = if reuse_first { 1 } else { 0 };
                self.allocator
                    .recycle_byte_blocks(&self.buffers, offset, self.buffer_upto as i32);
                for i in offset as usize..1 + self.buffer_upto {
                    self.buffers[i].clear();
                }
            }
            if reuse_first {
                self.buffer_upto = 1;
                self.byte_offset = 0;
            } else {
                self.buffer_upto = 0;
                self.byte_offset = -(BYTE_BLOCK_SIZE);
            }
        }
    }
    /**
     * Allocates a new buffer and advances the pool to it. This method should be called once after the
     * constructor to initialize the pool. In contrast to the constructor, a
     * ByteBlockPool#reset(bool, bool) call will advance the pool to its first buffer
     * immediately.
     */
    fn next_buffer(&mut self) {
        if self.buffer_upto == self.buffers.len() {
            self.buffers.push(Vec::new());
        }
        // Allocate new buffer and advance the pool to it
        self.buffers[self.buffer_upto] =
            Vec::with_capacity(self.allocator.get_byte_block() as usize);
        self.buffer_upto += 1;
        self.byte_offset = self.byte_offset.checked_add(BYTE_BLOCK_SIZE).unwrap();
    }

    /**
     * Fill the provided BytesRef with the bytes at the specified offset and length. This will
     * avoid copying the bytes if the slice fits into a single block; otherwise, it uses the provided
     * BytesRefBuilder to copy bytes over.
     */
    fn set_bytes_ref() {}

    /**
     * Append the provided byte array at the current position.
     *
     * @param bytes the byte array to write
     */
    fn append(&mut self, bytes: Vec<u8>) {
        let length = bytes.len() as i32;
        self.append_with_offset(bytes, 0, length);
    }
    /**
     * Append some portion of the provided byte array at the current position.
     *
     * @param bytes the byte array to write
     * @param offset the offset of the byte array
     * @param length the number of bytes to write
     */
    fn append_with_offset(&mut self, bytes: Vec<u8>, mut offset: i32, length: i32) {
        let mut bytes_left = length;
        let mut byte_up_to = self.buffers[self.buffer_upto].len() as i32;
        while bytes_left > 0 {
            let buffer_left = BYTE_BLOCK_SIZE - byte_up_to;
            if bytes_left < buffer_left {
                // fits within current buffer
                self.buffers[self.buffer_upto]
                    .extend_from_slice(&bytes[offset as usize..(offset + length) as usize]);
                byte_up_to += bytes_left;
                break;
            } else {
                // fill up this buffer and move to next one
                if buffer_left > 0 {
                    self.buffers[self.buffer_upto].extend_from_slice(
                        &bytes[offset as usize..(offset + buffer_left) as usize],
                    );
                }
                self.next_buffer();
                bytes_left -= buffer_left;
                offset += buffer_left;
            }
        }
    }
    /**
     * Reads bytes out of the pool starting at the given offset with the given length into the given
     * byte array at offset <code>off</code>.
     *
     * <p>Note: this method allows to copy across block boundaries.
     */
    fn read_bytes(
        &self,
        offset: i64,
        mut bytes: Vec<u8>,
        mut bytes_offset: i32,
        bytes_length: i32,
    ) -> Option<()> {
        let mut bytes_left = bytes_length;
        let mut buffer_index = (offset >> BYTE_BLOCK_SHIFT).checked_shr(0)? as usize;
        let mut pos = (offset & BYTE_BLOCK_MASK as i64) as i32;
        while bytes_left > 0 {
            assert!(buffer_index <= self.buffers.len());
            let chunk = min(BYTE_BLOCK_SIZE - pos, bytes_left);
            bytes[bytes_offset as usize..(bytes_offset + chunk) as usize]
                .copy_from_slice(&self.buffers[buffer_index][pos as usize..(pos + chunk) as usize]);
            bytes_offset += chunk;
            bytes_left -= chunk;
            buffer_index += 1;
            pos = 0;
        }
        None
    }
    /**
     * Read a single byte at the given offset
     *
     * @param offset the offset to read
     * @return the byte
     */
    fn read_byte(&self, offset: i64) -> Option<u8> {
        let buffer_index = (offset >> BYTE_BLOCK_SHIFT).checked_shr(0)? as usize;
        let pos = (offset & BYTE_BLOCK_MASK as i64) as i32;
        self.buffers[buffer_index][pos as usize];
        None
    }
    /** the current position (in absolute value) of this byte pool */
    fn get_position(&self) -> i64 {
        (self.buffer_upto as i32 * self.allocator.get_block_size()
            + (self.buffers[self.buffer_upto].len() as i32)) as i64
    }
    fn get_buffer(&self, buffer_index: i32) -> &Vec<u8> {
        &self.buffers[buffer_index as usize]
    }
}

/** allocating and freeing byte blocks. */
trait Allocator {
    fn recycle_byte_blocks(&self, blocks: &Vec<Vec<u8>>, start: i32, end: i32);
    fn get_byte_block(&self) -> i32;
    fn get_block_size(&self) -> i32;
}

struct DirectAllocator {}
impl Allocator for DirectAllocator {
    fn recycle_byte_blocks(&self, blocks: &Vec<Vec<u8>>, start: i32, end: i32) {
        todo!()
    }

    fn get_byte_block(&self) -> i32 {
        0
    }

    fn get_block_size(&self) -> i32 {
        todo!()
    }
}
struct DirectTrackingAllocator {}
impl Allocator for DirectTrackingAllocator {
    fn recycle_byte_blocks(&self, blocks: &Vec<Vec<u8>>, start: i32, end: i32) {
        todo!()
    }

    fn get_byte_block(&self) -> i32 {
        0
    }

    fn get_block_size(&self) -> i32 {
        todo!()
    }
}

pub enum AllocatorEnum {
    DA(DirectAllocator),
    DTA(DirectTrackingAllocator),
}
impl AllocatorEnum {
    fn recycle_byte_blocks(&self, blocks: &Vec<Vec<u8>>, start: i32, end: i32) {
        match self {
            AllocatorEnum::DA(da) => da.recycle_byte_blocks(blocks, start, end),
            AllocatorEnum::DTA(dta) => dta.recycle_byte_blocks(blocks, start, end),
        }
    }
    fn get_block_size(&self) -> i32 {
        match self {
            AllocatorEnum::DA(da) => da.get_block_size(),
            AllocatorEnum::DTA(dta) => dta.get_block_size(),
        }
    }
    fn get_byte_block(&self) -> i32 {
        match self {
            AllocatorEnum::DA(da) => da.get_byte_block(),
            AllocatorEnum::DTA(dta) => dta.get_byte_block(),
        }
    }
}
