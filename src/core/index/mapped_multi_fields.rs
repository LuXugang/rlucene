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
use crate::core::index::BytesRef;
use crate::core::index::automaton_terms_enum::AutomatonTermsEnum;
use crate::core::index::fields::Fields;
use crate::core::index::filter_leaf_reader::{FilterFields, FilterTerms, FilterTermsEnum};
use crate::core::index::filtered_terms_enum::{FilteredTermsEnum, FilteredTermsEnumBase};
use crate::core::index::mapping_multi_postings_enum::MappingMultiPostingsEnum;
use crate::core::index::merge_state::{MergeState, MergeStateMeta};
use crate::core::index::multi_fields::{MultiFields, MultiFieldsTerms};
use crate::core::index::multi_terms::IteratorType;
use crate::core::index::multi_terms_enum::{MultiTermsEnum, MultiTermsEnumType};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::{EmptyTermsEnum, SeekStatus, TermsEnum, TermsEnumEnum2};
use crate::core::store::IndexInput;
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;

/// A [`Fields`] implementation that merges multiple `Fields` into one,
/// while accounting for deleted documents.
///
/// This implementation is used during index merging.
pub struct MappedMultiFields<'a, F>
where
    F: Fields,
{
    merge_state_meta: MergeStateMeta,
    base: FilterFields<&'a MultiFields<F>>,
}

impl<'a, F> MappedMultiFields<'a, F>
where
    F: Fields,
{
    pub fn new<I>(merge_state: &MergeState<I>, multi_fields: &'a MultiFields<F>) -> Self
    where
        I: IndexInput,
    {
        let merge_state_meta = merge_state.get_meta();
        let base = FilterFields::new(multi_fields);
        MappedMultiFields {
            merge_state_meta,
            base,
        }
    }
}
impl<F> Fields for MappedMultiFields<'_, F>
where
    F: Fields,
{
    type FieldIter<'a>
        = <FilterFields<MultiFields<F>> as Fields>::FieldIter<'a>
    where
        Self: 'a;

    fn iterator(&self) -> Result<Self::FieldIter<'_>> {
        self.base.iterator()
    }

    type Terms = MappedMultiTerms<<F as Fields>::Terms>;

    fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
        let terms = self.base.in_.terms(field)?;
        match terms {
            Some(v) => Ok(Some(MappedMultiTerms::new(
                field.to_string(),
                self.merge_state_meta.clone(),
                v,
            ))),
            None => Ok(None),
        }
    }

    fn size(&self) -> Result<i32> {
        self.base.size()
    }
}

pub struct MappedMultiTerms<T>
where
    T: Terms,
{
    merge_state: MergeStateMeta,
    field: String,
    base: FilterTerms<MultiFieldsTerms<T>>,
}
impl<T> MappedMultiTerms<T>
where
    T: Terms,
{
    pub fn new(
        field: String,
        merge_state: MergeStateMeta,
        multi_terms: MultiFieldsTerms<T>,
    ) -> Self {
        let base = FilterTerms::new(multi_terms);
        MappedMultiTerms {
            merge_state,
            field,
            base,
        }
    }
}
pub type MappedMultiTermsTE<T> =
    TermsEnumEnum2<EmptyTermsEnum, MappedMultiTermsEnum<<T as Terms>::TermsEnum>>;
