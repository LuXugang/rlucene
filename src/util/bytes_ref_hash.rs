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
use crate::util::array_util::ArrayUtil;
use crate::util::bit_util::BitUtil;
use crate::util::bytes_ref_block_pool::BytesRefBlockPool;
use crate::util::error::lucene_error::LuceneError;
use crate::util::{Counter, CounterEnum, StringHelper, GOOD_FAST_HASH_SEED};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct BytesRefHash {
    pool: Rc<RefCell<BytesRefBlockPool>>,
    hash_size: i32,
    hash_half_size: i32,
    hash_mask: i32,
    count: i32,
    last_count: i32,
    ids: Vec<i32>,
    bytes_start_array: Rc<RefCell<BytesStartArrayEnum>>,
    bytes_used: Arc<Mutex<CounterEnum>>,
}
impl BytesRefHash {
    pub const DEFAULT_CAPACITY: i32 = 16;

    pub fn new() -> Self {
        let pool = Rc::new(RefCell::new(BytesRefBlockPool::new()));
        BytesRefHash::from_pool(pool)
    }
    pub fn from_pool(pool: Rc<RefCell<BytesRefBlockPool>>) -> Self {
        let bytes_start_array = Rc::new(RefCell::new(BytesStartArrayEnum::Direct(
            DirectBytesStartArray::new(BytesRefHash::DEFAULT_CAPACITY),
        )));
        BytesRefHash::from_bytes_start_array(pool, 16, bytes_start_array)
    }
    pub fn from_bytes_start_array(
        pool: Rc<RefCell<BytesRefBlockPool>>,
        capacity: i32,
        bytes_start_array: Rc<RefCell<BytesStartArrayEnum>>,
    ) -> Self {
        let bytes_used = bytes_start_array.borrow_mut().bytes_used();
        BytesRefHash {
            pool,
            hash_size: capacity,
            hash_half_size: capacity >> 1,
            hash_mask: capacity - 1,
            count: 0,
            last_count: -1,
            ids: vec![-1; capacity as usize],
            bytes_start_array,
            bytes_used,
        }
    }
    /// Returns the number of [`BytesRef`] values in this [`BytesRefHash`].
    ///
    /// # Returns
    /// The number of [`BytesRef`] values in this [`BytesRefHash`].
    pub fn size(&self) -> i32 {
        self.count
    }
    /// Populates and returns a [`BytesRef`] with the bytes for the given `bytesID`.
    ///
    /// # Note
    /// The given `bytesID` must be a positive integer less than the current size (`size()`).
    ///
    /// # Arguments
    /// - `bytesID`: The ID.
    /// - `ref`: The [`BytesRef`] to populate.
    ///
    /// # Returns
    /// The given [`BytesRef`] instance populated with the bytes for the given `bytesID`.
    pub fn get(&self, bytes_id: i32, ref_: &mut BytesRef) {
        debug_assert!(
            self.bytes_start_array.borrow_mut().byte_start().is_some(),
            "bytes_start is null - not initialized"
        );
        let mut byte_start_borrow = self.bytes_start_array.borrow_mut();
        let bytes_start = byte_start_borrow.byte_start().as_mut().unwrap();
        debug_assert!(
            (bytes_id as usize) < bytes_start.len(),
            "bytesID exceeds bytes_start len"
        );
        let value = bytes_start[bytes_id as usize];
        self.pool.borrow_mut().fill_bytes_ref(ref_, value);
    }

