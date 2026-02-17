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

use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::access::{SharedAccessVec, WritableVec};
use crate::core::util::accountable::Accountable;
use crate::core::util::allocator_byte::{AllocatorByteEnum, DirectAllocatorByte};
use crate::core::util::bit_util::BitUtil;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::sortable_bytes_ref_array::SortableBytesRefArray;
use crate::core::util::{
    ByteBlockPool, BytesRefComparator, Counter, MSBRadixSorterBase, SharedCounter, SliceCopyOps,
    Sorter, StableStringSorter, StableStringSorterBase, StringSorter, StringSorterBase, TryIntoInt,
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
pub struct BytesRefArray {
    pool: ByteBlockPool,
    offsets: Vec<usize>,
    last_element: usize,
    current_offset: usize,
    byte_used: SharedCounter,
}

impl BytesRefArray {
    pub(crate) fn new(byte_used: SharedCounter) -> Result<BytesRefArray> {
        let allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
        let mut pool = ByteBlockPool::new(allocator);
        pool.next_buffer()?;
        let offsets = Vec::new();
        byte_used.add_and_get(BitUtil::INT_BYTES as i64);
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
        index: usize,
    ) -> Result<BytesRef<Vec<u8>>> {
        if index >= self.last_element {
            return Err(LuceneError::array_index_out_of_bounds(format!(
                "index: {}, last_element: {}",
                index, self.last_element
            )));
        }

        let offset = self.offsets[index];
        let length = if index == self.last_element - 1 {
            self.current_offset - offset
        } else {
            self.offsets[index + 1] - offset
        };

        spare.grow_no_copy(length);
        spare.set_length(length);

        spare.bytes_ref().bytes.access_mut(|bytes| {
            self.pool
                .read_bytes(offset as i64, bytes, 0, length.try_convert()?)?;
            // Help the compiler infer types.
            Ok::<(), LuceneError>(())
        })?;

        Ok(std::mem::take(spare.bytes_ref()))
    }

    /// Used only by the sorting function below to set a [`BytesRef`] with the
    /// specified slice, avoiding copying bytes in the common case when the
    /// slice is contained in a single block in the byte block pool.
    fn set_bytes_ref<AV: SharedAccessVec<u8> + WritableVec<u8>>(
        &self,
        spare: &mut BytesRefBuilder<AV>,
        result: &mut BytesRef<AV>,
        index: usize,
    ) -> Result<()> {
        if index >= self.last_element {
            return Err(LuceneError::array_index_out_of_bounds(format!(
                "index: {}, last_element: {}",
                index, self.last_element
            )));
        }

        let offset = self.offsets[index];
        let length = if index == self.last_element - 1 {
            self.current_offset - offset
        } else {
            self.offsets[index + 1] - offset
        };

        self.pool
            .set_bytes_ref(spare, result, offset as i64, length.try_convert()?)?;
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
    pub fn sort(&self, comp: impl BytesRefComparator, stable: bool) -> Result<SortState> {
        let size = self.size();
        let mut ordered_entries: Vec<usize> = (0..size).collect();
        if stable {
            let delegate = StableStringSorterImpl {
                tmp: vec![0; size],
                ordered_entries: ordered_entries.as_mut_slice(),
                bytes_ref_array: self,
            };
            let stable_string_sorter = StableStringSorter::new(delegate);
            let mut string_sorter = StringSorter::new(stable_string_sorter, comp);
            string_sorter.sort(0, size)?;
        } else {
            let delegate = StringSorterImpl {
                ordered_entries: ordered_entries.as_mut_slice(),
                bytes_ref_array: self,
            };
            let mut string_sorter = StringSorter::new(delegate, comp);
            string_sorter.sort(0, size)?;
        }
        Ok(SortState::new(Some(ordered_entries)))
    }
    pub fn iterator(&'_ self) -> IndexedBytesRefIteratorImpl<'_> {
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
    ) -> IndexedBytesRefIteratorImpl<'_> {
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
impl<'a> SortableBytesRefArray<'a> for BytesRefArray {
    fn append(&mut self, bytes: &BytesRef<Vec<u8>>) -> Result<usize> {
        self.pool.append_bytes_ref(bytes)?;
        self.offsets.push(self.current_offset);
        self.last_element += 1;
        self.current_offset += bytes.length;
        self.byte_used.add_and_get(BitUtil::INT_BYTES as i64);
        Ok(self.last_element - 1)
    }

    fn clear(&mut self) {
        self.last_element = 0;
        self.current_offset = 0;
        self.offsets.clear();
        self.pool.reset(false, true) // no need to 0 fill the buffers we control
        // the allocator
    }

    fn size(&self) -> usize {
        self.last_element
    }

    /// Returns a [`BytesRefIterator`] with point-in-time semantics. The
    /// iterator provides access to all [`BytesRef`] instances appended so
    /// far.
    ///
    /// # Parameters
    /// - `comp`: An optional `Comparator` to specify the order of iteration.
    ///   the iterator will iterate the byte values in the order specified by
    ///   the comparator.
    ///
    /// # Note
    /// - This is a non-destructive operation.
    type Iter = IndexedBytesRefIteratorImpl<'a>;

    fn iterator(&'a self, comp: impl BytesRefComparator) -> Result<Self::Iter> {
        let ords = self.sort(comp, false)?;
        Ok(self.iterator_with_state(Arc::from(ords)))
    }
}

#[derive(Clone, Debug)]
pub struct SortState {
    pub indices: Option<Vec<usize>>,
}
impl SortState {
    pub fn new(indices: Option<Vec<usize>>) -> SortState {
        SortState { indices }
    }
}
impl Accountable for SortState {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

pub struct IndexedBytesRefIteratorImpl<'a> {
    pos: usize,
    pub(crate) ord: usize,
    sort_state: Arc<SortState>,
    spare: BytesRefBuilder<Vec<u8>>,
    size: usize,
    bytes_ref_array: &'a BytesRefArray,
    result: BytesRef<Vec<u8>>,
}
impl<'a> IndexedBytesRefIteratorImpl<'a> {
    fn new(
        sort_state: Arc<SortState>,
        bytes_ref_array: &'a BytesRefArray,
    ) -> IndexedBytesRefIteratorImpl<'a> {
        Self {
            pos: 0,
            ord: 0,
            sort_state,
            spare: BytesRefBuilder::new(),
            size: bytes_ref_array.size(),
            bytes_ref_array,
            result: BytesRef::new(),
        }
    }
}
impl BytesRefIterator for IndexedBytesRefIteratorImpl<'_> {
    fn next(&'_ mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        if self.pos < self.size {
            self.ord = match self.sort_state.indices.as_ref() {
                None => self.pos,
                Some(indices) => indices[self.pos],
            };

            self.bytes_ref_array
                .set_bytes_ref(&mut self.spare, &mut self.result, self.ord)?;
            self.pos += 1;
            Ok(Some(Cow::Borrowed(&self.result)))
        } else {
            Ok(None)
        }
    }
}
impl IndexedBytesRefIterator for IndexedBytesRefIteratorImpl<'_> {
    fn ord(&self) -> usize {
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
    fn ord(&self) -> usize;
}

struct StableStringSorterImpl<'a> {
    tmp: Vec<usize>,
    ordered_entries: &'a mut [usize],
    bytes_ref_array: &'a BytesRefArray,
}
impl Sorter for StableStringSorterImpl<'_> {
    fn swap(&mut self, i: usize, j: usize) -> Result<()> {
        self.ordered_entries.swap(i, j);
        Ok(())
    }
}

