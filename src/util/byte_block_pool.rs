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
use crate::index::{BytesRef, BytesRefBuilder};
use crate::util::{Counter, CounterEnum};
use std::cmp::min;

//todo
#[allow(dead_code)]
const BASE_RAM_BYTES: i64 = 0;
/**
 * Use this to find the index of the buffer containing a byte, given an offset to that byte.
 *
*/
const BYTE_BLOCK_SHIFT: i32 = 15;
/** The size of each buffer in the pool. */
pub const BYTE_BLOCK_SIZE: i32 = 1 << BYTE_BLOCK_SHIFT;
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
pub struct ByteBlockPool<'a> {
    buffers: Vec<Vec<u8>>,
    // Current head buffer's index
    buffer_upto: i32,
    allocator: AllocatorEnum<'a>,
    byte_offset: i32,
    byte_up_to: i32,
}
impl<'a> ByteBlockPool<'a> {
    pub fn new(allocator: AllocatorEnum<'a>) -> Self {
        ByteBlockPool {
            buffers: vec![],
            buffer_upto: -1,
            allocator,
            byte_offset: -BYTE_BLOCK_SIZE,
            byte_up_to: BYTE_BLOCK_SIZE,
        }
    }
    /**
     * Expert: Resets the pool to its initial state, while optionally reusing the first buffer.
     * Buffers that are not reused are reclaimed by Allocator#recycleByteBlocks`(vec<vec<u8>>`, i32,
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
    pub fn reset(&mut self, zero_fill_buffers: bool, reuse_first: bool) {
        if self.buffer_upto != -1 {
            if zero_fill_buffers {
                for i in 0..(self.buffer_upto + 1) as usize {
                    self.buffers[i].fill(0);
                }
            }
            if self.buffer_upto > 0 || !reuse_first {
                let offset = if reuse_first { 1 } else { 0 };
                self.allocator
                    .recycle_byte_blocks(&self.buffers, offset, self.buffer_upto + 1);
                for _i in offset as usize..(self.buffer_upto + 1) as usize {
                    self.buffers.pop();
                }
            }

            if reuse_first {
                self.buffer_upto = 0;
                self.byte_up_to = 0;
                self.byte_offset = 0;
            } else {
                self.buffer_upto = -1;
                self.byte_up_to = BYTE_BLOCK_SIZE;
                self.byte_offset = -BYTE_BLOCK_SIZE;
            }
        }
    }
    /**
     * Allocates a new buffer and advances the pool to it. This method should be called once after the
     * constructor to initialize the pool. In contrast to the constructor, a
     * ByteBlockPool#reset(bool, bool) call will advance the pool to its first buffer
     * immediately.
     */
    pub fn next_buffer(&mut self) -> Option<()> {
        if self.buffer_upto + 1 == self.buffers.len() as i32 {
            self.buffers.push(self.allocator.get_byte_block());
        }
        // Allocate new buffer and advance the pool to it
        self.buffer_upto += 1;
        self.byte_up_to = 0;
        self.byte_offset = self.byte_offset.checked_add(BYTE_BLOCK_SIZE)?;
        None
    }

    /**
     * Fill the provided BytesRef with the bytes at the specified offset and length. This will
     * avoid copying the bytes if the slice fits into a single block; otherwise, it uses the provided
     * BytesRefBuilder to copy bytes over.
     */
    pub fn set_bytes_ref(
        &self,
        builder: &mut BytesRefBuilder,
        result: &mut BytesRef,
        offset: i64,
        length: i32,
    ) {
        result.length = length;
        let buffer_index = (offset >> BYTE_BLOCK_SHIFT) as i32;
        let pos = (offset & BYTE_BLOCK_MASK as i64) as i32;
        if pos + length <= BYTE_BLOCK_SIZE {
            // Common case: The slice lives in a single block.
            result
                .bytes
                .clone_from(&self.buffers[buffer_index as usize]);
            result.offset = pos;
        } else {
            // builder.grow_no_copy(length);
            result.offset = 0;
            self.read_bytes(offset, &mut result.bytes, 0, length);
            builder.get().bytes.clone_from(&result.bytes);
        }
    }
    /** Appends the bytes in the provided BytesRef at the current position. */
    pub fn append_bytes_ref(&mut self, bytes: BytesRef) {
        self.append_range(bytes.bytes, bytes.offset, bytes.length);
    }
    /**
     * Append the bytes from a source ByteBlockPool at a given offset and length
     *
     * @param srcPool the source pool to copy from
     * @param srcOffset the source pool offset
     * @param length the number of bytes to copy
     */
    pub fn append_from_byte_block_pool(
        &mut self,
        src_pool: &ByteBlockPool,
        mut src_offset: i64,
        length: i32,
    ) {
        let mut bytes_left = length;
        while bytes_left > 0 {
            let buffer_left = BYTE_BLOCK_SIZE - self.byte_up_to;
            if bytes_left < buffer_left {
                // fits within current buffer
                self.append_bytes_single_buffer(src_pool, src_offset, bytes_left);
                break;
            } else {
                // fill up this buffer and move to next one
                if buffer_left > 0 {
                    self.append_bytes_single_buffer(src_pool, src_offset, buffer_left);
                    bytes_left -= buffer_left;
                    src_offset += buffer_left as i64;
                }
                self.next_buffer();
            }
        }
    }
    fn append_bytes_single_buffer(
        &mut self,
        src_pool: &ByteBlockPool,
        mut src_offset: i64,
        mut length: i32,
    ) {
        assert!(length <= BYTE_BLOCK_SIZE - self.byte_up_to);
        while length > 0 {
            let src_pos = src_offset & BYTE_BLOCK_MASK as i64;
            let bytes_to_copy = min(BYTE_BLOCK_SIZE - src_pos as i32, length);
            self.buffers[self.buffer_upto as usize]
                [self.byte_up_to as usize..(self.byte_up_to + bytes_to_copy) as usize]
                .copy_from_slice(
                    &src_pool.buffers[(src_offset >> BYTE_BLOCK_SHIFT) as usize]
                        [src_pos as usize..(src_pos + bytes_to_copy as i64) as usize],
                );
            length -= bytes_to_copy;
            src_offset += bytes_to_copy as i64;
            self.byte_up_to += bytes_to_copy;
        }
    }

