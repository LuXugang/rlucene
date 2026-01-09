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
use crate::core::codecs::doc_values_producer::DocValuesProducer;
use crate::core::codecs::dummy::dummy_binary_doc_values::DummyBinaryDocValues;
use crate::core::codecs::dummy::dummy_doc_values_skipper::DummyDocValuesSkipper;
use crate::core::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::core::codecs::dummy::dummy_sorted_numeric_doc_values::DummySortedNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_set_doc_values::DummySortedSetDocValues;
use crate::core::codecs::lucene90::lucene90_doc_values_producer::{
    Lucene90BinaryDocValuesEnum, Lucene90NumericDocValuesEnum, Lucene90SortedNumericDocValuesEnum,
};
use crate::core::codecs::lucene90_doc_values_producer::{
    Lucene90SortedDocValuesEnum, Lucene90SortedSetDocValuesEnum,
};
use crate::core::index::binary_doc_values::{BinaryDocValues, BinaryDocValuesEnum3};
use crate::core::index::doc_values::DocValues;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::core::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::core::index::dummy::dummy_term_state_type::DummyTermState;
use crate::core::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::filtered_terms_enum::{
    AcceptStatus, FilteredTermsEnum, FilteredTermsEnumBase,
};
use crate::core::index::merge_state::{DocMapEnum, MergeState};
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::ordinal_map::{OrdinalMap, SegmentToGlobalOrds};
use crate::core::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::core::index::sorted_doc_values::{SortedDocValues, SortedDocValuesEnum2};
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValuesEnum2;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::sorted_set_doc_values_writer::SortedSetDocValuesEnum2;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum, TermsEnumEnum2};
use crate::core::index::{BytesRef, DocIDMerger, DocIDMergerEnum, Sub, SubBase, of};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::store::IndexInput;
use crate::core::util::CoreHelper;
use crate::core::util::bits::Bits;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::dummy::dummy_attribute_source::DummyAttributeSource;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::long_bit_set::LongBitSet;
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::PackedInts;
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

pub trait DocValuesConsumer {
    fn add_numeric_field<D>(&mut self, field: &Arc<FieldInfo>, values_producer: &D) -> Result<()>
    where
        D: DocValuesProducer;
    fn add_binary_field<D>(&mut self, field: &Arc<FieldInfo>, values_producer: &D) -> Result<()>
    where
        D: DocValuesProducer;
    fn add_sorted_field<D>(&mut self, field: &Arc<FieldInfo>, values_producer: &D) -> Result<()>
    where
        D: DocValuesProducer;
    fn add_sorted_numeric_field<D>(
        &mut self,
        field: &Arc<FieldInfo>,
        values_producer: &D,
    ) -> Result<()>
    where
        D: DocValuesProducer;
    fn add_sorted_set_field<D>(
        &mut self,
        field: &Arc<FieldInfo>,
        values_producer: &D,
    ) -> Result<()>
    where
        D: DocValuesProducer;

