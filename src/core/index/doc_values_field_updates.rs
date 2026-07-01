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
use parking_lot::Mutex;
use std::borrow::Cow;
use std::sync::Arc;

use crate::core::index::BytesRef;
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::core::index::binary_doc_values_field_updates::{
  AbstractIteratorBinary, BinaryDocValuesFieldUpdates,
};
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::numeric_doc_values_field_updates::{
  AbstractIteratorNumeric, NumericDocValuesFieldUpdates, SingleValueNumericDocValuesFieldUpdates,
};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::accountable::Accountable;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::intro_sorter::IntroSorter;
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::abstract_paged_mutable::AbstractPagedMutable;
use crate::core::util::packed::mutable_packed64_enum::MutablePacked64Enum;
use crate::core::util::packed::paged_mutable::PagedMutable;
use crate::core::util::packed::{Mutable, PackedInts, Reader};
use crate::core::util::priority_queue::{Compare, PriorityQueue};
use crate::core::util::ram_usage_estimator::size_of_string;
use crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet;
use crate::core::util::{Sorter, ToInt, TryIntoInt};
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test::support::core::index::misc::TestSingleUpdateDocValuesFieldIterator;
#[cfg(test)]
use crate::test::support::core::index::misc::TestSingleUpdateDocValuesFieldUpdates;
use std::mem::size_of_val;

/// Holds updates for a single DocValues field, for a set of documents within
/// one segment.
///
/// # Note
/// This is an experimental feature and may change in future versions.
pub(crate) struct DocValuesFieldUpdates<D>
where
  D: DocValuesFieldUpdatesBase,
{
  pub(crate) field: String,
  pub(crate) type_: DocValuesType,
  pub(crate) del_gen: i64,
  max_doc: i32,
  inner: Mutex<DocValuesFieldInner>,
  pub(crate) sub_update: D,
}
pub(crate) struct DocValuesFieldInner {
  finished: bool,
  pub docs: AbstractPagedMutable<PagedMutable>,
  pub(crate) size: usize,
  // for reused iterator
  pub docs_iter: Option<Arc<AbstractPagedMutable<PagedMutable>>>,
}

pub(crate) struct DocValuesFieldInnerIter {
  size: usize,
  // for reused iterator
  docs: Arc<AbstractPagedMutable<PagedMutable>>,
}

impl DocValuesFieldInner {
  pub(crate) fn new(bits_per_value: i32) -> Result<Self> {
    let sub_mutable =
      PagedMutable::with_overhead_ratio(PAGE_SIZE, bits_per_value, PackedInts::DEFAULT);
    let writer = AbstractPagedMutable::new(1, PAGE_SIZE, sub_mutable)?;
    Ok(Self {
      finished: false,
      docs: writer,
      size: 0,
      docs_iter: None,
    })
  }
  pub(crate) fn resize(&mut self, size: i32) -> Result<()> {
    self.docs = self.docs.resize(size as usize)?;
    Ok(())
  }
  pub(crate) fn grow(&mut self, size: i32) -> Result<()> {
    let result = self.docs.grow_with_size(size as usize)?;
    if let Some(docs) = result {
      self.docs = docs;
    }
    Ok(())
  }
  pub(crate) fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    let tmp_doc = self.docs.get(j)?;
    let value_i = self.docs.get(i)?;
    self.docs.set(j, value_i);
    self.docs.set(i, tmp_doc);
    Ok(())
  }
}
impl DocValuesFieldUpdates<DocValuesFieldUpdatesBaseEnum> {
  pub(crate) fn new<T, V>(
    max_doc: i32,
    del_gen: i64,
    field: T,
    doc_values_type: DocValuesType,
    sub_update: V,
  ) -> Result<Self>
  where
    T: Into<String>,
    V: Into<DocValuesFieldUpdatesBaseEnum>,
  {
    Self::with_sub(max_doc, del_gen, field, doc_values_type, sub_update.into())
  }
}

