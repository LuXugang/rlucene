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
use crate::util::error::runtime_error::RuntimeError;
use crate::util::intro_sorter::IntroSorter;
use crate::util::long_values::LongValues;
use crate::util::packed::abstract_paged_mutable::AbstractPagedMutable;
use crate::util::packed::paged_mutable::PagedMutable;
use crate::util::packed::{MutablePacked64Enum, PackedInts, Reader};
use crate::util::priority_queue::{Compare, PriorityQueue};
use crate::util::Sorter;

const PAGE_SIZE: u32 = 1024;
const HAS_VALUE_MASK: u64 = 1;
const HAS_NO_VALUE_MASK: u64 = 0;
// we use the first bit of each value to mark if the doc has a value or not
const SHIFT: u32 = 1;
pub struct DocValuesFieldUpdates {
    pub field: String,
    pub doc_values_type: DocValuesType,
    pub del_gen: u64,
    bits_per_value: u32,
    finished: bool,
    max_doc: u32,
    docs: AbstractPagedMutable<PagedMutable>,
    size: u32,
}
impl DocValuesFieldUpdates {
    pub fn new(
        max_doc: u32,
        del_gen: u64,
        field: String,
        doc_values_type: DocValuesType,
    ) -> Result<Self, RuntimeError> {
        let bits_per_value = PackedInts::bits_required(max_doc as i64 - 1)? + SHIFT;
        let sub_mutable =
            PagedMutable::new_with_overhead_ratio(PAGE_SIZE, bits_per_value, PackedInts::DEFAULT);
        let writer = AbstractPagedMutable::new(bits_per_value, 1, PAGE_SIZE, sub_mutable)?;
        Ok(Self {
            field,
            doc_values_type,
            del_gen,
            bits_per_value,
            finished: false,
            max_doc,
            docs: writer,
            size: 0,
        })
    }

    pub fn merged_iterator<T>(
        subs: Vec<Iterator<T>>,
    ) -> Result<Option<Iterator<IteratorPQImpl<T>>>, RuntimeError>
    where
        T: IteratorBase + DocIdSetIterator + Default,
    {
        // Due to the characteristics of the Rust language, in order to reduce complexity,
        // we add the element to the queue for processing even if there is only one element.
        // if subs.len() == 1 {
        //
        // }

        // Priority queue to sort iterators by doc_id and del_gen
        let mut queue = PriorityQueue::new(subs.len() as i32, IteratorPQCmp::new())?;

        for mut sub in subs {
            if sub.next_doc() != NO_MORE_DOCS {
                queue.add(sub);
            }
        }

        if queue.size() == 0 {
            return Ok(None);
        }
        let value = IteratorPQImpl::new(queue.size() as i32, IteratorPQCmp::new())?;
        let result = Iterator::new(value);
        Ok(Some(result))
    }

    fn get_finished(&self) -> bool {
        self.finished
    }
}

struct IntroSorterImpl<'a> {
    ords: &'a mut MutablePacked64Enum,
    docs: &'a mut AbstractPagedMutable<PagedMutable>,
    pivot_doc: i64,
    pivot_ord: i64,
    size: u32,
}

impl<'a> Sorter for IntroSorterImpl<'a> {
    fn compare(&mut self, i: i32, j: i32) -> Result<i32, RuntimeError> {
        // increasing docID order:
        // NOTE: we can have ties here, when the same docID was updated in the same segment, in
        // which case we rely on sort being
        // stable and preserving original order so the last update to that docID wins
        let cmp = (self.docs.get(i as u64)? >> 1).cmp(&(self.docs.get(j as u64)? >> 1));

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

    fn swap(&mut self, i: i32, j: i32) -> Result<(), RuntimeError> {
        // let tmp_ord = self.ords.get(i)?
        Ok(())
    }

    fn set_pivot(&mut self, i: i32) -> Result<(), RuntimeError> {
        self.pivot_doc = self.docs.get(i as u64)? >> 1;
        self.pivot_ord = self.ords.get(i as usize)?;
        Ok(())
    }

    fn compare_pivot(&mut self, j: i32) -> Result<i32, RuntimeError> {
        let mut cmp = (self.pivot_doc).cmp(&((self.docs.get(j as u64)? as u64 >> 1) as i64));
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
}

impl IntroSorter for IntroSorterImpl<'_> {}

pub trait DocValuesFieldUpdatesBase {
    fn add(&mut self, doc: u32, value: i64);
    fn add_byte_ref(&mut self, doc: u32, value: BytesRef);
    /// Adds the value for the given `doc_id`.
    ///
    /// This method prevents conditional calls to [`IteratorBase::long_value`] or
    /// [`IteratorBase::binary_value`], since the implementation knows whether it is
    /// a long value iterator or a binary value iterator.
    fn add_iterator<T>(&mut self, doc_id: u32, iterator: Iterator<T>)
    where
        T: IteratorBase + DocIdSetIterator + Default;
    /// Returns an iterator for updated documents and their values.
    fn iterator<T>(&self) -> Iterator<T>
    where
        T: IteratorBase + DocIdSetIterator + Default;
}

/// An iterator over documents and their updated values.
///
/// Only documents with updates are returned by this iterator, and the documents are returned
/// in increasing order.
#[derive(Default)]
pub struct Iterator<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    sub_iterator: T,
}

impl<T> Iterator<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn new(sub_iterator: T) -> Self {
        Self { sub_iterator }
    }
    fn get_binary_doc_values(iterator: Iterator<T>) {
        BinaryDocValuesImpl::new(iterator);
    }
    fn get_numeric_doc_values(iterator: Iterator<T>) {
        NumericDocValuesImpl::new(iterator);
    }
}