    fn merge_numeric_field<I>(
        &mut self,
        merge_field_info: &Arc<FieldInfo>,
        merge_state: &mut MergeState<I>,
    ) -> Result<()>
    where
        I: IndexInput,
    {
        let producer = EmptyDocValuesProducerMerge1 {
            merge_field_info: merge_field_info.clone(),
            merge_state,
        };
        self.add_numeric_field(merge_field_info, &producer)?;
        Ok(())
    }
    fn merge_binary_filed<I: IndexInput>(
        &mut self,
        merge_field_info: &Arc<FieldInfo>,
        merge_state: &mut MergeState<I>,
    ) -> Result<()> {
        let producer = EmptyDocValuesProducerMerge2 {
            merge_field_info: merge_field_info.clone(),
            merge_state,
        };
        self.add_binary_field(merge_field_info, &producer)
    }
    fn merge_sorted_numeric_field<I: IndexInput>(
        &mut self,
        merge_field_info: &Arc<FieldInfo>,
        merge_state: &mut MergeState<I>,
    ) -> Result<()> {
        let producer = EmptyDocValuesProducerMerge3 {
            merge_field_info: merge_field_info.clone(),
            merge_state,
        };
        self.add_sorted_numeric_field(merge_field_info, &producer)
    }
    fn merge_sorted_field<I: IndexInput>(
        &mut self,
        field_info: &Arc<FieldInfo>,
        merge_state: &mut MergeState<I>,
    ) -> Result<()> {
        let mut to_merge = Vec::with_capacity(merge_state.doc_values_producers.len());

        for i in 0..merge_state.doc_values_producers.len() {
            let mut values = None;

            if let Some(doc_values_producer) = &merge_state.doc_values_producers[i]
                && let Some(reader_field_info) =
                    merge_state.field_infos[i].field_info_by_name(&field_info.name)
                && *reader_field_info.get_doc_values_type() == DocValuesType::Sorted
            {
                values = Some(SortedDocValuesEnum2::A(
                    doc_values_producer.get_sorted(&reader_field_info)?,
                ));
            }
            if values.is_none() {
                values = Some(SortedDocValuesEnum2::B(DocValues::empty_sorted()));
            }
            to_merge.push(values.unwrap());
        }

        let num_readers = to_merge.len();
        // step 1: iterate thru each sub and mark terms still in use
        let mut live_terms = Vec::with_capacity(num_readers);
        let mut weights: Vec<i64> = vec![0; num_readers];

        for (sub, mut dvs) in to_merge.into_iter().enumerate() {
            let live_docs_opt = merge_state.live_docs[sub].as_ref();

            match live_docs_opt {
                None => {
                    let value_count = dvs.get_value_count()?;
                    weights[sub] = value_count as i64;
                    let terms_enum = dvs.take_terms_enum()?;
                    live_terms.push(Some(TermsEnumEnum2::A(terms_enum)));
                },
                Some(live_docs) => {
                    let value_count = dvs.get_value_count()? as usize;
                    let mut bitset = LongBitSet::new(value_count)?;

                    loop {
                        let doc_id = dvs.next_doc()?;
                        if doc_id == NO_MORE_DOCS {
                            break;
                        }
                        if live_docs.get(doc_id as usize) {
                            let ord = dvs.ord_value()?;
                            if ord >= 0 {
                                bitset.set(ord as usize);
                            }
                        }
                    }

                    let cardinality = bitset.cardinality();
                    weights[sub] = cardinality as i64;
                    let terms_enum = BitsFilteredTermsEnum::new(dvs.take_terms_enum()?, bitset);
                    live_terms.push(Some(TermsEnumEnum2::B(terms_enum)));
                },
            }
        }
        // step 2: create ordinal map (this conceptually does the "merging")
        let ordinal_map = OrdinalMap::build(None, &mut live_terms, &weights, PackedInts::COMPACT)?;
        let producer = EmptyDocValuesProducerMerge4 {
            field_info: field_info.clone(),
            merge_state,
            map: Rc::new(ordinal_map),
        };
        self.add_sorted_field(field_info, &producer)
    }
    fn merge_sorted_set_field<I: IndexInput>(
        &mut self,
        merge_field_info: &Arc<FieldInfo>,
        merge_state: &mut MergeState<I>,
    ) -> Result<()> {
        let mut to_merge = Vec::with_capacity(merge_state.doc_values_producers.len());

        for i in 0..merge_state.doc_values_producers.len() {
            let mut values = None;

            if let Some(doc_values_producer) = &merge_state.doc_values_producers[i]
                && let Some(field_info) =
                    merge_state.field_infos[i].field_info_by_name(&merge_field_info.name)
                && *field_info.get_doc_values_type() == DocValuesType::SortedSet
            {
                values = Some(SortedSetDocValuesEnum2::A(
                    doc_values_producer.get_sorted_set(&field_info)?,
                ));
            }

            if values.is_none() {
                values = Some(SortedSetDocValuesEnum2::B(DocValues::empty_sorted_set()?));
            }
            to_merge.push(values.unwrap());
        }

        // step 1: iterate thru each sub and mark terms still in use
        let num_readers = to_merge.len();
        let mut live_terms = Vec::with_capacity(num_readers);
        let mut weights: Vec<i64> = vec![0; num_readers];

        for (sub, mut dv) in to_merge.into_iter().enumerate() {
            let live_docs_opt = merge_state.live_docs[sub].as_ref();

            match live_docs_opt {
                None => {
                    let value_count = dv.get_value_count()?;
                    weights[sub] = value_count;
                    let terms_enum = dv.take_terms_enum()?;
                    live_terms.push(Some(TermsEnumEnum2::A(terms_enum)));
                },
                Some(live_docs) => {
                    let value_count = dv.get_value_count()? as usize;
                    let mut bitset = LongBitSet::new(value_count)?;

                    loop {
                        let doc_id = dv.next_doc()?;
                        if doc_id == NO_MORE_DOCS {
                            break;
                        }
                        if live_docs.get(doc_id as usize) {
                            let count = dv.doc_value_count()?;
                            for _ in 0..count {
                                let ord = dv.next_ord()?;
                                bitset.set(ord as usize);
                            }
                        }
                    }

                    let cardinality = bitset.cardinality();
                    weights[sub] = cardinality as i64;

                    let terms_enum = BitsFilteredTermsEnum::new(dv.take_terms_enum()?, bitset);
                    live_terms.push(Some(TermsEnumEnum2::B(terms_enum)));
                },
            }
        }

        // step 2: create ordinal map (this conceptually does the "merging")
        let _ordinal_map = OrdinalMap::build(None, &mut live_terms, &weights, PackedInts::COMPACT)?;
        todo!()
    }
}
pub struct BitsFilteredTermsEnum {
    live_terms: LongBitSet,
}
impl BitsFilteredTermsEnum {
    fn new<TE>(in_: TE, live_terms: LongBitSet) -> FilteredTermsEnum<TE, Self>
    where
        TE: TermsEnum,
    {
        let sub = Self { live_terms };
        FilteredTermsEnum::new(in_, sub)
    }
}
impl FilteredTermsEnumBase for BitsFilteredTermsEnum {
    fn accept(&mut self, _term: &BytesRef<Vec<u8>>, ord: i64) -> Result<AcceptStatus> {
        if self.live_terms.get(ord as usize) {
            Ok(AcceptStatus::Yes)
        } else {
            Ok(AcceptStatus::No)
        }
    }
}

