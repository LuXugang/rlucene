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
use crate::codecs::doc_values_consumer::DocValuesConsumer;
use crate::codecs::doc_values_producer::DocValuesProducer;
use crate::codecs::dummy::dummy_binary_doc_values::DummyBinaryDocValues;
use crate::codecs::dummy::dummy_doc_values_skipper::DummyDocValuesSkipper;
use crate::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::codecs::dummy::dummy_sorted_numeric_doc_values::DummySortedNumericDocValues;
use crate::codecs::dummy::dummy_sorted_set_doc_values::DummySortedSetDocValues;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::docs_with_field_set::{DocsWithFieldSet, DocsWithFieldSetEnum};
use crate::index::field_info::FieldInfo;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMap;
use crate::search::doc_id_set::DocIdSet;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::directory::Directory;
use crate::util::accountable::Accountable;
use crate::util::bit_set::BitSet;
use crate::util::either_enums::EitherNumericDocValues;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::packed::packed_long_values::{
    PackedLongValues, PackedLongValuesBuilder, PackedLongValuesIterator,
};
use crate::util::packed::PackedInts;
use crate::util::{Counter, CounterEnumBorrow};
use std::cell::Cell;
use std::rc::Rc;
/// Buffers up pending long per doc, then flushes when segment flushes.
pub(crate) struct NumericDocValuesWriter {
    pending: PackedLongValuesBuilder,
    final_values: Option<PackedLongValues>,
    iw_bytes_used: CounterEnumBorrow,
    bytes_used: i64,
    docs_with_field: DocsWithFieldSet,
    field_info: Rc<FieldInfo>,
    last_doc_id: i32,
}

impl NumericDocValuesWriter {
    pub(crate) fn new(field_info: Rc<FieldInfo>, iw_bytes_used: CounterEnumBorrow) -> Result<Self> {
        let pending =
            PackedLongValues::delta_packed_long_values_builder_default(PackedInts::COMPACT)?;
        let docs_with_field = DocsWithFieldSet::new();
        let bytes_used = pending.ram_bytes_used()? + docs_with_field.ram_bytes_used()?;

        iw_bytes_used.borrow_mut().add_and_get(bytes_used);

        Ok(Self {
            pending,
            final_values: None,
            iw_bytes_used,
            bytes_used,
            docs_with_field,
            field_info,
            last_doc_id: -1,
        })
    }

    pub(crate) fn add_value(&mut self, doc_id: i32, value: i64) -> Result<()> {
        if doc_id <= self.last_doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "DocValuesField \"{}\" appears more than once in this document (only one value is allowed per field)",
                self.field_info.name
            )));
        }

        self.pending.add(value)?;
        self.docs_with_field.add(doc_id)?;
        self.update_bytes_used()?;
        self.last_doc_id = doc_id;
        Ok(())
    }

    fn update_bytes_used(&mut self) -> Result<()> {
        let new_bytes_used =
            self.pending.ram_bytes_used()? + self.docs_with_field.ram_bytes_used()?;
        self.iw_bytes_used
            .borrow_mut()
            .add_and_get(new_bytes_used - self.bytes_used);
        self.bytes_used = new_bytes_used;
        Ok(())
    }
    pub(crate) fn flush<D, DM, DC>(
        &mut self,
        state: &SegmentWriteState<D>,
        sort_map: Option<Rc<DM>>,
        dv_consumer: &mut DC,
    ) -> Result<()>
    where
        D: Directory,
        DM: DocMap,
        DC: DocValuesConsumer,
    {
        if self.final_values.is_none() {
            self.final_values = Some(std::mem::take(&mut self.pending).build()?)
        }
        let mut producer = ndvw_util::get_doc_values_producer(
            self.field_info.clone(),
            self.final_values.as_ref().unwrap(),
            std::mem::take(&mut self.docs_with_field),
            sort_map,
        )?;
        dv_consumer.add_numeric_field(&self.field_info, &mut producer)?;
        Ok(())
    }
}
pub(crate) struct DocValuesProducerImpl {
    sorted: Option<NumericDVs<FixedBitSet>>,
    docs_with_field: DocsWithFieldSet,
    values: PackedLongValues,
    writer_field_info: Rc<FieldInfo>,
}
impl DocValuesProducerImpl {
    pub(crate) fn new(
        sorted: Option<NumericDVs<FixedBitSet>>,
        docs_with_field: DocsWithFieldSet,
        values: PackedLongValues,
        writer_field_info: Rc<FieldInfo>,
    ) -> Result<Self> {
        Ok(Self {
            sorted,
            docs_with_field,
            values,
            writer_field_info,
        })
    }
}
impl DocValuesProducer for DocValuesProducerImpl {
    type NumericDocValues =
        EitherNumericDocValues<BufferedNumericDocValues, SortingNumericDocValues<FixedBitSet>>;

