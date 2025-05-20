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
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::index::terms_hash_per_field::{
    MTPostingsArrayWrapper, PostingsArrayWrapper, PostingsBytesStartArray, STPostingsArrayWrapper,
};
use crate::index::{BytesRef, BytesRefBuilder};
use crate::util::access::Access;
use crate::util::accountable::Accountable;
use crate::util::allocator_byte::{AllocatorByteEnum, DirectAllocatorByte};
use crate::util::array_util::ArrayUtil;
use crate::util::bit_util::BitUtil;
use crate::util::bytes_ref_block_pool::BytesRefBlockPool;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{
    ByteBlockPool, ByteBlockPoolBorrow, ByteBlockPoolLock, BytesRefComparator, Comparator, Counter,
    CounterEnum, CounterEnumBorrow, CounterEnumLock, MSBRadixSorter, MSBRadixSorterBase, Natural,
    Sorter, StringHelper, StringSorter, StringSorterBase, GOOD_FAST_HASH_SEED, HISTOGRAM_SIZE,
    LEVEL_THRESHOLD,
};

/// `BytesRefHash` is a special purpose hash-map like data structure optimized
/// for `BytesRef` instances. `BytesRefHash` maintains mappings of byte arrays
/// to IDs (`Map<BytesRef, int>`), storing the hashed bytes efficiently in
/// continuous storage. The mapping to the ID is encapsulated inside
/// `BytesRefHash` and is guaranteed to be increased for each added `BytesRef`.
///
/// # Note
/// - The maximum capacity `BytesRef` instance passed to
///   [`add`](BytesRefHash::add) must not be longer than
///   [`ByteBlockPool::BYTE_BLOCK_SIZE`](ByteBlockPool) - 2.
/// - The internal storage is limited to 2GB total byte storage.
///
/// [`ByteBlockPool::BYTE_BLOCK_SIZE`]: ByteBlockPool::BYTE_BLOCK_SIZE
pub(crate) struct BytesRefHash<C, B, A, P>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C, P>>,
    P: Access<PostingsArrayWrapper>,
{
    pool: BytesRefBlockPool<C, B>,
    hash_size: i32,
    hash_half_size: i32,
    hash_mask: i32,
    pub(crate) count: i32,
    last_count: i32,
    pub ids: Vec<i32>,
    bytes_start_array: A,
    bytes_used: C,
    _phantom1: PhantomData<B>,
    _phantom2: PhantomData<P>,
}
#[allow(unused)]
impl MTBytesRefHash {
    pub const DEFAULT_CAPACITY: i32 = 16;
    pub fn new_sync() -> Self {
        let allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
        let pool = Arc::new(Mutex::new(ByteBlockPool::new_sync(allocator)));
        BytesRefHash::from_pool_sync(pool)
    }
    pub fn from_pool_sync(pool: ByteBlockPoolLock) -> Self {
        let bytes_start_array = Arc::new(Mutex::new(BytesStartArrayEnum::Direct(
            DirectBytesStartArray::new_sync(BytesRefHash::DEFAULT_CAPACITY),
        )));
        BytesRefHash::from_bytes_start_array(pool, 16, bytes_start_array)
    }
}
#[allow(unused)]
impl STBytesRefHash {
    pub fn new() -> Self {
        let allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
        let pool = Rc::new(RefCell::new(ByteBlockPool::new(allocator)));
        BytesRefHash::from_pool(pool)
    }
    pub fn from_pool(pool: ByteBlockPoolBorrow) -> Self {
        let bytes_start_array = Rc::new(RefCell::new(BytesStartArrayEnum::Direct(
            DirectBytesStartArray::new(BytesRefHash::DEFAULT_CAPACITY),
        )));
        BytesRefHash::from_bytes_start_array(pool, 16, bytes_start_array)
    }
    pub fn do_hash(bytes: &[u8], offset: usize, length: usize) -> i32 {
        StringHelper::murmurhash3_x86_32_with_byte(bytes, offset, length, *GOOD_FAST_HASH_SEED)
    }
}