// 1. NumericDocValues
/// Tracks state of one numeric sub-reader that we are merging.
pub(crate) struct NumericDocValuesSub<I>
where
    I: IndexInput,
{
    values: Lucene90NumericDocValuesEnum<I>,
    doc_map: Rc<DocMapEnum>,
}

impl<I> NumericDocValuesSub<I>
where
    I: IndexInput,
{
    fn new(doc_map: Rc<DocMapEnum>, values: Lucene90NumericDocValuesEnum<I>) -> Self {
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
impl<I> Default for NumericDocValuesSub<I>
where
    I: IndexInput,
{
    fn default() -> Self {
        NumericDocValuesSub {
            values: Lucene90NumericDocValuesEnum::C(Default::default()),
            doc_map: Rc::new(DocMapEnum::default()),
        }
    }
}
pub struct NumericDocValuesMerge<I>
where
    I: IndexInput,
{
    doc_id: i32,
    current: Option<usize>,
    doc_id_merger: DocIDMergerEnum<NumericDocValuesSub<I>>,
    final_cost: i64,
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
        Ok(self.final_cost)
    }
}

impl<I> NumericDocValues for NumericDocValuesMerge<I>
where
    I: IndexInput,
{
    fn long_value(&mut self) -> Result<i64> {
        match self.current {
            Some(ref current) => {
                let v = &mut self.doc_id_merger.get_subs_mut()[*current];
                v.sub.values.long_value()
            },
            None => Err(LuceneError::unreachable("should not be here")),
        }
    }
}
pub(crate) struct EmptyDocValuesProducerMerge1<'a, I>
where
    I: IndexInput,
{
    merge_field_info: Arc<FieldInfo>,
    merge_state: &'a mut MergeState<I>,
}

impl<I> Clone for EmptyDocValuesProducerMerge1<'_, I>
where
    I: IndexInput,
{
    fn clone(&self) -> Self {
        unreachable!(
            "{} {}",
            std::any::type_name::<Self>(),
            CoreHelper::CLONE_WARRING
        )
    }
}

impl<I> DocValuesProducer for EmptyDocValuesProducerMerge1<'_, I>
where
    I: IndexInput,
{
    type NumericDocValues = NumericDocValuesMerge<I>;

    fn get_numeric(&self, field_info: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
        if !Arc::ptr_eq(field_info, &self.merge_field_info) {
            return Err(LuceneError::illegal_argument("wrong fieldInfo"));
        }

        let mut subs = vec![];
        debug_assert!(
            self.merge_state.doc_maps.len() == self.merge_state.doc_values_producers.len()
        );
        for i in 0..self.merge_state.doc_values_producers.len() {
            let mut values = None;
            let doc_values_producer_opt = &self.merge_state.doc_values_producers[i];
            if let Some(doc_values_producer) = doc_values_producer_opt {
                let reader_field_info =
                    self.merge_state.field_infos[i].field_info_by_name(&self.merge_field_info.name);
                if let Some(reader_field_info) = &reader_field_info
                    && *reader_field_info.get_doc_values_type() == DocValuesType::Numeric
                {
                    values = Some(doc_values_producer.get_numeric(reader_field_info)?);
                }
            }

            if let Some(values) = values {
                let doc_map = self.merge_state.doc_maps[i].clone();
                subs.push(Sub::new(NumericDocValuesSub::new(doc_map, values)));
            }
        }
        merge_numeric_values(subs, self.merge_state.needs_index_sort)
    }

    type BinaryDocValues = DummyBinaryDocValues;
    type SortedDocValues = DummySortedDocValues;

    type SortedNumericDocValues = DummySortedNumericDocValues;
    type SortedSetDocValues = DummySortedSetDocValues;
    type DocValuesSkipper = DummyDocValuesSkipper;
}
// 2. BinaryDocValues
/// Tracks state of one binary sub-reader that we are merging.
struct BinaryDocValuesSub<I>
where
    I: IndexInput,
{
    values: Lucene90BinaryDocValuesEnum<I>,
    doc_map: Rc<DocMapEnum>,
}

impl<I> BinaryDocValuesSub<I>
where
    I: IndexInput,
{
    fn new(doc_map: Rc<DocMapEnum>, values: Lucene90BinaryDocValuesEnum<I>) -> Self {
        debug_assert!(values.doc_id() == -1);
        BinaryDocValuesSub { values, doc_map }
    }
}

impl<I> SubBase for BinaryDocValuesSub<I>
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
impl<I> Default for BinaryDocValuesSub<I>
where
    I: IndexInput,
{
    fn default() -> Self {
        BinaryDocValuesSub {
            values: BinaryDocValuesEnum3::C(Default::default()),
            doc_map: Rc::new(DocMapEnum::default()),
        }
    }
}

pub struct BinaryDocValuesMerge<I>
where
    I: IndexInput,
{
    doc_id: i32,
    current: Option<usize>,
    doc_id_merger: DocIDMergerEnum<BinaryDocValuesSub<I>>,
    final_cost: i64,
}

impl<I> DocValuesIterator for BinaryDocValuesMerge<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl<I> DocIdSetIterator for BinaryDocValuesMerge<I>
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
                let mapped_doc_id = self.doc_id_merger.get_subs()[*current].mapped_doc_id;
                self.doc_id = mapped_doc_id;
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
        Ok(self.final_cost)
    }
}

