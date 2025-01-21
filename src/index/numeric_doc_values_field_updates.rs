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
    DocValuesFieldUpdatesBase, SingleValueDocValuesFieldUpdatesBase, PAGE_SIZE,
};
use crate::index::doc_values_type::DocValuesType;
use crate::index::BytesRef;
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::LuceneError;
use crate::util::long_values::LongValues;
use crate::util::packed::abstract_paged_mutable::{AbstractPagedMutable, AbstractPagedMutableBase};
use crate::util::packed::paged_growable_writer::PagedGrowableWriter;
use crate::util::packed::paged_mutable::PagedMutable;
use crate::util::packed::PackedInts;
use std::sync::{Arc, Mutex};

pub struct NumericDocValuesFieldUpdates<T>
where
    T: AbstractPagedMutableBase,
{
    values: AbstractPagedMutable<T>,
    min_value: i64,
    lock: Arc<Mutex<()>>,
}
impl NumericDocValuesFieldUpdates<PagedGrowableWriter> {
    pub fn new() -> Result<NumericDocValuesFieldUpdates<PagedGrowableWriter>, LuceneError> {
        let sub_reader = PagedGrowableWriter::new_with_fill_page(1, PackedInts::DEFAULT);
        let values = AbstractPagedMutable::new(1, 1, PAGE_SIZE, sub_reader)?;
        Ok(NumericDocValuesFieldUpdates {
            values,
            min_value: 0,
            lock: Arc::new(Mutex::new(())),
        })
    }
}
impl NumericDocValuesFieldUpdates<PagedMutable> {
    pub fn new_with_range(
        min_value: i64,
        max_value: i64,
    ) -> Result<
        NumericDocValuesFieldUpdates<
            impl AbstractPagedMutableBase<PagedMutableBase = PagedMutable>,
        >,
        LuceneError,
    > {
        let bits_per_value = PackedInts::unsigned_bits_required(max_value - min_value);
        let sub_reader =
            PagedMutable::new_with_overhead_ratio(PAGE_SIZE, bits_per_value, PackedInts::DEFAULT);
        let values = AbstractPagedMutable::new(1, 1, PAGE_SIZE, sub_reader)?;
        Ok(NumericDocValuesFieldUpdates {
            values,
            min_value,
            lock: Arc::new(Mutex::new(())),
        })
    }
}

impl<T> Accountable for NumericDocValuesFieldUpdates<T>
where
    T: AbstractPagedMutableBase<PagedMutableBase = T>,
{
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

impl<T> DocValuesFieldUpdatesBase for NumericDocValuesFieldUpdates<T>
where
    T: AbstractPagedMutableBase<PagedMutableBase = T> + Default,
{
    fn add_value(&mut self, _doc: i32, value: i64, index: i32) -> Result<(), LuceneError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock".to_string()))?;
        self.values.set(index as i64, value - self.min_value)
    }

    fn add_byte_ref(
        &mut self,
        _doc: i32,
        _value: BytesRef,
        _index: i32,
    ) -> Result<(), LuceneError> {
        unreachable!("NumericDocValuesFieldUpdates does not support add_byte_ref")
    }

    fn add_iterator<I: DocValuesFieldIterator>(
        &mut self,
        doc_id: i32,
        mut iterator: I,
    ) -> Result<(), LuceneError> {
        self.add_value(doc_id, iterator.long_value()?, 0)
    }

    fn iterator(
        &mut self,
        inner: Arc<Mutex<DocValuesFieldInner>>,
        del_gen: i64,
    ) -> Result<impl DocValuesFieldIterator, LuceneError> {
        let sub_iterator =
            NumericDocValuesFieldUpdatesIterator::new(Some(&mut self.values), 0, self.min_value);
        Ok(AbstractIterator::new(inner, del_gen, sub_iterator))
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        let tmp_val = self.values.get(j as i64)?;
        let value = self.values.get(i as i64)?;
        self.values.set(j as i64, value)?;
        self.values.set(i as i64, tmp_val)?;
        Ok(())
    }

    fn grow(&mut self, size: i32) -> Result<(), LuceneError> {
        let value_result = self.values.grow_with_size(size as i64)?;
        if value_result.is_some() {
            self.values = value_result.unwrap();
        }
        Ok(())
    }

    fn resize(&mut self, _size: i32) -> Result<(), LuceneError> {
        self.values = self.values.resize(_size as i64)?;
        Ok(())
    }

    fn sub_type(&self) -> DocValuesType {
        DocValuesType::Numeric
    }
}
#[derive(Default)]
pub struct NumericDocValuesFieldUpdatesIterator<'a, T>
where
    T: AbstractPagedMutableBase<PagedMutableBase = T>,
{
    values: Option<&'a mut AbstractPagedMutable<T>>,
    value: i64,
    min_value: i64,
}
impl<'a, T> NumericDocValuesFieldUpdatesIterator<'a, T>
where
    T: AbstractPagedMutableBase<PagedMutableBase = T>,
{
    pub fn new(
        values: Option<&'a mut AbstractPagedMutable<T>>,
        value: i64,
        min_value: i64,
    ) -> Self {
        debug_assert!(values.is_some());
        NumericDocValuesFieldUpdatesIterator {
            values,
            value,
            min_value,
        }
    }
}
impl<T> AbstractIteratorBase for NumericDocValuesFieldUpdatesIterator<'_, T>
where
    T: AbstractPagedMutableBase<PagedMutableBase = T>,
{
    fn set(&mut self, idx: i64) -> Result<(), LuceneError> {
        self.value = self.values.as_mut().unwrap().get(idx)? + self.min_value;
        Ok(())
    }

    fn long_value(&mut self) -> Result<i64, LuceneError> {
        Ok(self.value)
    }

    fn binary_value(&mut self) -> Result<BytesRef, LuceneError> {
        unreachable!("NumericDocValuesFieldUpdatesIterator does not support binary_value")
    }
}

#[derive(Default)]
pub struct SingleValueNumericDocValuesFieldUpdates {
    value: i64,
}
impl SingleValueNumericDocValuesFieldUpdates {
    pub fn new(value: i64) -> SingleValueNumericDocValuesFieldUpdates {
        SingleValueNumericDocValuesFieldUpdates { value }
    }
}
impl SingleValueDocValuesFieldUpdatesBase for SingleValueNumericDocValuesFieldUpdates {
    fn binary_value(&self) -> Result<BytesRef, LuceneError> {
        unreachable!("SingleValueNumericDocValuesFieldUpdates does not support binary_value")
    }

    fn long_value(&self) -> Result<i64, LuceneError> {
        Ok(self.value)
    }

    fn sub_type(&self) -> DocValuesType {
        DocValuesType::Numeric
    }
}
