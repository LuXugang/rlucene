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
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::fields::Fields;
use crate::core::index::filter_leaf_reader::{FilterFields, FilterTermsEnum};
use crate::core::index::filtered_terms_enum::{FilteredTermsEnum, FilteredTermsEnumBase};
use crate::core::index::mapping_multi_postings_enum::MappingMultiPostingsEnum;
use crate::core::index::merge_state::{MergeState, MergeStateMeta};
use crate::core::index::multi_fields::{MultiFields, MultiFieldsTerms};
use crate::core::index::multi_terms::IteratorType;
use crate::core::index::multi_terms_enum::{MultiTermsEnum, MultiTermsEnumType};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::{EmptyTermsEnum, SeekStatus, TermsEnum, TermsEnumEnum2};
use crate::core::store::directory::Directory;
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;

/// A [`Fields`] implementation that merges multiple `Fields` into one,
/// while accounting for deleted documents.
///
/// This implementation is used during index merging.
pub struct MappedMultiFields<'a, F, CR>
where
    F: Fields,
    CR: CodecReader,
{
    merge_state_meta: MergeStateMeta<CR>,
    base: FilterFields<&'a MultiFields<F>>,
}

impl<'a, F, CR> MappedMultiFields<'a, F, CR>
where
    F: Fields,
    CR: CodecReader,
{
    pub fn new<D>(merge_state: &MergeState<D, CR>, multi_fields: &'a MultiFields<F>) -> Self
    where
        D: Directory,
        CR: CodecReader,
    {
        let merge_state_meta = merge_state.get_meta();
        let base = FilterFields::new(multi_fields);
        MappedMultiFields {
            merge_state_meta,
            base,
        }
    }
}
impl<F, CR> Fields for MappedMultiFields<'_, F, CR>
where
    F: Fields,
    CR: CodecReader,
{
    type FieldIter<'a>
        = <FilterFields<MultiFields<F>> as Fields>::FieldIter<'a>
    where
        Self: 'a;

    fn iterator(&self) -> Result<Self::FieldIter<'_>> {
        self.base.iterator()
    }

    type Terms = MappedMultiTerms<<F as Fields>::Terms, CR>;

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

pub struct MappedMultiTerms<T, CR>
where
    T: Terms,
    CR: CodecReader,
{
    merge_state: MergeStateMeta<CR>,
    field: String,
    inner: MultiFieldsTerms<T>,
}
impl<T, CR> MappedMultiTerms<T, CR>
where
    T: Terms,
    CR: CodecReader,
{
    pub fn new(
        field: String,
        merge_state: MergeStateMeta<CR>,
        multi_terms: MultiFieldsTerms<T>,
    ) -> Self {
        MappedMultiTerms {
            merge_state,
            field,
            inner: multi_terms,
        }
    }
}
pub type MappedMultiTermsTE<T, CR> =
    TermsEnumEnum2<EmptyTermsEnum, MappedMultiTermsEnum<<T as Terms>::TermsEnum, CR>>;
impl<T, CR> Terms for MappedMultiTerms<T, CR>
where
    T: Terms,
    CR: CodecReader,
{
    type TermsEnum = MappedMultiTermsTE<T, CR>;

    fn iterator(&self) -> Result<Self::TermsEnum> {
        let iterator = self.inner.iterator()?;
        match iterator {
            IteratorType::<T>::B(empty) => Ok(MappedMultiTermsTE::<T, CR>::A(empty)),
            IteratorType::<T>::A(v) => match v {
                MultiTermsEnumType::A(v) => {
                    let v =
                        MappedMultiTermsEnum::new(self.field.clone(), self.merge_state.clone(), v);
                    Ok(MappedMultiTermsTE::<T, CR>::B(v))
                },
                MultiTermsEnumType::B(empty) => Ok(MappedMultiTermsTE::<T, CR>::A(empty)),
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
        self.inner.has_freqs()
    }

    fn has_offsets(&self) -> bool {
        self.inner.has_offsets()
    }

    fn has_positions(&self) -> bool {
        self.inner.has_positions()
    }

    fn has_payloads(&self) -> bool {
        self.inner.has_payloads()
    }

    fn get_min(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        self.inner.get_min()
    }

    fn get_max(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        self.inner.get_max()
    }

    fn get_stats(&self) -> Result<String> {
        self.inner.get_stats()
    }
}

pub struct MappedMultiTermsEnum<TE, CR>
where
    TE: TermsEnum,
    CR: CodecReader,
{
    field: String,
    merge_state_meta: MergeStateMeta<CR>,
    in_: MultiTermsEnum<TE>,
}
impl<TE, CR> MappedMultiTermsEnum<TE, CR>
where
    TE: TermsEnum,
    CR: CodecReader,
{
    pub fn new(
        field: String,
        merge_state: MergeStateMeta<CR>,
        multi_terms_enum: MultiTermsEnum<TE>,
    ) -> Self {
        Self {
            field,
            merge_state_meta: merge_state,
            in_: multi_terms_enum,
        }
    }
}

impl<TE, CR> BytesRefIterator for MappedMultiTermsEnum<TE, CR>
where
    TE: TermsEnum,
    CR: CodecReader,
{
    fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        self.in_.next()
    }
}

impl<TE, CR> TermsEnum for MappedMultiTermsEnum<TE, CR>
where
    TE: TermsEnum,
    CR: CodecReader,
{
    type AttributeSource = <FilterTermsEnum<MultiTermsEnum<TE>> as TermsEnum>::AttributeSource;

    fn attributes(&self) -> Result<Self::AttributeSource> {
        self.in_.attributes()
    }

    fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
        self.in_.seek_exact(term)
    }

    fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
        self.in_.prepare_seek_exact(text)
    }

    fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
        self.in_.get_prepare_seek_exact_status(target)
    }

    fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        self.in_.seek_ceil(term)
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        self.in_.seek_exact_with_ord(ord)
    }

    fn seek_exact_with_state(
        &mut self,
        term: &BytesRef<Vec<u8>>,
        state: &Self::TermState,
    ) -> Result<()> {
        self.in_.seek_exact_with_state(term, state)
    }

    fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        self.in_.term()
    }

    fn ord(&self) -> Result<i64> {
        self.in_.ord()
    }

    fn doc_freq(&mut self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }

    type PostingsEnum = MappingMultiPostingsEnum<<TE as TermsEnum>::PostingsEnum, CR>;

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
        let docs_and_positions_enum = self.in_.postings_with_flags(v, flags)?;
        mapping_docs_and_positions_enum.reset(docs_and_positions_enum)?;
        Ok(mapping_docs_and_positions_enum)
    }

    type ImpactsEnum = <FilterTermsEnum<MultiTermsEnum<TE>> as TermsEnum>::ImpactsEnum;

    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
        self.in_.impacts(flags)
    }

    type TermState = <FilterTermsEnum<MultiTermsEnum<TE>> as TermsEnum>::TermState;

    fn term_state(&mut self) -> Result<Self::TermState> {
        self.in_.term_state()
    }
}