impl<I> BinaryDocValues for BinaryDocValuesMerge<I>
where
    I: IndexInput,
{
    fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        match self.current {
            Some(ref current) => {
                let v = &mut self.doc_id_merger.get_subs_mut()[*current].sub;
                v.values.binary_value()
            },
            None => Err(LuceneError::unreachable("should not be here")),
        }
    }
}
pub(crate) struct EmptyDocValuesProducerMerge2<'a, I>
where
    I: IndexInput,
{
    merge_field_info: Arc<FieldInfo>,
    merge_state: &'a mut MergeState<I>,
}

impl<I> Clone for EmptyDocValuesProducerMerge2<'_, I>
where
    I: IndexInput,
{
    fn clone(&self) -> Self {
        unreachable!(
            "{} {}",
            std::any::type_name::<Self>(),
            CoreHelper::CLONE_WARRING
        )
    }
}

impl<I> DocValuesProducer for EmptyDocValuesProducerMerge2<'_, I>
where
    I: IndexInput,
{
    type NumericDocValues = DummyNumericDocValues;
    type BinaryDocValues = BinaryDocValuesMerge<I>;

    fn get_binary(&self, field_info: &Arc<FieldInfo>) -> Result<Self::BinaryDocValues> {
        if !Arc::ptr_eq(field_info, &self.merge_field_info) {
            return Err(LuceneError::illegal_argument("wrong fieldInfo"));
        }

        let mut subs = vec![];
        let mut cost = 0;
        debug_assert!(
            self.merge_state.doc_maps.len() == self.merge_state.doc_values_producers.len()
        );

        for i in 0..self.merge_state.doc_values_producers.len() {
            let mut values = None;
            let doc_values_producer_opt = &self.merge_state.doc_values_producers[i];

            if let Some(doc_values_producer) = doc_values_producer_opt {
                let reader_field_info =
                    self.merge_state.field_infos[i].field_info_by_name(&self.merge_field_info.name);
                if let Some(reader_field_info) = &reader_field_info
                    && *reader_field_info.get_doc_values_type() == DocValuesType::Binary
                {
                    values = Some(doc_values_producer.get_binary(reader_field_info)?);
                }
            }

            if let Some(values) = values {
                cost += values.cost()?;
                let doc_map = self.merge_state.doc_maps[i].clone();
                subs.push(Sub::new(BinaryDocValuesSub::new(doc_map, values)));
            }
        }
        let doc_id_merger = of(subs, self.merge_state.needs_index_sort)?;
        let doc_value = BinaryDocValuesMerge {
            doc_id: -1,
            current: None,
            doc_id_merger,
            final_cost: cost,
        };
        Ok(doc_value)
    }

    type SortedDocValues = DummySortedDocValues;

    type SortedNumericDocValues = DummySortedNumericDocValues;
    type SortedSetDocValues = DummySortedSetDocValues;
    type DocValuesSkipper = DummyDocValuesSkipper;
}
// 3. SortedNumericDocValues
/// Tracks state of one sorted numeric sub-reader that we are merging.
struct SortedNumericDocValuesSub<I>
where
    I: IndexInput,
{
    values: Lucene90SortedNumericDocValuesEnum<I>,
    doc_map: Rc<DocMapEnum>,
}

