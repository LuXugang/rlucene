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
use std::borrow::Cow;
use std::sync::Arc;

use crate::index::{BytesRef, BytesRefBuilder};
use crate::util::access::{SharedAccess, SharedAccessVec};
use crate::util::accountable::Accountable;
use crate::util::allocator_byte::{AllocatorByteEnum, DirectAllocatorByte};
use crate::util::bit_util::BitUtil;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::sortable_bytes_ref_array::SortableBytesRefArray;
use crate::util::{
    ByteBlockPool, BytesRefComparator, Comparator, Counter, CounterEnum, CounterEnumBorrow,
    CounterEnumLock, MSBRadixSorterBase, SliceCopyOps, Sorter, StableStringSorter,
    StableStringSorterBase, StringSorter, StringSorterBase,
};

/// A simple append-only random-access array that stores full copies of the
/// appended bytes in a [`ByteBlockPool`].
///
/// # Note
/// This struct is **not thread-safe!**
///
/// # Internal
/// This is an internal and experimental component.
#[derive(Debug)]
pub struct BytesRefArray<A>
where
    A: SharedAccess<CounterEnum>,
{
    pool: ByteBlockPool<A>,
    offsets: Vec<i32>,
    last_element: i32,
    current_offset: i32,
    byte_used: A,
}

/// for single-threaded scenarios
pub type STBytesRefArray = BytesRefArray<CounterEnumBorrow>;
/// for multi-threaded scenarios
pub type MTBytesRefArray = BytesRefArray<CounterEnumLock>;

macro_rules! impl_bytes_ref_array {
    ($enum_ty:ty, $method:ident, $pool_ctor:ident, $ret:ty, $doc:expr_2021) => {
        impl BytesRefArray<$enum_ty> {
            #[doc = $doc]
            pub fn $method(byte_used: $enum_ty) -> Result<$ret> {
                let allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
                let pool = ByteBlockPool::$pool_ctor(allocator);
                BytesRefArray::new_impl(pool, byte_used)
            }
        }
    };
}
impl_bytes_ref_array!(
    CounterEnumBorrow,
    new,
    new,
    STBytesRefArray,
    "for single-threaded scenarios"
);
impl_bytes_ref_array!(
    CounterEnumLock,
    new_sync,
    new_sync,
    MTBytesRefArray,
    "for multi-threaded scenarios"
);