impl<D> DocValuesFieldUpdates<D>
where
  D: DocValuesFieldUpdatesBase,
{
  pub(crate) fn with_sub<T>(
    max_doc: i32,
    del_gen: i64,
    field: T,
    doc_values_type: DocValuesType,
    sub_update: D,
  ) -> Result<Self>
  where
    T: Into<String>,
  {
    let bits_per_value = PackedInts::bits_required(max_doc as i64 - 1)? + SHIFT;
    let inner = DocValuesFieldInner::new(bits_per_value)?;
    Ok(Self {
      field: field.into(),
      type_: doc_values_type,
      del_gen,
      max_doc,
      inner: Mutex::new(inner),
      sub_update,
    })
  }

  pub(crate) fn get_finished(&self) -> Result<bool> {
    let inner = self.inner.lock();
    Ok(inner.finished)
  }
  /// # Warning
  /// In Java Lucene, these two methods are executed within the same critical
  /// section.However, from a logical perspective, this is not necessary.
  pub(crate) fn add_value(&mut self, doc: i32, value: i64) -> Result<()> {
    let index = if self.sub_update.need_add_doc() {
      self.add(doc)?
    } else {
      0
    };
    self.sub_update.add_value(doc, value, index)
  }
  /// # Warning
  /// In Java Lucene, these two methods are executed within the same critical
  /// section.However, from a logical perspective, this is not necessary.
  pub(crate) fn add_byte_ref(&mut self, doc: i32, value: &BytesRef<Vec<u8>>) -> Result<()> {
    let index = self.add(doc)?;
    self.sub_update.add_byte_ref(doc, value, index)
  }
  /// Returns an iterator for updated documents and their values.
  pub(crate) fn iterator(&self) -> Result<DocValuesFieldIteratorEnum> {
    self.ensure_finished()?;
    let inner = self.inner.lock();
    let v = DocValuesFieldInnerIter {
      size: inner.size,
      docs: inner.docs_iter.as_ref().unwrap().clone(),
    };
    self.sub_update.iterator(v, self.del_gen)
  }
  /// Adds the value for the given `doc_id`.
  ///
  /// This method prevents conditional calls to [`DocValuesFieldIterator::long_value`]
  /// or [`DocValuesFieldIterator::binary_value`], since the implementation knows
  /// whether it is a long value iterator or a binary value iterator.
  pub(crate) fn add_iterator<T>(&mut self, doc_id: i32, iterator: &mut T) -> Result<()>
  where
    T: DocValuesFieldIterator,
  {
    let index = if self.sub_update.need_add_doc() {
      self.add(doc_id)?
    } else {
      0
    };
    self.sub_update.add_iterator(doc_id, iterator, index)
  }
  pub(crate) fn finish(&mut self) -> Result<()> {
    let mut inner = self.inner.lock();
    if inner.finished {
      return Err(LuceneError::illegal_argument("already finished"));
    }
    inner.finished = true;
    let size = inner.size;
    // shrink wrap
    if inner.size < inner.docs.size() {
      inner.resize(size as i32)?;
      self.sub_update.resize(size as i32)?;
    }

    if inner.size > 0 {
      // We need a stable sort but InPlaceMergeSorter performs lots of
      // swaps which hurt performance due to all the packed
      // ints we are using. Another option would be TimSorter,
      // but it needs additional API (copy to temp storage,
      // compare with item in temp storage, etc.), so we instead
      // use quicksort and record ords of each update to guarantee
      // stability.
      let mut ords = PackedInts::get_mutable(
        inner.size as i32,
        PackedInts::bits_required((inner.size - 1) as i64)?,
        PackedInts::DEFAULT,
      );
      for i in 0..inner.size {
        ords.set(i as i32, i as i64)
      }
      let mut sorter = IntroSorterImpl {
        ords: &mut ords,
        pivot_doc: 0,
        pivot_ord: 0,
        sub_update: &mut self.sub_update,
        inner: &mut inner,
      };
      sorter.sort(0, size)?;
    }
    inner.docs_iter = Some(Arc::new(std::mem::take(&mut inner.docs)));
    self.sub_update.finish();
    Ok(())
  }
  /// Returns true if this instance contains any updates.
  pub(crate) fn any(&self) -> bool {
    let inner = self.inner.lock();
    let result = inner.size > 0;
    if self.sub_update.need_any() {
      self.sub_update.any(result)
    } else {
      result
    }
  }
  /// Adds an update that resets the document value.
  pub(crate) fn reset(&mut self, doc: i32) -> Result<()> {
    if self.sub_update.need_reset() {
      self.sub_update.reset(doc)
    } else {
      self.add_internal(doc, HAS_NO_VALUE_MASK).map(|_| ())
    }
  }

  pub(crate) fn add(&mut self, doc: i32) -> Result<usize> {
    self.add_internal(doc, HAS_VALUE_MASK)
  }
  fn add_internal(&mut self, doc: i32, has_value_mask: i64) -> Result<usize> {
    let mut inner = self.inner.lock();
    if inner.finished {
      return Err(LuceneError::illegal_argument("already finished"));
    }
    let size = inner.size;
    debug_assert!(doc < self.max_doc, "doc must be less than max_doc");
    // TODO: If the Sorter trait changes to take long indexes, we can
    // remove that limitation
    if size == i32::MAX as usize {
      return Err(LuceneError::illegal_state(
        "cannot support more than Integer.MAX_VALUE doc/value entries",
      ));
    }
    // grow the structures to have room for more elements
    if inner.docs.size() == size {
      inner.grow(size as i32 + 1)?;
      self.sub_update.grow(size as i32 + 1)?;
    }
    let value = ((doc as i64) << 1) | has_value_mask;
    inner.docs.set(size, value);
    inner.size += 1;
    Ok(inner.size - 1)
  }
  // pub(crate) fn swap(&mut self, i: usize, j: usize) -> Result<()> {
  //     self.sub_update.swap(i, j)?;
  //     let mut inner = self.inner.lock();
  //     inner.swap(i, j)?;
  //     Ok(())
  // }
  pub(crate) fn grow(&mut self, size: i32) -> Result<()> {
    self.sub_update.grow(size)?;
    let mut inner = self.inner.lock();
    inner.grow(size)?;
    Ok(())
  }
  pub(crate) fn resize(&mut self, size: i32) -> Result<()> {
    self.sub_update.resize(size)?;
    let mut inner = self.inner.lock();
    inner.resize(size)?;
    Ok(())
  }
  pub(crate) fn ensure_finished(&self) -> Result<()> {
    let inner = self.inner.lock();
    if !inner.finished {
      return Err(LuceneError::illegal_state("call finish first"));
    }
    Ok(())
  }
}