impl<I> SortedNumericDocValuesSub<I>
where
    I: IndexInput,
{
    fn new(doc_map: Rc<DocMapEnum>, values: Lucene90SortedNumericDocValuesEnum<I>) -> Self {
        debug_assert!(values.doc_id() == -1);
        SortedNumericDocValuesSub { values, doc_map }
    }
}

impl<I> SubBase for SortedNumericDocValuesSub<I>
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

pub struct SortedNumericDocValuesMerge<I>
where
    I: IndexInput,
{
    doc_id: i32,
    current_sub: Option<usize>,
    doc_id_merger: DocIDMergerEnum<SortedNumericDocValuesSub<I>>,
    final_cost: i64,
}

impl<I> DocValuesIterator for SortedNumericDocValuesMerge<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl<I> DocIdSetIterator for SortedNumericDocValuesMerge<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.current_sub = self.doc_id_merger.next()?;
        match self.current_sub {
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
        Ok(self.final_cost)
    }
}

impl<I> SortedNumericDocValues for SortedNumericDocValuesMerge<I>
where
    I: IndexInput,
{
    fn next_value(&mut self) -> Result<i64> {
        match self.current_sub {
            Some(ref current) => {
                let v = &mut self.doc_id_merger.get_subs_mut()[*current].sub;
                v.values.next_value()
            },
            None => Err(LuceneError::unreachable("should not be here")),
        }
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        match self.current_sub {
            Some(ref current) => {
                let v = &mut self.doc_id_merger.get_subs_mut()[*current];
                v.sub.values.doc_value_count()
            },
            None => Err(LuceneError::unreachable("should not be here")),
        }
    }

    // TODO: is it correct?
    type NumericDocValues = DummyNumericDocValues;
}
pub(crate) struct EmptyDocValuesProducerMerge3<'a, I>
where
    I: IndexInput,
{
    merge_field_info: Arc<FieldInfo>,
    merge_state: &'a mut MergeState<I>,
}

impl<I> Clone for EmptyDocValuesProducerMerge3<'_, I>
where
    I: IndexInput,
{
    fn clone(&self) -> Self {
        unreachable!(
            "{} {}",
            std::any::type_name::<Self>(),
            CoreHelper::CLONE_WARRING
        )
    }
}

impl<I> DocValuesProducer for EmptyDocValuesProducerMerge3<'_, I>
where
    I: IndexInput,
{
    type NumericDocValues = DummyNumericDocValues;
    type BinaryDocValues = DummyBinaryDocValues;
    type SortedDocValues = DummySortedDocValues;
    type SortedNumericDocValues = SortedNumericDocValuesEnum2<
        SingletonSortedNumericDocValues<NumericDocValuesMerge<I>>,
        SortedNumericDocValuesMerge<I>,
    >;

    fn get_sorted_numeric(
        &self,
        field_info: &Arc<FieldInfo>,
    ) -> Result<Self::SortedNumericDocValues> {
        if !Arc::ptr_eq(field_info, &self.merge_field_info) {
            return Err(LuceneError::illegal_argument("wrong FieldInfo"));
        }
        // We must make new iterators + DocIDMerger for each iterator:
        let mut subs = vec![];
        let mut cost = 0;
        let mut all_singletons = true;

        for i in 0..self.merge_state.doc_values_producers.len() {
            let mut values = None;
            let doc_values_producer_opt = &self.merge_state.doc_values_producers[i];
            if let Some(doc_values_producer) = doc_values_producer_opt {
                let reader_field_info =
                    self.merge_state.field_infos[i].field_info_by_name(&self.merge_field_info.name);
                if let Some(reader_field_info) = reader_field_info
                    && *reader_field_info.get_doc_values_type() == DocValuesType::SortedNumeric
                {
                    values = Some(doc_values_producer.get_sorted_numeric(&reader_field_info)?);
                }
            }

            if values.is_none() {
                values = Some(Lucene90SortedNumericDocValuesEnum::D(
                    DocValues::empty_sorted_numeric()?,
                ));
            }
            {
                let values_ref = values.as_ref().unwrap();
                cost += values_ref.cost()?;
                if all_singletons && matches!(values_ref, Lucene90SortedNumericDocValuesEnum::C(_))
                {
                    all_singletons = false;
                }
            }
            if let Some(values) = values {
                let doc_map = self.merge_state.doc_maps[i].clone();
                subs.push(Sub::new(SortedNumericDocValuesSub::new(doc_map, values)));
            }
        }

        if all_singletons {
            // All subs are single-valued.
            // We specialize for that case since it makes it easier for codecs
            // to optimize for single-valued fields.
            let mut single_valued_subs = vec![];
            for sub in &mut subs {
                let single_valued_values = match &mut sub.sub.values {
                    Lucene90SortedNumericDocValuesEnum::C(inner) => {
                        inner.get_numeric_doc_values()?
                    },
                    _ => return Err(LuceneError::unreachable("")),
                };
                single_valued_subs.push(Sub::new(NumericDocValuesSub::new(
                    sub.sub.doc_map.clone(),
                    single_valued_values,
                )));
            }
            let dv = merge_numeric_values(single_valued_subs, self.merge_state.needs_index_sort)?;
            return Ok(SortedNumericDocValuesEnum2::A(
                DocValues::singleton_numeric(dv)?,
            ));
        }
        let doc_id_merger = of(subs, self.merge_state.needs_index_sort)?;
        Ok(SortedNumericDocValuesEnum2::B(
            SortedNumericDocValuesMerge {
                doc_id: -1,
                current_sub: None,
                doc_id_merger,
                final_cost: cost,
            },
        ))
    }

    type SortedSetDocValues = DummySortedSetDocValues;
    type DocValuesSkipper = DummyDocValuesSkipper;
}

