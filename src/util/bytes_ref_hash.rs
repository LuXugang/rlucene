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
use crate::util::array_util::ArrayUtil;
use crate::util::bit_util::BitUtil;
use crate::util::bytes_ref_block_pool::BytesRefBlockPool;
use crate::util::error::lucene_error::LuceneError;
use crate::util::{CounterEnum, StringHelper, GOOD_FAST_HASH_SEED};
use std::sync::{Arc, Mutex};

pub struct BytesRefHash {
    pool: BytesRefBlockPool,
    bytes_start: Option<Vec<i32>>,
    hash_size: i32,
    hash_half_size: i32,
    hash_mask: i32,
    count: i32,
    last_count: i32,
    ids: Option<Vec<i32>>,
    bytes_start_array: Box<dyn BytesStartArray>,
    bytes_used: Arc<Mutex<CounterEnum>>,
}
impl BytesRefHash {
    pub fn do_hash(bytes: &[u8], offset: usize, length: usize) -> i32 {
        StringHelper::murmurhash3_x86_32_with_byte(bytes, offset, length, *GOOD_FAST_HASH_SEED)
    }
}

/// Manages allocation of the per-term addresses.
pub trait BytesStartArray {
    /// Initializes the BytesStartArray. This call will allocate memory.
    ///
    /// # Returns
    /// The initialized bytes start array.
    fn init(&mut self);

    /// Grows the [`BytesStartArray`].
    ///
    /// # Returns
    /// The grown array.
    fn grow(&mut self) -> Result<(), LuceneError>;

    /// Clears the [`BytesStartArray`] and returns the cleared instance.
    ///
    /// # Returns
    /// The cleared instance, this might be `None`.
    fn clear(&mut self) -> Result<(), LuceneError>;

    /// A reference holding the number of bytes used by this `BytesStartArray`.
    /// The [`BytesRefHash`] uses this reference to track its memory usage.
    ///
    /// # Returns
    /// A reference holding the number of bytes used by this `BytesStartArray`.
    fn bytes_used(&mut self) -> Arc<Mutex<CounterEnum>>;
}
/// A simple [`BytesStartArray`] that tracks memory allocation using a private `Counter` instance.
pub struct DirectBytesStartArray {
    init_size: i32,
    bytes_start: Option<Vec<i32>>,
    bytes_used: Arc<Mutex<CounterEnum>>,
}

impl DirectBytesStartArray {
    pub fn new_with_counter(init_size: i32, counter: Arc<Mutex<CounterEnum>>) -> Self {
        DirectBytesStartArray {
            init_size,
            bytes_start: None,
            bytes_used: counter,
        }
    }
    pub fn new(init_size: i32) -> Self {
        DirectBytesStartArray::new_with_counter(
            init_size,
            Arc::new(Mutex::new(CounterEnum::new_counter(false))),
        )
    }
}
impl BytesStartArray for DirectBytesStartArray {
    fn init(&mut self) {
        self.bytes_start = Some(vec![
            0;
            ArrayUtil::oversize(self.init_size, BitUtil::INT_BYTES as i32)
                as usize
        ]);
    }

    fn grow(&mut self) -> Result<(), LuceneError> {
        debug_assert!(self.bytes_start.is_some());
        ArrayUtil::grow_with_len(self.bytes_start.as_mut().unwrap(), self.init_size)
    }

    fn clear(&mut self) -> Result<(), LuceneError> {
        self.bytes_start = None;
        Ok(())
    }

    fn bytes_used(&mut self) -> Arc<Mutex<CounterEnum>> {
        self.bytes_used.clone()
    }
}