impl<T> Terms for MappedMultiTerms<T>
where
    T: Terms,
{
    type TermsEnum = MappedMultiTermsTE<T>;

    fn iterator(&self) -> Result<Self::TermsEnum> {
        let iterator = self.base.in_.iterator()?;
        match iterator {
            IteratorType::<T>::B(empty) => Ok(MappedMultiTermsTE::<T>::A(empty)),
            IteratorType::<T>::A(v) => match v {
                MultiTermsEnumType::A(v) => {
                    let v =
                        MappedMultiTermsEnum::new(self.field.clone(), self.merge_state.clone(), v);
                    Ok(MappedMultiTermsTE::<T>::B(v))
                },
                MultiTermsEnumType::B(empty) => Ok(MappedMultiTermsTE::<T>::A(empty)),
            },
        }
    }

    type IntersectIter
        = FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>
    where
        Self::TermsEnum: BytesRefIterator,
        AutomatonTermsEnum: FilteredTermsEnumBase;

    fn intersect(
        &self,
        compiled: &mut CompiledAutomaton,
        start_term: Option<&BytesRef<Vec<u8>>>,
    ) -> Result<Self::IntersectIter> {
        self.default_intersect(compiled, start_term)
    }

    fn size(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn get_sum_total_term_freq(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn get_sum_doc_freq(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn get_doc_count(&self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn has_freqs(&self) -> bool {
        self.base.has_freqs()
    }

    fn has_offsets(&self) -> bool {
        self.base.has_offsets()
    }

    fn has_positions(&self) -> bool {
        self.base.has_positions()
    }

    fn has_payloads(&self) -> bool {
        self.base.has_payloads()
    }

    fn get_min(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        self.base.get_min()
    }

    fn get_max(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        self.base.get_max()
    }

    fn get_stats(&self) -> Result<String> {
        self.base.get_stats()
    }
}

pub struct MappedMultiTermsEnum<TE>
where
    TE: TermsEnum,
{
    field: String,
    merge_state_meta: MergeStateMeta,
    base: FilterTermsEnum<MultiTermsEnum<TE>>,
}
impl<TE> MappedMultiTermsEnum<TE>
where
    TE: TermsEnum,
{
    pub fn new(
        field: String,
        merge_state: MergeStateMeta,
        multi_terms_enum: MultiTermsEnum<TE>,
    ) -> Self {
        let base = FilterTermsEnum::new(multi_terms_enum);
        Self {
            field,
            merge_state_meta: merge_state,
            base,
        }
    }
}

impl<TE> BytesRefIterator for MappedMultiTermsEnum<TE> where TE: TermsEnum {}

impl<TE> TermsEnum for MappedMultiTermsEnum<TE>
where
    TE: TermsEnum,
{
    type AttributeSource = <FilterTermsEnum<MultiTermsEnum<TE>> as TermsEnum>::AttributeSource;

    fn attributes(&self) -> Result<Self::AttributeSource> {
        self.base.attributes()
    }

    fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
        self.base.seek_exact(term)
    }

    fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
        self.base.prepare_seek_exact(text)
    }

    fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
        self.base.get_prepare_seek_exact_status(target)
    }

    fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        self.base.seek_ceil(term)
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        self.base.seek_exact_with_ord(ord)
    }

    fn seek_exact_with_state(
        &mut self,
        term: &BytesRef<Vec<u8>>,
        state: &Self::TermState,
    ) -> Result<()> {
        self.base.seek_exact_with_state(term, state)
    }

    fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        self.base.term()
    }

    fn ord(&self) -> Result<i64> {
        self.base.ord()
    }

    fn doc_freq(&mut self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }

    type PostingsEnum = MappingMultiPostingsEnum<<TE as TermsEnum>::PostingsEnum>;

    fn postings_with_flags(
        &mut self,
        reuse: Option<Self::PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnum> {
        let mut mapping_docs_and_positions_enum = match reuse {
            Some(postings) => {
                if postings.field == self.field {
                    postings
                } else {
                    MappingMultiPostingsEnum::new(self.field.clone(), &self.merge_state_meta)?
                }
            },
            None => MappingMultiPostingsEnum::new(self.field.clone(), &self.merge_state_meta)?,
        };
        let v = mapping_docs_and_positions_enum.take_multi_docs_and_positions_enum();
        let docs_and_positions_enum = self.base.in_.postings_with_flags(v, flags)?;
        mapping_docs_and_positions_enum.reset(docs_and_positions_enum)?;
        Ok(mapping_docs_and_positions_enum)
    }

    type ImpactsEnum = <FilterTermsEnum<MultiTermsEnum<TE>> as TermsEnum>::ImpactsEnum;

    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
        self.base.impacts(flags)
    }

    type TermState = <FilterTermsEnum<MultiTermsEnum<TE>> as TermsEnum>::TermState;

    fn term_state(&mut self) -> Result<Self::TermState> {
        self.base.term_state()
    }
}