impl<D> Accountable for DocValuesFieldUpdates<D>
where
  D: DocValuesFieldUpdatesBase,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    let inner = self.inner.lock();
    let docs_size = if let Some(docs) = &inner.docs_iter {
      (size_of_val(docs.as_ref()) as i64).saturating_add(docs.ram_bytes_used()?)
    } else {
      inner.docs.ram_bytes_used()?
    };
    Ok(
      size_of_string(&self.field)
        .saturating_add(docs_size)
        .saturating_add(self.sub_update.ram_bytes_used()?),
    )
  }
}

pub(crate) trait DocValuesFieldUpdatesBase: Accountable {
  fn finish(&mut self);
  fn add_value(&mut self, doc: i32, value: i64, index: usize) -> Result<()>;
  fn add_byte_ref(&mut self, doc: i32, value: &BytesRef<Vec<u8>>, index: usize) -> Result<()>;
  fn add_iterator<T>(&mut self, doc_id: i32, iterator: &mut T, index: usize) -> Result<()>
  where
    T: DocValuesFieldIterator;
  /// This method could be called once
  fn iterator(
    &self,
    inner: DocValuesFieldInnerIter,
    del_gen: i64,
  ) -> Result<DocValuesFieldIteratorEnum>;
  fn swap(&mut self, _i: usize, _j: usize) -> Result<()> {
    Err(LuceneError::not_implemented(""))
  }
  fn grow(&mut self, _size: i32) -> Result<()> {
    Err(LuceneError::not_implemented(""))
  }
  fn resize(&mut self, _size: i32) -> Result<()> {
    Ok(())
  }
  fn reset(&mut self, _doc: i32) -> Result<()> {
    Err(LuceneError::not_implemented(""))
  }
  fn need_reset(&self) -> bool {
    false
  }
  fn any(&self, _super_any: bool) -> bool {
    unimplemented!("must be implemented if you need to use it")
  }
  fn need_any(&self) -> bool {
    false
  }
  fn sub_type(&self) -> DocValuesType;
  fn need_add_doc(&self) -> bool {
    true
  }
}
pub type DocValuesFieldUpdatesEnum = DocValuesFieldUpdates<DocValuesFieldUpdatesBaseEnum>;
pub(crate) enum DocValuesFieldUpdatesBaseEnum {
  Numeric(NumericDocValuesFieldUpdates),
  Binary(BinaryDocValuesFieldUpdates),
  SingleValue(SingleValueDocValuesFieldUpdates),
  #[cfg(test)]
  SingleUpdate(TestSingleUpdateDocValuesFieldUpdates),
}
#[cfg(test)]
impl DocValuesFieldUpdatesBaseEnum {
  pub fn long_value(&self) -> Result<i64> {
    match self {
      DocValuesFieldUpdatesBaseEnum::SingleValue(n) => n.long_value(),
      _ => Err(LuceneError::illegal_state("not support long_value")),
    }
  }
}
impl_from_for_enum!(
    DocValuesFieldUpdatesBaseEnum,
    NumericDocValuesFieldUpdates => Numeric,
    BinaryDocValuesFieldUpdates => Binary,
    SingleValueDocValuesFieldUpdates => SingleValue,
);
#[cfg(test)]
impl From<TestSingleUpdateDocValuesFieldUpdates> for DocValuesFieldUpdatesBaseEnum {
  fn from(value: TestSingleUpdateDocValuesFieldUpdates) -> Self {
    DocValuesFieldUpdatesBaseEnum::SingleUpdate(value)
  }
}
impl Accountable for DocValuesFieldUpdatesBaseEnum {
  fn ram_bytes_used(&self) -> Result<i64> {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.ram_bytes_used(),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.ram_bytes_used(),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.ram_bytes_used(),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.ram_bytes_used(),
    }
  }
}