impl<A> BytesRefArray<A>
where
    A: SharedAccess<CounterEnum>,
{
    fn new_impl(mut pool: ByteBlockPool<A>, byte_used: A) -> Result<BytesRefArray<A>> {
        pool.next_buffer()?;
        let offsets = Vec::new();
        byte_used.access_mut(|b| b.add_and_get(BitUtil::INT_BYTES as i64));
        Ok(BytesRefArray {
            pool,
            offsets,
            last_element: 0,
            current_offset: 0,
            byte_used,
        })
    }
    /// Returns the nth element of this [`BytesRefArray`].
    ///
    /// # Parameters
    /// - `spare`: A mutable reference to a [`BytesRefBuilder`] instance used as
    ///   a buffer.
    /// - `index`: The index of the element to retrieve.
    ///
    /// # Returns
    /// The nth element of this [`BytesRefArray`] as a [`BytesRef`].
    ///
    /// # Errors
    /// Returns [`LuceneError::array_index_out_of_bounds`] if the index is
    /// invalid.
    pub fn get(
        &self,
        spare: &mut BytesRefBuilder<Vec<u8>>,
        index: i32,
    ) -> Result<BytesRef<Vec<u8>>> {
        if index < 0 || index >= self.last_element {
            return Err(LuceneError::array_index_out_of_bounds(format!(
                "index: {}, last_element: {}",
                index, self.last_element
            )));
        }

        let offset = self.offsets[index as usize];
        let length = if index == self.last_element - 1 {
            self.current_offset - offset
        } else {
            self.offsets[index as usize + 1] - offset
        };

        spare.grow_no_copy(length as usize);
        spare.set_length(length as usize);

        spare.bytes_ref().bytes.access_mut(|bytes| {
            self.pool.read_bytes(offset as i64, bytes, 0, length)?;
            // Help the compiler infer types.
            Ok::<(), LuceneError>(())
        })?;

        Ok(std::mem::take(spare.bytes_ref()))
    }

    /// Used only by the sorting function below to set a [`BytesRef`] with the
    /// specified slice, avoiding copying bytes in the common case when the
    /// slice is contained in a single block in the byte block pool.
    fn set_bytes_ref<AV: SharedAccessVec<u8>>(
        &self,
        spare: &mut BytesRefBuilder<AV>,
        result: &mut BytesRef<AV>,
        index: i32,
    ) -> Result<()> {
        if index < 0 || index >= self.last_element {
            return Err(LuceneError::array_index_out_of_bounds(format!(
                "index: {}, last_element: {}",
                index, self.last_element
            )));
        }

        let offset = self.offsets[index as usize];
        let length = if index == self.last_element - 1 {
            self.current_offset - offset
        } else {
            self.offsets[index as usize + 1] - offset
        };

        self.pool
            .set_bytes_ref(spare, result, offset as i64, length)?;
        Ok(())
    }

    /// Returns a [`SortState`] representing the order of elements in this
    /// array. This is a non-destructive operation.
    ///
    /// # Parameters
    /// - `comp`: The comparator to compare [`BytesRef`]s. A radix sort
    ///   optimization is available if the comparator implements
    ///   [`BytesRefComparator`].
    /// - `stable`: Indicates if the sort needs to be stable.
    ///
    /// # Returns
    /// A [`SortState`] that can be used in
    /// [`BytesRefArray::iterator_with_state`] with the given sort state.
    pub fn sort(
        &mut self,
        comp: impl BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
        stable: bool,
    ) -> Result<SortState> {
        let size = self.size();
        let mut ordered_entries: Vec<i32> = (0..size).collect();
        if stable {
            let delegate_sorter = StableStringSorterImpl {
                tmp: vec![0; size as usize],
                ordered_entries: ordered_entries.as_mut_slice(),
                bytes_ref_array: self,
            };
            let stable_string_sorter = StableStringSorter::new(delegate_sorter);
            let mut string_sorter = StringSorter::new(stable_string_sorter, comp);
            string_sorter.sort(0, size)?;
        } else {
            let delegate_sorter = StringSorterImpl {
                ordered_entries: ordered_entries.as_mut_slice(),
                bytes_ref_array: self,
            };
            let mut string_sorter = StringSorter::new(delegate_sorter, comp);
            string_sorter.sort(0, size)?;
        }
        Ok(SortState::new(Some(ordered_entries)))
    }
    pub fn iterator(&'_ self) -> IndexedBytesRefIteratorImpl<'_, A> {
        self.iterator_with_state(Arc::from(SortState::new(None)))
    }
    /// Returns an [`IndexedBytesRefIteratorImpl`] with point-in-time semantics.
    /// The iterator provides access to all [`BytesRef`] instances appended so
    /// far.
    ///
    /// # Parameters
    /// - `sort_state`:  the iterator will iterate the byte values in the order
    ///   defined by the `sort_state`.
    ///
    /// # Note
    /// - This is a non-destructive operation.
    /// # See Also
    /// [`IndexedBytesRefIterator`]
    ///
    /// [`BytesRef`]
    pub fn iterator_with_state(
        &'_ self,
        sort_state: Arc<SortState>,
    ) -> IndexedBytesRefIteratorImpl<'_, A> {
        IndexedBytesRefIteratorImpl::new(sort_state, self)
    }
}
/// Appends a copy of the given [`BytesRef`] to this [`BytesRefArray`].
///
/// # Parameters
/// - `bytes`: The `BytesRef` to append.
///
/// # Returns
/// The index of the appended bytes.
///
/// [`BytesRef`]
///
/// [`BytesRefArray`]
impl<'a, A> SortableBytesRefArray<'a> for BytesRefArray<A>
where
    A: SharedAccess<CounterEnum> + 'a,
{
    fn append(&mut self, bytes: &BytesRef<Vec<u8>>) -> Result<i32> {
        self.pool.append_bytes_ref(bytes)?;
        self.offsets.push(self.current_offset);
        self.last_element += 1;
        self.current_offset += bytes.length as i32;
        self.byte_used
            .access_mut(|b| b.add_and_get(BitUtil::INT_BYTES as i64));
        Ok(self.last_element - 1)
    }

    fn clear(&mut self) {
        self.last_element = 0;
        self.current_offset = 0;
        self.offsets.clear();
        self.pool.reset(false, true) // no need to 0 fill the buffers we control
        // the allocator
    }

    fn size(&self) -> i32 {
        self.last_element
    }

    /// Returns a [`BytesRefIterator`] with point-in-time semantics. The
    /// iterator provides access to all [`BytesRef`] instances appended so
    /// far.
    ///
    /// # Parameters
    /// - `comp`: An optional [`Comparator`] to specify the order of iteration.
    ///   the iterator will iterate the byte values in the order specified by
    ///   the comparator.
    ///
    /// # Note
    /// - This is a non-destructive operation.
    type Iter = IndexedBytesRefIteratorImpl<'a, A>;

    fn iterator(
        &'a mut self,
        comp: impl BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
    ) -> Result<Self::Iter> {
        let ords = self.sort(comp, false)?;
        Ok(self.iterator_with_state(Arc::from(ords)))
    }
}

