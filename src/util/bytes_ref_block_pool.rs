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
use crate::index::BytesRef;
use crate::util::accountable::Accountable;
use crate::util::bit_util::BitUtil;
use crate::util::bytes_ref_hash::BytesRefHash;
use crate::util::error::lucene_error::LuceneError;
use crate::util::{AllocatorEnum, ByteBlockPool, DirectAllocator, VecCopyOps};
use std::cell::RefCell;
use std::rc::Rc;

pub struct BytesRefBlockPool {
    byte_block_pool: Rc<RefCell<ByteBlockPool>>,
}
impl Default for BytesRefBlockPool {
    fn default() -> Self {
        Self::new()
    }
}

impl BytesRefBlockPool {
    // TODO: memory calculation not implemented
    const BASE_RAM_BYTES: i32 = 0;
    pub fn new() -> BytesRefBlockPool {
        BytesRefBlockPool {
            byte_block_pool: Rc::new(RefCell::new(ByteBlockPool::new(AllocatorEnum::DA(
                DirectAllocator::new(),
            )))),
        }
    }
    pub fn from_byte_block_pool(byte_block_pool: Rc<RefCell<ByteBlockPool>>) -> BytesRefBlockPool {
        BytesRefBlockPool { byte_block_pool }
    }
    pub fn byte_block_pool(&mut self) -> Rc<RefCell<ByteBlockPool>> {
        self.byte_block_pool.clone()
    }
    /// Resets this buffer to the empty state.
    pub fn reset(&mut self) -> Result<(), LuceneError> {
        self.byte_block_pool.borrow_mut().reset(false, false) // we don't need to 0-fill the buffers
    }

    /// Populates the given `BytesRef` with the term starting at `start`.
    pub fn fill_bytes_ref(&mut self, term: &mut BytesRef, start: i32) {
        let mut pool = self.byte_block_pool.borrow_mut();
        let block = pool.get_buffer(start >> ByteBlockPool::BYTE_BLOCK_SHIFT);
        let pos = (start & ByteBlockPool::BYTE_BLOCK_MASK) as usize;

        let (length, offset) = if (block[pos] & 0x80) == 0 {
            // Length is 1 byte
            (block[pos] as i32, (pos + 1) as i32)
        } else {
            // Length is 2 bytes
            (
                (BitUtil::get_i16_be(block, pos) & 0x7FFF) as i32,
                (pos + 2) as i32,
            )
        };

        term.bytes = vec![0; length as usize];
        term.bytes
            .copy_from(&block[offset as usize..(offset + length) as usize], 0);
        term.offset = 0;
        term.length = length;
        debug_assert!(term.length >= 0);
    }
    /// Add a term, returning the start position on the underlying `ByteBlockPool`.
    /// This can be used to read back the value using `fill_bytes_ref`.
    ///
    /// # See Also
    /// * `fill_bytes_ref(BytesRef, int)`
    pub fn add_bytes_ref(&mut self, bytes: &BytesRef) -> Result<i32, LuceneError> {
        let length = bytes.length;
        let len2 = 2 + bytes.length;
        let mut pool = self.byte_block_pool.borrow_mut();
        if len2 + pool.byte_upto > ByteBlockPool::BYTE_BLOCK_SIZE {
            if len2 > ByteBlockPool::BYTE_BLOCK_SIZE {
                return Err(LuceneError::max_bytes_length_exceeded(format!(
                    "bytes can be at most {} in length; got {}",
                    ByteBlockPool::BYTE_BLOCK_SIZE,
                    bytes.length
                )));
            }
            pool.next_buffer()?;
        }

        let buffer_upto = pool.byte_upto;
        let text_start = buffer_upto + pool.byte_offset;
        let buffer_index = pool.buffer_upto;
        let buffer = pool.get_buffer(buffer_index);

        // We first encode the length, followed by the bytes. Length is encoded as vInt,
        // but will consume 1 or 2 bytes at most (we reject too-long terms, above).
        let new_length = if length < 128 {
            // 1 byte to store length
            buffer[buffer_upto as usize] = length as u8;
            debug_assert!(length >= 0, "Length must be positive: {}", length);
            buffer.copy_from(
                &bytes.bytes[bytes.offset as usize..(bytes.offset + length) as usize],
                buffer_upto as usize + 1,
            );
            length + 1
        } else {
            // 2 byte to store length
            BitUtil::set_i16_be(buffer, buffer_upto as usize, (length | 0x8000) as i16);
            buffer.copy_from(
                &bytes.bytes[bytes.offset as usize..(bytes.offset + length) as usize],
                buffer_upto as usize + 2,
            );
            length + 2
        };
        pool.byte_upto += new_length;
        Ok(text_start)
    }
    /// Computes the hash of the BytesRef at the given start.
    pub fn hash(&mut self, start: i32) -> i32 {
        let offset = (start & ByteBlockPool::BYTE_BLOCK_MASK) as usize;
        let mut pool = self.byte_block_pool.borrow_mut();
        let bytes = pool.get_buffer(start >> ByteBlockPool::BYTE_BLOCK_SHIFT);

        let (len, pos) = if (bytes[offset] & 0x80) == 0 {
            // length is 1 byte
            (bytes[offset] as usize, offset + 1)
        } else {
            // length is 2 bytes (16-bit value, but only using lower 15 bits)
            let len = BitUtil::get_i16_be(bytes, offset) & 0x7FFF;
            (len as usize, offset + 2)
        };

        BytesRefHash::do_hash(bytes, pos, len)
    }
    /// Computes the equality between the BytesRef at the given start position and the provided BytesRef.
    pub fn equals(&mut self, start: i32, b: &BytesRef) -> bool {
        let pos = (start & ByteBlockPool::BYTE_BLOCK_MASK) as usize;
        let mut pool = self.byte_block_pool.borrow_mut();
        let bytes = pool.get_buffer(start >> ByteBlockPool::BYTE_BLOCK_SHIFT);

        let (length, offset) = if (bytes[pos] & 0x80) == 0 {
            // length is 1 byte
            (bytes[pos] as usize, pos + 1)
        } else {
            // length is 2 bytes (16-bit value, but only using lower 15 bits)
            let length = BitUtil::get_i16_be(bytes, pos) & 0x7FFF;
            (length as usize, pos + 2)
        };

        // Compare slices of bytes
        bytes[offset..offset + length] == b.bytes[b.offset as usize..(b.offset + b.length) as usize]
    }
}
impl Accountable for BytesRefBlockPool {
    fn ram_bytes_used(&self) -> i64 {
        let pool = self.byte_block_pool.borrow_mut();
        BytesRefBlockPool::BASE_RAM_BYTES as i64 + pool.ram_bytes_used()
    }
}