impl DocValuesFieldUpdatesBase for DocValuesFieldUpdatesBaseEnum {
  fn finish(&mut self) {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.finish(),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.finish(),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.finish(),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.finish(),
    }
  }

  fn add_value(&mut self, doc: i32, value: i64, index: usize) -> Result<()> {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.add_value(doc, value, index),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.add_value(doc, value, index),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.add_value(doc, value, index),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.add_value(doc, value, index),
    }
  }

  fn add_byte_ref(&mut self, doc: i32, value: &BytesRef<Vec<u8>>, index: usize) -> Result<()> {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.add_byte_ref(doc, value, index),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.add_byte_ref(doc, value, index),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.add_byte_ref(doc, value, index),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.add_byte_ref(doc, value, index),
    }
  }

  fn add_iterator<T>(&mut self, doc_id: i32, iterator: &mut T, index: usize) -> Result<()>
  where
    T: DocValuesFieldIterator,
  {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.add_iterator(doc_id, iterator, index),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.add_iterator(doc_id, iterator, index),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.add_iterator(doc_id, iterator, index),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.add_iterator(doc_id, iterator, index),
    }
  }

  fn iterator(
    &self,
    inner: DocValuesFieldInnerIter,
    del_gen: i64,
  ) -> Result<DocValuesFieldIteratorEnum> {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.iterator(inner, del_gen),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.iterator(inner, del_gen),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.iterator(inner, del_gen),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.iterator(inner, del_gen),
    }
  }

  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.swap(i, j),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.swap(i, j),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.swap(i, j),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.swap(i, j),
    }
  }

  fn grow(&mut self, _size: i32) -> Result<()> {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.grow(_size),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.grow(_size),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.grow(_size),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.grow(_size),
    }
  }

  fn resize(&mut self, _size: i32) -> Result<()> {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.resize(_size),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.resize(_size),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.resize(_size),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.resize(_size),
    }
  }

  fn reset(&mut self, _doc: i32) -> Result<()> {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.reset(_doc),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.reset(_doc),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.reset(_doc),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.reset(_doc),
    }
  }

  fn need_reset(&self) -> bool {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.need_reset(),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.need_reset(),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.need_reset(),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.need_reset(),
    }
  }

  fn any(&self, _super_any: bool) -> bool {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.any(_super_any),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.any(_super_any),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.any(_super_any),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.any(_super_any),
    }
  }

  fn need_any(&self) -> bool {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.need_any(),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.need_any(),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.need_any(),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.need_any(),
    }
  }

  fn sub_type(&self) -> DocValuesType {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.sub_type(),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.sub_type(),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.sub_type(),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.sub_type(),
    }
  }

  fn need_add_doc(&self) -> bool {
    match self {
      DocValuesFieldUpdatesBaseEnum::Numeric(n) => n.need_add_doc(),
      DocValuesFieldUpdatesBaseEnum::Binary(b) => b.need_add_doc(),
      DocValuesFieldUpdatesBaseEnum::SingleValue(s) => s.need_add_doc(),
      #[cfg(test)]
      DocValuesFieldUpdatesBaseEnum::SingleUpdate(s) => s.need_add_doc(),
    }
  }
}

struct IntroSorterImpl<'a, D>
where
  D: DocValuesFieldUpdatesBase,
{
  ords: &'a mut MutablePacked64Enum,
  pivot_doc: i64,
  pivot_ord: i64,
  sub_update: &'a mut D,
  inner: &'a mut DocValuesFieldInner,
}

impl<D> Sorter for IntroSorterImpl<'_, D>
where
  D: DocValuesFieldUpdatesBase,
{
  fn compare(&mut self, i: usize, j: usize) -> Result<i32> {
    // increasing docID order:
    // NOTE: we can have ties here, when the same docID was updated in the
    // same segment, in which case we rely on sort being
    // stable and preserving the original order so the last update to that
    // docID wins
    let cmp = (self.inner.docs.get(i)? >> 1).cmp(&(self.inner.docs.get(j)? >> 1));

    if cmp == std::cmp::Ordering::Equal {
      Ok((self.ords.get(i) - self.ords.get(j)) as i32)
    } else {
      Ok(cmp.to_int())
    }
  }

  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    let tmp_ord = self.ords.get(i);
    let value = self.ords.get(j);
    self.ords.set(i.try_convert()?, value);
    self.ords.set(j.try_convert()?, tmp_ord);
    self.inner.swap(i, j)?;
    self.sub_update.swap(i, j)?;
    Ok(())
  }

  fn set_pivot(&mut self, i: usize) -> Result<()> {
    self.pivot_doc = self.inner.docs.get(i)? >> 1;
    self.pivot_ord = self.ords.get(i);
    Ok(())
  }

  fn compare_pivot(&mut self, j: usize) -> Result<i32> {
    let mut cmp = self
      .pivot_doc
      .cmp(&((self.inner.docs.get(j)? as u64 >> 1) as i64));
    if cmp == std::cmp::Ordering::Equal {
      // If docIDs are the same, compare pivot_ord with ords[j]
      cmp = (self.pivot_ord - self.ords.get(j)).cmp(&0);
    }
    Ok(cmp.to_int())
  }

  fn sort(&mut self, from: usize, to: usize) -> Result<()> {
    IntroSorter::sort_range(self, from, to)?;
    Ok(())
  }
}

impl<D> IntroSorter for IntroSorterImpl<'_, D> where D: DocValuesFieldUpdatesBase {}

/// An iterator over documents and their updated values.
///
/// Only documents with updates are returned by this iterator, and the documents
/// are returned in increasing order.
pub trait DocValuesFieldIterator: DocValuesIterator {
  /// Returns a long value for the current document if this iterator is a long
  /// iterator.
  fn long_value(&self) -> Result<i64>;

  /// Returns a binary value for the current document if this iterator is a
  /// binary value iterator.
  fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>>;

  /// Returns the delGen for this packet.
  fn del_gen(&self) -> i64;

  /// Returns true if this document has a value.
  fn has_value(&self) -> Result<bool>;
}
pub enum DocValuesFieldIteratorEnum {
  AbstractBinary(AbstractIterator<AbstractIteratorBinary>),
  AbstractNumeric(AbstractIterator<AbstractIteratorNumeric>),
  SingleValue(SingleValueDocValuesFieldUpdatesIterator),
  #[cfg(test)]
  SingleUpdate(TestSingleUpdateDocValuesFieldIterator),
}

impl DocValuesIterator for DocValuesFieldIteratorEnum {
  fn advance_exact(&mut self, target: i32) -> Result<bool> {
    match self {
      DocValuesFieldIteratorEnum::AbstractBinary(it) => it.advance_exact(target),
      DocValuesFieldIteratorEnum::AbstractNumeric(it) => it.advance_exact(target),
      DocValuesFieldIteratorEnum::SingleValue(it) => it.advance_exact(target),
      #[cfg(test)]
      DocValuesFieldIteratorEnum::SingleUpdate(it) => it.advance_exact(target),
    }
  }
}

