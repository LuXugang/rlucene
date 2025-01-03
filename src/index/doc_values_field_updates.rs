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
use crate::index::binary_doc_values::BinaryDocValues;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::doc_values_type::DocValuesType;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::util::accountable::Accountable;
use crate::util::bit_set::BitSet;
use crate::util::bit_set_iterator::BitSetIterator;
use crate::util::bits::Bits;
use crate::util::error::lucene_error::LuceneError;
use crate::util::intro_sorter::IntroSorter;
use crate::util::long_values::LongValues;
use crate::util::packed::abstract_paged_mutable::AbstractPagedMutable;
use crate::util::packed::paged_mutable::PagedMutable;
use crate::util::packed::{Mutable, MutablePacked64Enum, PackedInts, Reader};
use crate::util::priority_queue::{Compare, PriorityQueue};
use crate::util::sparse_fixed_bit_set::SparseFixedBitSet;
use crate::util::Sorter;
use std::sync::{Arc, Mutex};

pub(crate) const PAGE_SIZE: u32 = 1024;
const HAS_VALUE_MASK: u64 = 1;
const HAS_NO_VALUE_MASK: u64 = 0;
// we use the first bit of each value to mark if the doc has a value or not
const SHIFT: u32 = 1;
/// Holds updates for a single DocValues field, for a set of documents within one segment.
///
/// # Note
/// This is an experimental feature and may change in future versions.
pub struct DocValuesFieldUpdates<D>
where
    D: DocValuesFieldUpdatesBase,
{
    pub field: String,
    pub doc_values_type: DocValuesType,
    pub del_gen: u64,
    max_doc: u32,
    inner: Arc<Mutex<DocValuesFieldInner>>,
    pub sub_update: D,
}
#[derive(Default)]
pub struct DocValuesFieldInner {
    pub finished: bool,
    pub docs: AbstractPagedMutable<PagedMutable>,
    pub size: u32,
}
impl DocValuesFieldInner {
    pub(crate) fn new(bits_per_value: u32) -> Result<Self, LuceneError> {
        let sub_mutable =
            PagedMutable::new_with_overhead_ratio(PAGE_SIZE, bits_per_value, PackedInts::DEFAULT);
        let writer = AbstractPagedMutable::new(bits_per_value, 1, PAGE_SIZE, sub_mutable)?;
        Ok(Self {
            finished: false,
            docs: writer,
            size: 0,
        })
    }
    pub fn resize(&mut self, size: u32) -> Result<(), LuceneError> {
        self.docs = self.docs.resize(size as u64)?;
        Ok(())
    }
    pub fn grow(&mut self, size: u32) -> Result<(), LuceneError> {
        let result = self.docs.grow_with_size(size as u64)?;
        if result.is_some() {
            self.docs = result.unwrap();
        }
        Ok(())
    }
    pub fn swap(&mut self, i: u32, j: u32) -> Result<(), LuceneError> {
        let tmp_doc = self.docs.get(j as u64)?;
        let value_i = self.docs.get(i as u64)?;
        self.docs.set(j as u64, value_i)?;
        self.docs.set(i as u64, tmp_doc)?;
        Ok(())
    }
}
impl<D> DocValuesFieldUpdates<D>
where
    D: DocValuesFieldUpdatesBase,
{
    pub fn new(
        max_doc: u32,
        del_gen: u64,
        field: String,
        doc_values_type: DocValuesType,
        sub_update: D,
    ) -> Result<Self, LuceneError> {
        let bits_per_value = PackedInts::bits_required(max_doc as i64 - 1)? + SHIFT;
        let inner = DocValuesFieldInner::new(bits_per_value)?;
        Ok(Self {
            field,
            doc_values_type,
            del_gen,
            max_doc,
            inner: Arc::new(Mutex::new(inner)),
            sub_update,
        })
    }
    fn get_finished(&self) -> bool {
        self.inner.lock().unwrap().finished
    }
    /// # Warning
    /// In Java Lucene, these two methods are executed within the same critical section.However, from a logical perspective, this is not necessary.
    pub fn add_value(&mut self, doc: u32, value: i64) -> Result<(), LuceneError> {
        let index = if self.sub_update.need_add_doc() {
            self.add(doc)?
        } else {
            0
        };
        self.sub_update.add_value(doc, value, index)
    }
    /// # Warning
    /// In Java Lucene, these two methods are executed within the same critical section.However, from a logical perspective, this is not necessary.
    fn add_byte_ref(&mut self, doc: u32, value: BytesRef) -> Result<(), LuceneError> {
        let index = self.add(doc)?;
        self.sub_update.add_byte_ref(doc, value, index)
    }
    /// Returns an iterator for updated documents and their values.
    pub fn iterator(&mut self) -> Result<impl DocValuesFieldIterator + use<'_, D>, LuceneError> {
        self.ensure_finished()?;
        self.sub_update.iterator(self.inner.clone(), self.del_gen)
    }
    /// Adds the value for the given `doc_id`.
    ///
    /// This method prevents conditional calls to [`IteratorBase::long_value`] or
    /// [`IteratorBase::binary_value`], since the implementation knows whether it is
    /// a long value iterator or a binary value iterator.
    fn add_iterator<T>(&mut self, doc_id: u32, iterator: T) -> Result<(), LuceneError>
    where
        T: DocValuesFieldIterator,
    {
        self.sub_update.add_iterator(doc_id, iterator)
    }
    pub fn finish(&mut self) -> Result<(), LuceneError> {
        let mut inner = self.inner.lock().unwrap();
        if inner.finished {
            return Err(LuceneError::illegal_argument(
                "already finished".to_string(),
            ));
        }
        inner.finished = true;
        let size = inner.size;
        // shrink wrap
        if (inner.size as u64) < inner.docs.size() {
            inner.resize(size)?;
            self.sub_update.resize(size)?;
        }

        if inner.size > 0 {
            // We need a stable sort but InPlaceMergeSorter performs lots of swaps
            // which hurts performance due to all the packed ints we are using.
            // Another option would be TimSorter, but it needs additional API (copy to
            // temp storage, compare with item in temp storage, etc.) so we instead
            // use quicksort and record ords of each update to guarantee stability.
            let mut ords = PackedInts::get_mutable(
                inner.size,
                PackedInts::bits_required((inner.size - 1) as i64)?,
                PackedInts::DEFAULT,
            )?;
            for i in 0..inner.size {
                ords.set(i as usize, i as i64)?
            }
            let mut sorter = IntroSorterImpl {
                ords: &mut ords,
                pivot_doc: 0,
                pivot_ord: 0,
                sub_update: &mut self.sub_update,
                inner: &mut inner,
            };
            sorter.sort(0, size as i32)?;
        }
        Ok(())
    }
    /// Returns true if this instance contains any updates.
    pub fn any(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        let result = inner.size > 0;
        if self.sub_update.need_any() {
            self.sub_update.any(result)
        } else {
            result
        }
    }
    /// Adds an update that resets the documents value.
    pub fn reset(&mut self, doc: u32) -> Result<(), LuceneError> {
        if self.sub_update.need_reset() {
            self.sub_update.reset(doc)
        } else {
            self.add_internal(doc, HAS_NO_VALUE_MASK).map(|_| ())
        }
    }

    pub fn add(&mut self, doc: u32) -> Result<u32, LuceneError> {
        self.add_internal(doc, HAS_VALUE_MASK)
    }
    fn add_internal(&mut self, doc: u32, has_value_mask: u64) -> Result<u32, LuceneError> {
        let mut inner = self.inner.lock().unwrap();
        if inner.finished {
            return Err(LuceneError::illegal_argument(
                "already finished".to_string(),
            ));
        }
        let size = inner.size;
        assert!(doc < self.max_doc, "doc must be less than max_doc");
        // TODO: if the Sorter interface changes to take long indexes, we can remove that limitation
        if size == i32::MAX as u32 {
            return Err(LuceneError::illegal_state(
                "cannot support more than Integer.MAX_VALUE doc/value entries".to_string(),
            ));
        }
        // grow the structures to have room for more elements
        if inner.docs.size() == size as u64 {
            inner.grow(size + 1)?;
            self.sub_update.grow(size + 1)?;
        }
        let value = ((doc as u64) << 1) | has_value_mask;
        inner.docs.set(size as u64, value as i64)?;
        inner.size += 1;
        Ok(inner.size - 1)
    }
    pub(crate) fn swap(&mut self, i: u32, j: u32) -> Result<(), LuceneError> {
        self.sub_update.swap(i, j)?;
        let mut inner = self.inner.lock().unwrap();
        inner.swap(i, j)?;
        Ok(())
    }
    pub fn grow(&mut self, size: u32) -> Result<(), LuceneError> {
        self.sub_update.grow(size)?;
        let mut inner = self.inner.lock().unwrap();
        inner.grow(size)?;
        Ok(())
    }
    pub fn resize(&mut self, size: u32) -> Result<(), LuceneError> {
        self.sub_update.resize(size)?;
        let mut inner = self.inner.lock().unwrap();
        inner.resize(size)?;
        Ok(())
    }
    pub(crate) fn ensure_finished(&self) -> Result<(), LuceneError> {
        let inner = self.inner.lock().unwrap();
        if !inner.finished {
            return Err(LuceneError::illegal_state("call finish first".to_string()));
        }
        Ok(())
    }
}
pub fn merged_iterator<T>(subs: Vec<T>) -> Result<Option<PriorityQueueIterator<T>>, LuceneError>
where
    T: DocValuesFieldIterator,
{
    // Due to the characteristics of the Rust language, in order to reduce complexity,
    // we add the element to the queue for processing even if there is only one element.
    // if subs.len() == 1 {
    //
    // }

    // Priority queue to sort iterators by doc_id and del_gen
    let mut queue = PriorityQueue::new(subs.len() as i32, IteratorPQCmp::new())?;

    for mut sub in subs {
        if sub.next_doc()? != NO_MORE_DOCS {
            queue.add(sub);
        }
    }

    if queue.size() == 0 {
        return Ok(None);
    }
    let value = PriorityQueueIterator::new(queue)?;
    Ok(Some(value))
}
impl<D> Accountable for DocValuesFieldUpdates<D>
where
    D: DocValuesFieldUpdatesBase,
{
    fn ram_bytes_used(&self) -> u64 {
        todo!()
    }
}
pub trait DocValuesFieldUpdatesBase: Accountable {
    fn add_value(&mut self, doc: u32, value: i64, index: u32) -> Result<(), LuceneError>;
    fn add_byte_ref(&mut self, doc: u32, value: BytesRef, index: u32) -> Result<(), LuceneError>;
    fn add_iterator<T: DocValuesFieldIterator>(&mut self, doc_id: u32, iterator: T) -> Result<(), LuceneError>;
    /// Returns an iterator for updated documents and their values.
    fn iterator(
        &mut self,
        inner: Arc<Mutex<DocValuesFieldInner>>,
        del_gen: u64,
    ) -> Result<impl DocValuesFieldIterator, LuceneError>;
    fn swap(&mut self, _i: u32, _j: u32) -> Result<(), LuceneError> {
        unimplemented!("any must be implemented if you need to use it")
    }
    fn grow(&mut self, _size: u32) -> Result<(), LuceneError> {
        unimplemented!("any must be implemented if you need to use it")
    }
    fn resize(&mut self, _size: u32) -> Result<(), LuceneError> {
        Ok(())
    }
    fn reset(&mut self, _doc: u32) -> Result<(), LuceneError> {
        unimplemented!("any must be implemented if you need to use it")
    }
    fn need_reset(&self) -> bool {
        false
    }
    fn any(&self, _super_any: bool) -> bool {
        unimplemented!("any must be implemented if you need to use it")
    }
    fn need_any(&self) -> bool {
        false
    }
    fn sub_type(&self) -> DocValuesType;
    fn need_add_doc(&self) -> bool {
        true
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
    fn compare(&mut self, i: i32, j: i32) -> Result<i32, LuceneError> {
        // increasing docID order:
        // NOTE: we can have ties here, when the same docID was updated in the same segment, in
        // which case we rely on sort being
        // stable and preserving original order so the last update to that docID wins
        let cmp = (self.inner.docs.get(i as u64)? >> 1).cmp(&(self.inner.docs.get(j as u64)? >> 1));

        if cmp == std::cmp::Ordering::Equal {
            Ok((self.ords.get(i as usize)? - self.ords.get(j as usize)?) as i32)
        } else {
            match cmp {
                std::cmp::Ordering::Less => Ok(-1),
                std::cmp::Ordering::Greater => Ok(1),
                std::cmp::Ordering::Equal => Ok(0),
            }
        }
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        let tmp_ord = self.ords.get(i as usize)?;
        let value = self.ords.get(j as usize)?;
        self.ords.set(i as usize, value)?;
        self.ords.set(j as usize, tmp_ord)?;
        self.inner.swap(i as u32, j as u32)?;
        self.sub_update.swap(i as u32, j as u32)?;
        Ok(())
    }

    fn set_pivot(&mut self, i: i32) -> Result<(), LuceneError> {
        self.pivot_doc = self.inner.docs.get(i as u64)? >> 1;
        self.pivot_ord = self.ords.get(i as usize)?;
        Ok(())
    }

    fn compare_pivot(&mut self, j: i32) -> Result<i32, LuceneError> {
        let mut cmp = (self.pivot_doc).cmp(&((self.inner.docs.get(j as u64)? as u64 >> 1) as i64));
        if cmp == std::cmp::Ordering::Equal {
            // If docIDs are the same, compare pivot_ord with ords[j]
            cmp = (self.pivot_ord - self.ords.get(j as usize)?).cmp(&0);
        }
        match cmp {
            std::cmp::Ordering::Less => Ok(-1),
            std::cmp::Ordering::Greater => Ok(1),
            std::cmp::Ordering::Equal => Ok(0),
        }
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), LuceneError> {
        IntroSorter::sort_range(self, from, to)?;
        Ok(())
    }
}

impl<D> IntroSorter for IntroSorterImpl<'_, D> where D: DocValuesFieldUpdatesBase {}

/// An iterator over documents and their updated values.
///
/// Only documents with updates are returned by this iterator, and the documents are returned
/// in increasing order.
pub trait DocValuesFieldIterator: DocValuesIterator + Default + PartialEq {
    fn get_binary_doc_values<T: DocValuesFieldIterator>(iterator: T) {
        BinaryDocValuesImpl::new(iterator);
    }
    fn get_numeric_doc_values<T: DocValuesFieldIterator>(iterator: T) {
        NumericDocValuesImpl::new(iterator);
    }
    /// Returns a long value for the current document if this iterator is a long iterator.
    fn long_value(&mut self) -> Result<i64, LuceneError>;