#[derive(Clone, Debug)]
pub struct SortState {
    pub indices: Option<Vec<i32>>,
}
impl SortState {
    pub fn new(indices: Option<Vec<i32>>) -> SortState {
        SortState { indices }
    }
}
impl Accountable for SortState {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

pub struct IndexedBytesRefIteratorImpl<'a, A>
where
    A: SharedAccess<CounterEnum>,
{
    pos: i32,
    pub(crate) ord: i32,
    sort_state: Arc<SortState>,
    spare: BytesRefBuilder<Vec<u8>>,
    size: i32,
    bytes_ref_array: &'a BytesRefArray<A>,
}
impl<'a, A> IndexedBytesRefIteratorImpl<'a, A>
where
    A: SharedAccess<CounterEnum>,
    BytesRefArray<A>: SortableBytesRefArray<'a>,
{
    fn new(
        sort_state: Arc<SortState>,
        bytes_ref_array: &'a BytesRefArray<A>,
    ) -> IndexedBytesRefIteratorImpl<'a, A> {
        Self {
            pos: -1,
            ord: -1,
            sort_state,
            spare: BytesRefBuilder::new(),
            size: bytes_ref_array.size(),
            bytes_ref_array,
        }
    }
    pub fn ord(&self) -> i32 {
        self.ord
    }
}
impl<A> BytesRefIterator for IndexedBytesRefIteratorImpl<'_, A>
where
    A: SharedAccess<CounterEnum>,
{
    fn next(&'_ mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        let mut result = BytesRef::new();
        self.pos += 1;
        if self.pos < self.size {
            self.ord = if self.sort_state.indices.is_none() {
                self.pos
            } else {
                self.sort_state.indices.as_ref().unwrap()[self.pos as usize]
            };
            self.bytes_ref_array
                .set_bytes_ref(&mut self.spare, &mut result, self.ord)?;
            Ok(Some(Cow::Owned(result)))
        } else {
            Ok(None)
        }
    }
}
impl<A> IndexedBytesRefIterator for IndexedBytesRefIteratorImpl<'_, A>
where
    A: SharedAccess<CounterEnum>,
{
    fn ord(&self) -> i32 {
        self.ord
    }
}

pub trait IndexedBytesRefIterator {
    /// Returns the ordinal position of the element that was returned in the
    /// latest call to [`next`](BytesRefIterator::next).
    ///
    /// # Warning
    /// This method must not be called if [`next`](BytesRefIterator::next) has
    /// not been called yet, or if the last call to
    /// [`next`](BytesRefIterator::next) returned `None`.
    fn ord(&self) -> i32;
}

struct StableStringSorterImpl<'a, A>
where
    A: SharedAccess<CounterEnum>,
{
    tmp: Vec<i32>,
    ordered_entries: &'a mut [i32],
    bytes_ref_array: &'a mut BytesRefArray<A>,
}
impl<A> Sorter for StableStringSorterImpl<'_, A>
where
    A: SharedAccess<CounterEnum>,
{
    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.ordered_entries.swap(i as usize, j as usize);
        Ok(())
    }
}