impl DocIdSetIterator for DocValuesFieldIteratorEnum {
  fn doc_id(&self) -> i32 {
    match self {
      DocValuesFieldIteratorEnum::AbstractBinary(it) => it.doc_id(),
      DocValuesFieldIteratorEnum::AbstractNumeric(it) => it.doc_id(),
      DocValuesFieldIteratorEnum::SingleValue(it) => it.doc_id(),
      #[cfg(test)]
      DocValuesFieldIteratorEnum::SingleUpdate(it) => it.doc_id(),
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    match self {
      DocValuesFieldIteratorEnum::AbstractBinary(it) => it.next_doc(),
      DocValuesFieldIteratorEnum::AbstractNumeric(it) => it.next_doc(),
      DocValuesFieldIteratorEnum::SingleValue(it) => it.next_doc(),
      #[cfg(test)]
      DocValuesFieldIteratorEnum::SingleUpdate(it) => it.next_doc(),
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    match self {
      DocValuesFieldIteratorEnum::AbstractBinary(it) => it.advance(target),
      DocValuesFieldIteratorEnum::AbstractNumeric(it) => it.advance(target),
      DocValuesFieldIteratorEnum::SingleValue(it) => it.slow_advance(target),
      #[cfg(test)]
      DocValuesFieldIteratorEnum::SingleUpdate(it) => it.slow_advance(target),
    }
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    match self {
      DocValuesFieldIteratorEnum::AbstractBinary(it) => it.slow_advance(target),
      DocValuesFieldIteratorEnum::AbstractNumeric(it) => it.slow_advance(target),
      DocValuesFieldIteratorEnum::SingleValue(it) => it.slow_advance(target),
      #[cfg(test)]
      DocValuesFieldIteratorEnum::SingleUpdate(it) => it.slow_advance(target),
    }
  }

  fn cost(&self) -> Result<i64> {
    match self {
      DocValuesFieldIteratorEnum::AbstractBinary(it) => it.cost(),
      DocValuesFieldIteratorEnum::AbstractNumeric(it) => it.cost(),
      DocValuesFieldIteratorEnum::SingleValue(it) => it.cost(),
      #[cfg(test)]
      DocValuesFieldIteratorEnum::SingleUpdate(it) => it.cost(),
    }
  }
}

impl DocValuesFieldIterator for DocValuesFieldIteratorEnum {
  fn long_value(&self) -> Result<i64> {
    match self {
      DocValuesFieldIteratorEnum::AbstractBinary(_) => Err(LuceneError::illegal_state(
        "long_value is not supported for binary doc values",
      )),
      DocValuesFieldIteratorEnum::AbstractNumeric(it) => it.long_value(),
      DocValuesFieldIteratorEnum::SingleValue(it) => it.long_value(),
      #[cfg(test)]
      DocValuesFieldIteratorEnum::SingleUpdate(it) => it.long_value(),
    }
  }

  fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    match self {
      DocValuesFieldIteratorEnum::AbstractBinary(it) => it.binary_value(),
      DocValuesFieldIteratorEnum::AbstractNumeric(it) => it.binary_value(),
      DocValuesFieldIteratorEnum::SingleValue(it) => it.binary_value(),
      #[cfg(test)]
      DocValuesFieldIteratorEnum::SingleUpdate(it) => it.binary_value(),
    }
  }

  fn del_gen(&self) -> i64 {
    match self {
      DocValuesFieldIteratorEnum::AbstractBinary(it) => it.del_gen(),
      DocValuesFieldIteratorEnum::AbstractNumeric(it) => it.del_gen(),
      DocValuesFieldIteratorEnum::SingleValue(it) => it.del_gen(),
      #[cfg(test)]
      DocValuesFieldIteratorEnum::SingleUpdate(it) => it.del_gen(),
    }
  }

  fn has_value(&self) -> Result<bool> {
    match self {
      DocValuesFieldIteratorEnum::AbstractBinary(it) => it.has_value(),
      DocValuesFieldIteratorEnum::AbstractNumeric(it) => it.has_value(),
      DocValuesFieldIteratorEnum::SingleValue(it) => it.has_value(),
      #[cfg(test)]
      DocValuesFieldIteratorEnum::SingleUpdate(it) => it.has_value(),
    }
  }
}

/// Wraps the given iterator as a BinaryDocValues instance.
pub(crate) struct BinaryDocValuesDVFU<T>
where
  T: DocValuesFieldIterator,
{
  pub(crate) iterator: T,
}
impl<T> BinaryDocValuesDVFU<T>
where
  T: DocValuesFieldIterator,
{
  pub fn new(iterator: T) -> Self {
    Self { iterator }
  }
}

impl<T> DocValuesIterator for BinaryDocValuesDVFU<T>
where
  T: DocValuesFieldIterator,
{
  fn advance_exact(&mut self, target: i32) -> Result<bool> {
    self.iterator.advance_exact(target)
  }
}

impl<T> DocIdSetIterator for BinaryDocValuesDVFU<T>
where
  T: DocValuesFieldIterator,
{
  fn doc_id(&self) -> i32 {
    self.iterator.doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.iterator.next_doc()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.iterator.advance(target)
  }

  fn cost(&self) -> Result<i64> {
    self.iterator.cost()
  }
}

impl<T> BinaryDocValues for BinaryDocValuesDVFU<T>
where
  T: DocValuesFieldIterator,
{
  fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    self.iterator.binary_value()
  }
}