    fn get_numeric(&mut self, field_info: &Rc<FieldInfo>) -> Result<Self::NumericDocValues> {
        if !Rc::ptr_eq(field_info, &self.writer_field_info) {
            return Err(LuceneError::illegal_argument("wrong fieldInfo"));
        }
        match self.sorted {
            Some(ref sorted) => Ok(EitherNumericDocValues::S(SortingNumericDocValues::new(
                sorted.clone(),
            ))),
            None => Ok(EitherNumericDocValues::F(BufferedNumericDocValues::new(
                &self.values,
                self.docs_with_field.iterator()?.unwrap(),
            )?)),
        }
    }

    type BinaryDocValues = DummyBinaryDocValues;
    type SortedDocValues = DummySortedDocValues;
    type SortedNumericDocValues = DummySortedNumericDocValues;
    type SortedSetDocValues = DummySortedSetDocValues;
    type DocValuesSkipper = DummyDocValuesSkipper;
}

pub mod ndvw_util {
    use crate::index::docs_with_field_set::DocsWithFieldSet;
    use crate::index::field_info::FieldInfo;
    use crate::index::numeric_doc_values::NumericDocValues;
    use crate::index::numeric_doc_values_writer::{
        BufferedNumericDocValues, DocValuesProducerImpl, NumericDVs,
    };
    use crate::index::sorter::DocMap;
    use crate::search::doc_id_set::DocIdSet;
    use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::util::bit_set::BitSet;
    use crate::util::error::lucene_error::{LuceneError, Result};
    use crate::util::fixed_bit_set::FixedBitSet;
    use crate::util::packed::packed_long_values::PackedLongValues;
    use std::rc::Rc;

    pub(crate) fn sort_doc_values<DV, M>(
        max_doc: i32,
        sort_map: &M,
        old_doc_values: &mut DV,
        dense: bool,
    ) -> Result<NumericDVs<FixedBitSet>>
    where
        DV: NumericDocValues,
        M: DocMap,
    {
        let mut docs_with_field = if !dense {
            Some(FixedBitSet::new(max_doc))
        } else {
            None
        };

        let mut values = vec![0i64; max_doc as usize];

        loop {
            let doc_id = old_doc_values.next_doc()?;
            if doc_id == NO_MORE_DOCS {
                break;
            }

            let new_doc_id = sort_map.old_to_new(doc_id);
            if let Some(bits) = &mut docs_with_field {
                bits.set(new_doc_id);
            }

            values[new_doc_id as usize] = old_doc_values.long_value()?;
        }
        Ok(NumericDVs::new(values, docs_with_field))
    }

    pub(crate) fn get_doc_values_producer<DM>(
        writer_field_info: Rc<FieldInfo>,
        values: &PackedLongValues,
        docs_with_field: DocsWithFieldSet,
        sort_map: Option<Rc<DM>>,
    ) -> Result<DocValuesProducerImpl>
    where
        DM: DocMap,
    {
        let sorter = if let Some(sort_map) = sort_map {
            let dense = sort_map.size() == docs_with_field.cardinality() as usize;
            let iter = match docs_with_field.iterator()? {
                Some(iter) => iter,
                None => return Err(LuceneError::illegal_state("DocsWithFieldSet is None")),
            };
            let mut old_values = BufferedNumericDocValues::new(values, iter)?;
            debug_assert!(sort_map.size() <= i32::MAX as usize);
            let sorted =
                sort_doc_values(sort_map.size() as i32, &*sort_map, &mut old_values, dense)?;
            Some(sorted)
        } else {
            None
        };
        DocValuesProducerImpl::new(sorter, docs_with_field, values.clone(), writer_field_info)
    }
}
// iterates over the values we have in ram
pub(crate) struct BufferedNumericDocValues {
    iter: PackedLongValuesIterator,
    doc_with_field: DocsWithFieldSetEnum,
    value: i64,
}
impl BufferedNumericDocValues {
    pub(crate) fn new(
        values: &PackedLongValues,
        doc_with_field: DocsWithFieldSetEnum,
    ) -> Result<Self> {
        Ok(Self {
            iter: values.iterator()?,
            doc_with_field,
            value: 0,
        })
    }
}