pub(crate) fn merge_numeric_values<I>(
    mut subs: Vec<Sub<NumericDocValuesSub<I>>>,
    index_is_sorted: bool,
) -> Result<NumericDocValuesMerge<I>>
where
    I: IndexInput,
{
    let mut cost = 0;
    for sub in &mut subs {
        cost = sub.sub.values.cost()?;
    }
    let doc_id_merger = of(subs, index_is_sorted)?;
    Ok(NumericDocValuesMerge {
        doc_id: -1,
        current: None,
        doc_id_merger,
        final_cost: cost,
    })
}
// 4. SortedDocValues
struct SortedDocValuesSub<I>
where
    I: IndexInput,
{
    values: Lucene90SortedDocValuesEnum<I>,
    map: Rc<SegmentToGlobalOrds>,
    doc_map: Rc<DocMapEnum>,
}
impl<I> SortedDocValuesSub<I>
where
    I: IndexInput,
{
    fn new(
        doc_map: Rc<DocMapEnum>,
        values: Lucene90SortedDocValuesEnum<I>,
        map: Rc<SegmentToGlobalOrds>,
    ) -> Self {
        debug_assert!(values.doc_id() == -1);
        SortedDocValuesSub {
            values,
            map,
            doc_map,
        }
    }
}

impl<I> SubBase for SortedDocValuesSub<I>
where
    I: IndexInput,
{
    fn next_doc(&mut self) -> Result<i32> {
        todo!()
    }

    fn get_doc_map(&self) -> Result<&Rc<DocMapEnum>> {
        Ok(&self.doc_map)
    }
}

pub(crate) struct EmptyDocValuesProducerMerge4<'a, I>
where
    I: IndexInput,
{
    field_info: Arc<FieldInfo>,
    merge_state: &'a mut MergeState<I>,
    map: Rc<OrdinalMap>,
}

impl<I> Clone for EmptyDocValuesProducerMerge4<'_, I>
where
    I: IndexInput,
{
    fn clone(&self) -> Self {
        unreachable!(
            "{} {}",
            std::any::type_name::<Self>(),
            CoreHelper::CLONE_WARRING
        )
    }
}

impl<I> DocValuesProducer for EmptyDocValuesProducerMerge4<'_, I>
where
    I: IndexInput,
{
    type NumericDocValues = DummyNumericDocValues;
    type BinaryDocValues = DummyBinaryDocValues;
    type SortedDocValues = SortedDocValuesMerge<I>;

    fn get_sorted(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedDocValues> {
        if !Arc::ptr_eq(field, &self.field_info) {
            return Err(LuceneError::illegal_argument("wrong FieldInfo"));
        }

        // We must make new iterators + DocIDMerger for each iterator:
        let mut subs = Vec::with_capacity(self.merge_state.doc_values_producers.len());

        for i in 0..self.merge_state.doc_values_producers.len() {
            let mut values = None;

            if let Some(doc_values_producer) = &self.merge_state.doc_values_producers[i]
                && let Some(reader_field_info) =
                    self.merge_state.field_infos[i].field_info_by_name(&self.field_info.name)
                && *reader_field_info.get_doc_values_type() == DocValuesType::Sorted
            {
                values = Some(Lucene90SortedDocValuesEnum::A(
                    doc_values_producer.get_sorted(&reader_field_info)?,
                ));
            }
            if values.is_none() {
                values = Some(Lucene90SortedDocValuesEnum::B(DocValues::empty_sorted()));
            }

            let doc_map = self.merge_state.doc_maps[i].clone();
            let map = self.map.get_global_ords(i).clone();

            subs.push(Sub::new(SortedDocValuesSub::new(
                doc_map,
                values.unwrap(),
                map,
            )));
        }

        merge_sorted_values(subs, self.merge_state.needs_index_sort, self.map.clone())
    }

    type SortedNumericDocValues = DummySortedNumericDocValues;
    type SortedSetDocValues = DummySortedSetDocValues;
    type DocValuesSkipper = DummyDocValuesSkipper;
}

pub struct SortedDocValuesMerge<I>
where
    I: IndexInput,
{
    doc_id: i32,
    current: Option<usize>,
    doc_id_merger: DocIDMergerEnum<SortedDocValuesSub<I>>,
    final_cost: i64,
    map: Rc<OrdinalMap>,
}

impl<I> DocValuesIterator for SortedDocValuesMerge<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl<I> DocIdSetIterator for SortedDocValuesMerge<I>
where
    I: IndexInput,
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
        Ok(self.final_cost)
    }
}