/// Wraps the given iterator as a NumericDocValues instance.
pub(crate) struct NumericDocValuesDVFU<T>
where
  T: DocValuesFieldIterator,
{
  pub(crate) iterator: T,
}
impl<T> NumericDocValuesDVFU<T>
where
  T: DocValuesFieldIterator,
{
  pub fn new(iterator: T) -> Self {
    Self { iterator }
  }
}

impl<T> DocValuesIterator for NumericDocValuesDVFU<T>
where
  T: DocValuesFieldIterator,
{
  fn advance_exact(&mut self, target: i32) -> Result<bool> {
    self.iterator.advance_exact(target)
  }
}

impl<T> DocIdSetIterator for NumericDocValuesDVFU<T>
where
  T: DocValuesFieldIterator,
{
  fn doc_id(&self) -> i32 {
    self.iterator.doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.iterator.next_doc()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.iterator.advance(target)
  }

  fn cost(&self) -> Result<i64> {
    self.iterator.cost()
  }
}

impl<T> NumericDocValues for NumericDocValuesDVFU<T>
where
  T: DocValuesFieldIterator,
{
  fn long_value(&mut self) -> Result<i64> {
    self.iterator.long_value()
  }
}

pub(crate) struct IteratorPQCmp;
impl IteratorPQCmp {
  pub fn new() -> Self {
    Self {}
  }
}
impl<T> Compare<T> for IteratorPQCmp
where
  T: DocValuesFieldIterator,
{
  fn less_than(&self, a: &T, b: &T) -> Result<bool> {
    // Sort by smaller doc_id
    let mut cmp = a.doc_id().cmp(&b.doc_id());
    if cmp == std::cmp::Ordering::Equal {
      // If doc_id is equal, sort by larger del_gen
      cmp = b.del_gen().cmp(&a.del_gen());
      // delGen values are unique across sub-iterators, so cmp should
      // never be equal
      assert_ne!(cmp, std::cmp::Ordering::Equal);
    }
    Ok(cmp == std::cmp::Ordering::Less)
  }
}

pub(crate) struct MergedIterator<T>
where
  T: DocValuesFieldIterator,
{
  queue: PriorityQueue<T, IteratorPQCmp>,
  doc: i32,
}
impl<T> MergedIterator<T>
where
  T: DocValuesFieldIterator,
{
  pub fn new(queue: PriorityQueue<T, IteratorPQCmp>) -> Result<Self> {
    Ok(Self { queue, doc: -1 })
  }
}

impl<T> DocValuesIterator for MergedIterator<T> where T: DocValuesFieldIterator {}

impl<T> DocValuesFieldIterator for MergedIterator<T>
where
  T: DocValuesFieldIterator,
{
  fn long_value(&self) -> Result<i64> {
    self
      .queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("no top available"))?
      .long_value()
  }

  fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    self
      .queue
      .top_mut()
      .ok_or_else(|| LuceneError::illegal_state("no top available"))?
      .binary_value()
  }

  fn del_gen(&self) -> i64 {
    unreachable!("del_gen is not supported")
  }

  fn has_value(&self) -> Result<bool> {
    match self.queue.top() {
      Some(top) => top.has_value(),
      None => Err(LuceneError::illegal_state(
        "no top element in priority queue",
      )),
    }
  }
}
impl<T> DocIdSetIterator for MergedIterator<T>
where
  T: DocValuesFieldIterator,
{
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    loop {
      if self.queue.size() == 0 {
        self.doc = NO_MORE_DOCS;
        break;
      }
      let new_doc = self
        .queue
        .top()
        .ok_or_else(|| LuceneError::illegal_state("no top available"))?
        .doc_id();

      if new_doc != self.doc {
        // Ensure the new document ID is greater than the current
        // document ID
        debug_assert!(new_doc > self.doc, "doc={} new_doc={}", self.doc, new_doc);
        self.doc = new_doc;
        break;
      }

      if self
        .queue
        .top_mut()
        .ok_or_else(|| LuceneError::illegal_state("no top available"))?
        .next_doc()?
        == NO_MORE_DOCS
      {
        self.queue.pop_unchecked()?;
      } else {
        self.queue.update_top()?;
      }
    }
    Ok(self.doc)
  }
}
pub(crate) struct AbstractIterator<A>
where
  A: AbstractIteratorBase,
{
  inner: DocValuesFieldInnerIter,
  idx: usize,
  doc: i32,
  del_gen: i64,
  has_value: bool,
  sub: A,
}

impl<A> AbstractIterator<A>
where
  A: AbstractIteratorBase,
{
  pub fn new(inner: DocValuesFieldInnerIter, del_gen: i64, sub: A) -> Self {
    AbstractIterator {
      inner,
      idx: 0,
      doc: -1,
      del_gen,
      has_value: false,
      sub,
    }
  }
}

impl<A> DocValuesIterator for AbstractIterator<A> where A: AbstractIteratorBase {}

