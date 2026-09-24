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

use crate::core::index::doc_values_field_updates::{
  AbstractIterator, AbstractIteratorBase, DocValuesFieldInnerIter, DocValuesFieldIterator,
  DocValuesFieldIteratorEnum, DocValuesFieldUpdatesBase, PAGE_SIZE,
};
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::PackedInts;
use crate::core::util::packed::abstract_paged_mutable::AbstractPagedMutable;
use crate::core::util::packed::paged_growable_writer::PagedGrowableWriter;
use crate::core::util::ram_usage_estimator::size_of_vec;
use std::mem::size_of_val;

/// A [`DocValuesFieldUpdates`](crate::core::index::doc_values_field_updates::DocValuesFieldUpdates) which holds updates for documents of a single [`BinaryDocValuesField`](crate::core::document::binary_doc_values_field::BinaryDocValuesField).
///
/// # Note
/// This API is experimental and may change in future versions.
pub(crate) struct BinaryDocValuesFieldUpdates {
  offsets: AbstractPagedMutable<PagedGrowableWriter>,
  lengths: AbstractPagedMutable<PagedGrowableWriter>,
  values: BytesRefBuilder<Vec<u8>>,

  ranges_iter: Option<
    Arc<(
      AbstractPagedMutable<PagedGrowableWriter>,
      AbstractPagedMutable<PagedGrowableWriter>,
    )>,
  >,
}
impl BinaryDocValuesFieldUpdates {
  pub(crate) fn new() -> Result<BinaryDocValuesFieldUpdates> {
    let sub_reader1 = PagedGrowableWriter::with_fill_page(1, PackedInts::FAST);
    let offsets = AbstractPagedMutable::new(1, PAGE_SIZE, sub_reader1)?;
    let sub_reader2 = PagedGrowableWriter::with_fill_page(1, PackedInts::FAST);
    let lengths = AbstractPagedMutable::new(1, PAGE_SIZE, sub_reader2)?;
    Ok(BinaryDocValuesFieldUpdates {
      offsets,
      lengths,
      values: BytesRefBuilder::new(),
      ranges_iter: None,
    })
  }
}

impl Accountable for BinaryDocValuesFieldUpdates {
  fn ram_bytes_used(&self) -> Result<i64> {
    let offsets_size = if let Some(ranges) = &self.ranges_iter {
      (size_of_val(&ranges.0) as i64).saturating_add(ranges.0.ram_bytes_used()?)
    } else {
      self.offsets.ram_bytes_used()?
    };
    let lengths_size = if let Some(ranges) = &self.ranges_iter {
      (size_of_val(&ranges.1) as i64).saturating_add(ranges.1.ram_bytes_used()?)
    } else {
      self.lengths.ram_bytes_used()?
    };
    Ok(
      offsets_size
        .saturating_add(lengths_size)
        .saturating_add(size_of_vec(&self.values.bytes().bytes)),
    )
  }
}

impl DocValuesFieldUpdatesBase for BinaryDocValuesFieldUpdates {
  fn finish(&mut self) {
    self.ranges_iter = Some(Arc::new((self.offsets.take(), self.lengths.take())));
  }

  fn add_value(&mut self, _doc: i32, _value: i64, _index: usize) -> Result<()> {
    Err(LuceneError::unreachable(
      "BinaryDocValuesFieldUpdates does not support add_value",
    ))
  }

  fn add_byte_ref(&mut self, _doc: i32, value: &BytesRef<Vec<u8>>, index: usize) -> Result<()> {
    self.offsets.set(index, self.values.length() as i64)?;
    self.lengths.set(index, value.length as i64)?;
    self.values.append(value)?;
    Ok(())
  }

  fn add_iterator<T>(&mut self, doc_id: i32, iterator: &T, index: usize) -> Result<()>
  where
    T: DocValuesFieldIterator,
  {
    let value = iterator.binary_value()?;
    self.add_byte_ref(doc_id, value, index)
  }

  fn iterator(
    &self,
    inner: DocValuesFieldInnerIter,
    del_gen: i64,
  ) -> Result<DocValuesFieldIteratorEnum> {
    let ranges_iter = self.ranges_iter.as_ref().ok_or_else(|| {
      LuceneError::illegal_state("finished binary updates have no offsets iterator")
    })?;
    let base = AbstractIteratorBinary::new(
      ranges_iter.clone(),
      // TODO: avoid copy here if iterator is called busy
      self.values.get_bytes_ref_copy()?,
    );
    Ok(DocValuesFieldIteratorEnum::AbstractBinary(
      AbstractIterator::new(inner, del_gen, base),
    ))
  }
  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    let temp_offset = self.offsets.get(j)?;
    let value = self.offsets.get(i)?;
    self.offsets.set(j, value)?;
    self.offsets.set(i, temp_offset)?;

    let tem_length = self.lengths.get(j)?;
    let length = self.lengths.get(i)?;
    self.lengths.set(j, length)?;
    self.lengths.set(i, tem_length)?;
    Ok(())
  }

  fn grow(&mut self, size: usize) -> Result<()> {
    let offset_result = self.offsets.grow_with_size(size)?;
    if let Some(offsets) = offset_result {
      self.offsets = offsets;
    }

    let length_result = self.lengths.grow_with_size(size)?;
    if let Some(lengths) = length_result {
      self.lengths = lengths;
    }
    Ok(())
  }

  fn resize(&mut self, size: usize) -> Result<()> {
    self.offsets = self.offsets.resize(size)?;
    self.lengths = self.lengths.resize(size)?;
    Ok(())
  }

  fn sub_type(&self) -> DocValuesType {
    DocValuesType::Binary
  }
}

/// # Note
/// To implement Default, we wrap the mutable reference fields here with Option.
pub struct AbstractIteratorBinary {
  ranges: Arc<(
    AbstractPagedMutable<PagedGrowableWriter>,
    AbstractPagedMutable<PagedGrowableWriter>,
  )>,
  values: BytesRef<Vec<u8>>,
}

impl AbstractIteratorBinary {
  pub fn new(
    ranges: Arc<(
      AbstractPagedMutable<PagedGrowableWriter>,
      AbstractPagedMutable<PagedGrowableWriter>,
    )>,
    values: BytesRef<Vec<u8>>,
  ) -> AbstractIteratorBinary {
    AbstractIteratorBinary { ranges, values }
  }
}
impl AbstractIteratorBase for AbstractIteratorBinary {
  fn set(&mut self, idx: usize) -> Result<()> {
    debug_assert!(self.ranges.0.get(idx)? <= i32::MAX as i64);
    self.values.offset = self.ranges.0.get(idx)? as usize;
    debug_assert!(self.ranges.1.get(idx)? <= i32::MAX as i64);
    self.values.length = self.ranges.1.get(idx)? as usize;
    Ok(())
  }

  fn long_value(&self) -> Result<i64> {
    Err(LuceneError::not_implemented(
      "BinaryDocValuesIterator does not support long_value",
    ))
  }

  fn binary_value(&self) -> Result<&BytesRef<Vec<u8>>> {
    Ok(&self.values)
  }
}