impl<I> SortedDocValues for SortedDocValuesMerge<I>
where
    I: IndexInput,
{
    fn ord_value(&mut self) -> Result<i32> {
        let current = *self.current.as_ref().unwrap();
        let current_sub = &mut self.doc_id_merger.get_subs_mut()[current];
        let sub_ord = current_sub.sub.values.ord_value()?;
        debug_assert!(sub_ord != -1);
        Ok(current_sub.sub.map.get(sub_ord as usize)? as i32)
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        let segment_number = self.map.get_first_segment_number(ord as usize)?;
        let segment_ord = self.map.get_first_segment_ord(ord as usize)? as i32;
        self.doc_id_merger.get_subs_mut()[segment_number as usize]
            .sub
            .values
            .lookup_ord(segment_ord)
    }

    fn get_value_count(&self) -> Result<i32> {
        Ok(self.map.get_value_count() as i32)
    }

    type TermsEnumRef<'a>
        = MergedTermsEnum<<Lucene90SortedDocValuesEnum<I> as SortedDocValues>::TermsEnumRef<'a>>
    where
        Self: 'a;
    type TermsEnum1 =
        MergedTermsEnum<<Lucene90SortedDocValuesEnum<I> as SortedDocValues>::TermsEnum1>;

    fn terms_enum(&mut self) -> Result<Self::TermsEnumRef<'_>> {
        let subs = self.doc_id_merger.get_subs_mut();
        let mut terms_enum_subs = Vec::with_capacity(subs.len());
        for sub in subs {
            terms_enum_subs.push(sub.sub.values.terms_enum()?);
        }
        Ok(MergedTermsEnum::new(self.map.clone(), terms_enum_subs))
    }

    fn take_terms_enum(self) -> Result<Self::TermsEnum1> {
        let subs = self.doc_id_merger.take_subs();
        let mut terms_enum_subs = Vec::with_capacity(subs.len());
        for sub in subs {
            terms_enum_subs.push(sub.sub.values.take_terms_enum()?);
        }
        Ok(MergedTermsEnum::new(self.map.clone(), terms_enum_subs))
    }
}
/// A merged [`TermsEnum`]. This helps avoid relying on the default terms enum, which calls
/// [`SortedDocValues::lookup_ord`] or [`SortedSetDocValues::lookup_ord`] on every call to
/// [`TermsEnum::next`].
pub struct MergedTermsEnum<TE>
where
    TE: TermsEnum,
{
    subs: Vec<TE>,
    ordinal_map: Rc<OrdinalMap>,
    value_count: i64,
    ord: i64,
    term: BytesRef<Vec<u8>>,
}
impl<TE> MergedTermsEnum<TE>
where
    TE: TermsEnum,
{
    fn new(ordinal_map: Rc<OrdinalMap>, subs: Vec<TE>) -> Self {
        Self {
            subs,
            ordinal_map,
            value_count: 0,
            ord: -1,
            term: BytesRef::new(),
        }
    }
}

impl<TE> BytesRefIterator for MergedTermsEnum<TE>
where
    TE: TermsEnum,
{
    fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        self.ord += 1;
        if self.ord >= self.value_count {
            return Ok(None);
        }
        let ord = self.ord as usize;
        let sub_num = self.ordinal_map.get_first_segment_number(ord)?;
        let sub_ord = self.ordinal_map.get_first_segment_ord(ord)?;

        let sub = &mut self.subs[sub_num as usize];
        let mut end;
        loop {
            end = sub.next()?.is_none();
            if sub.ord()? >= sub_ord {
                debug_assert!(sub.ord()? == sub_ord);
                return if end {
                    Ok(None)
                } else {
                    self.term = sub.term()?.into_owned();
                    Ok(Some(Cow::Borrowed(&self.term)))
                };
            }
        }
    }
}