impl<C, B, A, P> BytesRefHash<C, B, A, P>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C, P>>,
    P: Access<PostingsArrayWrapper>,
{
    pub fn from_bytes_start_array(pool: B, capacity: i32, bytes_start_array: A) -> Self {
        let bytes_used = bytes_start_array.access_mut(|bytes_start_array| {
            bytes_start_array.init();
            bytes_start_array.bytes_used()
        });
        let ref_pool = BytesRefBlockPool::from_byte_block_pool(pool);
        BytesRefHash {
            pool: ref_pool,
            hash_size: capacity,
            hash_half_size: capacity >> 1,
            hash_mask: capacity - 1,
            count: 0,
            last_count: -1,
            ids: vec![-1; capacity as usize],
            bytes_start_array,
            bytes_used,
            _phantom1: Default::default(),
            _phantom2: Default::default(),
        }
    }
    /// Returns the number of [`BytesRef`] values in this [`BytesRefHash`].
    ///
    /// # Returns
    /// The number of [`BytesRef`] values in this [`BytesRefHash`].
    pub fn size(&self) -> i32 {
        self.count
    }
    /// Populates and returns a [`BytesRef`] with the bytes for the given
    /// `bytesID`.
    ///
    /// # Note
    /// The given `bytesID` must be a positive integer less than the current
    /// size (`size()`).
    ///
    /// # Arguments
    /// - `bytesID`: The ID.
    /// - `ref`: The [`BytesRef`] to populate.
    ///
    /// # Returns
    /// The given [`BytesRef`] instance populated with the bytes for the given
    /// `bytesID`.
    pub fn get(&self, bytes_id: i32, ref_: &mut BytesRef<Vec<u8>>) {
        self.bytes_start_array.access_mut(|bytes_start_array| {
            debug_assert!(
                bytes_start_array.len() > 0,
                "bytes_start is null - not initialized"
            );
            debug_assert!(
                (bytes_id as usize) < bytes_start_array.len(),
                "bytesID exceeds bytes_start len"
            );
            let value = bytes_start_array.get_value(bytes_id as usize);
            self.pool.fill_bytes_ref(ref_, value)
        })
    }

    /// Returns the id array in arbitrary order. Valid ids start at offset 0 and
    /// end at a limit of `size()` - 1.
    ///
    /// # Note
    /// This is a destructive operation. `Clear()` must be called to reuse this
    /// `BytesRefHash` instance.
    pub fn compact(&mut self) -> &Vec<i32> {
        debug_assert!(
            self.bytes_start_array
                .access(|bytes_start_array| bytes_start_array.len() > 0),
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
    /// Returns the values array sorted by the referenced byte values.
    pub fn sort(&mut self) -> Result<()> {
        let compact = self.compact();
        let mut length = compact.len();
        let tmp_offset = self.count;
        let sub_sorter = StringSorterImpl::new(
            tmp_offset,
            &mut self.ids,
            &mut self.pool,
            self.bytes_start_array.clone(),
        );
        let mut sorter = StringSorter::new(sub_sorter, Natural::default());
        sorter.sort(0, self.count)?;
        debug_assert!(
            (self.count * 2) as usize <= length,
            "We need load factor <= 0.5f to speed up this sort"
        );
        length = self.ids.len();
        for i in tmp_offset as usize..length {
            self.ids[i] = -1;
        }
        Ok(())
    }
    fn shrink(&mut self, target_size: i32) -> bool {
        // Cannot use ArrayUtil.shrink because we require power of 2:
        let mut new_size = self.hash_size;

        while new_size >= 8 && new_size / 4 > target_size {
            new_size /= 2;
        }

        if new_size != self.hash_size {
            // TODO: memory calculation not implemented
            self.bytes_used
                .access_mut(|bytes_used| bytes_used.add_and_get(0));
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
    pub fn clear_with_reset_pool(&mut self, reset_pool: bool) {
        self.last_count = self.count;
        self.count = 0;

        if reset_pool {
            self.pool.reset();
        }

        self.bytes_start_array.access_mut(|bytes_start_array| {
            bytes_start_array.clear();
        });

        if self.last_count != -1 && self.shrink(self.last_count) {
            // shrink clears the hash entries
            return;
        }
        self.ids.fill(-1);
    }
    pub fn clear(&mut self) {
        self.clear_with_reset_pool(true)
    }

    /// Closes the `BytesRefHash` and releases all internally used memory.
    #[allow(unused)]
    pub fn close(&mut self) {
        self.clear_with_reset_pool(true);
        self.ids.clear();
        // TODO: memory calculation not implemented
        self.bytes_used.access_mut(|bytes_used| {
            bytes_used.add_and_get(0);
        });
    }
    /// Adds a new [`BytesRef`].
    ///
    /// # Arguments
    /// - `bytes`: The bytes to hash.
    ///
    /// # Returns
    /// The id the given bytes are hashed to if there was no mapping for the
    /// given bytes, otherwise `(-(id) - 1)`. This guarantees that the
    /// return value will always be >= 0 if the given bytes haven't been
    /// hashed before.
    ///
    /// # Errors
    /// Returns `MaxBytesLengthExceededException` if the given bytes are greater
    /// than 2 + [`ByteBlockPool::BYTE_BLOCK_SIZE`].
    pub fn add(&mut self, bytes: &BytesRef<Vec<u8>>) -> Result<i32> {
        debug_assert!(
            self.bytes_start_array
                .access(|bytes_start_array| bytes_start_array.len() > 0),
            "Bytesstart is null - not initialized"
        );

        // final position
        let hash_pos = self.find_hash(bytes);
        let mut e = self.ids[hash_pos as usize];
        if e == -1 {
            {
                self.bytes_start_array.access_mut(|bytes_start_array| {
                    let length = bytes_start_array.len();
                    // new entry
                    if self.count as usize >= length {
                        bytes_start_array.grow()?;
                        debug_assert!(
                            (self.count as usize) < bytes_start_array.len() + 1,
                            "count: {} len: {}",
                            self.count,
                            bytes_start_array.len()
                        );
                    }

                    let byte_ref = self.pool.add_bytes_ref(bytes)?;
                    bytes_start_array.set_value(self.count as usize, byte_ref);
                    // Help the compiler infer types.
                    Ok::<(), LuceneError>(())
                })?;
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
    /// The id of the given bytes, or `-1` if there is no mapping for the given
    /// bytes.
    pub fn find(&self, bytes: &BytesRef<Vec<u8>>) -> i32 {
        let hash_pos = self.find_hash(bytes);
        self.ids[hash_pos as usize]
    }
    fn find_hash(&self, bytes: &BytesRef<Vec<u8>>) -> i32 {
        self.bytes_start_array.access_mut(|bytes_start_array| {
            debug_assert!(
                bytes_start_array.len() > 0,
                "bytesStart is null - not initialized"
            );

            let mut code = BytesRefHash::do_hash(&bytes.bytes, bytes.offset, bytes.length);

            // final position
            let mut hash_pos = code & self.hash_mask;
            let mut e = self.ids[hash_pos as usize];

            if e != -1
                && !self
                    .pool
                    .equals(bytes_start_array.get_value(e as usize), bytes)
            {
                // Conflict; use linear probe to find an open slot
                // (see LUCENE-5604):
                loop {
                    code += 1;
                    hash_pos = code & self.hash_mask;
                    e = self.ids[hash_pos as usize];
                    if e == -1
                        || self
                            .pool
                            .equals(bytes_start_array.get_value(e as usize), bytes)
                    {
                        break;
                    }
                }
            }
            hash_pos
        })
    }
    /// Adds an "arbitrary" integer offset instead of a `BytesRef` term.
    ///
    /// This is used in the indexer to hold the hash for term vectors, because
    /// they do not redundantly store the byte[] term directly and instead
    /// reference the byte[] term already stored by the postings
    /// `BytesRefHash`.
    pub fn add_by_pool_offset(&mut self, offset: i32) -> Result<i32> {
        debug_assert!(
            self.bytes_start_array
                .access(|bytes_start_array| bytes_start_array.len() > 0),
            "Bytesstart is null - not initialized"
        );

        // Final position
        let mut code = offset;
        let mut hash_pos = offset & self.hash_mask;
        let mut e = self.ids[hash_pos as usize];
        let length = self.bytes_start_array.access_mut(|bytes_start_array| {
            let length = bytes_start_array.len();
            // Resolve hash conflicts
            while e != -1 && bytes_start_array.get_value(e as usize) != offset {
                code += 1;
                hash_pos = code & self.hash_mask;
                e = self.ids[hash_pos as usize];
            }
            length
        });

        if e == -1 {
            // New entry
            self.bytes_start_array.access_mut(|bytes_start_array| {
                if self.count as usize >= length {
                    bytes_start_array.grow()?;
                    debug_assert!(
                        self.count < bytes_start_array.len() as i32 + 1,
                        "count: {}, len: {}",
                        self.count,
                        bytes_start_array.len()
                    );
                }

                e = self.count;
                self.count += 1;
                bytes_start_array.set_value(e as usize, offset);

                assert_eq!(self.ids[hash_pos as usize], -1);
                self.ids[hash_pos as usize] = e;
                // Help the compiler infer types.
                Ok::<(), LuceneError>(())
            })?;

            if self.count == self.hash_half_size {
                self.rehash(2 * self.hash_size, false);
            }

            return Ok(e);
        }

        Ok(-(e + 1))
    }
    /// Called when hash is too small (> 50% occupied) or too large (< 20%
    /// occupied).
    fn rehash(&mut self, new_size: i32, hash_on_data: bool) {
        let new_mask = new_size - 1;
        // TODO: memory calculation not implemented
        self.bytes_used.access_mut(|bytes_used| {
            bytes_used.add_and_get(0);
        });
        let mut new_hash = vec![-1; new_size as usize];
        self.bytes_start_array.access_mut(|bytes_start_array| {
            for i in 0..self.hash_size {
                let e0 = self.ids[i as usize];
                if e0 != -1 {
                    let code = if hash_on_data {
                        self.pool.hash(bytes_start_array.get_value(e0 as usize))
                    } else {
                        bytes_start_array.get_value(e0 as usize)
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
        });
        self.hash_mask = new_mask;
        // TODO: memory calculation not implemented
        self.bytes_used.access_mut(|bytes_used| {
            bytes_used.add_and_get(0);
        });
        self.ids = new_hash;
        self.hash_size = new_size;
        self.hash_half_size = new_size / 2;
    }

    /// Reinitializes the [`BytesRefHash`] after a previous `clear()` call.
    /// If `clear()` has not been called previously, this method has no effect.
    pub fn reinit(&mut self) {
        self.bytes_start_array.access_mut(|bytes_start_array| {
            if bytes_start_array.len() == 0 {
                bytes_start_array.init();
            }
        });

        if self.ids.is_empty() {
            self.ids = vec![-1; self.hash_size as usize];
            // TODO: memory calculation not implemented
            self.bytes_used
                .access_mut(|bytes_used| bytes_used.add_and_get(0));
        }
    }
    // pub fn set_bytes_start_array(&mut self, bytes_start_array:
    // Rc<RefCell<BytesStartArrayEnum>>) {     self.bytes_start_array =
    // bytes_start_array; }
    /// Returns the `bytesStart` offset into the internally used
    /// `SingleThreadedByteBlockPool` for the given `bytes_id`.
    ///
    /// # Arguments
    /// * `bytes_id` - The ID to look up.
    ///
    /// # Returns
    /// The `bytesStart` offset into the internally used
    /// `SingleThreadedByteBlockPool` for the given ID.
    #[allow(dead_code)]
    #[cfg(feature = "test_only")]
    pub fn byte_start(&self, bytes_id: i32) -> i32 {
        self.bytes_start_array.access(|bytes_start_array| {
            debug_assert!(
                bytes_start_array.len() > 0,
                "bytes_start is null - not initialized"
            );
            debug_assert!(bytes_id >= 0 || bytes_id < self.count);
            bytes_start_array.get_value(bytes_id as usize)
        })
    }
}
impl<C, B, A, P> Accountable for BytesRefHash<C, B, A, P>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C, P>>,
    P: Access<PostingsArrayWrapper>,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        // TODO: memory calculation not implemented
        Ok(0)
    }
}
/// for single-threaded scenarios
pub(crate) type STBytesRefHash = BytesRefHash<
    CounterEnumBorrow,
    ByteBlockPoolBorrow,
    BytesStartArrayEnumBorrow,
    STPostingsArrayWrapper,
>;
/// for multi-threaded scenarios
#[allow(unused)]
pub(crate) type MTBytesRefHash = BytesRefHash<
    CounterEnumLock,
    ByteBlockPoolLock,
    BytesStartArrayEnumLock,
    MTPostingsArrayWrapper,
>;

pub(crate) struct StringSorterImpl<'a, C, B, A, P>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C, P>>,
    P: Access<PostingsArrayWrapper>,
{
    tmp_offset: i32,
    compact: &'a mut Vec<i32>,
    pool: &'a mut BytesRefBlockPool<C, B>,
    bytes_start_array: A,
    k: i32,
    cmp: Natural,
    _phantom1: PhantomData<P>,
}
impl<'a, C, B, A, P> StringSorterImpl<'a, C, B, A, P>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C, P>>,
    P: Access<PostingsArrayWrapper>,
{
    pub fn new(
        tmp_offset: i32,
        compact: &'a mut Vec<i32>,
        pool: &'a mut BytesRefBlockPool<C, B>,
        bytes_start_array: A,
    ) -> Self {
        StringSorterImpl {
            tmp_offset,
            compact,
            pool,
            bytes_start_array,
            k: 0,
            cmp: Natural::default(),
            _phantom1: Default::default(),
        }
    }
    fn swap_bucket_cache(&mut self, i: i32, j: i32) -> Result<()> {
        self.swap(i, j)?;
        self.compact.swap(
            (self.tmp_offset + i) as usize,
            (self.tmp_offset + j) as usize,
        );
        Ok(())
    }
}
impl<C, B, A, P> MSBRadixSorterBase for StringSorterImpl<'_, C, B, A, P>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C, P>>,
    P: Access<PostingsArrayWrapper>,
{
    fn byte_at(&mut self, i: i32, k: i32) -> Result<i32> {
        let mut scratch = BytesRefBuilder::new();
        let mut scratch_bytes = BytesRef::new();
        self.get(&mut scratch, &mut scratch_bytes, i)?;
        Ok(self.cmp.byte_at(&scratch_bytes, k))
    }

    fn reorder(
        &mut self,
        from: i32,
        _to: i32,
        start_offsets: &mut [i32],
        end_offsets: &mut [i32],
        k: i32,
    ) -> Result<()> {
        debug_assert_eq!(self.k, k);
        for i in 0..HISTOGRAM_SIZE {
            let limit = end_offsets[i];
            while start_offsets[i] < limit {
                let h1 = start_offsets[i];
                let b = self.compact[(self.tmp_offset + from + h1) as usize] as usize;
                let h2 = start_offsets[b];
                start_offsets[b] += 1;
                self.swap_bucket_cache(from + h1, from + h2)?;
            }
        }
        Ok(())
    }

    fn build_histogram(
        &mut self,
        prefix_common_bucket: i32,
        prefix_common_len: i32,
        from: i32,
        to: i32,
        k: i32,
        histogram: &mut [i32],
    ) -> Result<()> {
        self.k = k;
        histogram[prefix_common_bucket as usize] = prefix_common_len;
        self.compact[(self.tmp_offset + from - prefix_common_len) as usize
            ..(self.tmp_offset + from) as usize]
            .fill(prefix_common_bucket);
        for i in from..to {
            let b = self.get_bucket(i, k)?;
            self.compact[(self.tmp_offset + i) as usize] = b;
            histogram[b as usize] += 1;
        }
        Ok(())
    }

    fn should_fallback(&self, from: i32, to: i32, l: i32) -> bool {
        // We lower the fallback threshold because the bucket cache speeds up
        // the reorder
        to - from <= ((LEVEL_THRESHOLD as i32) / 2) || l >= LEVEL_THRESHOLD as i32
    }
}
impl<C, B, A, P> Sorter for StringSorterImpl<'_, C, B, A, P>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C, P>>,
    P: Access<PostingsArrayWrapper>,
{
    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.compact.swap(i as usize, j as usize);
        Ok(())
    }
}
impl<C, B, A, P> StringSorterBase for StringSorterImpl<'_, C, B, A, P>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C, P>>,
    P: Access<PostingsArrayWrapper>,
{
    fn get(
        &mut self,
        _builder: &mut BytesRefBuilder<Vec<u8>>,
        result: &mut BytesRef<Vec<u8>>,
        i: i32,
    ) -> Result<()> {
        self.bytes_start_array.access(|bytes_start_array| {
            let start = bytes_start_array.get_value(self.compact[i as usize] as usize);
            self.pool.fill_bytes_ref(result, start);
        });
        Ok(())
    }

    fn radix_sorter<'b, C1>(&'b mut self, cmp: &'b mut C1) -> impl Sorter + 'b
    where
        C1: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
        Self: Sorter + Sized,
    {
        let length = cmp.compared_bytes_count();
        let delegate_sorter = MSBStringHashRadixSorter::new(cmp, self);
        MSBRadixSorter::new(length, delegate_sorter)
    }
}

/// Manages allocation of the per-term addresses.
#[allow(clippy::len_without_is_empty)]
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
    fn grow(&mut self) -> Result<()>;

    /// Clears the [`BytesStartArray`] and returns the cleared instance.
    ///
    /// # Returns
    /// The cleared instance, this might be `None`.
    fn clear(&mut self);

    /// A reference holding the number of bytes used by this `BytesStartArray`.
    /// The [`BytesRefHash`] uses this reference to track its memory usage.
    ///
    /// # Returns
    /// A reference holding the number of bytes used by this `BytesStartArray`.
    type Counter: Access<CounterEnum>;
    fn bytes_used(&mut self) -> Self::Counter;
    fn get_value(&self, index: usize) -> i32;
    fn set_value(&mut self, index: usize, value: i32);
    fn len(&self) -> usize;
}
/// A simple [`BytesStartArray`] that tracks memory allocation using a private
/// `Counter` instance.
pub struct DirectBytesStartArray<C>
where
    C: Access<CounterEnum>,
{
    init_size: i32,
    bytes_start: Vec<i32>,
    bytes_used: C,
}
impl DirectBytesStartArray<CounterEnumBorrow> {
    pub fn new(init_size: i32) -> Self {
        DirectBytesStartArray::with_counter(
            init_size,
            Rc::new(RefCell::new(CounterEnum::new_counter(false))),
        )
    }
    pub fn with_counter(init_size: i32, counter: CounterEnumBorrow) -> Self {
        DirectBytesStartArray {
            init_size,
            bytes_start: vec![],
            bytes_used: counter,
        }
    }
}
impl DirectBytesStartArray<CounterEnumLock> {
    pub fn new_sync(init_size: i32) -> Self {
        DirectBytesStartArray::with_counter_sync(
            init_size,
            Arc::new(Mutex::new(CounterEnum::new_counter(false))),
        )
    }
    pub fn with_counter_sync(init_size: i32, counter: CounterEnumLock) -> Self {
        DirectBytesStartArray {
            init_size,
            bytes_start: vec![],
            bytes_used: counter,
        }
    }
}

impl<C> BytesStartArray for DirectBytesStartArray<C>
where
    C: Access<CounterEnum>,
{
    fn init(&mut self) {
        self.bytes_start =
            vec![0; ArrayUtil::oversize(self.init_size as usize, BitUtil::INT_BYTES)];
    }

    fn grow(&mut self) -> Result<()> {
        debug_assert!(!self.bytes_start.is_empty());
        let length = self.bytes_start.len() as i32;
        ArrayUtil::grow_i32(&mut self.bytes_start, length as usize + 1)?;
        Ok(())
    }

    fn clear(&mut self) {
        self.bytes_start.clear();
    }

    type Counter = C;

    fn bytes_used(&mut self) -> Self::Counter {
        self.bytes_used.clone()
    }

    fn get_value(&self, index: usize) -> i32 {
        self.bytes_start[index]
    }

    fn set_value(&mut self, index: usize, value: i32) {
        self.bytes_start[index] = value;
    }

    fn len(&self) -> usize {
        self.bytes_start.len()
    }
}

pub(crate) enum BytesStartArrayEnum<C, P>
where
    C: Access<CounterEnum>,
    P: Access<PostingsArrayWrapper>,
{
    Direct(DirectBytesStartArray<C>),
    Postings(PostingsBytesStartArray<C, P>),
}
impl<C, P> BytesStartArray for BytesStartArrayEnum<C, P>
where
    C: Access<CounterEnum>,
    P: Access<PostingsArrayWrapper>,
{
    fn init(&mut self) {
        match self {
            BytesStartArrayEnum::Direct(d) => d.init(),
            BytesStartArrayEnum::Postings(p) => p.init(),
        }
    }

    fn grow(&mut self) -> Result<()> {
        match self {
            BytesStartArrayEnum::Direct(d) => d.grow(),
            BytesStartArrayEnum::Postings(p) => p.grow(),
        }
    }

    fn clear(&mut self) {
        match self {
            BytesStartArrayEnum::Direct(d) => d.clear(),
            BytesStartArrayEnum::Postings(p) => p.clear(),
        }
    }

    type Counter = C;

    fn bytes_used(&mut self) -> Self::Counter {
        match self {
            BytesStartArrayEnum::Direct(d) => d.bytes_used(),
            BytesStartArrayEnum::Postings(p) => p.bytes_used(),
        }
    }

    fn get_value(&self, index: usize) -> i32 {
        match self {
            BytesStartArrayEnum::Direct(d) => d.get_value(index),
            BytesStartArrayEnum::Postings(p) => p.get_value(index),
        }
    }

    fn set_value(&mut self, index: usize, value: i32) {
        match self {
            BytesStartArrayEnum::Direct(d) => d.set_value(index, value),
            BytesStartArrayEnum::Postings(p) => p.set_value(index, value),
        }
    }

    fn len(&self) -> usize {
        match self {
            BytesStartArrayEnum::Direct(d) => d.len(),
            BytesStartArrayEnum::Postings(p) => p.len(),
        }
    }
}
pub(crate) type BytesStartArrayEnumBorrow =
    Rc<RefCell<BytesStartArrayEnum<CounterEnumBorrow, STPostingsArrayWrapper>>>;
pub(crate) type BytesStartArrayEnumLock =
    Arc<Mutex<BytesStartArrayEnum<CounterEnumLock, MTPostingsArrayWrapper>>>;

/// # Note
/// In Java Lucene, BytesRefHash uses MSBStringRadixSorter. Due to language
/// limitations, a new MSBStringHashRadixSorter is currently being used.
pub struct MSBStringHashRadixSorter<'a, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
    cmp: &'a mut C,
    delegate_sorter: &'a mut T,
}
impl<'a, T, C> MSBStringHashRadixSorter<'a, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
    pub fn new(cmp: &'a mut C, delegate_sorter: &'a mut T) -> MSBStringHashRadixSorter<'a, T, C> {
        MSBStringHashRadixSorter {
            cmp,
            delegate_sorter,
        }
    }
}

impl<T, C> Sorter for MSBStringHashRadixSorter<'_, T, C>
where
    T: Sorter + StringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.delegate_sorter.swap(i, j)
    }
}

impl<T, C> MSBRadixSorterBase for MSBStringHashRadixSorter<'_, T, C>
where
    T: Sorter + StringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
    fn byte_at(&mut self, i: i32, k: i32) -> Result<i32> {
        self.delegate_sorter.byte_at(i, k)
    }

    fn get_fallback_sorter(&mut self, k: i32, _length: i32) -> impl Sorter {
        self.delegate_sorter
            .fall_back_sorter::<T, C>(self.cmp, Some(k))
    }

    fn reorder(
        &mut self,
        from: i32,
        to: i32,
        start_offsets: &mut [i32],
        end_offsets: &mut [i32],
        k: i32,
    ) -> Result<()> {
        self.delegate_sorter
            .reorder(from, to, start_offsets, end_offsets, k)
    }

    fn build_histogram(
        &mut self,
        prefix_common_bucket: i32,
        prefix_common_len: i32,
        from: i32,
        to: i32,
        k: i32,
        histogram: &mut [i32],
    ) -> Result<()> {
        self.delegate_sorter.build_histogram(
            prefix_common_bucket,
            prefix_common_len,
            from,
            to,
            k,
            histogram,
        )
    }

    fn should_fallback(&self, from: i32, to: i32, l: i32) -> bool {
        self.delegate_sorter.should_fallback(from, to, l)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;

    use parking_lot::Mutex;
    use rand::Rng;

    use crate::index::{BytesRef, BytesRefBuilder};
    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::allocator_byte::{AllocatorByteEnum, DirectAllocatorByte};
    use crate::util::bytes_ref_hash::{
        BytesRefHash, BytesStartArrayEnum, DirectBytesStartArray, MTBytesRefHash,
    };
    use crate::util::error::lucene_error::{LuceneError, Result};
    use crate::util::{ByteBlockPool, ByteBlockPoolLock};

    #[allow(dead_code)] // for quick search
    pub struct TestBytesRefHash;

    fn new_pool() -> ByteBlockPoolLock {
        let allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
        Arc::new(Mutex::new(ByteBlockPool::new_sync(allocator)))
    }
    fn new_hash<R: Rng + ?Sized>(random: &mut R, block_pool: ByteBlockPoolLock) -> MTBytesRefHash {
        let init_size = 2 << (1 + random.random_range(0..5));
        if random.random_bool(0.5) {
            BytesRefHash::from_pool_sync(block_pool)
        } else {
            BytesRefHash::from_bytes_start_array(
                block_pool,
                init_size,
                Arc::new(Mutex::new(BytesStartArrayEnum::Direct(
                    DirectBytesStartArray::new_sync(init_size),
                ))),
            )
        }
    }
    #[test]
    fn test_size() -> Result<()> {
        let mut random = random();
        let mut hash = new_hash(&mut random, new_pool());
        let mut ref_builder = BytesRefBuilder::new();

        let num = at_least(&mut random, 2);
        for _ in 0..num {
            let mod_val = random.random_range(1..40);
            for i in 0..797 {
                let mut str_value;
                loop {
                    str_value =
                        TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
                    if !str_value.is_empty() {
                        break;
                    }
                }
                ref_builder.copy_chars_with_string(&str_value);
                let count = hash.size();
                let key = hash.add(ref_builder.get_bytes_mut_ref())?;

                if key < 0 {
                    assert_eq!(hash.size(), count,);
                } else {
                    assert_eq!(hash.size(), count + 1);
                }

                if i % mod_val == 0 {
                    hash.clear();
                    assert_eq!(hash.size(), 0);
                    hash.reinit();
                }
            }
        }
        Ok(())
    }
    #[test]
    fn test_get() -> Result<()> {
        let mut random = random();
        let mut hash = new_hash(&mut random, new_pool());
        let mut ref_builder = BytesRefBuilder::new();
        let mut scratch = BytesRef::new();

        let num = at_least(&mut random, 2);
        for _ in 0..num {
            let mut strings: HashMap<String, i32> = HashMap::new();
            let mut unique_count = 0;

            for _ in 0..797 {
                let mut str_value;
                loop {
                    str_value =
                        TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
                    if !str_value.is_empty() {
                        break;
                    }
                }

                ref_builder.copy_chars_with_string(&str_value);
                let count = hash.size();
                let key = hash.add(ref_builder.get_bytes_mut_ref())?;

                if key >= 0 {
                    assert!(strings.insert(str_value.clone(), key).is_none());
                    assert_eq!(unique_count, key);
                    unique_count += 1;
                    assert_eq!(hash.size(), count + 1);
                } else {
                    assert!((-key - 1) < count);
                    assert_eq!(hash.size(), count);
                }
            }

            for (key, value) in &strings {
                ref_builder.copy_chars_with_string(key);
                hash.get(*value, &mut scratch);
                assert_eq!(*ref_builder.get_bytes_mut_ref(), scratch);
            }

            hash.clear();
            assert_eq!(hash.size(), 0);
            hash.reinit();
        }
        Ok(())
    }
    #[test]
    fn test_compact() -> Result<()> {
        let mut random = random();
        let mut hash = new_hash(&mut random, new_pool());
        let mut ref_builder = BytesRefBuilder::new();

        let num = at_least(&mut random, 2);
        for _ in 0..num {
            let mut num_entries = 0;
            let size = 797;
            let mut bits = bit_set::BitSet::new();

            for _ in 0..size {
                let mut str_value;
                loop {
                    str_value =
                        TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
                    if !str_value.is_empty() {
                        break;
                    }
                }

                ref_builder.copy_chars_with_string(&str_value);
                let key = hash.add(ref_builder.get_bytes_mut_ref())?;

                if key < 0 {
                    assert!(bits.contains(((-key) - 1) as usize));
                } else {
                    assert!(!bits.contains(key as usize));
                    bits.insert(key as usize);
                    num_entries += 1;
                }
            }
            assert_eq!(hash.size() as usize, bits.len());
            assert_eq!(num_entries as usize, bits.len());
            assert_eq!(num_entries, hash.size());

            let compact = hash.compact();
            assert!(num_entries < compact.len() as i32);

            for &id in compact {
                bits.remove(id as usize);
            }

            assert_eq!(bits.len(), 0);

            hash.clear();
            assert_eq!(hash.size(), 0);
            hash.reinit();
        }
        Ok(())
    }
    #[test]
    fn test_sort() -> Result<()> {
        let mut random = random();
        let mut hash = new_hash(&mut random, new_pool());
        let mut ref_builder = BytesRefBuilder::new();

        let num = at_least(&mut random, 2);
        for _ in 0..num {
            let mut strings = std::collections::BTreeSet::new();

            for _ in 0..797 {
                let mut str_value;
                loop {
                    str_value =
                        TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
                    if !str_value.is_empty() {
                        break;
                    }
                }

                ref_builder.copy_chars_with_string(&str_value);
                hash.add(ref_builder.get_bytes_mut_ref())?;
                strings.insert(str_value);
            }

            for _ in 0..3 {
                hash.sort()?;
                let len = hash.ids.len();
                assert!(strings.len() < len);
                let mut scratch = BytesRef::new();
                for (i, string) in strings.iter().enumerate() {
                    ref_builder.copy_chars_with_string(string);
                    let bytes_id = hash.ids[i];
                    hash.get(bytes_id, &mut scratch);
                    let sorted_ref = scratch.clone();
                    assert_eq!(
                        *ref_builder.get_bytes_mut_ref(),
                        sorted_ref,
                        "Sorted value mismatch at index {}",
                        i
                    );
                }
            }

            hash.clear();
            assert_eq!(hash.size(), 0, "Hash should be empty after clear.");
            hash.reinit();
        }
        Ok(())
    }

    #[test]
    fn test_add() -> Result<()> {
        let mut random = random();
        let mut hash = new_hash(&mut random, new_pool());
        let mut ref_builder = BytesRefBuilder::new();
        let mut scratch = BytesRef::new();

        let num = at_least(&mut random, 2);
        for _ in 0..num {
            let mut strings = HashSet::new();
            let mut unique_count = 0;

            for _ in 0..797 {
                let mut str_value;
                loop {
                    str_value =
                        TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
                    if !str_value.is_empty() {
                        break;
                    }
                }

                ref_builder.copy_chars_with_string(&str_value);
                let count = hash.size();
                let key = hash.add(ref_builder.get_bytes_mut_ref())?;

                if key >= 0 {
                    assert!(strings.insert(str_value.clone()));
                    assert_eq!(unique_count, key);
                    assert_eq!(hash.size(), count + 1);
                    unique_count += 1;
                } else {
                    assert!(!strings.insert(str_value.clone()));
                    assert!((-key - 1) < count);
                    hash.get(-key - 1, &mut scratch);
                    assert_eq!(str_value, scratch.utf8_to_string()?);
                    assert_eq!(count, hash.size());
                }
            }

            assert_all_in(&strings, &mut hash)?;
            hash.clear();
            assert_eq!(hash.size(), 0);
            hash.reinit();
        }
        Ok(())
    }
    #[test]
    fn test_find() -> Result<()> {
        let mut random = random();
        let mut hash = new_hash(&mut random, new_pool());
        let mut ref_builder = BytesRefBuilder::new();
        let mut scratch = BytesRef::new();

        let num = at_least(&mut random, 2);
        for _ in 0..num {
            let mut strings = HashSet::new();
            let mut unique_count = 0;

            for _ in 0..797 {
                let mut str_value;
                loop {
                    str_value =
                        TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
                    if !str_value.is_empty() {
                        break;
                    }
                }

                ref_builder.copy_chars_with_string(&str_value);
                let count = hash.size();
                let key = hash.find(ref_builder.get_bytes_mut_ref());

                if key >= 0 {
                    assert!(!strings.insert(str_value.clone()));
                    assert!(key < count);
                    hash.get(key, &mut scratch);
                    assert_eq!(str_value, scratch.utf8_to_string()?);
                    assert_eq!(count, hash.size());
                } else {
                    let key = hash.add(ref_builder.get_bytes_mut_ref())?;
                    assert!(strings.insert(str_value.clone()));
                    assert_eq!(unique_count, key);
                    assert_eq!(hash.size(), count + 1);
                    unique_count += 1;
                }
            }

            assert_all_in(&strings, &mut hash)?;
            hash.clear();
            assert_eq!(hash.size(), 0);
            hash.reinit();
        }
        Ok(())
    }
    #[test]
    fn test_concurrent_access_to_bytes_ref_hash() -> Result<()> {
        let mut random = random();
        let num = at_least(&mut random, 2);

        for _ in 0..num {
            let num_strings = 797;
            let strings = Arc::new(Mutex::new(Vec::with_capacity(num_strings)));
            let hash = Arc::new(Mutex::new(new_hash(&mut random, new_pool())));

            {
                let mut hash_guard = hash.lock();
                for _ in 0..num_strings {
                    let str_value =
                        TestUtil::random_realistic_unicode_string_range(&mut random, 1, 1000);
                    hash_guard.add(&BytesRef::from_string(&str_value))?;
                    strings.lock().push(str_value);
                }
            }

            let hash_size = hash.lock().size();

            let not_found = Arc::new(AtomicI32::new(0));
            let not_equals = Arc::new(AtomicI32::new(0));
            let wrong_size = Arc::new(AtomicI32::new(0));

            let num_threads = at_least(&mut random, 3);
            let barrier = Arc::new(Barrier::new(num_threads as usize));
            let mut handles = vec![];

            for _ in 0..num_threads {
                let hash_clone = Arc::clone(&hash);
                let strings_clone = Arc::clone(&strings);
                let not_found_clone = Arc::clone(&not_found);
                let not_equals_clone = Arc::clone(&not_equals);
                let wrong_size_clone = Arc::clone(&wrong_size);
                let barrier_clone = Arc::clone(&barrier);
                let loops = at_least(&mut random, 100);

                let handle = thread::spawn(move || {
                    let mut scratch = BytesRef::new();
                    barrier_clone.wait();

                    for k in 0..loops {
                        let strings_guard = strings_clone.lock();
                        let find =
                            BytesRef::from_string(&strings_guard[k as usize % strings_guard.len()]);
                        drop(strings_guard);

                        let hash_guard = hash_clone.lock();
                        let id = hash_guard.find(&find);

                        if id < 0 {
                            not_found_clone.fetch_add(1, Ordering::SeqCst);
                        } else {
                            hash_guard.get(id, &mut scratch);
                            if scratch != find {
                                not_equals_clone.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                        if hash_guard.size() != hash_size {
                            wrong_size_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().expect("Thread panicked");
            }

            assert_eq!(
                not_found.load(Ordering::SeqCst),
                0,
                "No entries should be missing."
            );
            assert_eq!(
                not_equals.load(Ordering::SeqCst),
                0,
                "All entries should match."
            );
            assert_eq!(
                wrong_size.load(Ordering::SeqCst),
                0,
                "Hash size should remain consistent."
            );

            hash.lock().clear();
            assert_eq!(hash.lock().size(), 0, "Hash should be empty after clear.");
            hash.lock().reinit();
        }

        Ok(())
    }
    #[test]
    fn test_large_value() -> Result<()> {
        let mut random = random();
        let mut hash = new_hash(&mut random, new_pool());

        let sizes = [
            random.random_range(0..5),
            ByteBlockPool::BYTE_BLOCK_SIZE - 33 + random.random_range(0..31),
            ByteBlockPool::BYTE_BLOCK_SIZE - 1 + random.random_range(0..37),
        ];

        for (i, &size) in sizes.iter().enumerate() {
            let mut ref_bytes = BytesRef::new();
            ref_bytes.bytes = vec![0; size as usize];
            ref_bytes.offset = 0;
            ref_bytes.length = size as usize;

            match hash.add(&ref_bytes) {
                Ok(key) => {
                    assert_eq!(i as i32, key, "Expected index {} but got {}", i, key);
                },
                Err(e) => {
                    if i < sizes.len() - 1 {
                        unreachable!("Unexpected exception at size: {}: {:?}", size, e);
                    }
                    assert!(matches!(e, LuceneError::MaxBytesLengthExceeded(_)));
                },
            }
        }

        Ok(())
    }
    #[test]
    fn test_add_by_pool_offset() -> Result<()> {
        let mut random = random();
        let pool = new_pool();
        let mut hash = new_hash(&mut random, pool.clone());
        let mut offset_hash = new_hash(&mut random, pool);
        let mut ref_builder = BytesRefBuilder::new();
        let mut scratch = BytesRef::new();

        let num = at_least(&mut random, 2);
        for _ in 0..num {
            let mut strings = HashSet::new();
            let mut unique_count = 0;

            for _ in 0..797 {
                let mut str_value;
                loop {
                    str_value =
                        TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
                    if !str_value.is_empty() {
                        break;
                    }
                }

                ref_builder.copy_chars_with_string(&str_value);
                let count = hash.size();
                let key = hash.add(ref_builder.get_bytes_mut_ref())?;

                if key >= 0 {
                    assert!(strings.insert(str_value.clone()));
                    assert_eq!(unique_count, key);
                    assert_eq!(hash.size(), count + 1);

                    let offset_key = offset_hash.add_by_pool_offset(hash.byte_start(key))?;
                    assert_eq!(unique_count, offset_key);
                    assert_eq!(offset_hash.size(), count + 1);

                    unique_count += 1;
                } else {
                    assert!(!strings.insert(str_value.clone()));
                    assert!((-key - 1) < count);
                    hash.get(-key - 1, &mut scratch);
                    assert_eq!(str_value, scratch.utf8_to_string()?);
                    assert_eq!(count, hash.size());
                    let offset_key = offset_hash.add_by_pool_offset(hash.byte_start(-key - 1))?;
                    assert!((-offset_key - 1) < count);
                    hash.get(-offset_key - 1, &mut scratch);
                    assert_eq!(str_value, scratch.utf8_to_string()?);
                    assert_eq!(count, hash.size());
                }
            }

            assert_all_in(&strings, &mut hash)?;

            for string in &strings {
                ref_builder.copy_chars_with_string(string);
                let key = hash.add(ref_builder.get_bytes_mut_ref())?;
                offset_hash.get(-key - 1, &mut scratch);
                let bytes_ref = scratch.clone();
                assert_eq!(
                    *ref_builder.get_bytes_mut_ref(),
                    bytes_ref,
                    "Values should match."
                );
            }

            hash.clear();
            assert_eq!(hash.size(), 0, "Hash should be empty after clear.");
            offset_hash.clear();
            assert_eq!(
                offset_hash.size(),
                0,
                "Offset hash should be empty after clear."
            );

            hash.reinit();
            offset_hash.reinit();
        }
        Ok(())
    }

    fn assert_all_in(strings: &HashSet<String>, hash: &mut MTBytesRefHash) -> Result<()> {
        let mut ref_builder = BytesRefBuilder::new();
        let mut scratch = BytesRef::new();
        let count = hash.size();

        for string in strings {
            ref_builder.copy_chars_with_string(string);
            let key = hash.add(ref_builder.get_bytes_mut_ref())?; // add again to check duplicates
            hash.get((-key) - 1, &mut scratch);
            assert_eq!(*string, scratch.utf8_to_string()?);
            assert_eq!(
                count,
                hash.size(),
                "Hash size should remain unchanged after duplicate insertion."
            );
            assert!(
                key < count,
                "Key {} should be less than count {}, string: {}",
                key,
                count,
                string
            );
        }

        Ok(())
    }
}
