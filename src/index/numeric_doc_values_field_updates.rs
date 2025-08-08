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

use parking_lot::Mutex;

use crate::index::doc_values_field_updates::{
    dvfu_util, AbstractIterator, AbstractIteratorBase, DocValuesFieldInner, DocValuesFieldIterator,
    DocValuesFieldUpdatesBase, SingleValueDocValuesFieldUpdatesBase,
};
use crate::index::doc_values_type::DocValuesType;
use crate::index::BytesRef;
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::long_values::LongValues;
use crate::util::packed::abstract_paged_mutable::{AbstractPagedMutable, AbstractPagedMutableBase};
use crate::util::packed::paged_growable_writer::PagedGrowableWriter;
use crate::util::packed::paged_mutable::PagedMutable;
use crate::util::packed::PackedInts;

pub(crate) struct NumericDocValuesFieldUpdates<T>
where
    T: AbstractPagedMutableBase,
{
    values: AbstractPagedMutable<T>,
    min_value: i64,
    lock: Mutex<()>,
}
#[allow(unused)]
impl NumericDocValuesFieldUpdates<PagedGrowableWriter> {
    pub fn new() -> Result<NumericDocValuesFieldUpdates<PagedGrowableWriter>> {
        let sub_reader = PagedGrowableWriter::with_fill_page(1, PackedInts::DEFAULT);
        let values = AbstractPagedMutable::new(1, dvfu_util::PAGE_SIZE, sub_reader)?;
        Ok(NumericDocValuesFieldUpdates {
            values,
            min_value: 0,
            lock: Mutex::new(()),
        })
    }
}
#[allow(unused)]
impl NumericDocValuesFieldUpdates<PagedMutable> {
    pub(crate) fn with_range(
        min_value: i64,
        max_value: i64,
    ) -> Result<NumericDocValuesFieldUpdates<impl AbstractPagedMutableBase>> {
        let bits_per_value = PackedInts::unsigned_bits_required(max_value - min_value);
        let sub_reader = PagedMutable::with_overhead_ratio(
            dvfu_util::PAGE_SIZE,
            bits_per_value,
            PackedInts::DEFAULT,
        );
        let values = AbstractPagedMutable::new(1, dvfu_util::PAGE_SIZE, sub_reader)?;
        Ok(NumericDocValuesFieldUpdates {
            values,
            min_value,
            lock: Mutex::new(()),
        })
    }
}

impl<T> Accountable for NumericDocValuesFieldUpdates<T>
where
    T: AbstractPagedMutableBase,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

impl<T> DocValuesFieldUpdatesBase for NumericDocValuesFieldUpdates<T>
where
    T: AbstractPagedMutableBase + Default,
{
    fn add_value(&mut self, _doc: i32, value: i64, index: i32) -> Result<()> {
        let _guard = self.lock.lock();
        self.values.set(index as i64, value - self.min_value);
        Ok(())
    }

    fn add_byte_ref(&mut self, _doc: i32, _value: &BytesRef<Vec<u8>>, _index: i32) -> Result<()> {
        Err(LuceneError::unreachable(
            "umericDocValuesFieldUpdates does not support add_byte_ref",
        ))
    }

    fn add_iterator<I: DocValuesFieldIterator>(
        &mut self,
        doc_id: i32,
        mut iterator: I,
    ) -> Result<()> {
        self.add_value(doc_id, iterator.long_value()?, 0)
    }

    fn iterator(
        &mut self,
        inner: Arc<Mutex<DocValuesFieldInner>>,
        del_gen: i64,
    ) -> Result<impl DocValuesFieldIterator> {
        let base = AbstractIteratorBaseImpl::new(Some(&mut self.values), 0, self.min_value);
        Ok(AbstractIterator::new(inner, del_gen, base))
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        let tmp_val = self.values.get(j as i64)?;
        let value = self.values.get(i as i64)?;
        self.values.set(j as i64, value);
        self.values.set(i as i64, tmp_val);
        Ok(())
    }

    fn grow(&mut self, size: i32) -> Result<()> {
        let value_result = self.values.grow_with_size(size as i64)?;
        if value_result.is_some() {
            self.values = value_result.unwrap();
        }
        Ok(())
    }

    fn resize(&mut self, _size: i32) -> Result<()> {
        self.values = self.values.resize(_size as i64)?;
        Ok(())
    }

    fn sub_type(&self) -> DocValuesType {
        DocValuesType::Numeric
    }
}
#[derive(Default)]
pub(crate) struct AbstractIteratorBaseImpl<'a, T>
where
    T: AbstractPagedMutableBase,
{
    values: Option<&'a mut AbstractPagedMutable<T>>,
    value: i64,
    min_value: i64,
}
impl<'a, T> AbstractIteratorBaseImpl<'a, T>
where
    T: AbstractPagedMutableBase,
{
    pub(crate) fn new(
        values: Option<&'a mut AbstractPagedMutable<T>>,
        value: i64,
        min_value: i64,
    ) -> Self {
        debug_assert!(values.is_some());
        AbstractIteratorBaseImpl {
            values,
            value,
            min_value,
        }
    }
}
impl<T> AbstractIteratorBase for AbstractIteratorBaseImpl<'_, T>
where
    T: AbstractPagedMutableBase,
{
    fn set(&mut self, idx: i64) -> Result<()> {
        self.value = self.values.as_mut().unwrap().get(idx)? + self.min_value;
        Ok(())
    }

    fn long_value(&mut self) -> Result<i64> {
        Ok(self.value)
    }

    fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
        unreachable!("NumericDocValuesFieldUpdatesIterator does not support binary_value")
    }
}

#[derive(Default)]
pub struct SingleValueNumericDocValuesFieldUpdates {
    value: i64,
}
#[allow(unused)]
impl SingleValueNumericDocValuesFieldUpdates {
    pub(crate) fn new(value: i64) -> SingleValueNumericDocValuesFieldUpdates {
        SingleValueNumericDocValuesFieldUpdates { value }
    }
}
impl SingleValueDocValuesFieldUpdatesBase for SingleValueNumericDocValuesFieldUpdates {
    fn binary_value(&self) -> Result<&BytesRef<Vec<u8>>> {
        Err(LuceneError::unreachable(
            "SingleValueNumericDocValuesFieldUpdates does not support binary_value",
        ))
    }

    fn long_value(&self) -> Result<i64> {
        Ok(self.value)
    }

    fn sub_type(&self) -> DocValuesType {
        DocValuesType::Numeric
    }
}
