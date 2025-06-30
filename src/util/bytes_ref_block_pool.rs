/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::index::BytesRef;
use crate::util::access::Access;
use crate::util::accountable::Accountable;
use crate::util::allocator_byte::{DirectAllocatorByte, MTAllocatorByteEnum, STAllocatorByteEnum};
use crate::util::bit_util::BitUtil;
use crate::util::bytes_ref_hash::BytesRefHash;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{
    byte_block_pool_util, ByteBlockPool, ByteBlockPoolBorrow, ByteBlockPoolLock, CounterEnum,
    CounterEnumBorrow, CounterEnumLock, SliceCopyOps,
};

pub struct BytesRefBlockPool<C, B>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
{
    byte_block_pool: B,
    _phantom: PhantomData<C>,
}

impl BytesRefBlockPool<CounterEnumLock, ByteBlockPoolLock> {
    pub fn new_sync() -> BytesRefBlockPool<CounterEnumLock, ByteBlockPoolLock> {
        let allocator = MTAllocatorByteEnum::DA(DirectAllocatorByte::new());
        let pool = Arc::new(Mutex::new(ByteBlockPool::new_sync(allocator)));
        BytesRefBlockPool {
            byte_block_pool: pool,
            _phantom: Default::default(),
        }
    }
}
impl Default for BytesRefBlockPool<CounterEnumBorrow, ByteBlockPoolBorrow> {
    fn default() -> Self {
        Self::new()
    }
}

impl BytesRefBlockPool<CounterEnumBorrow, ByteBlockPoolBorrow> {
    pub fn new() -> BytesRefBlockPool<CounterEnumBorrow, ByteBlockPoolBorrow> {
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
    pub fn reset(&mut self) {
        self.byte_block_pool
            .access_mut(|byte_block_pool| byte_block_pool.reset(false, false))
    }

    /// Populates the given `BytesRef` with the term starting at `start`.
    pub fn fill_bytes_ref(&self, term: &mut BytesRef<Vec<u8>>, start: i32) {
        self.byte_block_pool.access_mut(|pool| {
            {
                let block = pool.get_buffer(start >> byte_block_pool_util::BYTE_BLOCK_SHIFT);
                let pos = (start & byte_block_pool_util::BYTE_BLOCK_MASK) as usize;

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
                term.length = length as usize;
            };
        })
    }
    /// Add a term, returning the start position on the underlying
    /// `ByteBlockPool`. This can be used to read back the value using
    /// `fill_bytes_ref`.
    ///
    /// # See Also
    /// * `fill_bytes_ref(BytesRef, int)`
    pub fn add_bytes_ref(&mut self, bytes: &BytesRef<Vec<u8>>) -> Result<i32> {
        let length = bytes.length as i32;
        let len2 = 2 + bytes.length as i32;
        self.byte_block_pool.access_mut(|pool| {
            if len2 + pool.byte_upto > byte_block_pool_util::BYTE_BLOCK_SIZE {
                if len2 > byte_block_pool_util::BYTE_BLOCK_SIZE {
                    return Err(LuceneError::max_bytes_length_exceeded(format!(
                        "bytes can be at most {} in length; got {}",
                        byte_block_pool_util::BYTE_BLOCK_SIZE,
                        bytes.length
                    )));
                }
                pool.next_buffer()?;
            }

            let buffer_upto = pool.byte_upto;
            let text_start = buffer_upto + pool.byte_offset;
            let buffer_index = pool.buffer_upto;
            let buffer = pool.get_buffer_mut(buffer_index);

            // We first encode the length, followed by the bytes. Length is
            // encoded as vInt, but will consume 1 or 2 bytes at
            // most (we reject too-long terms, above).
            let new_length = if length < 128 {
                // 1 byte to store length
                buffer[buffer_upto as usize] = length as u8;
                debug_assert!(length >= 0, "Length must be positive: {}", length);
                buffer.copy_from(
                    &bytes.bytes[bytes.offset..bytes.offset + length as usize],
                    buffer_upto as usize + 1,
                );
                length + 1
            } else {
                // 2 byte to store length
                BitUtil::set_i16_be(buffer, buffer_upto as usize, (length | 0x8000) as i16);
                buffer.copy_from(
                    &bytes.bytes[bytes.offset..bytes.offset + length as usize],
                    buffer_upto as usize + 2,
                );
                length + 2
            };
            pool.byte_upto += new_length;
            Ok(text_start)
        })
    }
    /// Computes the hash of the BytesRef at the given start.
    pub fn hash(&mut self, start: i32) -> i32 {
        let offset = (start & byte_block_pool_util::BYTE_BLOCK_MASK) as usize;
        self.byte_block_pool.access_mut(|pool| {
            let bytes = pool.get_buffer(start >> byte_block_pool_util::BYTE_BLOCK_SHIFT);

            let (len, pos) = if (bytes[offset] & 0x80) == 0 {
                // length is 1 byte
                (bytes[offset] as usize, offset + 1)
            } else {
                // length is 2 bytes (16-bit value, but only using lower 15
                // bits)
                let len = BitUtil::get_i16_be(bytes, offset) & 0x7FFF;
                (len as usize, offset + 2)
            };

            BytesRefHash::do_hash(bytes, pos, len)
        })
    }
    /// Computes the equality between the BytesRef at the given start position
    /// and the provided BytesRef.
    pub fn equals(&self, start: i32, b: &BytesRef<Vec<u8>>) -> bool {
        let pos = (start & byte_block_pool_util::BYTE_BLOCK_MASK) as usize;
        self.byte_block_pool.access_mut(|pool| {
            let bytes = pool.get_buffer(start >> byte_block_pool_util::BYTE_BLOCK_SHIFT);

            let (length, offset) = if (bytes[pos] & 0x80) == 0 {
                // length is 1 byte
                (bytes[pos] as usize, pos + 1)
            } else {
                // length is 2 bytes (16-bit value, but only using lower 15
                // bits)
                let length = BitUtil::get_i16_be(bytes, pos) & 0x7FFF;
                (length as usize, pos + 2)
            };

            // Compare slices of bytes
            bytes[offset..offset + length] == b.bytes[b.offset..(b.offset + b.length)]
        })
    }
}
impl<C, B> Accountable for BytesRefBlockPool<C, B>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        let result = self.byte_block_pool.access(|pool| pool.ram_bytes_used());
        if result.is_err() {
            // TODO:
            Ok(0)
        } else {
            result
        }
    }
}
// for single thread
pub type BytesRefBlockPoolBorrow =
    Rc<RefCell<BytesRefBlockPool<CounterEnumBorrow, ByteBlockPoolBorrow>>>;
// for multi thread
pub type BytesRefBlockPoolLock = Arc<Mutex<BytesRefBlockPool<CounterEnumLock, ByteBlockPoolLock>>>;