    /// Returns the ids array in arbitrary order. Valid ids start at offset 0 and end at a limit of `size()` - 1.
    ///
    /// # Note
    /// This is a destructive operation. `Clear()` must be called to reuse this `BytesRefHash` instance.
    pub fn compact(&mut self) -> &Vec<i32> {
        debug_assert!(
            self.bytes_start_array.borrow_mut().byte_start().is_some(),
            "bytes_start is null - not initialized"
        );

        let mut upto = 0;
        for i in 0..self.hash_size {
            if self.ids[i as usize] != -1 {
                if upto < i {
                    self.ids[upto as usize] = self.ids[i as usize];
                    self.ids[i as usize] = -1;
                }
                upto += 1;
            }
        }
        debug_assert!(upto == self.count);
        self.last_count = self.count;

        &self.ids
    }
    fn shrink(&mut self, target_size: i32) -> bool {
        // Cannot use ArrayUtil.shrink because we require power of 2:
        let mut new_size = self.hash_size;

        while new_size >= 8 && new_size / 4 > target_size {
            new_size /= 2;
        }

        if new_size != self.hash_size {
            // TODO: memory calculation not implemented
            self.bytes_used.lock().unwrap().add_and_get(0);
            self.hash_size = new_size;
            self.ids = vec![-1; self.hash_size as usize];
            self.hash_half_size = new_size / 2;
            self.hash_mask = new_size - 1;
            true
        } else {
            false
        }
    }
    /// Clears the [`BytesRef`] which maps to the given [`BytesRef`].
    pub fn clear_with_reset_pool(&mut self, reset_pool: bool) -> Result<(), LuceneError> {
        self.last_count = self.count;
        self.count = 0;

        if reset_pool {
            self.pool.borrow_mut().reset()?;
        }

        self.bytes_start_array.borrow_mut().clear()?;

        if self.last_count != -1 && self.shrink(self.last_count) {
            // shrink clears the hash entries
            return Ok(());
        }
        self.ids.fill(-1);
        Ok(())
    }
    pub fn clear(&mut self) -> Result<(), LuceneError> {
        self.clear_with_reset_pool(true)
    }

