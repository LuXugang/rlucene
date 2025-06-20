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
use crate::index::sorter::DocMap;
use crate::index::{BytesRef, BytesRefBuilder};
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::DataInput;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::packed::packed_long_values::{PackedLongValues, PackedLongValuesIterator};
use crate::util::{BytesRefArray, CounterEnum, CounterEnumBorrow, SortableBytesRefArray};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct BinaryDocValuesWriter;

// iterates over the values we have in ram
pub(crate) struct BufferedBinaryDocValues<D, DI>
where
    D: DocIdSetIterator,
    DI: DataInput,
{
    value: BytesRefBuilder<Vec<u8>>,
    lengths_iterator: PackedLongValuesIterator,
    docs_with_field: D,
    bytes_iter: DI,
}

impl<D, DI> BufferedBinaryDocValues<D, DI>
where
    D: DocIdSetIterator,
    DI: DataInput,
{
    pub(crate) fn new(
        lengths: &PackedLongValues,
        max_length: usize,
        bytes_iter: DI,
        docs_with_field: D,
    ) -> Result<Self> {
        let mut value = BytesRefBuilder::new();
        value.grow(max_length);
        Ok(Self {
            value,
            lengths_iterator: lengths.iterator()?,
            docs_with_field,
            bytes_iter,
        })
    }
}

impl<D, DI> DocIdSetIterator for BufferedBinaryDocValues<D, DI>
where
    D: DocIdSetIterator,
    DI: DataInput,
{
    fn doc_id(&self) -> i32 {
        self.docs_with_field.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let doc_id = self.docs_with_field.next_doc()?;
        if doc_id != NO_MORE_DOCS {
            let length: i32 = self.lengths_iterator.next_value()?.try_into()?;
            self.value.set_length(length as usize);
            self.bytes_iter
                .read_bytes(&mut self.value.bytes_ref.bytes, 0, length)?;
        }
        Ok(doc_id)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation("use next_doc instead"))
    }

    fn cost(&self) -> Result<i64> {
        self.docs_with_field.cost()
    }
}

impl<D, DI> DocValuesIterator for BufferedBinaryDocValues<D, DI>
where
    D: DocIdSetIterator,
    DI: DataInput,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation("use next_doc instead"))
    }
}

impl<D, DI> BinaryDocValues for BufferedBinaryDocValues<D, DI>
where
    D: DocIdSetIterator,
    DI: DataInput,
{
    fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
        Ok(self.value.get_bytes_ref())
    }
}

pub(crate) struct SortingBinaryDocValues {
    dvs: BinaryDVs,
    spare: BytesRefBuilder<Vec<u8>>,
    doc_id: i32,
}

impl SortingBinaryDocValues {
    pub fn new(dvs: BinaryDVs) -> Self {
        Self {
            dvs,
            spare: BytesRefBuilder::new(),
            doc_id: -1,
        }
    }
}

impl DocIdSetIterator for SortingBinaryDocValues {
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        loop {
            self.doc_id += 1;
            if self.doc_id as usize == self.dvs.offsets.len() {
                self.doc_id = NO_MORE_DOCS;
                break;
            }
            if self.dvs.offsets[self.doc_id as usize] > 0 {
                break;
            }
        }
        Ok(self.doc_id)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation("use next_doc instead"))
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.dvs.values.size() as i64)
    }
}

impl DocValuesIterator for SortingBinaryDocValues {
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation("use next_doc instead"))
    }
}

impl BinaryDocValues for SortingBinaryDocValues {
    fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
        let idx = self.dvs.offsets[self.doc_id as usize] - 1;
        self.dvs.values.get(&mut self.spare, idx)?;
        Ok(self.spare.get_bytes_ref())
    }
}

#[derive(Clone)]
pub(crate) struct BinaryDVs {
    pub(crate) offsets: Rc<Vec<i32>>,
    pub(crate) values: Rc<BytesRefArray<CounterEnumBorrow>>,
}

impl BinaryDVs {
    pub fn new<DM>(
        max_doc: i32,
        sort_map: &DM,
        old_values: &mut impl BinaryDocValues,
    ) -> Result<Self>
    where
        DM: DocMap,
    {
        let mut offsets = vec![0i32; max_doc as usize];
        let counter = Rc::new(RefCell::new(CounterEnum::new_counter(false)));
        let mut values = BytesRefArray::new(counter)?;
        let mut offset = 1i32; // 0 means no values for this document
        let mut doc_id;
        loop {
            doc_id = old_values.next_doc()?;
            if doc_id == NO_MORE_DOCS {
                break;
            }
            let new_doc = sort_map.old_to_new(doc_id) as usize;
            let val = old_values.binary_value()?;
            values.append(val)?;
            offsets[new_doc] = offset;
            offset += 1;
        }
        Ok(BinaryDVs {
            offsets: Rc::new(offsets),
            values: Rc::new(values),
        })
    }
}
