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
use crate::core::util::allocator_byte::AllocatorByteEnum;
use crate::core::util::{ByteBlockPool, ByteBlockPoolLock, SharedCounter};
use parking_lot::Mutex;
use std::sync::Arc;

pub struct TermsHash {
    pub(crate) byte_pool: ByteBlockPoolLock,
    pub(crate) term_byte_pool: Option<ByteBlockPoolLock>,
    pub(crate) bytes_used: SharedCounter,
}
impl TermsHash {
    pub(crate) fn new(byte_block_allocator: AllocatorByteEnum, bytes_used: SharedCounter) -> Self {
        Self {
            byte_pool: Arc::new(Mutex::new(ByteBlockPool::new(byte_block_allocator))),
            term_byte_pool: None,
            bytes_used,
        }
    }
    pub(crate) fn reset(&mut self) {
        self.byte_pool.lock().reset(false, false)
    }
}
