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
use crate::codecs::doc_values_enum::norms::Lucene90NormNumericDocValuesEnum;
use crate::codecs::lucene90_norms_consumer::Lucene90NormsConsumer;
use crate::codecs::norms_producer::NormsProducer;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::field_info::FieldInfo;
use crate::index::merge_state::{DocMapEnum, MergeState};
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::{doc_id_merger_util, DocIDMerger, DocIDMergerEnum, Sub, SubBase};
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::{IndexInput, IndexOutput};
use crate::util::error::lucene_error::{LuceneError, Result};
use std::cell::RefCell;
use std::rc::Rc;

/// Consumes normalization values.
///
/// Concrete implementations actually do *something* with the norms,
/// such as writing them into the index in a specific format.
///
/// # Lifecycle
///
/// 1. `NormsConsumer` is created by [`NormsFormat::norms_consumer`](crate::codecs::norms_format::NormsFormat::norms_consumer).
/// 2. [`add_norms_field`](NormsConsumer::add_norms_field) is called for each field with normalization values.
///    The API is *pull*-based rather than *push*-based; the implementation is free
///    to iterate over the values multiple times.
/// 3. After all fields are added, the consumer is closed.
pub trait NormsConsumer {
    /// Writes normalization values for a field.
    ///
    /// # Arguments
    /// * `field` - Field metadata
    /// * `norms_producer` - Provides numeric norms for the field
    ///
    /// # Errors
    /// If an I/O error occurs during writing.
    fn add_norms_field(
        &mut self,
        field: &Rc<FieldInfo>,
        norms_producer: &mut impl NormsProducer,
    ) -> Result<()>;
    /// Merges in the fields from the readers in `merge_state`.
    ///
    /// The default implementation calls [`merge_norms_field`](NormsConsumer::merge_norms_field) for each field,
    /// filling segments with missing norms for the field with zeros.
    ///
    /// Implementations can override this method for more sophisticated merging (e.g. bulk-byte copying).
    fn merge<I>(&mut self, merge_state: &mut MergeState<I>) -> Result<()>
    where
        I: IndexInput,
    {
        for producer in merge_state.norms_producers.iter_mut().flatten() {
            producer.check_integrity()?;
        }

        for field_info in &*merge_state.merge_field_infos.clone() {
            if field_info.has_norms() {
                self.merge_norms_field(field_info, merge_state)?;
            }
        }

        Ok(())
    }
    /// Merges the norms from `to_merge`.
    ///
    /// The default implementation calls [`add_norms_field`](NormsConsumer::add_norms_field), passing an iterator
    /// that merges and filters deleted documents on the fly.
    fn merge_norms_field<I>(
        &mut self,
        merge_field_info: &Rc<FieldInfo>,
        merge_state: &mut MergeState<I>,
    ) -> Result<()>
    where
        I: IndexInput,
    {
        let mut norms_producer = NormsProducerMerge {
            merge_field_info: merge_field_info.clone(),
            merge_state,
        };
        // TODO: try to share code with default merge of DVConsumer by passing MatchAllBits ?
        self.add_norms_field(merge_field_info, &mut norms_producer)?;
        Ok(())
    }
}