    /// Returns a binary value for the current document if this iterator is a binary value iterator.
    fn binary_value(&mut self) -> Result<BytesRef, LuceneError>;

    /// Returns the delGen for this packet.
    fn del_gen(&self) -> u64;

    /// Returns true if this document has a value.
    fn has_value(&mut self) -> bool;
}
/// Wraps the given iterator as a BinaryDocValues instance.
pub struct BinaryDocValuesImpl<T>
where
    T: DocValuesFieldIterator,
{
    iterator: T,
}
impl<T> BinaryDocValuesImpl<T>
where
    T: DocValuesFieldIterator,
{
    pub fn new(iterator: T) -> Self {
        Self { iterator }
    }
}

impl<T> DocValuesIterator for BinaryDocValuesImpl<T>
where
    T: DocValuesFieldIterator,
{
    fn advance_exact(&self, target: i32) -> bool {
        self.iterator.advance_exact(target)
    }
}

impl<T> DocIdSetIterator for BinaryDocValuesImpl<T>
where
    T: DocValuesFieldIterator,
{
    fn doc_id(&self) -> i32 {
        self.iterator.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32, LuceneError> {
        self.iterator.next_doc()
    }

    fn advance(&mut self, _target: i32) -> Result<i32, LuceneError> {
        self.iterator.advance(_target)
    }

    fn cost(&self) -> i64 {
        self.iterator.cost()
    }
}

impl<T> BinaryDocValues for BinaryDocValuesImpl<T>
where
    T: DocValuesFieldIterator,
{
    fn binary_value(&mut self) -> Result<BytesRef, LuceneError> {
        self.iterator.binary_value()
    }
}

/// Wraps the given iterator as a NumericDocValues instance.
pub struct NumericDocValuesImpl<T>
where
    T: DocValuesFieldIterator,
{
    iterator: T,
}
impl<T> NumericDocValuesImpl<T>
where
    T: DocValuesFieldIterator,
{
    pub fn new(iterator: T) -> Self {
        Self { iterator }
    }
}

impl<T> DocValuesIterator for NumericDocValuesImpl<T>
where
    T: DocValuesFieldIterator,
{
    fn advance_exact(&self, target: i32) -> bool {
        self.iterator.advance_exact(target)
    }
}

impl<T> DocIdSetIterator for NumericDocValuesImpl<T>
where
    T: DocValuesFieldIterator,
{
    fn doc_id(&self) -> i32 {
        self.iterator.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32, LuceneError> {
        self.iterator.next_doc()
    }

    fn advance(&mut self, _target: i32) -> Result<i32, LuceneError> {
        self.iterator.advance(_target)
    }

    fn cost(&self) -> i64 {
        self.iterator.cost()
    }
}

impl<T> NumericDocValues for NumericDocValuesImpl<T>
where
    T: DocValuesFieldIterator,
{
    fn long_value(&mut self) -> Result<i64, LuceneError> {
        self.iterator.long_value()
    }
}

pub struct IteratorPQCmp<T>
where
    T: DocValuesFieldIterator,
{
    _t: std::marker::PhantomData<T>,
}
impl<T> Default for IteratorPQCmp<T>
where
    T: DocValuesFieldIterator,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IteratorPQCmp<T>
where
    T: DocValuesFieldIterator,
{
    pub fn new() -> Self {
        Self {
            _t: std::marker::PhantomData,
        }
    }
}
impl<T> Compare<T> for IteratorPQCmp<T>
where
    T: DocValuesFieldIterator,
{
    fn less_than(&self, a: &T, b: &T) -> bool {
        // Sort by smaller doc_id
        let mut cmp = a.doc_id().cmp(&b.doc_id());
        if cmp == std::cmp::Ordering::Equal {
            // If doc_id is equal, sort by larger del_gen
            cmp = b.del_gen().cmp(&a.del_gen());
            // delGen values are unique across sub-iterators, so cmp should never be equal
            assert_ne!(cmp, std::cmp::Ordering::Equal);
        }
        cmp == std::cmp::Ordering::Less
    }
}

pub struct PriorityQueueIterator<T>
where
    T: DocValuesFieldIterator,
{
    queue: PriorityQueue<T, IteratorPQCmp<T>>,
    doc: i32,
}
impl<T> PriorityQueueIterator<T>
where
    T: DocValuesFieldIterator,
{
    pub fn new(queue: PriorityQueue<T, IteratorPQCmp<T>>) -> Result<Self, LuceneError> {
        Ok(Self { queue, doc: -1 })
    }
}

impl<T> DocValuesIterator for PriorityQueueIterator<T> where T: DocValuesFieldIterator {}

impl<T> PartialEq for PriorityQueueIterator<T>
where
    T: DocValuesFieldIterator,
{
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl<T> DocValuesFieldIterator for PriorityQueueIterator<T>
where
    T: DocValuesFieldIterator,
{
    fn long_value(&mut self) -> Result<i64, LuceneError> {
        self.queue.top().long_value()
    }

    fn binary_value(&mut self) -> Result<BytesRef, LuceneError> {
        self.queue.top().binary_value()
    }

    fn del_gen(&self) -> u64 {
        unreachable!("del_gen is not supported")
    }

    fn has_value(&mut self) -> bool {
        self.queue.top().has_value()
    }
}
impl<T> DocIdSetIterator for PriorityQueueIterator<T>
where
    T: DocValuesFieldIterator,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32, LuceneError> {
        loop {
            if self.queue.size() == 0 {
                self.doc = NO_MORE_DOCS;
                break;
            }
            let new_doc = self.queue.top().doc_id();

            if new_doc != self.doc {
                // Ensure the new document ID is greater than the current document ID
                debug_assert!(new_doc > self.doc, "doc={} new_doc={}", self.doc, new_doc);
                self.doc = new_doc;
                break;
            }

            if self.queue.top().next_doc()? == NO_MORE_DOCS {
                self.queue.pop();
            } else {
                self.queue.update_top();
            }
        }
        Ok(self.doc)
    }
}
impl<T> Default for PriorityQueueIterator<T>
where
    T: DocValuesFieldIterator,
{
    fn default() -> Self {
        Self {
            queue: PriorityQueue::new(0, IteratorPQCmp::new()).unwrap(),
            doc: -1,
        }
    }
}

#[derive(Default)]
pub struct AbstractIterator<A>
where
    A: AbstractIteratorBase + Default,
{
    inner: Arc<Mutex<DocValuesFieldInner>>,
    idx: u64,
    doc: i32,
    del_gen: u64,
    has_value: bool,
    sub: A,
}

impl<A> AbstractIterator<A>
where
    A: AbstractIteratorBase + Default,
{
    pub fn new(inner: Arc<Mutex<DocValuesFieldInner>>, del_gen: u64, sub: A) -> Self {
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

impl<A> DocValuesIterator for AbstractIterator<A> where A: AbstractIteratorBase + Default {}

impl<A> DocIdSetIterator for AbstractIterator<A>
where
    A: AbstractIteratorBase + Default,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32, LuceneError> {
        let mut inner = self.inner.lock().unwrap();
        if self.idx >= inner.size as u64 {
            self.doc = NO_MORE_DOCS;
            return Ok(self.doc);
        }
        let mut long_doc = inner.docs.get(self.idx)?;
        self.idx += 1;

        while self.idx < inner.size as u64 {
            // Scan forward to last update to this doc
            let next_long_doc = inner.docs.get(self.idx)?;
            if (long_doc as u64 >> 1) != (next_long_doc as u64 >> 1) {
                break;
            }
            long_doc = next_long_doc;
            self.idx += 1;
        }

        self.has_value = (long_doc & HAS_VALUE_MASK as i64) > 0;
        if self.has_value {
            self.sub.set(self.idx - 1)?;
        }
        debug_assert!((long_doc as u64 >> SHIFT) <= i32::MAX as u64);
        self.doc = (long_doc as u64 >> SHIFT) as i32;
        Ok(self.doc)
    }
}

impl<A> PartialEq for AbstractIterator<A>
where
    A: AbstractIteratorBase + Default,
{
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl<A> DocValuesFieldIterator for AbstractIterator<A>
where
    A: AbstractIteratorBase + Default,
{
    fn long_value(&mut self) -> Result<i64, LuceneError> {
        self.sub.long_value()
    }

    fn binary_value(&mut self) -> Result<BytesRef, LuceneError> {
        self.sub.binary_value()
    }

    fn del_gen(&self) -> u64 {
        self.del_gen
    }

    fn has_value(&mut self) -> bool {
        self.has_value
    }
}
pub trait AbstractIteratorBase {
    /// Called when the iterator moves to the next document.
    ///
    /// # Arguments
    ///
    /// * `idx` - The internal index to set the value to.
    fn set(&mut self, idx: u64) -> Result<(), LuceneError>;
    fn long_value(&mut self) -> Result<i64, LuceneError>;
    fn binary_value(&mut self) -> Result<BytesRef, LuceneError>;
}

pub struct SingleValueDocValuesFieldUpdates<S>
where
    S: SingleValueDocValuesFieldUpdatesBase + Default,
{
    sub_update: S,
    bit_set: SparseFixedBitSet,
    has_no_value: Option<SparseFixedBitSet>,
    max_doc: u32,
    del_gen: u64,
    has_at_least_one_value: bool,
    lock: Arc<Mutex<()>>,
    dov_values_type: DocValuesType,
}
impl<S> SingleValueDocValuesFieldUpdates<S>
where
    S: SingleValueDocValuesFieldUpdatesBase + Default,
{
    pub fn new(
        sub: S,
        max_doc: u32,
        del_gen: u64,
        dov_values_type: DocValuesType,
    ) -> Result<Self, LuceneError> {
        Ok(Self {
            sub_update: sub,
            bit_set: SparseFixedBitSet::new(max_doc as i32)?,
            has_no_value: None,
            max_doc,
            del_gen,
            has_at_least_one_value: false,
            lock: Arc::new(Mutex::new(())),
            dov_values_type,
        })
    }
    pub fn binary_value(&self) -> Result<BytesRef, LuceneError> {
        self.sub_update.binary_value()
    }
    pub fn long_value(&self) -> Result<i64, LuceneError> {
        self.sub_update.long_value()
    }
}

impl<S> Accountable for SingleValueDocValuesFieldUpdates<S>
where
    S: Default + SingleValueDocValuesFieldUpdatesBase,
{
    fn ram_bytes_used(&self) -> u64 {
        todo!()
    }
}

impl<S> DocValuesFieldUpdatesBase for SingleValueDocValuesFieldUpdates<S>
where
    S: SingleValueDocValuesFieldUpdatesBase + Default,
{
    fn add_value(&mut self, doc: u32, value: i64, _index: u32) -> Result<(), LuceneError> {
        debug_assert!(self.sub_update.long_value()? == value);
        debug_assert!(doc <= i32::MAX as u32);
        self.bit_set.set(doc as i32);
        self.has_at_least_one_value = true;
        if self.has_no_value.is_some() {
            self.has_no_value
                .as_mut()
                .unwrap()
                .clear_with_index(doc as i32);
        }
        Ok(())
    }

    fn add_byte_ref(&mut self, doc: u32, value: BytesRef, _index: u32) -> Result<(), LuceneError> {
        debug_assert!(self.sub_update.binary_value()? == value);
        debug_assert!(doc <= i32::MAX as u32);
        self.bit_set.set(doc as i32);
        self.has_at_least_one_value = true;
        if self.has_no_value.is_some() {
            self.has_no_value
                .as_mut()
                .unwrap()
                .clear_with_index(doc as i32);
        }
        Ok(())
    }

    fn add_iterator<T: DocValuesFieldIterator>(&mut self, _doc_id: u32, _iterator: T) -> Result<(), LuceneError> {
        unreachable!("add_iterator is not supported")
    }

    fn iterator(
        &mut self,
        _inner: Arc<Mutex<DocValuesFieldInner>>,
        _del_gen: u64,
    ) -> Result<impl DocValuesFieldIterator, LuceneError> {
        let iterator = BitSetIterator::new(&self.bit_set, self.max_doc as i64)?;
        SingleValueDocValuesFieldUpdatesIterator::new(
            Some(iterator),
            self.del_gen,
            self.has_no_value.as_mut(),
            Some(&mut self.sub_update),
        )
    }

    fn reset(&mut self, doc: u32) -> Result<(), LuceneError> {
        let _guide = self.lock.lock().unwrap();
        self.bit_set.set(doc as i32);
        self.has_at_least_one_value = true;
        if self.has_no_value.is_none() {
            self.has_no_value = Some(SparseFixedBitSet::new(self.max_doc as i32)?);
        }
        self.has_no_value.as_mut().unwrap().set(doc as i32);
        Ok(())
    }

    fn need_reset(&self) -> bool {
        true
    }

    fn any(&self, super_any: bool) -> bool {
        let _guide = self.lock.lock().unwrap();
        super_any || self.has_at_least_one_value
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
    fn binary_value(&self) -> Result<BytesRef, LuceneError>;
    fn long_value(&self) -> Result<i64, LuceneError>;
    fn sub_type(&self) -> DocValuesType;
}
/// # Note
/// To implement Default, we wrap the mutable reference fields here with Option.
pub struct SingleValueDocValuesFieldUpdatesIterator<'a, S>
where
    S: SingleValueDocValuesFieldUpdatesBase + Default,
{
    del_gen: u64,
    has_no_value: Option<&'a mut SparseFixedBitSet>,
    iterator: Option<BitSetIterator<'a, SparseFixedBitSet>>,
    single: Option<&'a mut S>,
}
impl<'a, S> SingleValueDocValuesFieldUpdatesIterator<'a, S>
where
    S: SingleValueDocValuesFieldUpdatesBase + Default,
{
    /// Creates a new instance of `SingleValueDocValuesFieldUpdatesIterator`.
    ///
    /// # Note
    /// Avoid using the `Default` trait. This constructor should be used instead.
    pub fn new(
        iterator: Option<BitSetIterator<'a, SparseFixedBitSet>>,
        del_gen: u64,
        has_no_value: Option<&'a mut SparseFixedBitSet>,
        single: Option<&'a mut S>,
    ) -> Result<Self, LuceneError> {
        debug_assert!(single.is_some());
        Ok(Self {
            del_gen,
            has_no_value,
            iterator,
            single,
        })
    }
}

impl<S> DocValuesIterator for SingleValueDocValuesFieldUpdatesIterator<'_, S> where
    S: SingleValueDocValuesFieldUpdatesBase + Default
{
}

impl<S> Default for SingleValueDocValuesFieldUpdatesIterator<'_, S>
where
    S: SingleValueDocValuesFieldUpdatesBase + Default,
{
    /// # Warning
    /// Implementing Default is solely for enabling sorting within the PriorityQueue.
    fn default() -> Self {
        Self {
            del_gen: 0,
            has_no_value: None,
            iterator: None,
            single: None,
        }
    }
}

impl<S> PartialEq for SingleValueDocValuesFieldUpdatesIterator<'_, S>
where
    S: Default + SingleValueDocValuesFieldUpdatesBase,
{
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl<S> DocValuesFieldIterator for SingleValueDocValuesFieldUpdatesIterator<'_, S>
where
    S: SingleValueDocValuesFieldUpdatesBase + Default,
{
    fn long_value(&mut self) -> Result<i64, LuceneError> {
        self.single.as_ref().unwrap().long_value()
    }

    fn binary_value(&mut self) -> Result<BytesRef, LuceneError> {
        self.single.as_ref().unwrap().binary_value()
    }

    fn del_gen(&self) -> u64 {
        self.del_gen
    }

    fn has_value(&mut self) -> bool {
        if self.has_no_value.is_some() {
            !self
                .has_no_value
                .as_mut()
                .unwrap()
                .get(self.iterator.as_ref().unwrap().doc_id())
        } else {
            true
        }
    }
}
impl<S> DocIdSetIterator for SingleValueDocValuesFieldUpdatesIterator<'_, S>
where
    S: SingleValueDocValuesFieldUpdatesBase + Default,
{
    /// # Warning
    /// Since SingleValueDocValuesFieldUpdatesIterator may be used for PriorityQueue sorting,
    /// and PriorityQueue requires elements to implement Default,
    /// only SingleValueDocValuesFieldUpdatesIterator instances generated via Default will have their iterator set to None.
    fn doc_id(&self) -> i32 {
        if self.iterator.is_none() {
            // The smaller the document ID, the higher its sorting priority.
            // Therefore, we set the document ID to the maximum value in this case.
            i32::MAX
        } else {
            self.iterator.as_ref().unwrap().doc_id()
        }
    }

    fn next_doc(&mut self) -> Result<i32, LuceneError> {
        self.iterator.as_mut().unwrap().next_doc()
    }
}
