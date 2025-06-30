/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::codecs::norms_consumer::NormsConsumer;
use crate::codecs::norms_producer::NormsProducer;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::docs_with_field_set::{DocsWithFieldSet, DocsWithFieldSetEnum};
use crate::index::field_info::FieldInfo;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::numeric_doc_values_writer::{ndvw_util, NumericDVs, SortingNumericDocValues};
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMap;
use crate::search::doc_id_set::DocIdSet;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::directory::Directory;
use crate::util::accountable::Accountable;
use crate::util::either_enums::EitherNumericDocValues;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::packed::packed_long_values::{
    PackedLongValues, PackedLongValuesBuilder, PackedLongValuesIterator,
};
use crate::util::packed::PackedInts;
use crate::util::{Counter, CounterEnumBorrow};
use std::rc::Rc;

/// Buffers up pending long per doc, then flushes when segment flushes.
pub(crate) struct NormValuesWriter {
    docs_with_field: DocsWithFieldSet,
    pending: PackedLongValuesBuilder,
    iw_bytes_used: CounterEnumBorrow,
    bytes_used: i64,
    field_info: Rc<FieldInfo>,
    last_doc_id: i32,
}
impl NormValuesWriter {
    pub(crate) fn new(field_info: Rc<FieldInfo>, iw_bytes_used: CounterEnumBorrow) -> Result<Self> {
        Ok(Self {
            docs_with_field: DocsWithFieldSet::new(),
            pending: PackedLongValues::delta_packed_long_values_builder_default(
                PackedInts::COMPACT,
            )?,
            iw_bytes_used,
            bytes_used: 0,
            field_info,
            last_doc_id: -1,
        })
    }
    pub(crate) fn add_value(&mut self, doc_id: i32, value: i64) -> Result<()> {
        if doc_id <= self.last_doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "Norm for \"{}\" appears more than once in this document (only one value is allowed per field)",
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
    pub(crate) fn finish(&mut self, _max_doc: i32) {
        self.docs_with_field.finish()
    }

    pub(crate) fn flush<D, DM, N>(
        &mut self,
        state: &SegmentWriteState<D>,
        sort_map: Option<Rc<DM>>,
        norms_consumer: &mut N,
    ) -> Result<()>
    where
        D: Directory,
        DM: DocMap,
        N: NormsConsumer,
    {
        self.finish(state.segment_info.max_doc()?);
        let values = std::mem::take(&mut self.pending).build()?;
        let sorted = match sort_map {
            Some(sort_map) => {
                let dense = sort_map.size() == self.docs_with_field.cardinality();
                let iter = match self.docs_with_field.iterator()? {
                    Some(iter) => iter,
                    None => return Err(LuceneError::illegal_state("DocsWithFieldSet is None")),
                };
                let mut buffer_norms = BufferedNorms::new(&values, iter);
                let sorted = ndvw_util::sort_doc_values(
                    state.segment_info.max_doc()?,
                    &*sort_map,
                    &mut buffer_norms,
                    dense,
                )?;
                Some(sorted)
            },
            None => None,
        };

        let mut norms_producer =
            NormsProducerImpl::new(sorted, std::mem::take(&mut self.docs_with_field), values)?;
        norms_consumer.add_norms_field(&self.field_info, &mut norms_producer)?;

        Ok(())
    }
}

struct NormsProducerImpl {
    sorted: Option<NumericDVs<FixedBitSet>>,
    docs_with_field: DocsWithFieldSet,
    values: PackedLongValues,
}
impl NormsProducerImpl {
    pub(crate) fn new(
        sorted: Option<NumericDVs<FixedBitSet>>,
        docs_with_field: DocsWithFieldSet,
        values: PackedLongValues,
    ) -> Result<Self> {
        Ok(Self {
            sorted,
            docs_with_field,
            values,
        })
    }
}
impl NormsProducer for NormsProducerImpl {
    type NumericDocValues =
        EitherNumericDocValues<BufferedNorms, SortingNumericDocValues<FixedBitSet>>;

    fn get_norms(&mut self, _field_info2: &Rc<FieldInfo>) -> Result<Self::NumericDocValues> {
        match &self.sorted {
            Some(sorted) => Ok(EitherNumericDocValues::S(SortingNumericDocValues::new(
                sorted.clone(),
            ))),
            None => Ok(EitherNumericDocValues::F(BufferedNorms::new(
                &self.values,
                self.docs_with_field.iterator()?.unwrap(),
            ))),
        }
    }

    fn check_integrity(&mut self) -> Result<()> {
        Ok(())
    }
}

/// iterates over the values we have in ram
struct BufferedNorms {
    iter: PackedLongValuesIterator,
    doc_with_field: DocsWithFieldSetEnum,
    value: i64,
}
impl BufferedNorms {
    pub(crate) fn new(values: &PackedLongValues, doc_with_field: DocsWithFieldSetEnum) -> Self {
        Self {
            iter: values.iterator(),
            doc_with_field,
            value: 0,
        }
    }
}

impl DocValuesIterator for BufferedNorms {
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl DocIdSetIterator for BufferedNorms {
    fn doc_id(&self) -> i32 {
        self.doc_with_field.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let doc_id = self.doc_with_field.next_doc()?;
        if doc_id != NO_MORE_DOCS {
            self.value = self.iter.next_value();
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

impl NumericDocValues for BufferedNorms {
    fn long_value(&mut self) -> Result<i64> {
        Ok(self.value)
    }
}