impl<T> DocIdSetIterator for Iterator<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn doc_id(&self) -> i32 {
        self.sub_iterator.doc_id()
    }

    fn next_doc(&mut self) -> i32 {
        self.sub_iterator.next_doc()
    }

    fn advance(&mut self, _target: i32) -> i32 {
        unreachable!("advance is not supported")
    }

    fn cost(&self) -> i64 {
        unreachable!("cost is not supported")
    }
}

impl<T> DocValuesIterator for Iterator<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn advance_exact(&self, _target: i32) -> bool {
        unreachable!("advance_exact is not supported")
    }
}
impl<T> PartialEq for Iterator<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn eq(&self, other: &Self) -> bool {
        self.sub_iterator.doc_id() == other.sub_iterator.doc_id()
            && self.sub_iterator.del_gen() == other.sub_iterator.del_gen()
    }
}
pub trait IteratorBase {
    /// Returns a long value for the current document if this iterator is a long iterator.
    fn long_value(&mut self) -> i64;

    /// Returns a binary value for the current document if this iterator is a binary value iterator.
    fn binary_value(&mut self) -> BytesRef;

    /// Returns the delGen for this packet.
    fn del_gen(&self) -> u64;

    /// Returns true if this document has a value.
    fn has_value(&mut self) -> bool;
}
/// Wraps the given iterator as a BinaryDocValues instance.
pub struct BinaryDocValuesImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    iterator: Iterator<T>,
}
impl<T> BinaryDocValuesImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    pub fn new(iterator: Iterator<T>) -> Self {
        Self { iterator }
    }
}

impl<T> DocValuesIterator for BinaryDocValuesImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn advance_exact(&self, target: i32) -> bool {
        self.iterator.advance_exact(target)
    }
}

impl<T> DocIdSetIterator for BinaryDocValuesImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn doc_id(&self) -> i32 {
        self.iterator.doc_id()
    }

    fn next_doc(&mut self) -> i32 {
        self.iterator.next_doc()
    }

    fn advance(&mut self, target: i32) -> i32 {
        self.iterator.advance(target)
    }

    fn cost(&self) -> i64 {
        self.iterator.cost()
    }
}

impl<T> BinaryDocValues for BinaryDocValuesImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn binary_value(&mut self) -> Result<BytesRef, RuntimeError> {
        Ok(self.iterator.sub_iterator.binary_value())
    }
}

/// Wraps the given iterator as a NumericDocValues instance.
pub struct NumericDocValuesImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    iterator: Iterator<T>,
}
impl<T> NumericDocValuesImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    pub fn new(iterator: Iterator<T>) -> Self {
        Self { iterator }
    }
}

impl<T> DocValuesIterator for NumericDocValuesImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn advance_exact(&self, target: i32) -> bool {
        self.iterator.advance_exact(target)
    }
}

impl<T> DocIdSetIterator for NumericDocValuesImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn doc_id(&self) -> i32 {
        self.iterator.doc_id()
    }

    fn next_doc(&mut self) -> i32 {
        self.iterator.next_doc()
    }

    fn advance(&mut self, target: i32) -> i32 {
        self.iterator.advance(target)
    }

    fn cost(&self) -> i64 {
        self.iterator.cost()
    }
}

impl<T> NumericDocValues for NumericDocValuesImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn long_value(&mut self) -> Result<i64, RuntimeError> {
        Ok(self.iterator.sub_iterator.long_value())
    }
}

pub struct IteratorPQCmp<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    _t: std::marker::PhantomData<T>,
}
impl<T> Default for IteratorPQCmp<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IteratorPQCmp<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    pub fn new() -> Self {
        Self {
            _t: std::marker::PhantomData,
        }
    }
}
impl<T> Compare<Iterator<T>> for IteratorPQCmp<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn less_than(&self, a: &Iterator<T>, b: &Iterator<T>) -> bool {
        // Sort by smaller doc_id
        let mut cmp = a.doc_id().cmp(&b.doc_id());
        if cmp == std::cmp::Ordering::Equal {
            // If doc_id is equal, sort by larger del_gen
            cmp = b.sub_iterator.del_gen().cmp(&a.sub_iterator.del_gen());
            // delGen values are unique across sub-iterators, so cmp should never be equal
            assert!(cmp != std::cmp::Ordering::Equal);
        }
        cmp == std::cmp::Ordering::Less
    }
}

pub struct IteratorPQImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    queue: PriorityQueue<Iterator<T>, IteratorPQCmp<T>>,
    doc: i32,
}
impl<T> IteratorPQImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    pub fn new(length: i32, cmp: IteratorPQCmp<T>) -> Result<Self, RuntimeError> {
        Ok(Self {
            queue: PriorityQueue::new(length, cmp)?,
            doc: -1,
        })
    }
}
impl<T> IteratorBase for IteratorPQImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn long_value(&mut self) -> i64 {
        self.queue.top().sub_iterator.long_value()
    }

    fn binary_value(&mut self) -> BytesRef {
        self.queue.top().sub_iterator.binary_value()
    }

    fn del_gen(&self) -> u64 {
        unreachable!("del_gen is not supported")
    }

    fn has_value(&mut self) -> bool {
        self.queue.top().sub_iterator.has_value()
    }
}
impl<T> DocIdSetIterator for IteratorPQImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> i32 {
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

            if self.queue.top().next_doc() == NO_MORE_DOCS {
                self.queue.pop();
            } else {
                self.queue.update_top();
            }
        }
        self.doc
    }
}
impl<T> Default for IteratorPQImpl<T>
where
    T: IteratorBase + DocIdSetIterator + Default,
{
    fn default() -> Self {
        Self {
            queue: PriorityQueue::new(0, IteratorPQCmp::new()).unwrap(),
            doc: -1,
        }
    }
}
