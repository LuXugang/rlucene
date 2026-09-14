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

use crate::core::index::{BytesRef, BytesRefBuilder, BytesRefValueEnum};
use crate::core::util::access::WritableVec;
use crate::core::util::accountable::Accountable;
use crate::core::util::allocator_byte::DirectTrackingAllocatorByte;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::byte_block_pool::{BYTE_BLOCK_MASK, BYTE_BLOCK_SHIFT, BYTE_BLOCK_SIZE};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::ram_usage_estimator::size_of_vec;
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
  pub fn new(byte_used: SharedCounter) -> Result<BytesRefArray> {
    let allocator = DirectTrackingAllocatorByte::new(byte_used.clone());
    let mut pool = ByteBlockPool::new(allocator);
    pool.next_buffer()?;
    let offsets = vec![0];
    byte_used.add_and_get(size_of_vec(&offsets));
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
  pub fn get<'a>(
    &self,
    spare: &'a mut BytesRefBuilder<Vec<u8>>,
    index: usize,
  ) -> Result<&'a BytesRef<Vec<u8>>> {
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

    spare.grow_no_copy(length)?;
    spare.set_length(length);

    spare.bytes_mut().bytes.access_mut(|bytes| {
      self.pool.read_bytes(offset as i64, bytes, 0, length)?;
      // Help the compiler infer types.
      Ok::<(), LuceneError>(())
    })?;

    Ok(spare.get_bytes_ref())
  }

  /// Used only by the sorting function below to set a [`BytesRef`] with the
  /// specified slice, avoiding copying bytes in the common case when the
  /// slice is contained in a single block in the byte block pool.
  #[allow(
    clippy::owned_cow,
    reason = "Matches ByteBlockPool's Vec carrier, which preserves reusable cross-block capacity"
  )]
  fn set_bytes_ref<'a>(
    &'a self,
    spare: &mut BytesRefBuilder<Vec<u8>>,
    result: &mut BytesRef<Cow<'a, Vec<u8>>>,
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

    self
      .pool
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
  pub fn sort<'a, C>(&'a self, comp: C, stable: bool) -> Result<SortState>
  where
    C: BytesRefComparator<Cow<'a, Vec<u8>>>,
  {
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
/// - `bytes`: The [`BytesRef`] to append.
///
/// # Returns
/// The index of the appended bytes.
///
/// [`BytesRef`]
///
/// [`BytesRefArray`]
impl<'a> SortableBytesRefArray<'a> for BytesRefArray {
  fn append(&mut self, bytes: &BytesRef<Vec<u8>>) -> Result<usize> {
    if self.last_element >= self.offsets.len() {
      let old_size = size_of_vec(&self.offsets);
      let min_size = self.offsets.len() + 1;
      ArrayUtil::grow_with_len(&mut self.offsets, min_size)?;
      self
        .byte_used
        .add_and_get(size_of_vec(&self.offsets) - old_size);
    }
    self.pool.append_bytes_ref(bytes)?;
    self.offsets[self.last_element] = self.current_offset;
    self.last_element += 1;
    self.current_offset += bytes.length;
    Ok(self.last_element - 1)
  }

  fn clear(&mut self) {
    self.last_element = 0;
    self.current_offset = 0;
    self.offsets.fill(0);
    self.pool.reset(false, true) // no need to 0 fill the buffers we control
    // the allocator
  }

  fn size(&self) -> usize {
    self.last_element
  }

  /// Returns an [`IndexedBytesRefIterator`] with point-in-time semantics. The
  /// iterator provides access to all [`BytesRef`] instances appended so
  /// far.
  ///
  /// # Parameters
  /// - `comp`: An optional [`Comparator`](crate::core::util::comparator::Comparator) to specify the order of iteration.
  ///   the iterator will iterate the byte values in the order specified by
  ///   the comparator.
  ///
  /// # Note
  /// - This is a non-destructive operation.
  type Iter = IndexedBytesRefIteratorImpl<'a>;

  fn iterator<C>(&'a self, comp: C) -> Result<Self::Iter>
  where
    C: BytesRefComparator<Cow<'a, Vec<u8>>>,
  {
    let ords = self.sort(comp, false)?;
    Ok(self.iterator_with_state(Arc::from(ords)))
  }
}

#[derive(Clone, Debug)]
pub struct SortState {
  pub(crate) indices: Option<Vec<usize>>,
}
impl SortState {
  pub(crate) fn new(indices: Option<Vec<usize>>) -> SortState {
    SortState { indices }
  }
}
impl Accountable for SortState {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(self.indices.as_ref().map_or(0, size_of_vec))
  }
}

pub struct IndexedBytesRefIteratorImpl<'a> {
  pos: usize,
  pub(crate) ord: usize,
  sort_state: Arc<SortState>,
  size: usize,
  bytes_ref_array: &'a BytesRefArray,
  result: BytesRef<Vec<u8>>,
}
impl<'a> IndexedBytesRefIteratorImpl<'a> {
  fn new(
    sort_state: Arc<SortState>,
    bytes_ref_array: &'a BytesRefArray,
  ) -> IndexedBytesRefIteratorImpl<'a> {
    if let Some(indices) = &sort_state.indices {
      debug_assert_eq!(indices.len(), bytes_ref_array.size());
    }
    Self {
      pos: 0,
      ord: 0,
      sort_state,
      size: bytes_ref_array.size(),
      bytes_ref_array,
      result: BytesRef::new(),
    }
  }
}
impl IndexedBytesRefIterator for IndexedBytesRefIteratorImpl<'_> {
  fn next(&mut self) -> Result<Option<(usize, BytesRefValueEnum<'_>)>> {
    if self.pos < self.size {
      self.ord = match self.sort_state.indices.as_ref() {
        None => self.pos,
        Some(indices) => indices[self.pos],
      };

      let array = self.bytes_ref_array;
      if self.ord >= array.last_element {
        return Err(LuceneError::array_index_out_of_bounds(format!(
          "index: {}, last_element: {}",
          self.ord, array.last_element
        )));
      }
      let offset = array.offsets[self.ord];
      let length = if self.ord == array.last_element - 1 {
        array.current_offset - offset
      } else {
        array.offsets[self.ord + 1] - offset
      };
      // Retain the growth error before updating the result or converting
      // the offset, even when a contiguous value no longer needs a copy.
      if self.result.bytes.len() < length {
        ArrayUtil::oversize(length, size_of::<u8>())?;
      }
      let offset = offset as i64;
      let pos = (offset & BYTE_BLOCK_MASK as i64) as usize;
      if pos + length > BYTE_BLOCK_SIZE as usize {
        ArrayUtil::grow_no_copy(&mut self.result.bytes, length)?;
      }
      self.result.length = length;
      let buffer_index: i32 = (offset >> BYTE_BLOCK_SHIFT).try_convert()?;
      let value = if pos + length <= BYTE_BLOCK_SIZE as usize {
        let bytes = &array.pool.get_buffer(buffer_index as usize)[pos..pos + length];
        self.result.offset = 0;
        BytesRefValueEnum::Slice(BytesRef {
          bytes,
          offset: 0,
          length,
        })
      } else {
        self.result.offset = 0;
        array
          .pool
          .read_bytes(offset, &mut self.result.bytes, 0, length)?;
        BytesRefValueEnum::Buffer(Cow::Borrowed(&self.result))
      };
      self.pos += 1;
      Ok(Some((self.ord, value)))
    } else {
      Ok(None)
    }
  }
  fn ord(&self) -> usize {
    self.ord
  }
}