impl<A> DocIdSetIterator for AbstractIterator<A>
where
  A: AbstractIteratorBase,
{
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    if self.idx >= self.inner.size {
      self.doc = NO_MORE_DOCS;
      return Ok(self.doc);
    }
    let mut long_doc = self.inner.docs.get(self.idx)?;
    self.idx += 1;

    while self.idx < self.inner.size {
      // Scan forward to last update to this doc
      let next_long_doc = self.inner.docs.get(self.idx)?;
      if (long_doc as u64 >> 1) != (next_long_doc as u64 >> 1) {
        break;
      }
      long_doc = next_long_doc;
      self.idx += 1;
    }

    self.has_value = (long_doc & HAS_VALUE_MASK) > 0;
    if self.has_value {
      self.sub.set(self.idx - 1)?;
    }
    debug_assert!((long_doc as u64 >> SHIFT) <= i32::MAX as u64);
    self.doc = (long_doc as u64 >> SHIFT) as i32;
    Ok(self.doc)
  }
}

impl<A> DocValuesFieldIterator for AbstractIterator<A>
where
  A: AbstractIteratorBase,
{
  fn long_value(&self) -> Result<i64> {
    self.sub.long_value()
  }

  fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    self.sub.binary_value()
  }

  fn del_gen(&self) -> i64 {
    self.del_gen
  }

  fn has_value(&self) -> Result<bool> {
    Ok(self.has_value)
  }
}
pub trait AbstractIteratorBase {
  /// Called when the iterator moves to the next document.
  ///
  /// # Arguments
  ///
  /// * `idx` - The internal index to set the value to.
  fn set(&mut self, idx: usize) -> Result<()>;
  fn long_value(&self) -> Result<i64>;
  fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>>;
}

pub(crate) struct SingleValueDocValuesFieldUpdates {
  sub_update: Arc<SingleValueNumericDocValuesFieldUpdates>,
  bit_set: SparseFixedBitSet,
  has_no_value: Option<SparseFixedBitSet>,
  max_doc: i32,
  del_gen: i64,
  has_at_least_one_value: bool,
  lock: Mutex<()>,
  dov_values_type: DocValuesType,

  // for reused iterators
  bit_set_iter: Option<Arc<SparseFixedBitSet>>,
  has_no_value_iter: Option<Arc<SparseFixedBitSet>>,
}

impl SingleValueDocValuesFieldUpdates {
  pub fn new(
    sub: SingleValueNumericDocValuesFieldUpdates,
    max_doc: i32,
    del_gen: i64,
    dov_values_type: DocValuesType,
  ) -> Result<Self> {
    Ok(Self {
      sub_update: Arc::new(sub),
      bit_set: SparseFixedBitSet::new(max_doc as usize)?,
      has_no_value: None,
      max_doc,
      del_gen,
      has_at_least_one_value: false,
      lock: Mutex::new(()),
      dov_values_type,
      bit_set_iter: None,
      has_no_value_iter: None,
    })
  }
  pub fn binary_value(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    self.sub_update.binary_value()
  }
  pub fn long_value(&self) -> Result<i64> {
    self.sub_update.long_value()
  }
}

impl Accountable for SingleValueDocValuesFieldUpdates {
  fn ram_bytes_used(&self) -> Result<i64> {
    let mut size = size_of_val(self.sub_update.as_ref()) as i64;
    if let Some(bit_set) = &self.bit_set_iter {
      size = size
        .saturating_add(size_of_val(bit_set.as_ref()) as i64)
        .saturating_add(bit_set.ram_bytes_used()?);
    } else {
      size = size.saturating_add(self.bit_set.ram_bytes_used()?);
    }
    if let Some(has_no_value) = &self.has_no_value_iter {
      size = size
        .saturating_add(size_of_val(has_no_value.as_ref()) as i64)
        .saturating_add(has_no_value.ram_bytes_used()?);
    } else if let Some(has_no_value) = &self.has_no_value {
      size = size.saturating_add(has_no_value.ram_bytes_used()?);
    }
    Ok(size)
  }
}

impl DocValuesFieldUpdatesBase for SingleValueDocValuesFieldUpdates {
  fn finish(&mut self) {
    self.bit_set_iter = Some(Arc::new(std::mem::take(&mut self.bit_set)));
    if let Some(has_no_value) = self.has_no_value.take() {
      self.has_no_value_iter = Some(Arc::new(has_no_value));
    }
  }

  fn add_value(&mut self, doc: i32, value: i64, _index: usize) -> Result<()> {
    debug_assert!(self.sub_update.long_value()? == value);
    self.bit_set.set(doc as usize);

    self.has_at_least_one_value = true;
    if let Some(has_no_value) = self.has_no_value.as_mut() {
      has_no_value.clear_with_index(doc as usize);
    }

    Ok(())
  }

  fn add_byte_ref(&mut self, doc: i32, value: &BytesRef<Vec<u8>>, _index: usize) -> Result<()> {
    debug_assert!(self.sub_update.binary_value()?.as_ref() == value);
    self.bit_set.set(doc as usize);
    self.has_at_least_one_value = true;
    if let Some(has_no_value) = self.has_no_value.as_mut() {
      has_no_value.clear_with_index(doc as usize);
    }
    Ok(())
  }

