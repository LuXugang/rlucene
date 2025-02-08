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
use crate::index::doc_values_field_updates::{
    AbstractIterator, AbstractIteratorBase, DocValuesFieldInner, DocValuesFieldIterator,
    DocValuesFieldUpdatesBase, PAGE_SIZE,
};
use crate::index::doc_values_type::DocValuesType;
use crate::index::{BytesRef, BytesRefBuilder};
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::LuceneError;
use crate::util::long_values::LongValues;
use crate::util::packed::abstract_paged_mutable::AbstractPagedMutable;
use crate::util::packed::paged_growable_writer::PagedGrowableWriter;
use crate::util::packed::PackedInts;
use std::sync::{Arc, Mutex};

/// A [`DocValuesFieldUpdates`](crate::index::doc_values_field_updates::DocValuesFieldUpdates) which holds updates for documents of a single `BinaryDocValuesField`.
///
/// # Note
/// This API is experimental and may change in future versions.
pub(crate) struct BinaryDocValuesFieldUpdates {
    offsets: AbstractPagedMutable<PagedGrowableWriter>,
    lengths: AbstractPagedMutable<PagedGrowableWriter>,
    values: BytesRefBuilder,
    lock: Arc<Mutex<()>>,
}
impl BinaryDocValuesFieldUpdates {
    #[allow(unused)]
    fn new() -> Result<BinaryDocValuesFieldUpdates, LuceneError> {
        let sub_reader1 = PagedGrowableWriter::with_fill_page(1, PackedInts::FAST);
        let offsets = AbstractPagedMutable::new(1, 1, PAGE_SIZE, sub_reader1)?;
        let sub_reader2 = PagedGrowableWriter::with_fill_page(1, PackedInts::FAST);
        let lengths = AbstractPagedMutable::new(1, 1, PAGE_SIZE, sub_reader2)?;
        Ok(BinaryDocValuesFieldUpdates {
            offsets,
            lengths,
            values: BytesRefBuilder::new(),
            lock: Arc::new(Mutex::new(())),
        })
    }
}

impl Accountable for BinaryDocValuesFieldUpdates {
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

impl DocValuesFieldUpdatesBase for BinaryDocValuesFieldUpdates {
    fn add_value(&mut self, _doc: i32, _value: i64, _index: i32) -> Result<(), LuceneError> {
        unreachable!("BinaryDocValuesFieldUpdates does not support add_value")
    }

    fn add_byte_ref(&mut self, _doc: i32, value: BytesRef, index: i32) -> Result<(), LuceneError> {
        let _guard = self.lock.lock().map_err(|_| {
            LuceneError::illegal_state("Failed to acquire lock on lock.".to_string())
        })?;
        self.offsets
            .set(index as i64, self.values.length() as i64)?;
        self.lengths.set(index as i64, value.length as i64)?;
        self.values.append_ref(&value);
        Ok(())
    }

    fn add_iterator<T: DocValuesFieldIterator>(
        &mut self,
        doc_id: i32,
        mut iterator: T,
    ) -> Result<(), LuceneError> {
        self.add_byte_ref(doc_id, iterator.binary_value()?, 0)
    }

    fn iterator(
        &mut self,
        inner: Arc<Mutex<DocValuesFieldInner>>,
        del_gen: i64,
    ) -> Result<impl DocValuesFieldIterator, LuceneError> {
        let sub_iterator = BinaryDocValuesIterator::new(
            Some(&mut self.offsets),
            Some(&mut self.lengths),
            Some(self.values.get()),
        );
        Ok(AbstractIterator::new(inner, del_gen, sub_iterator))
    }
    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        let temp_offset = self.offsets.get(j as i64)?;
        let value = self.offsets.get(i as i64)?;
        self.offsets.set(j as i64, value)?;
        self.offsets.set(i as i64, temp_offset)?;

        let tem_length = self.lengths.get(j as i64)?;
        let length = self.lengths.get(i as i64)?;
        self.lengths.set(j as i64, length)?;
        self.lengths.set(i as i64, tem_length)?;
        Ok(())
    }

    fn grow(&mut self, size: i32) -> Result<(), LuceneError> {
        let offset_result = self.offsets.grow_with_size(size as i64)?;
        if offset_result.is_some() {
            self.offsets = offset_result.unwrap();
        }
        let length_result = self.lengths.grow_with_size(size as i64)?;
        if length_result.is_some() {
            self.lengths = length_result.unwrap();
        }
        Ok(())
    }

    fn resize(&mut self, _size: i32) -> Result<(), LuceneError> {
        self.offsets = self.offsets.resize(_size as i64)?;
        self.lengths = self.lengths.resize(_size as i64)?;
        Ok(())
    }

    fn sub_type(&self) -> DocValuesType {
        DocValuesType::Binary
    }
}

/// # Note
/// To implement Default, we wrap the mutable reference fields here with Option.
///
/// Implementing Default is solely for enabling sorting within the PriorityQueue.
#[derive(Default)]
pub struct BinaryDocValuesIterator<'a> {
    offsets: Option<&'a mut AbstractPagedMutable<PagedGrowableWriter>>,
    offset: i32,
    lengths: Option<&'a mut AbstractPagedMutable<PagedGrowableWriter>>,
    length: i32,
    values: Option<&'a mut BytesRef>,
}
#[allow(unused)]
impl<'a> BinaryDocValuesIterator<'a> {
    pub fn new(
        offsets: Option<&'a mut AbstractPagedMutable<PagedGrowableWriter>>,
        lengths: Option<&'a mut AbstractPagedMutable<PagedGrowableWriter>>,
        values: Option<&'a mut BytesRef>,
    ) -> BinaryDocValuesIterator<'a> {
        BinaryDocValuesIterator {
            offsets,
            offset: 0,
            lengths,
            length: 0,
            values,
        }
    }
}
impl AbstractIteratorBase for BinaryDocValuesIterator<'_> {
    fn set(&mut self, idx: i64) -> Result<(), LuceneError> {
        debug_assert!(self.offsets.is_some());
        debug_assert!(self.lengths.is_some());
        debug_assert!(self.offsets.as_mut().unwrap().get(idx)? <= i32::MAX as i64);
        self.offset = self.offsets.as_mut().unwrap().get(idx)? as i32;
        debug_assert!(self.lengths.as_mut().unwrap().get(idx)? <= i32::MAX as i64);
        self.length = self.lengths.as_mut().unwrap().get(idx)? as i32;
        Ok(())
    }

    fn long_value(&mut self) -> Result<i64, LuceneError> {
        unreachable!("BinaryDocValuesIterator does not support long_value")
    }

    fn binary_value(&mut self) -> Result<BytesRef, LuceneError> {
        debug_assert!(self.values.is_some());
        self.values.as_mut().unwrap().offset = self.offset;
        self.values.as_mut().unwrap().length = self.length;
        // TODO: coule we avoid the copy here?
        // TODO:In Java Lucene, a reference is passed here, but this functionality cannot be achieved in Rust. Therefore, we should design a method to copy only a portion of the bytes to avoid a full copy.
        Ok(self.values.as_mut().unwrap().clone())
    }
}