impl<TE> TermsEnum for MergedTermsEnum<TE>
where
    TE: TermsEnum,
{
    type AttributeSource = DummyAttributeSource;

    fn attributes(&self) -> Result<Self::AttributeSource> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn seek_ceil(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        Ok(Cow::Borrowed(&self.term))
    }

    fn ord(&self) -> Result<i64> {
        Ok(self.ord)
    }

    fn doc_freq(&mut self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    type PostingsEnum = DummyPostingsEnum;

    fn postings_with_flags(
        &mut self,
        _reuse: Option<Self::PostingsEnum>,
        _flags: i32,
    ) -> Result<Self::PostingsEnum> {
        Err(LuceneError::unsupported_operation(""))
    }

    type ImpactsEnum = DummyImpactsEnum;

    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
        Err(LuceneError::unsupported_operation(""))
    }

    type TermState = DummyTermState;

    fn term_state(&mut self) -> Result<Self::TermState> {
        Err(LuceneError::unsupported_operation(""))
    }
}
fn merge_sorted_values<I>(
    subs: Vec<Sub<SortedDocValuesSub<I>>>,
    index_is_sorted: bool,
    map: Rc<OrdinalMap>,
) -> Result<SortedDocValuesMerge<I>>
where
    I: IndexInput,
{
    let mut cost = 0;
    for sub in &subs {
        cost += sub.sub.values.cost()?;
    }
    let final_cost = cost;

    let doc_id_merger = of(subs, index_is_sorted)?;
    Ok(SortedDocValuesMerge {
        doc_id: -1,
        current: None,
        doc_id_merger,
        final_cost,
        map,
    })
}
// 4. SortedSetDocValues
struct SortedSetDocValuesSub<I>
where
    I: IndexInput,
{
    values: Lucene90SortedSetDocValuesEnum<I>,
    map: Rc<SegmentToGlobalOrds>,
    doc_map: Rc<DocMapEnum>,
}

impl<I> SortedSetDocValuesSub<I>
where
    I: IndexInput,
{
    fn new(
        doc_map: Rc<DocMapEnum>,
        values: Lucene90SortedSetDocValuesEnum<I>,
        map: Rc<SegmentToGlobalOrds>,
    ) -> Self {
        debug_assert!(values.doc_id() == -1);
        Self {
            values,
            map,
            doc_map,
        }
    }
}

impl<I> SubBase for SortedSetDocValuesSub<I>
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
pub struct SortedSetDocValuesMerge<I>
where
    I: IndexInput,
{
    doc_id: i32,
    current_sub: Option<usize>,
    doc_id_merger: DocIDMergerEnum<SortedSetDocValuesSub<I>>,
    final_cost: i64,
    map: Rc<OrdinalMap>,
    to_merge: Vec<Lucene90SortedSetDocValuesEnum<I>>,
}

impl<I> DocIdSetIterator for SortedSetDocValuesMerge<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.current_sub = self.doc_id_merger.next()?;
        match self.current_sub {
            Some(idx) => {
                let v = self.doc_id_merger.get_subs()[idx].mapped_doc_id;
                self.doc_id = v;
                Ok(v)
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
        Ok(self.final_cost)
    }
}

impl<I> DocValuesIterator for SortedSetDocValuesMerge<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl<I> SortedSetDocValues for SortedSetDocValuesMerge<I>
where
    I: IndexInput,
{
    fn next_ord(&mut self) -> Result<i64> {
        let current = *self.current_sub.as_ref().unwrap();
        let current_sub = &mut self.doc_id_merger.get_subs_mut()[current];
        let sub_ord = current_sub.sub.values.next_ord()?;
        current_sub.sub.map.get(sub_ord as usize)
    }

    fn lookup_ord(&mut self, ord: i64) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        let segment_number = self.map.get_first_segment_number(ord as usize)?;
        let segment_ord = self.map.get_first_segment_ord(ord as usize)?;
        self.to_merge[segment_number as usize].lookup_ord(segment_ord)
    }

    fn get_value_count(&self) -> Result<i64> {
        Ok(self.map.get_value_count())
    }

    fn terms_enum(&mut self) -> Result<Self::TermsEnumRef<'_>> {
        let mut subs = Vec::with_capacity(self.to_merge.len());
        for dv in &mut self.to_merge {
            subs.push(dv.terms_enum()?);
        }
        todo!()
        // Ok(MergedTermsEnum::new(self.map.clone(), subs))
    }

    fn take_terms_enum(self) -> Result<Self::TermsEnum> {
        let mut subs = Vec::with_capacity(self.to_merge.len());
        for dv in self.to_merge {
            subs.push(dv.take_terms_enum()?);
        }
        todo!()
        // Ok(MergedTermsEnum::new(self.map.clone(), subs))
    }

    type TermsEnum = DummyTermsEnum;
    type SortedDocValues = DummySortedDocValues;

    fn doc_value_count(&mut self) -> Result<i32> {
        let current = *self.current_sub.as_ref().unwrap();
        self.doc_id_merger.get_subs_mut()[current]
            .sub
            .values
            .doc_value_count()
    }

    type TermsEnumRef<'a>
        = DummyTermsEnum
    where
        Self: 'a;
}