impl<A> StringSorterBase for StableStringSorterImpl<'_, A>
where
    A: SharedAccess<CounterEnum>,
{
    fn get(
        &mut self,
        builder: &mut BytesRefBuilder<Vec<u8>>,
        result: &mut BytesRef<Vec<u8>>,
        i: i32,
    ) -> Result<()> {
        self.bytes_ref_array
            .set_bytes_ref(builder, result, self.ordered_entries[i as usize])
    }
}

impl<A> StableStringSorterBase for StableStringSorterImpl<'_, A>
where
    A: SharedAccess<CounterEnum>,
{
    fn save(&mut self, i: i32, j: i32) {
        self.tmp[j as usize] = self.ordered_entries[i as usize];
    }
    fn restore(&mut self, i: i32, j: i32) {
        self.ordered_entries
            .copy_from(&self.tmp[i as usize..j as usize], i as usize);
    }
}
impl<A> MSBRadixSorterBase for StableStringSorterImpl<'_, A> where A: SharedAccess<CounterEnum> {}

struct StringSorterImpl<'a, A>
where
    A: SharedAccess<CounterEnum>,
{
    ordered_entries: &'a mut [i32],
    bytes_ref_array: &'a mut BytesRefArray<A>,
}
impl<A> Sorter for StringSorterImpl<'_, A>
where
    A: SharedAccess<CounterEnum>,
{
    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.ordered_entries.swap(i as usize, j as usize);
        Ok(())
    }
}
impl<A> StringSorterBase for StringSorterImpl<'_, A>
where
    A: SharedAccess<CounterEnum>,
{
    fn get(
        &mut self,
        builder: &mut BytesRefBuilder<Vec<u8>>,
        result: &mut BytesRef<Vec<u8>>,
        i: i32,
    ) -> Result<()> {
        self.bytes_ref_array
            .set_bytes_ref(builder, result, self.ordered_entries[i as usize])
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;

    use rand::Rng;

    use crate::index::{BytesRef, BytesRefBuilder};
    use crate::test::util::lucene_test_case::lucene_test_case_util::{at_least, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::bytes_ref_iterator::BytesRefIterator;
    use crate::util::error::lucene_error::Result;
    use crate::util::{BytesRefArray, CounterEnum, Natural, NaturalOrder, SortableBytesRefArray};

    #[allow(dead_code)] // for quick search
    struct TestBytesRefArray;
    #[test]
    fn test_append() -> Result<()> {
        let mut random = random();
        let counter = Rc::new(RefCell::new(CounterEnum::new_counter(false)));
        let mut list = BytesRefArray::new(counter)?;
        let mut string_list = Vec::new();

        for j in 0..2 {
            if j > 0 && random.random_bool(0.5) {
                list.clear();
                string_list.clear();
            }

            let entries = at_least(&mut random, 500) as i32;
            let mut spare = BytesRefBuilder::new();
            let init_size = list.size();

            for i in 0..entries {
                let random_realistic_unicode_string =
                    TestUtil::random_realistic_unicode_string(&mut random);
                spare.copy_chars_with_string(&random_realistic_unicode_string);
                assert_eq!(i + init_size, list.append(spare.get_bytes_mut_ref())?);
                string_list.push(random_realistic_unicode_string);
            }

            for i in 0..entries {
                assert_eq!(
                    string_list[i as usize],
                    list.get(&mut spare, i).unwrap().utf8_to_string()?,
                    "entry {} doesn't match",
                    i
                );
            }

            // Check random access
            for _i in 0..entries {
                let e = random.random_range(0..entries);
                assert_eq!(
                    string_list[e as usize],
                    list.get(&mut spare, e).unwrap().utf8_to_string()?,
                    "entry {} doesn't match",
                    e
                );
            }

            // Check iterator multiple times
            for _ in 0..2 {
                let mut iterator = list.iterator();
                for string in &string_list {
                    let value = iterator.next()?;
                    assert!(value.is_some());
                    assert_eq!(*string, value.unwrap().utf8_to_string()?,);
                }
            }
        }
        Ok(())
    }
    #[test]
    fn test_sort() -> Result<()> {
        let mut random = random();
        let counter = Rc::new(RefCell::new(CounterEnum::new_counter(false)));
        let mut list = BytesRefArray::new(counter)?;
        let mut string_list = Vec::new();

        for j in 0..5 {
            if j > 0 && random.random_bool(0.5) {
                list.clear();
                string_list.clear();
            }

            let entries = at_least(&mut random, 200) as i32;
            let mut spare = BytesRefBuilder::new();
            let init_size = list.size();

            for i in 0..entries {
                let random_realistic_unicode_string =
                    TestUtil::random_realistic_unicode_string(&mut random);
                spare.copy_chars_with_string(&random_realistic_unicode_string);
                assert_eq!(init_size + i, list.append(spare.get_bytes_mut_ref())?);
                string_list.push(random_realistic_unicode_string);
            }

            string_list.sort_by(|a, b| TestUtil::string_codepoint_comparator(a, b));
            {
                let mut iter1 = SortableBytesRefArray::iterator(&mut list, Natural::default())?;

                let mut i = 0;
                while let Some(next) = iter1.next()? {
                    assert_eq!(
                        string_list[i],
                        next.utf8_to_string()?,
                        "entry {} doesn't match",
                        i
                    );
                    i += 1;
                }
                assert!(iter1.next()?.is_none());
                assert_eq!(
                    i,
                    string_list.len(),
                    "Iterated count doesn't match sorted list size"
                );
            }

            let mut iter2 = SortableBytesRefArray::iterator(&mut list, NaturalOrder::default())?;
            let mut i = 0;
            while let Some(next) = iter2.next()? {
                assert_eq!(
                    string_list[i],
                    next.utf8_to_string()?,
                    "entry {} doesn't match",
                    i
                );
                i += 1;
            }
            assert!(iter2.next()?.is_none());
            assert_eq!(
                i,
                string_list.len(),
                "Iterated count doesn't match sorted list size"
            );
        }

        Ok(())
    }
    #[test]
    fn test_stable_sort() -> Result<()> {
        let mut random = random();

        let counter = Rc::new(RefCell::new(CounterEnum::new_counter(false)));
        let mut list = BytesRefArray::new(counter)?;

        let mut string_list = Vec::new();

        for j in 0..5 {
            if j > 0 && random.random_bool(0.5) {
                list.clear();
                string_list.clear();
            }

            let entries = at_least(&mut random, 200) as i32;

            let mut values = Vec::new();
            for _ in 0..20 {
                values.push(TestUtil::random_realistic_unicode_string(&mut random));
            }

            let mut spare = BytesRefBuilder::new();
            let init_size = list.size();
            for i in 0..entries {
                let random_realistic_unicode_string =
                    values[random.random_range(0..values.len())].clone();
                spare.copy_chars_with_string(&random_realistic_unicode_string);
                assert_eq!(init_size + i, list.append(spare.get_bytes_mut_ref())?);
                string_list.push(random_realistic_unicode_string);
            }

            string_list.sort_by(|a, b| TestUtil::string_codepoint_comparator(a, b));

            let sort_state = if random.random_bool(0.5) {
                list.sort(NaturalOrder::default(), true)?
            } else {
                list.sort(Natural::default(), true)?
            };
            let mut iter = list.iterator_with_state(Arc::new(sort_state));
            let mut i = 0;
            let mut last_ord = -1;
            let mut last = None;

            while let Some(next) = iter.next()? {
                let next = next.into_owned();
                assert_eq!(
                    string_list[i],
                    next.utf8_to_string()?,
                    "entry {} doesn't match",
                    i
                );

                if let Some(last_ref) = &last {
                    if next == *last_ref {
                        let ord = iter.ord();
                        assert!(ord > last_ord, "sort not stable: {} <= {}", ord, last_ord);
                    }
                }

                last = Some(BytesRef::deep_copy_of(&next));
                last_ord = iter.ord();
                i += 1;
            }

            assert!(iter.next()?.is_none());
            assert_eq!(
                i,
                string_list.len(),
                "Iterated count doesn't match sorted list size"
            );
        }

        Ok(())
    }
}
