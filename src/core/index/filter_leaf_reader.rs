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
use crate::core::index::filtered_terms_enum::{FilteredTermsEnum, FilteredTermsEnumBase};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;

pub trait FilterLeafReader {}

/// # Note
/// See [`JavaIntermediateBaseClass`](crate::migration_notes::JavaIntermediateBaseClass)
#[allow(dead_code)]
pub struct FilterFields<F>
where
    F: Fields,
{
    /// The underlying Fields instance.
    pub(crate) in_: F,
}
impl<F> FilterFields<F>
where
    F: Fields,
{
    pub fn new(inner: F) -> FilterFields<F> {
        Self { in_: inner }
    }
}
impl<F> Fields for FilterFields<F>
where
    F: Fields,
{
    type FieldIter<'a>
        = F::FieldIter<'a>
    where
        F: 'a;

    fn iterator(&self) -> Result<Self::FieldIter<'_>> {
        self.in_.iterator()
    }

    type Terms = F::Terms;

    fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
        self.in_.terms(field)
    }

    fn size(&self) -> Result<i32> {
        self.in_.size()
    }
}

/// # Note
/// See [`JavaIntermediateBaseClass`](crate::migration_notes::JavaIntermediateBaseClass)
#[allow(dead_code)]
pub struct FilterTerms<T>
where
    T: Terms,
{
    /// The underlying `Terms` instance.
    pub(crate) in_: T,
}

impl<T> FilterTerms<T>
where
    T: Terms,
{
    pub fn new(inner: T) -> Self {
        Self { in_: inner }
    }
}
impl<T> Terms for FilterTerms<T>
where
    T: Terms,
{
    type TermsEnum = T::TermsEnum;

    fn iterator(&self) -> Result<Self::TermsEnum> {
        self.in_.iterator()
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
        self.in_.size()
    }

    fn get_sum_total_term_freq(&self) -> Result<i64> {
        self.in_.get_sum_total_term_freq()
    }

    fn get_sum_doc_freq(&self) -> Result<i64> {
        self.in_.get_sum_doc_freq()
    }

    fn get_doc_count(&self) -> Result<i32> {
        self.in_.get_doc_count()
    }

    fn has_freqs(&self) -> bool {
        self.in_.has_freqs()
    }

    fn has_offsets(&self) -> bool {
        self.in_.has_offsets()
    }

    fn has_positions(&self) -> bool {
        self.in_.has_positions()
    }

    fn has_payloads(&self) -> bool {
        self.in_.has_payloads()
    }

    fn get_stats(&self) -> Result<String> {
        self.in_.get_stats()
    }
}

/// # Note
/// See [`JavaIntermediateBaseClass`](crate::migration_notes::JavaIntermediateBaseClass)
#[allow(dead_code)]
pub struct FilterTermsEnum<T>
where
    T: TermsEnum,
{
    pub(crate) in_: T,
}
impl<T> FilterTermsEnum<T>
where
    T: TermsEnum,
{
    pub fn new(terms_enum: T) -> Self {
        Self { in_: terms_enum }
    }
}

impl<T> BytesRefIterator for FilterTermsEnum<T>
where
    T: TermsEnum,
{
    fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        self.in_.next()
    }
}

impl<T> TermsEnum for FilterTermsEnum<T>
where
    T: TermsEnum,
{
    type AttributeSource = T::AttributeSource;

    fn attributes(&self) -> Result<Self::AttributeSource> {
        self.in_.attributes()
    }

    fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
        self.in_.seek_exact(term)
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
        self.in_.doc_freq()
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        self.in_.total_term_freq()
    }

    type PostingsEnum = T::PostingsEnum;

    fn postings_with_flags(
        &mut self,
        reuse: Option<Self::PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnum> {
        self.in_.postings_with_flags(reuse, flags)
    }

    type ImpactsEnum = T::ImpactsEnum;

    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
        self.in_.impacts(flags)
    }

    type TermState = T::TermState;

    fn term_state(&mut self) -> Result<Self::TermState> {
        self.in_.term_state()
    }
}