    /**
     * Append the provided byte array at the current position.
     *
     * @param bytes the byte array to write
     */
    pub fn append(&mut self, bytes: Vec<u8>) {
        let length = bytes.len() as i32;
        self.append_range(bytes, 0, length);
    }
    /**
     * Append some portion of the provided byte array at the current position.
     *
     * @param bytes the byte array to write
     * @param offset the offset of the byte array
     * @param length the number of bytes to write
     */
    pub fn append_range(&mut self, bytes: Vec<u8>, mut offset: i32, length: i32) {
        let mut bytes_left = length;
        while bytes_left > 0 {
            let buffer_left = BYTE_BLOCK_SIZE - self.byte_up_to;
            if bytes_left < buffer_left {
                // fits within current buffer
                self.buffers[self.buffer_upto as usize]
                    [self.byte_up_to as usize..(self.byte_up_to + bytes_left) as usize]
                    .copy_from_slice(&bytes[offset as usize..(offset + bytes_left) as usize]);
                self.byte_up_to += bytes_left;
                break;
            } else {
                // fill up this buffer and move to next one
                if buffer_left > 0 {
                    self.buffers[self.buffer_upto as usize]
                        [self.byte_up_to as usize..(self.byte_up_to + buffer_left) as usize]
                        .copy_from_slice(&bytes[offset as usize..(offset + buffer_left) as usize]);
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
     * <p>Note: this method allows to copy across block boundaries.</p>
     */
    pub fn read_bytes(
        &self,
        offset: i64,
        bytes: &mut [u8],
        mut bytes_offset: i32,
        bytes_length: i32,
    ) -> Option<()> {
        let mut bytes_left = bytes_length;
        let mut buffer_index = (offset >> BYTE_BLOCK_SHIFT).checked_shr(0)? as usize;
        let mut pos = (offset & BYTE_BLOCK_MASK as i64) as i32;
        while bytes_left > 0 {
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
    pub fn read_byte(&self, offset: i64) -> Option<u8> {
        let buffer_index = (offset >> BYTE_BLOCK_SHIFT).checked_shr(0)? as usize;
        let pos = (offset & BYTE_BLOCK_MASK as i64) as i32;
        Some(self.buffers[buffer_index][pos as usize])
    }
    /** the current position (in absolute value) of this byte pool */
    pub fn get_position(&self) -> i64 {
        (self.buffer_upto * self.allocator.get_block_size() + self.byte_up_to) as i64
    }
    pub fn get_buffer(&self, buffer_index: i32) -> &Vec<u8> {
        &self.buffers[buffer_index as usize]
    }
    pub fn get_bytes_used(&self) -> i64 {
        self.allocator.get_used()
    }
}

/** allocating and freeing byte blocks. */
pub trait Allocator {
    fn recycle_byte_blocks(&mut self, blocks: &[Vec<u8>], start: i32, end: i32);
    fn get_byte_block(&mut self) -> Vec<u8>;
    fn get_block_size(&self) -> i32;
}

pub struct DirectAllocator {
    block_size: i32,
}
impl Default for DirectAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectAllocator {
    pub fn new() -> Self {
        DirectAllocator {
            block_size: BYTE_BLOCK_SIZE,
        }
    }
}
impl Allocator for DirectAllocator {
    fn recycle_byte_blocks(&mut self, _blocks: &[Vec<u8>], _start: i32, _end: i32) {}

    fn get_byte_block(&mut self) -> Vec<u8> {
        vec![0; self.block_size as usize]
    }

    fn get_block_size(&self) -> i32 {
        self.block_size
    }
}
pub struct DirectTrackingAllocator<'a> {
    block_size: i32,
    byte_used: &'a mut CounterEnum,
}
impl<'a> DirectTrackingAllocator<'a> {
    pub fn new(byte_used: &'a mut CounterEnum) -> Self {
        DirectTrackingAllocator {
            block_size: BYTE_BLOCK_SIZE,
            byte_used,
        }
    }
}
impl Allocator for DirectTrackingAllocator<'_> {
    fn recycle_byte_blocks(&mut self, _blocks: &[Vec<u8>], start: i32, end: i32) {
        self.byte_used
            .add_and_get(-(end - start) as i64 * self.block_size as i64);
    }

    fn get_byte_block(&mut self) -> Vec<u8> {
        self.byte_used.add_and_get(self.block_size as i64);
        vec![0; self.block_size as usize]
    }

    fn get_block_size(&self) -> i32 {
        self.block_size
    }
}

pub enum AllocatorEnum<'a> {
    DA(DirectAllocator),
    DTA(DirectTrackingAllocator<'a>),
}
impl<'a> AllocatorEnum<'a> {
    fn get_used(&self) -> i64 {
        match self {
            AllocatorEnum::DA(_da) => 0,
            AllocatorEnum::DTA(dta) => dta.byte_used.get(),
        }
    }
    fn recycle_byte_blocks(&mut self, blocks: &[Vec<u8>], start: i32, end: i32) {
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
    fn get_byte_block(&mut self) -> Vec<u8> {
        match self {
            AllocatorEnum::DA(da) => da.get_byte_block(),
            AllocatorEnum::DTA(dta) => dta.get_byte_block(),
        }
    }
}