impl StringSorterBase for StableStringSorterImpl<'_> {
    fn get(
        &mut self,
        builder: &mut BytesRefBuilder<Vec<u8>>,
        result: &mut BytesRef<Vec<u8>>,
        i: usize,
    ) -> Result<()> {
        self.bytes_ref_array
            .set_bytes_ref(builder, result, self.ordered_entries[i])
    }
}

impl StableStringSorterBase for StableStringSorterImpl<'_> {
    fn save(&mut self, i: usize, j: usize) {
        self.tmp[j] = self.ordered_entries[i];
    }
    fn restore(&mut self, i: usize, j: usize) {
        self.ordered_entries.copy_from(&self.tmp[i..j], i);
    }
}
impl MSBRadixSorterBase for StableStringSorterImpl<'_> {}

struct StringSorterImpl<'a> {
    ordered_entries: &'a mut [usize],
    bytes_ref_array: &'a BytesRefArray,
}
impl Sorter for StringSorterImpl<'_> {
    fn swap(&mut self, i: usize, j: usize) -> Result<()> {
        self.ordered_entries.swap(i, j);
        Ok(())
    }
}
impl StringSorterBase for StringSorterImpl<'_> {
    fn get(
        &mut self,
        builder: &mut BytesRefBuilder<Vec<u8>>,
        result: &mut BytesRef<Vec<u8>>,
        i: usize,
    ) -> Result<()> {
        self.bytes_ref_array
            .set_bytes_ref(builder, result, self.ordered_entries[i])
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use rand::Rng;

    use crate::core::index::{BytesRef, BytesRefBuilder};
    use crate::core::util::bytes_ref_iterator::BytesRefIterator;
    use crate::core::util::error::lucene_error::Result;
    use crate::core::util::{
        AtomicCounter, BytesRefArray, IndexedBytesRefIterator, Natural, NaturalOrder,
        SortableBytesRefArray,
    };
    use crate::test::util::lucene_test_case::lucene_test_case_util::{at_least_usize, random};
    use crate::test::util::test_util::TestUtil;

    #[allow(dead_code)] // for quick search
    struct TestBytesRefArray;
    #[test]
    fn test_append() -> Result<()> {
        let mut random = random();
        let counter = Arc::new(AtomicCounter::new());
        let mut list = BytesRefArray::new(counter)?;
        let mut string_list = Vec::new();

        for j in 0..2 {
            if j > 0 && random.random_bool(0.5) {
                list.clear();
                string_list.clear();
            }

            let entries = at_least_usize(&mut random, 500);
            let mut spare = BytesRefBuilder::new();
            let init_size = list.size();
            for i in 0..entries {
                let random_realistic_unicode_string =
                    TestUtil::random_realistic_unicode_string(&mut random);
                spare.copy_chars_from_string(&random_realistic_unicode_string);
                assert_eq!(i + init_size, list.append(spare.get_bytes_mut_ref())?);
                string_list.push(random_realistic_unicode_string);
            }
            for (i, expected) in string_list.iter().take(entries).enumerate() {
                assert_eq!(
                    *expected,
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
        let counter = Arc::new(AtomicCounter::new());
        let mut list = BytesRefArray::new(counter)?;
        let mut string_list = Vec::new();

        for j in 0..5 {
            if j > 0 && random.random_bool(0.5) {
                list.clear();
                string_list.clear();
            }

            let entries = at_least_usize(&mut random, 200);
            let mut spare = BytesRefBuilder::new();
            let init_size = list.size();

            for i in 0..entries {
                let random_realistic_unicode_string =
                    TestUtil::random_realistic_unicode_string(&mut random);
                spare.copy_chars_from_string(&random_realistic_unicode_string);
                assert_eq!(init_size + i, list.append(spare.get_bytes_mut_ref())?);
                string_list.push(random_realistic_unicode_string);
            }

            string_list.sort_by(|a, b| TestUtil::string_codepoint_comparator(a, b));
            {
                let mut iter1 = SortableBytesRefArray::iterator(&list, Natural::default())?;

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

            let mut iter2 = SortableBytesRefArray::iterator(&list, NaturalOrder)?;
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

        let counter = Arc::new(AtomicCounter::new());
        let mut list = BytesRefArray::new(counter)?;

        let mut string_list = Vec::new();

        for j in 0..5 {
            if j > 0 && random.random_bool(0.5) {
                list.clear();
                string_list.clear();
            }

            let entries = at_least_usize(&mut random, 200);

            let mut values = Vec::new();
            for _ in 0..20 {
                values.push(TestUtil::random_realistic_unicode_string(&mut random));
            }

            let mut spare = BytesRefBuilder::new();
            let init_size = list.size();
            for i in 0..entries {
                let random_realistic_unicode_string =
                    values[random.random_range(0..values.len())].clone();
                spare.copy_chars_from_string(&random_realistic_unicode_string);
                assert_eq!(init_size + i, list.append(spare.get_bytes_mut_ref())?);
                string_list.push(random_realistic_unicode_string);
            }

            string_list.sort_by(|a, b| TestUtil::string_codepoint_comparator(a, b));

            let sort_state = if random.random_bool(0.5) {
                list.sort(NaturalOrder, true)?
            } else {
                list.sort(Natural::default(), true)?
            };
            let mut iter = list.iterator_with_state(Arc::new(sort_state));
            let mut i = 0;
            let mut last_ord = None;
            let mut last = None;

            while let Some(next) = iter.next()? {
                let next = next.into_owned();
                assert_eq!(
                    string_list[i],
                    next.utf8_to_string()?,
                    "entry {} doesn't match",
                    i
                );

                if let Some(last_ref) = &last
                    && next == *last_ref
                {
                    let ord = iter.ord();
                    assert!(last_ord.is_none() || Some(ord) > last_ord);
                }

                last = Some(BytesRef::deep_copy_of(&next));
                last_ord = Some(iter.ord());
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
