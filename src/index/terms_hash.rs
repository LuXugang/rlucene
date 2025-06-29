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
use crate::util::allocator_byte::AllocatorByteEnum;
use crate::util::int_block_pool::{AllocatorIntEnum, IntBlockPool, IntBlockPoolBorrow};
use crate::util::{ByteBlockPool, ByteBlockPoolBorrow, CounterEnumBorrow};
use std::cell::RefCell;
use std::rc::Rc;

pub struct TermsHash {
    pub(crate) int_pool: IntBlockPoolBorrow,
    pub(crate) byte_pool: ByteBlockPoolBorrow,
    pub(crate) term_byte_pool: Option<ByteBlockPoolBorrow>,
    pub(crate) bytes_used: CounterEnumBorrow,
}
impl TermsHash {
    pub(crate) fn new(
        int_block_allocator: AllocatorIntEnum<CounterEnumBorrow>,
        byte_block_allocator: AllocatorByteEnum<CounterEnumBorrow>,
        bytes_used: CounterEnumBorrow,
    ) -> Self {
        Self {
            int_pool: Rc::new(RefCell::new(IntBlockPool::with_allocator(
                int_block_allocator,
            ))),
            byte_pool: Rc::new(RefCell::new(ByteBlockPool::new(byte_block_allocator))),
            term_byte_pool: None,
            bytes_used,
        }
    }
    pub(crate) fn reset(&mut self) {
        self.int_pool.borrow_mut().reset(false, false);
        self.byte_pool.borrow_mut().reset(false, false)
    }
}
