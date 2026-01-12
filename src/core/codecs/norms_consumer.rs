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
use crate::core::codecs::norms_producer::{DefaultNormNumericDocValues, NormsProducer};
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::merge_state::{DocMapEnum, MergeState};
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::{DocIDMerger, DocIDMergerEnum, Sub, SubBase, of};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::store::directory::Directory;
use crate::core::util::CoreHelper;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::rc::Rc;
use std::sync::Arc;

/// Consumes normalization values.
///
/// Concrete implementations actually do *something* with the norms,
/// such as writing them into the index in a specific format.
///
/// # Lifecycle
///
/// 1. `NormsConsumer` is created by
///    [`NormsFormat::norms_consumer`](crate::core::codecs::norms_format::NormsFormat::norms_consumer).
/// 2. [`add_norms_field`](NormsConsumer::add_norms_field) is called for each
///    field with normalization values. The API is *pull*-based rather than
///    *push*-based; the implementation is free to iterate over the values
///    multiple times.
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
        field: &Arc<FieldInfo>,
        norms_producer: &mut impl NormsProducer,
    ) -> Result<()>;
    /// Merges in the fields from the readers in `merge_state`.
    ///
    /// The default implementation calls
    /// [`merge_norms_field`](NormsConsumer::merge_norms_field) for each field,
    /// filling segments with missing norms for the field with zeros.
    ///
    /// Implementations can override this method for more sophisticated merging
    /// (e.g. bulk-byte copying).
    fn merge<D>(&mut self, merge_state: &MergeState<D>) -> Result<()>
    where
        D: Directory,
    {
        for producer in merge_state.norms_producers.iter().flatten() {
            producer.check_integrity()?;
        }

        for field_info in merge_state.merge_field_infos.clone().as_ref() {
            if field_info.has_norms() {
                self.merge_norms_field(field_info, merge_state)?;
            }
        }

        Ok(())
    }
    /// Merges the norms from `to_merge`.
    ///
    /// The default implementation calls
    /// [`add_norms_field`](NormsConsumer::add_norms_field), passing an iterator
    /// that merges and filters deleted documents on the fly.
    fn merge_norms_field<D>(
        &mut self,
        merge_field_info: &Arc<FieldInfo>,
        merge_state: &MergeState<D>,
    ) -> Result<()>
    where
        D: Directory,
    {
        let mut norms_producer = NormsProducerMerge {
            merge_field_info: merge_field_info.clone(),
            merge_state,
        };
        // TODO: try to share code with default merge of DVConsumer by passing
        // MatchAllBits ?
        self.add_norms_field(merge_field_info, &mut norms_producer)?;
        Ok(())
    }
}

struct NormsProducerMerge<'a, D>
where
    D: Directory,
{
    merge_field_info: Arc<FieldInfo>,
    merge_state: &'a MergeState<D>,
}

impl<D> Clone for NormsProducerMerge<'_, D>
where
    D: Directory,
{
    fn clone(&self) -> Self {
        unreachable!(
            "{} {}",
            std::any::type_name::<Self>(),
            CoreHelper::CLONE_WARRING
        )
    }
}

impl<D> NormsProducer for NormsProducerMerge<'_, D>
where
    D: Directory,
{
    type NumericDocValues = NumericDocValuesMerge<DefaultNormNumericDocValues<D::IndexInput>>;

    fn get_norms(&self, field_info: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
        if Arc::ptr_eq(field_info, &self.merge_field_info) {
            return Err(LuceneError::illegal_argument("wrong fieldInfo"));
        }

        let mut subs = vec![];
        debug_assert!(
            self.merge_state.doc_maps.len() == self.merge_state.doc_values_producers.len()
        );
        for i in 0..self.merge_state.doc_values_producers.len() {
            let mut norms = None;
            let norms_producer_opt = &self.merge_state.norms_producers[i];
            if let Some(norms_producer) = norms_producer_opt {
                let reader_field_info =
                    self.merge_state.field_infos[i].field_info_by_name(&self.merge_field_info.name);
                if let Some(reader_field_info) = &reader_field_info
                    && reader_field_info.has_norms()
                {
                    norms = Some(norms_producer.get_norms(reader_field_info)?);
                }
            }

            if let Some(norms) = norms {
                let doc_map = self.merge_state.doc_maps[i].clone();
                subs.push(Sub::new(NumericDocValuesSub::new(doc_map, norms)));
            }
        }

        let doc_id_merger = of(subs, self.merge_state.needs_index_sort)?;
        Ok(NumericDocValuesMerge {
            doc_id: -1,
            current: None,
            doc_id_merger,
        })
    }

    fn check_integrity(&self) -> Result<()> {
        Ok(())
    }
}

pub struct NumericDocValuesMerge<N>
where
    N: NumericDocValues,
{
    doc_id: i32,
    current: Option<usize>,
    doc_id_merger: DocIDMergerEnum<NumericDocValuesSub<N>>,
}

impl<N> DocValuesIterator for NumericDocValuesMerge<N>
where
    N: NumericDocValues,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl<N> DocIdSetIterator for NumericDocValuesMerge<N>
where
    N: NumericDocValues,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.current = self.doc_id_merger.next()?;
        match self.current {
            Some(ref current) => {
                let v = self.doc_id_merger.get_subs()[*current].mapped_doc_id;
                self.doc_id = v;
                Ok(self.doc_id)
            },
            None => {
                self.doc_id = NO_MORE_DOCS;
                Ok(NO_MORE_DOCS)
            },
        }
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn cost(&self) -> Result<i64> {
        Ok(0)
    }
}

impl<N> NumericDocValues for NumericDocValuesMerge<N>
where
    N: NumericDocValues,
{
    fn long_value(&mut self) -> Result<i64> {
        match self.current {
            Some(ref current) => {
                let v = &mut self.doc_id_merger.get_subs_mut()[*current].sub;
                v.values.long_value()
            },
            None => Err(LuceneError::unreachable("should not be here")),
        }
    }
}
/// Tracks state of one numeric sub-reader that we are merging.
struct NumericDocValuesSub<N>
where
    N: NumericDocValues,
{
    values: N,
    doc_map: Rc<DocMapEnum>,
}

impl<N> NumericDocValuesSub<N>
where
    N: NumericDocValues,
{
    fn new(doc_map: Rc<DocMapEnum>, values: N) -> Self {
        debug_assert!(values.doc_id() == -1);
        NumericDocValuesSub { values, doc_map }
    }
}
impl<N> SubBase for NumericDocValuesSub<N>
where
    N: NumericDocValues,
{
    fn next_doc(&mut self) -> Result<i32> {
        self.values.next_doc()
    }

    fn get_doc_map(&self) -> Result<&Rc<DocMapEnum>> {
        Ok(&self.doc_map)
    }
}
