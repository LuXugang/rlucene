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
use crate::index::{BytesRef, BytesRefBuilder};
use crate::util::accountable::Accountable;
use crate::util::bit_util::INT_BYTES;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::runtime_error::RuntimeError;
use crate::util::sortable_bytes_ref_array::SortableBytesRefArray;
use crate::util::{AllocatorEnum, ByteBlockPool, BytesRefComparator, Comparator, Counter, CounterEnum, DirectTrackingAllocator, MSBRadixSorterBase, Natural, Sorter, StableStringSorter, StableStringSorterBase, StringSorter, StringSorterBase};
use std::sync::{Arc, Mutex};

/// A simple append-only random-access array that stores full copies of the appended
/// bytes in a [`ByteBlockPool`].
///
/// # Note
/// This struct is **not thread-safe!**
///
/// # Internal
/// This is an internal and experimental component.
pub struct BytesRefArray {
    pool: ByteBlockPool,
    offsets: Vec<u32>,
    last_element: i32,
    current_offset: u32,
    byte_used: Arc<Mutex<CounterEnum>>,
}
impl BytesRefArray {
    pub fn new(byte_used: Arc<Mutex<CounterEnum>>) -> BytesRefArray {
        let pool = ByteBlockPool::new(AllocatorEnum::DTA(DirectTrackingAllocator::new(
            byte_used.clone(),
        )));
        let offsets = Vec::new();
        byte_used.lock().unwrap().add_and_get(INT_BYTES as i64);
        BytesRefArray {
            pool,
            offsets,
            last_element: 0,
            current_offset: 0,
            byte_used,
        }
    }
    /// Returns the nth element of this [`BytesRefArray`].
    ///
    /// # Parameters
    /// - `spare`: A mutable reference to a [`BytesRefBuilder`] instance used as a buffer.
    /// - `index`: The index of the element to retrieve.
    ///
    /// # Returns
    /// The nth element of this [`BytesRefArray`] as a [`BytesRef`].
    ///
    /// # Errors
    /// Returns [`RuntimeError::array_index_out_of_bounds`] if the index is invalid.
    ///
    /// # Note
    /// In Java Lucene, this method returns the `bytes_ref` field from the `BytesRefBuilder`.
    /// However, in Rust Lucene, due to language constraints and complexity,
    /// we modify the `BytesRefBuilder` directly. The caller retrieves the updated `BytesRef`
    /// by accessing the modified `BytesRefBuilder`.
    pub fn get(&self, spare: &mut BytesRefBuilder, index: i32) -> Result<BytesRef, RuntimeError> {
        if index < 0 || index >= self.last_element {
            return Err(RuntimeError::array_index_out_of_bounds(format!(
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

        spare.grow_no_copy(length);
        spare.set_length(length);

        self.pool.read_bytes(
            offset as u64,
            spare.bytes_ref().bytes.as_mut_slice(),
            0,
            length,
        );
        // TODO: should we avoid Clone here?
        Ok(spare.get().clone())
    }

    /// Used only by the sorting function below to set a [`BytesRef`] with the specified slice,
    /// avoiding copying bytes in the common case when the slice is contained in a single block
    /// in the byte block pool.
    fn set_bytes_ref(
        &self,
        spare: &mut BytesRefBuilder,
        result: &mut BytesRef,
        index: i32,
    ) -> Result<(), RuntimeError> {
        if index < 0 || index >= self.last_element {
            return Err(RuntimeError::array_index_out_of_bounds(format!(
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
            .set_bytes_ref(spare, result, offset as u64, length);
        Ok(())
    }

    pub fn sort(
        &mut self,
        comp: impl BytesRefComparator + Comparator<BytesRef>,
        stable: bool,
    ) -> Result<SortState, RuntimeError> {
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
                tmp: vec![0; size as usize],
                ordered_entries: ordered_entries.as_mut_slice(),
                bytes_ref_array: self,
            };
            let mut string_sorter = StringSorter::new(delegate_sorter, comp);
            string_sorter.sort(0, size)?;
        }
        Ok(SortState::new(Some(ordered_entries)))
    }
    pub fn iterator_no_sort(& mut self) -> impl BytesRefIterator + use<'_>  {
        IndexedBytesRefIterator::new(SortState::new(None), self)
    }
}
impl<'a> SortableBytesRefArray<'a> for BytesRefArray {
    fn append(&mut self, bytes: &BytesRef) -> i32 {
        self.pool.append_bytes_ref(bytes);
        self.offsets.push(self.current_offset);
        self.last_element += 1;
        self.current_offset += bytes.length;
        self.byte_used.lock().unwrap().add_and_get(INT_BYTES as i64);
        self.last_element - 1
    }

    fn clear(&mut self) {
        self.last_element = 0;
        self.current_offset = 0;
        self.offsets.clear();
        self.pool.reset(false, true); // no need to 0 fill the buffers we control the allocator
    }

    fn size(&self) -> i32 {
        self.last_element
    }

   
    fn iterator(
        & mut self,
        comp: impl BytesRefComparator + Comparator<BytesRef>,
    ) -> Result<impl BytesRefIterator, RuntimeError> {
        let ords = self.sort(comp, false)?;
        Ok(IndexedBytesRefIterator::new(ords, self))
    }
   
}

pub struct SortState {
    pub indices: Option<Vec<i32>>,
}
impl SortState {
    pub fn new(indices: Option<Vec<i32>>) -> SortState {
        SortState { indices }
    }
}
impl Accountable for SortState {
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

pub struct IndexedBytesRefIterator<'a> {
    pos: i32,
    ord: i32,
    sort_state: SortState,
    spare: BytesRefBuilder,
    size: i32,
    bytes_ref_array: &'a mut BytesRefArray,
}
impl<'a> IndexedBytesRefIterator<'a> {
    fn new(
        sort_state: SortState,
        bytes_ref_array: &'a mut BytesRefArray,
    ) -> IndexedBytesRefIterator<'a> {
        Self {
            pos: -1,
            ord: -1,
            sort_state,
            spare: BytesRefBuilder::new(),
            size: bytes_ref_array.size(),
            bytes_ref_array,
        }
    }
    fn ord(&self) -> i32 {
        self.ord
    }
}
impl<'a> BytesRefIterator for IndexedBytesRefIterator<'_> {
    fn next(&mut self) -> Result<Option<BytesRef>, RuntimeError> {
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
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

struct StableStringSorterImpl<'a> {
    tmp: Vec<i32>,
    ordered_entries: &'a mut [i32],
    bytes_ref_array: &'a mut BytesRefArray,
}
impl Sorter for StableStringSorterImpl<'_> {
    fn swap(&mut self, i: i32, j: i32) {
        self.ordered_entries.swap(i as usize, j as usize);
    }
}

impl StringSorterBase for StableStringSorterImpl<'_> {
    fn get(
        &mut self,
        builder: &mut BytesRefBuilder,
        result: &mut BytesRef,
        i: i32,
    ) -> Result<(), RuntimeError> {
        self.bytes_ref_array
            .set_bytes_ref(builder, result, self.ordered_entries[i as usize])
    }
}

impl StableStringSorterBase for StableStringSorterImpl<'_> {
    fn save(&mut self, i: i32, j: i32) {
        self.tmp[j as usize] = self.ordered_entries[i as usize];
    }
    fn restore(&mut self, i: i32, j: i32) {
        self.ordered_entries[i as usize..j as usize]
            .copy_from_slice(&self.tmp[i as usize..j as usize]);
    }
}
impl MSBRadixSorterBase for StableStringSorterImpl<'_> {}

struct StringSorterImpl<'a> {
    tmp: Vec<i32>,
    ordered_entries: &'a mut [i32],
    bytes_ref_array: &'a mut BytesRefArray,
}
impl Sorter for StringSorterImpl<'_> {
    fn swap(&mut self, i: i32, j: i32) {
        self.ordered_entries.swap(i as usize, j as usize);
    }
}
impl StringSorterBase for StringSorterImpl<'_> {
    fn get(
        &mut self,
        builder: &mut BytesRefBuilder,
        result: &mut BytesRef,
        i: i32,
    ) -> Result<(), RuntimeError> {
        self.bytes_ref_array
            .set_bytes_ref(builder, result, self.ordered_entries[i as usize])
    }
}