struct NormsProducerMerge<'a, I>
where
    I: IndexInput,
{
    merge_field_info: Rc<FieldInfo>,
    merge_state: &'a mut MergeState<I>,
}
impl<'a, I> NormsProducer for NormsProducerMerge<'a, I>
where
    I: IndexInput,
{
    type NumericDocValues = NumericDocValuesMerge<I>;

    fn get_norms(&mut self, field_info: &Rc<FieldInfo>) -> Result<Self::NumericDocValues> {
        if Rc::ptr_eq(field_info, &self.merge_field_info) {
            return Err(LuceneError::illegal_argument("wrong fieldInfo"));
        }

        let mut subs = vec![];
        debug_assert!(
            self.merge_state.doc_maps.len() == self.merge_state.doc_values_producers.len()
        );
        for i in 0..self.merge_state.doc_values_producers.len() {
            let mut norms: Option<Lucene90NormNumericDocValuesEnum<I>> = None;
            let norms_producer_opt = &mut self.merge_state.norms_producers[i];
            if let Some(norms_producer) = norms_producer_opt {
                let reader_field_info =
                    self.merge_state.field_infos[i].field_info_by_name(&self.merge_field_info.name);
                if let Some(reader_field_info) = &reader_field_info {
                    if reader_field_info.has_norms() {
                        norms = Some(norms_producer.get_norms(reader_field_info)?);
                    }
                }
            }

            if let Some(norms) = norms {
                let doc_map = self.merge_state.doc_maps[i].clone();
                subs.push(Rc::new(RefCell::new(Sub::new(NumericDocValuesSub::new(
                    doc_map, norms,
                )))));
            }
        }

        let doc_id_merger = doc_id_merger_util::of(subs, self.merge_state.needs_index_sort)?;
        Ok(NumericDocValuesMerge {
            doc_id: -1,
            current: None,
            doc_id_merger,
        })
    }

    fn check_integrity(&mut self) -> Result<()> {
        Ok(())
    }

    type NormsProducer<'b, T: IndexInput>
        = NormsProducerMerge<'a, I>
    where
        Self: 'b,
        T: 'b;
}

pub struct NumericDocValuesMerge<I>
where
    I: IndexInput,
{
    doc_id: i32,
    current: Option<Rc<RefCell<Sub<NumericDocValuesSub<I>>>>>,
    doc_id_merger: DocIDMergerEnum<NumericDocValuesSub<I>>,
}

impl<I> DocValuesIterator for NumericDocValuesMerge<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl<I> DocIdSetIterator for NumericDocValuesMerge<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.current = self.doc_id_merger.next()?;
        match &self.current {
            Some(current) => {
                self.doc_id = current.borrow_mut().mapped_doc_id;
                Ok(self.doc_id)
            }
            None => {
                self.doc_id = NO_MORE_DOCS;
                Ok(NO_MORE_DOCS)
            }
        }
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn cost(&self) -> Result<i64> {
        Ok(0)
    }
}

impl<I> NumericDocValues for NumericDocValuesMerge<I>
where
    I: IndexInput,
{
    fn long_value(&mut self) -> Result<i64> {
        match self.current {
            Some(ref current) => {
                let mut current = current.borrow_mut();
                current.sub.values.long_value()
            }
            None => Err(LuceneError::unreachable("should not be here")),
        }
    }
}
/// Tracks state of one numeric sub-reader that we are merging.
struct NumericDocValuesSub<I>
where
    I: IndexInput,
{
    values: Lucene90NormNumericDocValuesEnum<I>,
    doc_map: Rc<DocMapEnum>,
}
#[allow(unused)]
impl<I> NumericDocValuesSub<I>
where
    I: IndexInput,
{
    fn new(doc_map: Rc<DocMapEnum>, values: Lucene90NormNumericDocValuesEnum<I>) -> Self {
        debug_assert!(values.doc_id() == -1);
        NumericDocValuesSub { values, doc_map }
    }
}
impl<I> SubBase for NumericDocValuesSub<I>
where
    I: IndexInput,
{
    fn next_doc(&mut self) -> Result<i32> {
        self.values.next_doc()
    }

    fn get_doc_map(&self) -> Result<&Rc<DocMapEnum>> {
        Ok(&self.doc_map)
    }
}
// used for padding
impl<I> Default for NumericDocValuesSub<I>
where
    I: IndexInput,
{
    fn default() -> Self {
        NumericDocValuesSub {
            values: Lucene90NormNumericDocValuesEnum::Empty(Default::default()),
            doc_map: Rc::new(DocMapEnum::Dummy(Default::default())),
        }
    }
}

pub enum NormsConsumerEnum<O>
where
    O: IndexOutput,
{
    Lucene90(Lucene90NormsConsumer<O>),
}
impl<O> NormsConsumer for NormsConsumerEnum<O>
where
    O: IndexOutput,
{
    fn add_norms_field(
        &mut self,
        field: &Rc<FieldInfo>,
        norms_producer: &mut impl NormsProducer,
    ) -> Result<()> {
        match self {
            NormsConsumerEnum::Lucene90(consumer) => {
                consumer.add_norms_field(field, norms_producer)
            }
        }
    }
}