    /// Closes the `BytesRefHash` and releases all internally used memory.
    pub fn close(&mut self) -> Result<(), LuceneError> {
        self.clear_with_reset_pool(true)?;
        self.ids.clear();
        // TODO: memory calculation not implemented
        self.bytes_used.lock().unwrap().add_and_get(0);
        Ok(())
    }
    /// Adds a new [`BytesRef`].
    ///
    /// # Arguments
    /// - `bytes`: The bytes to hash.
    ///
    /// # Returns
    /// The id the given bytes are hashed to if there was no mapping for the given bytes,
    /// otherwise `(-(id) - 1)`. This guarantees that the return value will always be >= 0
    /// if the given bytes haven't been hashed before.
    ///
    /// # Errors
    /// Returns `MaxBytesLengthExceededException` if the given bytes are greater than 2 + [`ByteBlockPool::BYTE_BLOCK_SIZE`](ByteBlockPool::BYTE_BLOCK_SIZE).
    pub fn add(&mut self, bytes: &BytesRef) -> Result<i32, LuceneError> {
        assert!(
            self.bytes_start_array.borrow_mut().byte_start().is_some(),
            "Bytesstart is null - not initialized"
        );

        // final position
        let hash_pos = self.find_hash(bytes);
        let mut e = self.ids[hash_pos as usize];
        if e == -1 {
            {
                let mut byte_start_borrow = self.bytes_start_array.borrow_mut();
                let bytes_start = byte_start_borrow.byte_start().as_mut().unwrap();
                // new entry
                if self.count >= bytes_start.len() as i32 {
                    self.bytes_start_array.borrow_mut().grow()?;
                    assert!(
                        self.count < bytes_start.len() as i32 + 1,
                        "count: {} len: {}",
                        self.count,
                        bytes_start.len()
                    );
                }

                let byte_ref = self.pool.borrow_mut().add_bytes_ref(bytes)?;
                bytes_start[self.count as usize] = byte_ref;
                e = self.count;
                self.count += 1;
                assert_eq!(self.ids[hash_pos as usize], -1);
                self.ids[hash_pos as usize] = e;
            }

            if self.count == self.hash_half_size {
                self.rehash(2 * self.hash_size, true);
            }

            return Ok(e);
        }
        Ok(-(e + 1))
    }
    /// Returns the id of the given [`BytesRef`].
    ///
    /// # Arguments
    /// - `bytes`: The `BytesRef` to look for.
    ///
    /// # Returns
    /// The id of the given bytes, or `-1` if there is no mapping for the given bytes.
    pub fn find(&mut self, bytes: &BytesRef) -> i32 {
        let hash_pos = self.find_hash(bytes);
        self.ids[hash_pos as usize]
    }
    fn find_hash(&mut self, bytes: &BytesRef) -> i32 {
        assert!(
            self.bytes_start_array.borrow_mut().byte_start().is_some(),
            "bytesStart is null - not initialized"
        );

        let mut code = Self::do_hash(&bytes.bytes, bytes.offset as usize, bytes.length as usize);

        // final position
        let mut hash_pos = code & self.hash_mask;
        let mut e = self.ids[hash_pos as usize];
        let mut byte_start_borrow = self.bytes_start_array.borrow_mut();
        let bytes_start_ref = byte_start_borrow.byte_start().as_ref().unwrap();

        let mut pool = self.pool.borrow_mut();
        if e != -1
            && !pool.equals(bytes_start_ref[e as usize], bytes)
        {
            // Conflict; use linear probe to find an open slot
            // (see LUCENE-5604):
            loop {
                code += 1;
                hash_pos = code & self.hash_mask;
                e = self.ids[hash_pos as usize];
                if e == -1
                    || pool.equals(bytes_start_ref[e as usize], bytes)
                {
                    break;
                }
            }
        }

        hash_pos
    }
    /// Adds an "arbitrary" integer offset instead of a `BytesRef` term.
    ///
    /// This is used in the indexer to hold the hash for term vectors, because they do not
    /// redundantly store the byte[] term directly and instead reference the byte[] term already
    /// stored by the postings `BytesRefHash`.
    pub fn add_by_pool_offset(&mut self, offset: i32) -> Result<i32, LuceneError> {
        assert!(
            self.bytes_start_array.borrow_mut().byte_start().is_some(),
            "Bytesstart is null - not initialized"
        );

        // Final position
        let mut code = offset;
        let mut hash_pos = offset & self.hash_mask;
        let mut e = self.ids[hash_pos as usize];
        let length;
        {
            let mut byte_start_borrow = self.bytes_start_array.borrow_mut();
            let bytes_start = byte_start_borrow.byte_start().as_mut().unwrap();
            length = bytes_start.len();
            // Resolve hash conflicts
            while e != -1 && bytes_start[e as usize] != offset {
                code += 1;
                hash_pos = code & self.hash_mask;
                e = self.ids[hash_pos as usize];
            }
        }

        if e == -1 {
            // New entry
            {
                let mut byte_start_borrow = self.bytes_start_array.borrow_mut();
                let bytes_start = byte_start_borrow.byte_start().as_mut().unwrap();
                if self.count >= length as i32 {
                    self.bytes_start_array.borrow_mut().grow()?;
                    assert!(
                        self.count < bytes_start.len() as i32 + 1,
                        "count: {}, len: {}",
                        self.count,
                        bytes_start.len()
                    );
                }

                e = self.count;
                self.count += 1;
                bytes_start[e as usize] = offset;

                assert_eq!(self.ids[hash_pos as usize], -1);
                self.ids[hash_pos as usize] = e;
            }

            if self.count == self.hash_half_size {
                self.rehash(2 * self.hash_size, false);
            }

            return Ok(e);
        }

        Ok(-(e + 1))
    }
    /// Called when hash is too small (> 50% occupied) or too large (< 20% occupied).
    fn rehash(&mut self, new_size: i32, hash_on_data: bool) {
        let new_mask = new_size - 1;
        // TODO: memory calculation not implemented
        self.bytes_used.lock().unwrap().add_and_get(0);
        let mut new_hash = vec![-1; new_size as usize];
        let mut byte_start_borrow = self.bytes_start_array.borrow_mut();
        let bytes_start = byte_start_borrow.byte_start().as_ref().unwrap();
        let mut pool = self.pool.borrow_mut();
        for i in 0..self.hash_size {
            let e0 = self.ids[i as usize];
            if e0 != -1 {
                let code = if hash_on_data {
                    pool.hash(
                        bytes_start[e0 as usize],
                    )
                } else {
                    bytes_start[e0 as usize]
                };

                let mut hash_pos = code & new_mask;
                debug_assert!(hash_pos >= 0);
                if new_hash[hash_pos as usize] != -1 {
                    // Conflict; use linear probe to find an open slot
                    // (see LUCENE-5604):
                    let mut code = code;
                    loop {
                        code += 1;
                        hash_pos = code & new_mask;
                        if new_hash[hash_pos as usize] == -1 {
                            break;
                        }
                    }
                }
                new_hash[hash_pos as usize] = e0;
            }
        }

        self.hash_mask = new_mask;
        // TODO: memory calculation not implemented
        self.bytes_used.lock().unwrap().add_and_get(0);
        self.ids = new_hash;
        self.hash_size = new_size;
        self.hash_half_size = new_size / 2;
    }
    pub fn do_hash(bytes: &[u8], offset: usize, length: usize) -> i32 {
        StringHelper::murmurhash3_x86_32_with_byte(bytes, offset, length, *GOOD_FAST_HASH_SEED)
    }
    /// Reinitializes the [`BytesRefHash`] after a previous `clear()` call.
    /// If `clear()` has not been called previously, this method has no effect.
    pub fn reinit(&mut self) {
        let mut bytes_start_array = self.bytes_start_array.borrow_mut();
        if bytes_start_array.byte_start().is_none() {
            bytes_start_array.init();
        }

        if self.ids.is_empty() {
            self.ids = vec![-1; self.hash_size as usize];
            // TODO: memory calculation not implemented
            self.bytes_used.lock().unwrap().add_and_get(0);
        }
    }
}
impl Accountable for BytesRefHash {
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

/// Manages allocation of the per-term addresses.
pub trait BytesStartArray {
    /// Initializes the BytesStartArray. This call will allocate memory.
    ///
    /// # Returns
    /// The initialized bytes start array.
    fn init(&mut self) -> &Vec<i32>;

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
    fn byte_start(&mut self) -> &mut Option<Vec<i32>>;
}
/// A simple [`BytesStartArray`] that tracks memory allocation using a private `Counter` instance.
pub struct DirectBytesStartArray {
    init_size: i32,
    bytes_start: Option<Vec<i32>>,
    bytes_used: Arc<Mutex<CounterEnum>>,
}

impl DirectBytesStartArray {
    pub fn with_counter(init_size: i32, counter: Arc<Mutex<CounterEnum>>) -> Self {
        DirectBytesStartArray {
            init_size,
            bytes_start: None,
            bytes_used: counter,
        }
    }
    pub fn new(init_size: i32) -> Self {
        DirectBytesStartArray::with_counter(
            init_size,
            Arc::new(Mutex::new(CounterEnum::new_counter(false))),
        )
    }
}
impl BytesStartArray for DirectBytesStartArray {
    fn init(&mut self) -> &Vec<i32> {
        self.bytes_start = Some(vec![
            0;
            ArrayUtil::oversize(self.init_size, BitUtil::INT_BYTES as i32)
                as usize
        ]);
        self.bytes_start.as_ref().unwrap()
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

    fn byte_start(&mut self) -> &mut Option<Vec<i32>> {
        &mut self.bytes_start
    }
}

pub(crate) enum BytesStartArrayEnum {
    Direct(DirectBytesStartArray),
}
impl BytesStartArray for BytesStartArrayEnum {
    fn init(&mut self) -> &Vec<i32> {
        match self {
            BytesStartArrayEnum::Direct(d) => d.init(),
        }
    }

    fn grow(&mut self) -> Result<(), LuceneError> {
        match self {
            BytesStartArrayEnum::Direct(d) => d.grow(),
        }
    }

    fn clear(&mut self) -> Result<(), LuceneError> {
        match self {
            BytesStartArrayEnum::Direct(d) => d.clear(),
        }
    }

    fn bytes_used(&mut self) -> Arc<Mutex<CounterEnum>> {
        match self {
            BytesStartArrayEnum::Direct(d) => d.bytes_used(),
        }
    }

    fn byte_start(&mut self) -> &mut Option<Vec<i32>> {
        match self {
            BytesStartArrayEnum::Direct(d) => d.byte_start(),
        }
    }
}