  fn add_iterator<T>(&mut self, _doc_id: i32, _iterator: &mut T, _index: usize) -> Result<()>
  where
    T: DocValuesFieldIterator,
  {
    unreachable!("add_iterator is not supported")
  }

  fn iterator(
    &self,
    _inner: DocValuesFieldInnerIter,
    _del_gen: i64,
  ) -> Result<DocValuesFieldIteratorEnum> {
    let iterator = BitSetIterator::new(
      self.bit_set_iter.as_ref().unwrap().clone(),
      self.max_doc as i64,
    )?;
    Ok(DocValuesFieldIteratorEnum::SingleValue(
      SingleValueDocValuesFieldUpdatesIterator::new(
        iterator,
        self.del_gen,
        self.has_no_value_iter.clone(),
        self.sub_update.clone(),
      )?,
    ))
  }

  fn reset(&mut self, doc: i32) -> Result<()> {
    let _guide = self.lock.lock();
    self.bit_set.set(doc as usize);
    self.has_at_least_one_value = true;
    if self.has_no_value.is_none() {
      self.has_no_value = Some(SparseFixedBitSet::new(self.max_doc as usize)?);
    }
    self.has_no_value.as_mut().unwrap().set(doc as usize);
    drop(_guide);
    Ok(())
  }

  fn need_reset(&self) -> bool {
    true
  }

  fn any(&self, super_any: bool) -> bool {
    let _guide = self.lock.lock();
    let v = super_any || self.has_at_least_one_value;
    drop(_guide);
    v
  }

  fn need_any(&self) -> bool {
    true
  }

  fn sub_type(&self) -> DocValuesType {
    self.dov_values_type
  }

  fn need_add_doc(&self) -> bool {
    false
  }
}

pub trait SingleValueDocValuesFieldUpdatesBase {
  fn binary_value(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>>;
  fn long_value(&self) -> Result<i64>;
  fn sub_type(&self) -> DocValuesType;
}
pub struct SingleValueDocValuesFieldUpdatesIterator {
  del_gen: i64,
  has_no_value: Option<Arc<SparseFixedBitSet>>,
  iterator: BitSetIterator<Arc<SparseFixedBitSet>>,
  single: Arc<SingleValueNumericDocValuesFieldUpdates>,
}
impl SingleValueDocValuesFieldUpdatesIterator {
  /// Creates a new instance of `SingleValueDocValuesFieldUpdatesIterator`.
  ///
  /// # Note
  /// Avoid using the `Default` trait. Use this method
  /// instead.
  pub fn new(
    iterator: BitSetIterator<Arc<SparseFixedBitSet>>,
    del_gen: i64,
    has_no_value: Option<Arc<SparseFixedBitSet>>,
    single: Arc<SingleValueNumericDocValuesFieldUpdates>,
  ) -> Result<Self> {
    Ok(Self {
      del_gen,
      has_no_value,
      iterator,
      single,
    })
  }
}

impl DocValuesIterator for SingleValueDocValuesFieldUpdatesIterator {
  fn advance_exact(&mut self, _target: i32) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl DocValuesFieldIterator for SingleValueDocValuesFieldUpdatesIterator {
  fn long_value(&self) -> Result<i64> {
    self.single.long_value()
  }

  fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    self.single.binary_value()
  }

  fn del_gen(&self) -> i64 {
    self.del_gen
  }

  fn has_value(&self) -> Result<bool> {
    if let Some(has_no_value) = self.has_no_value.as_ref() {
      Ok(!has_no_value.get(self.iterator.doc_id() as usize)?)
    } else {
      Ok(true)
    }
  }
}
impl DocIdSetIterator for SingleValueDocValuesFieldUpdatesIterator {
  fn doc_id(&self) -> i32 {
    self.iterator.doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.iterator.next_doc()
  }
}

pub(crate) const PAGE_SIZE: i32 = 1024;
const HAS_VALUE_MASK: i64 = 1;
const HAS_NO_VALUE_MASK: i64 = 0;
// we use the first bit of each value to mark if the doc has a value or not
const SHIFT: i32 = 1;
pub fn merged_iterator<T>(subs: Vec<T>) -> Result<Option<MergedIterator<T>>>
where
  T: DocValuesFieldIterator,
{
  // Due to the characteristics of the Rust language, to reduce complexity,
  // we add the element to the queue for processing even if there is only one
  // element. if subs.len() == 1 {
  //
  // }

  // Priority queue to sort iterators by doc_id and del_gen
  let mut queue = PriorityQueue::new(subs.len(), IteratorPQCmp::new())?;

  for mut sub in subs {
    if sub.next_doc()? != NO_MORE_DOCS {
      queue.add(sub)?;
    }
  }

  if queue.size() == 0 {
    return Ok(None);
  }
  let value = MergedIterator::new(queue)?;
  Ok(Some(value))
}
/// Wraps the given iterator as a BinaryDocValues instance.
fn get_binary_doc_values<T>(iterator: T)
where
  T: DocValuesFieldIterator,
{
  BinaryDocValuesDVFU::new(iterator);
}
/// Wraps the given iterator as a NumericDocValues instance.
fn get_numeric_doc_values<T>(iterator: T)
where
  T: DocValuesFieldIterator,
{
  NumericDocValuesDVFU::new(iterator);
}