pub trait IndexedBytesRefIterator {
  /// Returns the next ordinal and value, borrowing a pool block when contiguous.
  fn next(&mut self) -> Result<Option<(usize, BytesRefValueEnum<'_>)>>;

  /// Returns the ordinal position of the element that was returned in the
  /// latest call to [`next`](Self::next).
  ///
  /// # Warning
  /// This method must not be called if [`next`](Self::next) has
  /// not been called yet, or if the last call to
  /// [`next`](Self::next) returned `None`.
  fn ord(&self) -> usize;
}

struct StableStringSorterImpl<'a, 'o> {
  tmp: Vec<usize>,
  ordered_entries: &'o mut [usize],
  bytes_ref_array: &'a BytesRefArray,
}
impl Sorter for StableStringSorterImpl<'_, '_> {
  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    self.ordered_entries.swap(i, j);
    Ok(())
  }
}

impl<'a> StringSorterBase for StableStringSorterImpl<'a, '_> {
  type Bytes = Cow<'a, Vec<u8>>;

  fn get(
    &mut self,
    builder: &mut BytesRefBuilder<Vec<u8>>,
    result: &mut BytesRef<Cow<'a, Vec<u8>>>,
    i: usize,
  ) -> Result<()> {
    self
      .bytes_ref_array
      .set_bytes_ref(builder, result, self.ordered_entries[i])
  }
}

impl StableStringSorterBase for StableStringSorterImpl<'_, '_> {
  fn save(&mut self, i: usize, j: usize) {
    self.tmp[j] = self.ordered_entries[i];
  }
  fn restore(&mut self, i: usize, j: usize) {
    self.ordered_entries.copy_from(&self.tmp[i..j], i);
  }
}
impl MSBRadixSorterBase for StableStringSorterImpl<'_, '_> {}

struct StringSorterImpl<'a, 'o> {
  ordered_entries: &'o mut [usize],
  bytes_ref_array: &'a BytesRefArray,
}
impl Sorter for StringSorterImpl<'_, '_> {
  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    self.ordered_entries.swap(i, j);
    Ok(())
  }
}
impl<'a> StringSorterBase for StringSorterImpl<'a, '_> {
  type Bytes = Cow<'a, Vec<u8>>;

  fn get(
    &mut self,
    builder: &mut BytesRefBuilder<Vec<u8>>,
    result: &mut BytesRef<Cow<'a, Vec<u8>>>,
    i: usize,
  ) -> Result<()> {
    self
      .bytes_ref_array
      .set_bytes_ref(builder, result, self.ordered_entries[i])
  }
}