impl DocValuesIterator for BufferedNumericDocValues {}

impl DocIdSetIterator for BufferedNumericDocValues {
    fn doc_id(&self) -> i32 {
        self.doc_with_field.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let doc_id = self.doc_with_field.next_doc()?;
        if doc_id != NO_MORE_DOCS {
            self.value = self.iter.next_value()?;
        }
        Ok(doc_id)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn cost(&self) -> Result<i64> {
        self.doc_with_field.cost()
    }
}

impl NumericDocValues for BufferedNumericDocValues {
    fn long_value(&mut self) -> Result<i64> {
        Ok(self.value)
    }
}

pub(crate) struct SortingNumericDocValues<T>
where
    T: BitSet,
{
    dvs: NumericDVs<T>,
    doc_id: i32,
    cost: Cell<i64>,
}

impl<T> SortingNumericDocValues<T>
where
    T: BitSet,
{
    pub(crate) fn new(dvs: NumericDVs<T>) -> Self {
        Self {
            dvs,
            doc_id: -1,
            cost: Cell::new(-1),
        }
    }
}

impl<T> DocValuesIterator for SortingNumericDocValues<T> where T: BitSet {}

impl<T> DocIdSetIterator for SortingNumericDocValues<T>
where
    T: BitSet,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        if self.doc_id + 1 == self.dvs.max_doc() {
            self.doc_id = NO_MORE_DOCS;
        } else {
            self.doc_id = self.dvs.advance(self.doc_id + 1);
        }
        Ok(self.doc_id)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation("use nextDoc() instead"))
    }

    fn cost(&self) -> Result<i64> {
        if self.cost.get() == -1 {
            self.cost.set(self.dvs.cost());
        }
        Ok(self.cost.get())
    }
}

impl<T> NumericDocValues for SortingNumericDocValues<T>
where
    T: BitSet,
{
    fn long_value(&mut self) -> Result<i64> {
        Ok(self.dvs.values[self.doc_id as usize])
    }
}
#[derive(Clone)]
pub(crate) struct NumericDVs<T>
where
    T: BitSet,
{
    pub values: Rc<Vec<i64>>,
    pub docs_with_field: Option<Rc<T>>,
    pub max_doc: i32,
}

impl<T> NumericDVs<T>
where
    T: BitSet,
{
    pub fn new(values: Vec<i64>, docs_with_field: Option<T>) -> Self {
        debug_assert!(values.len() <= i32::MAX as usize);
        let docs_with_field = docs_with_field.map(Rc::new);
        let max_doc = values.len() as i32;
        Self {
            values: Rc::new(values),
            docs_with_field,
            max_doc,
        }
    }

    pub(crate) fn max_doc(&self) -> i32 {
        self.max_doc
    }

    fn advance_exact(&self, target: i32) -> bool {
        match &self.docs_with_field {
            Some(bits) => bits.get(target),
            None => true,
        }
    }
    pub(crate) fn advance(&self, target: i32) -> i32 {
        if let Some(bits) = &self.docs_with_field {
            bits.next_set_bit(target)
        } else {
            // Only called when target is less than maxDoc
            target
        }
    }
    pub(crate) fn cost(&self) -> i64 {
        match &self.docs_with_field {
            Some(bits) => bits.cardinality() as i64,
            None => self.max_doc as i64,
        }
    }
}
