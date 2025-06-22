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
use crate::util::allocator_byte::AllocatorByteEnum;
use crate::util::int_block_pool::{AllocatorIntEnum, IntBlockPool};
use crate::util::{ByteBlockPool, ByteBlockPoolBorrow, CounterEnumBorrow};
use std::cell::RefCell;
use std::rc::Rc;

pub struct TermsHash {
    pub(crate) int_pool: Rc<RefCell<IntBlockPool<CounterEnumBorrow>>>,
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
