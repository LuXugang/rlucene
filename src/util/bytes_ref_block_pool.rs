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
use crate::util::access::Access;
use crate::util::accountable::Accountable;
use crate::util::allocator_byte::{DirectAllocatorByte, MTAllocatorByteEnum, STAllocatorByteEnum};
use crate::util::bit_util::BitUtil;
use crate::util::bytes_ref_hash::BytesRefHash;
use crate::util::error::lucene_error::LuceneError;
use crate::util::{
    ByteBlockPool, CounterEnum, MTByteBlockPool, MTCounterEnum, STByteBlockPool, STCounterEnum,
    VecCopyOps,
};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct BytesRefBlockPool<C, B>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
{
    byte_block_pool: B,
    _phantom: PhantomData<C>,
}

impl BytesRefBlockPool<MTCounterEnum, MTByteBlockPool> {
    pub fn new_sync() -> BytesRefBlockPool<MTCounterEnum, MTByteBlockPool> {
        let allocator = MTAllocatorByteEnum::DA(DirectAllocatorByte::new());
        let pool = Arc::new(Mutex::new(ByteBlockPool::new_sync(allocator)));
        BytesRefBlockPool {
            byte_block_pool: pool,
            _phantom: Default::default(),
        }
    }
}
impl Default for BytesRefBlockPool<STCounterEnum, STByteBlockPool> {
    fn default() -> Self {
        Self::new()
    }
}

impl BytesRefBlockPool<STCounterEnum, STByteBlockPool> {
    pub fn new() -> BytesRefBlockPool<STCounterEnum, STByteBlockPool> {
        let allocator = STAllocatorByteEnum::DA(DirectAllocatorByte::new());
        let pool = Rc::new(RefCell::new(ByteBlockPool::new(allocator)));
        BytesRefBlockPool {
            byte_block_pool: pool,
            _phantom: Default::default(),
        }
    }
}

impl<C, B> BytesRefBlockPool<C, B>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
{
    // TODO: memory calculation not implemented
    #[allow(unused)]
    const BASE_RAM_BYTES: i32 = 0;
    pub fn from_byte_block_pool(byte_block_pool: B) -> BytesRefBlockPool<C, B> {
        BytesRefBlockPool {
            byte_block_pool,
            _phantom: Default::default(),
        }
    }
    pub fn byte_block_pool(&mut self) -> B {
        self.byte_block_pool.clone()
    }
    /// Resets this buffer to the empty state.
    pub fn reset(&mut self) -> Result<(), LuceneError> {
        self.byte_block_pool
            .with_ref_mut(|byte_block_pool| byte_block_pool.reset(false, false))?;
        Ok(())
    }

    /// Populates the given `BytesRef` with the term starting at `start`.
    pub fn fill_bytes_ref(&self, term: &mut BytesRef, start: i32) -> Result<(), LuceneError> {
        self.byte_block_pool.with_ref_mut(|pool| {
            Ok({
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
            })
        })
    }
    /// Add a term, returning the start position on the underlying `ByteBlockPool`.
    /// This can be used to read back the value using `fill_bytes_ref`.
    ///
    /// # See Also
    /// * `fill_bytes_ref(BytesRef, int)`
    pub fn add_bytes_ref(&mut self, bytes: &BytesRef) -> Result<i32, LuceneError> {
        let length = bytes.length;
        let len2 = 2 + bytes.length;
        self.byte_block_pool.with_ref_mut(|pool| {
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
        })
    }
    /// Computes the hash of the BytesRef at the given start.
    pub fn hash(&mut self, start: i32) -> Result<i32, LuceneError> {
        let offset = (start & ByteBlockPool::BYTE_BLOCK_MASK) as usize;
        self.byte_block_pool.with_ref_mut(|pool| {
            let bytes = pool.get_buffer(start >> ByteBlockPool::BYTE_BLOCK_SHIFT);

            let (len, pos) = if (bytes[offset] & 0x80) == 0 {
                // length is 1 byte
                (bytes[offset] as usize, offset + 1)
            } else {
                // length is 2 bytes (16-bit value, but only using lower 15 bits)
                let len = BitUtil::get_i16_be(bytes, offset) & 0x7FFF;
                (len as usize, offset + 2)
            };

            Ok(BytesRefHash::do_hash(bytes, pos, len))
        })
    }
    /// Computes the equality between the BytesRef at the given start position and the provided BytesRef.
    pub fn equals(&self, start: i32, b: &BytesRef) -> Result<bool, LuceneError> {
        let pos = (start & ByteBlockPool::BYTE_BLOCK_MASK) as usize;
        self.byte_block_pool.with_ref_mut(|pool| {
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
            Ok(bytes[offset..offset + length]
                == b.bytes[b.offset as usize..(b.offset + b.length) as usize])
        })
    }
}
impl<C, B> Accountable for BytesRefBlockPool<C, B>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
{
    fn ram_bytes_used(&self) -> i64 {
        let result = self
            .byte_block_pool
            .with_ref(|pool| Ok(pool.ram_bytes_used()));
        if result.is_err() {
            // TODO:
            0
        } else {
            result.unwrap()
        }
    }
}
// for single thread
pub type STBytesRefBlockPool = Rc<RefCell<BytesRefBlockPool<STCounterEnum, STByteBlockPool>>>;
// for multi thread
pub type MTBytesRefBlockPool = Arc<Mutex<BytesRefBlockPool<MTCounterEnum, MTByteBlockPool>>>;
