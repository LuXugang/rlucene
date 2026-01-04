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
use std::sync::Arc;

use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::accountable::Accountable;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::bytes_ref_block_pool::BytesRefBlockPool;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{
    AtomicCounter, ByteBlockPool, BytesRefComparator, Comparator, Counter, GOOD_FAST_HASH_SEED,
    HISTOGRAM_SIZE, LEVEL_THRESHOLD, MSBRadixSorter, MSBRadixSorterBase, Natural, SharedCounter,
    Sorter, StringHelper, StringSorter, StringSorterBase,
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
///   [`BYTE_BLOCK_SIZE`](ByteBlockPool) - 2.
/// - The internal storage is limited to 2GB total byte storage.
///
/// [`BYTE_BLOCK_SIZE`]: BYTE_BLOCK_SIZE
pub(crate) struct BytesRefHash<BSA>
where
    BSA: BytesStartArray,
{
    pool: BytesRefBlockPool,
    hash_size: i32,
    hash_half_size: i32,
    hash_mask: i32,
    pub(crate) count: i32,
    last_count: i32,
    pub ids: Vec<i32>,
    pub(crate) bytes_start_array: BSA,
    bytes_used: SharedCounter,
}
impl BytesRefHash<DirectBytesStartArray> {
    pub fn new() -> Self {
        let bytes_start_array = DirectBytesStartArray::new(DEFAULT_CAPACITY);
        BytesRefHash::from_bytes_start_array(16, bytes_start_array)
    }
}
pub fn do_hash(bytes: &[u8], offset: usize, length: usize) -> i32 {
    StringHelper::murmurhash3_x86_32_with_byte(bytes, offset, length, *GOOD_FAST_HASH_SEED)
}
impl<BSA> BytesRefHash<BSA>
where
    BSA: BytesStartArray,
{
    pub fn from_bytes_start_array(capacity: i32, mut bytes_start_array: BSA) -> Self {
        bytes_start_array.init();
        let bytes_used = bytes_start_array.bytes_used();
        let ref_pool = BytesRefBlockPool::new();
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
    pub fn get(&self, bytes_id: i32, ref_: &mut BytesRef<Vec<u8>>, pool: &ByteBlockPool) {
        debug_assert!(
            self.bytes_start_array.len() > 0,
            "bytes_start is null - not initialized"
        );
        debug_assert!(
            (bytes_id as usize) < self.bytes_start_array.len(),
            "bytesID exceeds bytes_start len"
        );
        let value = self.bytes_start_array.get_value(bytes_id as usize);
        self.pool.fill_bytes_ref(ref_, value, pool)
    }

    /// Returns the id array in arbitrary order. Valid ids start at offset 0 and
    /// end at a limit of `size()` - 1.
    ///
    /// # Note
    /// This is a destructive operation. `Clear()` must be called to reuse this
    /// `BytesRefHash` instance.
    pub fn compact(&mut self) -> &Vec<i32> {
        debug_assert!(
            self.bytes_start_array.len() > 0,
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
    pub fn sort(&mut self, byte_block_pool: &ByteBlockPool) -> Result<()> {
        let compact = self.compact();
        let mut length = compact.len();
        debug_assert!(
            (self.count * 2) as usize <= length,
            "We need load factor <= 0.5f to speed up this sort"
        );
        let tmp_offset = self.count;
        let sub_sorter = StringSorterImpl::new(
            tmp_offset as usize,
            &mut self.ids,
            &mut self.pool,
            byte_block_pool,
            &self.bytes_start_array,
        );
        let mut sorter = StringSorter::new(sub_sorter, Natural::default());
        sorter.sort(0, self.count as usize)?;

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
            self.bytes_used.add_and_get(0);
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
    pub fn clear_with_reset_pool(&mut self, reset_pool: bool, byte_block_pool: &mut ByteBlockPool) {
        self.last_count = self.count;
        self.count = 0;

        if reset_pool {
            self.pool.reset(byte_block_pool);
        }

        self.bytes_start_array.clear();

        if self.last_count != -1 && self.shrink(self.last_count) {
            // shrink clears the hash entries
            return;
        }
        self.ids.fill(-1);
    }
    pub fn clear(&mut self, byte_block_pool: &mut ByteBlockPool) {
        self.clear_with_reset_pool(true, byte_block_pool)
    }

    /// Closes the `BytesRefHash` and releases all internally used memory.
    pub fn close(&mut self, byte_block_pool: &mut ByteBlockPool) {
        self.clear_with_reset_pool(true, byte_block_pool);
        self.ids.clear();
        // TODO: memory calculation not implemented
        self.bytes_used.add_and_get(0);
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
    /// than 2 + [`BYTE_BLOCK_SIZE`].
    pub fn add(
        &mut self,
        bytes: &BytesRef<Vec<u8>>,
        byte_block_pool: &mut ByteBlockPool,
    ) -> Result<i32> {
        debug_assert!(
            self.bytes_start_array.len() > 0,
            "Bytesstart is null - not initialized"
        );

        // final position
        let hash_pos = self.find_hash(bytes, byte_block_pool);
        let mut e = self.ids[hash_pos as usize];
        if e == -1 {
            {
                let length = self.bytes_start_array.len();
                // new entry
                if self.count as usize >= length {
                    self.bytes_start_array.grow()?;
                    debug_assert!(
                        (self.count as usize) < self.bytes_start_array.len() + 1,
                        "count: {} len: {}",
                        self.count,
                        self.bytes_start_array.len()
                    );
                }

                let v = self.pool.add_bytes_ref(bytes, byte_block_pool)?;
                self.bytes_start_array.set_value(self.count as usize, v);
                e = self.count;
                self.count += 1;
                assert_eq!(self.ids[hash_pos as usize], -1);
                self.ids[hash_pos as usize] = e;
            }

            if self.count == self.hash_half_size {
                self.rehash(2 * self.hash_size, true, byte_block_pool);
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
    pub fn find(&self, bytes: &BytesRef<Vec<u8>>, byte_block_pool: &ByteBlockPool) -> i32 {
        self.ids[self.find_hash(bytes, byte_block_pool) as usize]
    }
    fn find_hash(&self, bytes: &BytesRef<Vec<u8>>, byte_block_pool: &ByteBlockPool) -> i32 {
        debug_assert!(
            self.bytes_start_array.len() > 0,
            "bytesStart is null - not initialized"
        );

        let mut code = do_hash(&bytes.bytes, bytes.offset, bytes.length);

        // final position
        let mut hash_pos = code & self.hash_mask;
        let mut e = self.ids[hash_pos as usize];
        if e != -1
            && !self.pool.equals(
                self.bytes_start_array.get_value(e as usize),
                bytes,
                byte_block_pool,
            )
        {
            loop {
                code += 1;
                hash_pos = code & self.hash_mask;
                e = self.ids[hash_pos as usize];
                if e == -1
                    || self.pool.equals(
                        self.bytes_start_array.get_value(e as usize),
                        bytes,
                        byte_block_pool,
                    )
                {
                    break;
                }
            }
        }

        hash_pos
    }
    /// Adds an "arbitrary" integer offset instead of a `BytesRef` term.
    ///
    /// This is used in the indexer to hold the hash for term vectors, because
    /// they do not redundantly store the byte[] term directly and instead
    /// reference the byte[] term already stored by the postings
    /// `BytesRefHash`.
    pub fn add_by_pool_offset(
        &mut self,
        offset: i32,
        byte_block_pool: &mut ByteBlockPool,
    ) -> Result<i32> {
        debug_assert!(
            self.bytes_start_array.len() > 0,
            "Bytesstart is null - not initialized"
        );

        // Final position
        let mut code = offset;
        let mut hash_pos = offset & self.hash_mask;
        let mut e = self.ids[hash_pos as usize];
        let length = self.bytes_start_array.len();
        // Resolve hash conflicts
        while e != -1 && self.bytes_start_array.get_value(e as usize) != offset {
            code += 1;
            hash_pos = code & self.hash_mask;
            e = self.ids[hash_pos as usize];
        }

        if e == -1 {
            // New entry
            if self.count as usize >= length {
                self.bytes_start_array.grow()?;
                debug_assert!(
                    self.count < self.bytes_start_array.len() as i32 + 1,
                    "count: {}, len: {}",
                    self.count,
                    self.bytes_start_array.len()
                );
            }

            e = self.count;
            self.count += 1;
            self.bytes_start_array.set_value(e as usize, offset);

            assert_eq!(self.ids[hash_pos as usize], -1);
            self.ids[hash_pos as usize] = e;

            if self.count == self.hash_half_size {
                self.rehash(2 * self.hash_size, false, byte_block_pool);
            }

            return Ok(e);
        }

        Ok(-(e + 1))
    }
    /// Called when hash is too small (> 50% occupied) or too large (< 20%
    /// occupied).
    fn rehash(&mut self, new_size: i32, hash_on_data: bool, byte_block_pool: &mut ByteBlockPool) {
        let new_mask = new_size - 1;
        // TODO: memory calculation not implemented
        self.bytes_used.add_and_get(0);
        let mut new_hash = vec![-1; new_size as usize];
        for i in 0..self.hash_size {
            let e0 = self.ids[i as usize];
            if e0 != -1 {
                let mut code = if hash_on_data {
                    self.pool.hash(
                        self.bytes_start_array.get_value(e0 as usize),
                        byte_block_pool,
                    )
                } else {
                    self.bytes_start_array.get_value(e0 as usize)
                };

                let mut hash_pos = code & new_mask;
                debug_assert!(hash_pos >= 0);
                if new_hash[hash_pos as usize] != -1 {
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
        self.bytes_used.add_and_get(0);
        self.ids = new_hash;
        self.hash_size = new_size;
        self.hash_half_size = new_size / 2;
    }

    /// Reinitializes the [`BytesRefHash`] after a previous `clear()` call.
    /// If `clear()` has not been called previously, this method has no effect.
    pub fn reinit(&mut self) {
        if self.bytes_start_array.need_init() {
            self.bytes_start_array.init();
        }

        if self.ids.is_empty() {
            self.ids = vec![-1; self.hash_size as usize];
            // TODO: memory calculation not implemented
            self.bytes_used.add_and_get(0);
        }
    }
    /// Returns the `bytesStart` offset into the internally used
    /// `SingleThreadedByteBlockPool` for the given `bytes_id`.
    ///
    /// # Arguments
    /// * `bytes_id` - The ID to look up.
    ///
    /// # Returns
    /// The `bytesStart` offset into the internally used
    /// `SingleThreadedByteBlockPool` for the given ID.
    #[cfg(debug_assertions)]
    pub fn byte_start(&self, bytes_id: i32) -> i32 {
        debug_assert!(
            self.bytes_start_array.len() > 0,
            "bytes_start is null - not initialized"
        );
        debug_assert!(bytes_id >= 0 || bytes_id < self.count);
        self.bytes_start_array.get_value(bytes_id as usize)
    }
}
impl<BSA> Accountable for BytesRefHash<BSA>
where
    BSA: BytesStartArray,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        // TODO: memory calculation not implemented
        Ok(0)
    }
}

// used for std::mem::take
impl Default for BytesRefHash<DirectBytesStartArray> {
    fn default() -> Self {
        BytesRefHash::new()
    }
}

pub(crate) struct StringSorterImpl<'a, BSA>
where
    BSA: BytesStartArray,
{
    tmp_offset: usize,
    compact: &'a mut Vec<i32>,
    pool: &'a mut BytesRefBlockPool,
    byte_block_pool: &'a ByteBlockPool,
    bytes_start_array: &'a BSA,
    k: usize,
    cmp: Natural,
}
impl<'a, BSA> StringSorterImpl<'a, BSA>
where
    BSA: BytesStartArray,
{
    pub fn new(
        tmp_offset: usize,
        compact: &'a mut Vec<i32>,
        pool: &'a mut BytesRefBlockPool,
        byte_block_pool: &'a ByteBlockPool,
        bytes_start_array: &'a BSA,
    ) -> Self {
        StringSorterImpl {
            tmp_offset,
            compact,
            pool,
            byte_block_pool,
            bytes_start_array,
            k: 0,
            cmp: Natural::default(),
        }
    }
    fn swap_bucket_cache(&mut self, i: usize, j: usize) -> Result<()> {
        self.swap(i, j)?;
        self.compact.swap(self.tmp_offset + i, self.tmp_offset + j);
        Ok(())
    }
}
impl<BSA> MSBRadixSorterBase for StringSorterImpl<'_, BSA>
where
    BSA: BytesStartArray,
{
    fn byte_at(&mut self, i: usize, k: usize) -> Result<i32> {
        let mut scratch = BytesRefBuilder::new();
        let mut scratch_bytes = BytesRef::new();
        self.get(&mut scratch, &mut scratch_bytes, i)?;
        self.cmp.byte_at(&scratch_bytes, k)
    }

    fn reorder(
        &mut self,
        from: usize,
        _to: usize,
        start_offsets: &mut [usize],
        end_offsets: &mut [usize],
        k: usize,
    ) -> Result<()> {
        debug_assert_eq!(self.k, k);
        for i in 0..HISTOGRAM_SIZE {
            let limit = end_offsets[i];
            loop {
                let h1 = start_offsets[i];
                if h1 >= limit {
                    break;
                }

                let idx = self.tmp_offset + from + h1;
                let b = self.compact[idx] as usize;

                let h2 = start_offsets[b];
                start_offsets[b] += 1;

                self.swap_bucket_cache(from + h1, from + h2)?;
            }
        }
        Ok(())
    }

    fn build_histogram(
        &mut self,
        prefix_common_bucket: usize,
        prefix_common_len: usize,
        from: usize,
        to: usize,
        k: usize,
        histogram: &mut [usize],
    ) -> Result<()> {
        self.k = k;
        histogram[prefix_common_bucket] = prefix_common_len;
        self.compact[(self.tmp_offset + from - prefix_common_len)..(self.tmp_offset + from)]
            .fill(prefix_common_bucket as i32);
        for i in from..to {
            let b = self.get_bucket(i, k)?;
            self.compact[self.tmp_offset + i] = b;
            histogram[b as usize] += 1;
        }
        Ok(())
    }

    fn should_fallback(&self, from: usize, to: usize, l: usize) -> bool {
        // We lower the fallback threshold because the bucket cache speeds up
        // the reorder
        to - from <= ((LEVEL_THRESHOLD) / 2) || l >= LEVEL_THRESHOLD
    }
}
impl<BSA> Sorter for StringSorterImpl<'_, BSA>
where
    BSA: BytesStartArray,
{
    fn swap(&mut self, i: usize, j: usize) -> Result<()> {
        self.compact.swap(i, j);
        Ok(())
    }
}
impl<BSA> StringSorterBase for StringSorterImpl<'_, BSA>
where
    BSA: BytesStartArray,
{
    fn get(
        &mut self,
        _builder: &mut BytesRefBuilder<Vec<u8>>,
        result: &mut BytesRef<Vec<u8>>,
        i: usize,
    ) -> Result<()> {
        let start = self.bytes_start_array.get_value(self.compact[i] as usize);
        self.pool
            .fill_bytes_ref(result, start, self.byte_block_pool);
        Ok(())
    }

    fn radix_sorter<'b, C1>(&'b mut self, cmp: &'b mut C1) -> impl Sorter + 'b
    where
        C1: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
        Self: Sorter + Sized,
    {
        let length = cmp.compared_bytes_count();
        let delegate = MSBStringHashRadixSorter::new(cmp, self);
        MSBRadixSorter::new(length, delegate)
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
    fn bytes_used(&mut self) -> SharedCounter;
    fn get_value(&self, index: usize) -> i32;
    fn set_value(&mut self, index: usize, value: i32);
    fn len(&self) -> usize;
    fn need_init(&self) -> bool;
}
/// A simple [`BytesStartArray`] that tracks memory allocation using a private
/// `Counter` instance.
pub struct DirectBytesStartArray {
    init_size: i32,
    bytes_start: Option<Vec<i32>>,
    bytes_used: SharedCounter,
}
impl DirectBytesStartArray {
    pub fn new(init_size: i32) -> Self {
        DirectBytesStartArray::with_counter(init_size, Arc::new(AtomicCounter::new()))
    }
    pub fn with_counter(init_size: i32, counter: SharedCounter) -> Self {
        DirectBytesStartArray {
            init_size,
            bytes_start: None,
            bytes_used: counter,
        }
    }
}

impl BytesStartArray for DirectBytesStartArray {
    fn init(&mut self) {
        self.bytes_start = Some(vec![
            0;
            ArrayUtil::oversize(
                self.init_size as usize,
                BitUtil::INT_BYTES
            )
        ]);
    }

    fn grow(&mut self) -> Result<()> {
        debug_assert!(self.bytes_start.is_some());
        let length = self.bytes_start.as_ref().unwrap().len() as i32;
        ArrayUtil::grow_i32(self.bytes_start.as_mut().unwrap(), length as usize + 1)?;
        Ok(())
    }

    fn clear(&mut self) {
        self.bytes_start = None;
    }

    fn bytes_used(&mut self) -> SharedCounter {
        self.bytes_used.clone()
    }

    fn get_value(&self, index: usize) -> i32 {
        self.bytes_start.as_ref().unwrap()[index]
    }

    fn set_value(&mut self, index: usize, value: i32) {
        self.bytes_start.as_mut().unwrap()[index] = value;
    }

    fn len(&self) -> usize {
        self.bytes_start.as_ref().unwrap().len()
    }

    fn need_init(&self) -> bool {
        self.bytes_start.is_none()
    }
}

/// # Note
/// In Java Lucene, BytesRefHash uses MSBStringRadixSorter. Due to language
/// limitations, a new MSBStringHashRadixSorter is currently being used.
pub struct MSBStringHashRadixSorter<'a, T, C>
where
    T: StringSorterBase,
    C: BytesRefComparator,
{
    cmp: &'a mut C,
    delegate: &'a mut T,
}
impl<'a, T, C> MSBStringHashRadixSorter<'a, T, C>
where
    T: StringSorterBase,
    C: BytesRefComparator,
{
    pub fn new(cmp: &'a mut C, delegate: &'a mut T) -> MSBStringHashRadixSorter<'a, T, C> {
        MSBStringHashRadixSorter { cmp, delegate }
    }
}

impl<T, C> Sorter for MSBStringHashRadixSorter<'_, T, C>
where
    T: StringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator,
{
    fn swap(&mut self, i: usize, j: usize) -> Result<()> {
        self.delegate.swap(i, j)
    }
}

impl<T, C> MSBRadixSorterBase for MSBStringHashRadixSorter<'_, T, C>
where
    T: StringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator,
{
    fn byte_at(&mut self, i: usize, k: usize) -> Result<i32> {
        self.delegate.byte_at(i, k)
    }

    fn get_fallback_sorter(&mut self, k: usize, _length: usize) -> impl Sorter {
        self.delegate.fall_back_sorter(self.cmp, Some(k))
    }

    fn reorder(
        &mut self,
        from: usize,
        to: usize,
        start_offsets: &mut [usize],
        end_offsets: &mut [usize],
        k: usize,
    ) -> Result<()> {
        self.delegate
            .reorder(from, to, start_offsets, end_offsets, k)
    }

    fn build_histogram(
        &mut self,
        prefix_common_bucket: usize,
        prefix_common_len: usize,
        from: usize,
        to: usize,
        k: usize,
        histogram: &mut [usize],
    ) -> Result<()> {
        self.delegate.build_histogram(
            prefix_common_bucket,
            prefix_common_len,
            from,
            to,
            k,
            histogram,
        )
    }

    fn should_fallback(&self, from: usize, to: usize, l: usize) -> bool {
        self.delegate.should_fallback(from, to, l)
    }
}

pub const DEFAULT_CAPACITY: i32 = 16;
pub(crate) type DirectBytesRefHash = BytesRefHash<DirectBytesStartArray>;
#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;

    use parking_lot::Mutex;
    use rand::Rng;

    use crate::core::index::{BytesRef, BytesRefBuilder};
    use crate::core::util::allocator_byte::{AllocatorByteEnum, DirectAllocatorByte};
    use crate::core::util::bytes_ref_hash::{
        BytesRefHash, DirectBytesRefHash, DirectBytesStartArray,
    };
    use crate::core::util::error::lucene_error::{LuceneError, Result};
    use crate::core::util::{BYTE_BLOCK_SIZE, ByteBlockPool};
    use crate::test::util::lucene_test_case::lucene_test_case_util::{at_least, random};
    use crate::test::util::test_util::TestUtil;

    #[allow(dead_code)] // for quick search
    pub struct TestBytesRefHash;

    fn new_pool() -> ByteBlockPool {
        let allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
        ByteBlockPool::new(allocator)
    }
    fn new_hash<R: Rng + ?Sized>(random: &mut R) -> DirectBytesRefHash {
        let init_size = 2 << (1 + random.random_range(0..5));
        if random.random_bool(0.5) {
            BytesRefHash::new()
        } else {
            BytesRefHash::from_bytes_start_array(init_size, DirectBytesStartArray::new(init_size))
        }
    }
    #[test]
    fn test_size() -> Result<()> {
        let mut random = random();
        let mut byte_block_pool = new_pool();
        let mut hash = new_hash(&mut random);
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
                let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut byte_block_pool)?;

                if key < 0 {
                    assert_eq!(hash.size(), count,);
                } else {
                    assert_eq!(hash.size(), count + 1);
                }

                if i % mod_val == 0 {
                    hash.clear(&mut byte_block_pool);
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
        let mut byte_block_pool = new_pool();
        let mut hash = new_hash(&mut random);
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
                let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut byte_block_pool)?;

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
                hash.get(*value, &mut scratch, &byte_block_pool);
                assert_eq!(*ref_builder.get_bytes_mut_ref(), scratch);
            }

            hash.clear(&mut byte_block_pool);
            assert_eq!(hash.size(), 0);
            hash.reinit();
        }
        Ok(())
    }
    #[test]
    fn test_compact() -> Result<()> {
        let mut random = random();
        let mut byte_block_pool = new_pool();
        let mut hash = new_hash(&mut random);
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
                let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut byte_block_pool)?;

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

            hash.clear(&mut byte_block_pool);
            assert_eq!(hash.size(), 0);
            hash.reinit();
        }
        Ok(())
    }
    #[test]
    fn test_sort() -> Result<()> {
        let mut random = random();
        let mut byte_block_pool = new_pool();
        let mut hash = new_hash(&mut random);
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
                hash.add(ref_builder.get_bytes_mut_ref(), &mut byte_block_pool)?;
                strings.insert(str_value);
            }

            for _ in 0..3 {
                hash.sort(&byte_block_pool)?;
                let len = hash.ids.len();
                assert!(strings.len() < len);
                let mut scratch = BytesRef::new();
                for (i, string) in strings.iter().enumerate() {
                    ref_builder.copy_chars_with_string(string);
                    let bytes_id = hash.ids[i];
                    hash.get(bytes_id, &mut scratch, &byte_block_pool);
                    let sorted_ref = scratch.clone();
                    assert_eq!(
                        *ref_builder.get_bytes_mut_ref(),
                        sorted_ref,
                        "Sorted value mismatch at index {}",
                        i
                    );
                }
            }

            hash.clear(&mut byte_block_pool);
            assert_eq!(hash.size(), 0, "Hash should be empty after clear.");
            hash.reinit();
        }
        Ok(())
    }

    #[test]
    fn test_add() -> Result<()> {
        let mut random = random();
        let mut byte_block_pool = new_pool();
        let mut hash = new_hash(&mut random);
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
                let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut byte_block_pool)?;

                if key >= 0 {
                    assert!(strings.insert(str_value.clone()));
                    assert_eq!(unique_count, key);
                    assert_eq!(hash.size(), count + 1);
                    unique_count += 1;
                } else {
                    assert!(!strings.insert(str_value.clone()));
                    assert!((-key - 1) < count);
                    hash.get(-key - 1, &mut scratch, &byte_block_pool);
                    assert_eq!(str_value, scratch.utf8_to_string()?);
                    assert_eq!(count, hash.size());
                }
            }

            assert_all_in(&strings, &mut hash, &mut byte_block_pool)?;
            hash.clear(&mut byte_block_pool);
            assert_eq!(hash.size(), 0);
            hash.reinit();
        }
        Ok(())
    }
    #[test]
    fn test_find() -> Result<()> {
        let mut random = random();
        let mut byte_block_pool = new_pool();
        let mut hash = new_hash(&mut random);
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
                let key = hash.find(ref_builder.get_bytes_mut_ref(), &byte_block_pool);

                if key >= 0 {
                    assert!(!strings.insert(str_value.clone()));
                    assert!(key < count);
                    hash.get(key, &mut scratch, &byte_block_pool);
                    assert_eq!(str_value, scratch.utf8_to_string()?);
                    assert_eq!(count, hash.size());
                } else {
                    let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut byte_block_pool)?;
                    assert!(strings.insert(str_value.clone()));
                    assert_eq!(unique_count, key);
                    assert_eq!(hash.size(), count + 1);
                    unique_count += 1;
                }
            }

            assert_all_in(&strings, &mut hash, &mut byte_block_pool)?;
            hash.clear(&mut byte_block_pool);
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
            let byte_block_pool = Arc::new(Mutex::new(new_pool()));
            let hash = Arc::new(Mutex::new(new_hash(&mut random)));

            {
                let mut hash_guard = hash.lock();
                for _ in 0..num_strings {
                    let str_value =
                        TestUtil::random_realistic_unicode_string_range(&mut random, 1, 1000);
                    hash_guard.add(
                        &BytesRef::from_string(&str_value),
                        &mut byte_block_pool.lock(),
                    )?;
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
                let byte_block_pool = byte_block_pool.clone();

                let handle = thread::spawn(move || {
                    let mut scratch = BytesRef::new();
                    barrier_clone.wait();

                    for k in 0..loops {
                        let strings_guard = strings_clone.lock();
                        let find =
                            BytesRef::from_string(&strings_guard[k as usize % strings_guard.len()]);
                        drop(strings_guard);

                        let hash_guard = hash_clone.lock();
                        let id = hash_guard.find(&find, &byte_block_pool.lock());

                        if id < 0 {
                            not_found_clone.fetch_add(1, Ordering::SeqCst);
                        } else {
                            hash_guard.get(id, &mut scratch, &byte_block_pool.lock());
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

            hash.lock().clear(&mut byte_block_pool.lock());
            assert_eq!(hash.lock().size(), 0, "Hash should be empty after clear.");
            hash.lock().reinit();
        }

        Ok(())
    }
    #[test]
    fn test_large_value() -> Result<()> {
        let mut random = random();
        let mut byte_block_pool = new_pool();
        let mut hash = new_hash(&mut random);

        let sizes = [
            random.random_range(0..5),
            BYTE_BLOCK_SIZE - 33 + random.random_range(0..31),
            BYTE_BLOCK_SIZE - 1 + random.random_range(0..37),
        ];

        for (i, &size) in sizes.iter().enumerate() {
            let mut ref_bytes = BytesRef::new();
            ref_bytes.bytes = vec![0; size as usize];
            ref_bytes.offset = 0;
            ref_bytes.length = size as usize;

            match hash.add(&ref_bytes, &mut byte_block_pool) {
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
        let mut pool = new_pool();
        let mut hash = new_hash(&mut random);
        let mut offset_hash = new_hash(&mut random);
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
                let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut pool)?;

                if key >= 0 {
                    assert!(strings.insert(str_value.clone()));
                    assert_eq!(unique_count, key);
                    assert_eq!(hash.size(), count + 1);

                    let offset_key =
                        offset_hash.add_by_pool_offset(hash.byte_start(key), &mut pool)?;
                    assert_eq!(unique_count, offset_key);
                    assert_eq!(offset_hash.size(), count + 1);

                    unique_count += 1;
                } else {
                    assert!(!strings.insert(str_value.clone()));
                    assert!((-key - 1) < count);
                    hash.get(-key - 1, &mut scratch, &pool);
                    assert_eq!(str_value, scratch.utf8_to_string()?);
                    assert_eq!(count, hash.size());
                    let offset_key =
                        offset_hash.add_by_pool_offset(hash.byte_start(-key - 1), &mut pool)?;
                    assert!((-offset_key - 1) < count);
                    hash.get(-offset_key - 1, &mut scratch, &pool);
                    assert_eq!(str_value, scratch.utf8_to_string()?);
                    assert_eq!(count, hash.size());
                }
            }

            assert_all_in(&strings, &mut hash, &mut pool)?;

            for string in &strings {
                ref_builder.copy_chars_with_string(string);
                let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut pool)?;
                offset_hash.get(-key - 1, &mut scratch, &pool);
                let bytes_ref = scratch.clone();
                assert_eq!(
                    *ref_builder.get_bytes_mut_ref(),
                    bytes_ref,
                    "Values should match."
                );
            }

            hash.clear(&mut pool);
            assert_eq!(hash.size(), 0, "Hash should be empty after clear.");
            offset_hash.clear(&mut pool);
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

    fn assert_all_in(
        strings: &HashSet<String>,
        hash: &mut DirectBytesRefHash,
        pool: &mut ByteBlockPool,
    ) -> Result<()> {
        let mut ref_builder = BytesRefBuilder::new();
        let mut scratch = BytesRef::new();
        let count = hash.size();

        for string in strings {
            ref_builder.copy_chars_with_string(string);
            let key = hash.add(ref_builder.get_bytes_mut_ref(), pool)?; // add again to check duplicates
            hash.get((-key) - 1, &mut scratch, pool);
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
