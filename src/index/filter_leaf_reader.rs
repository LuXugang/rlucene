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
use crate::index::automaton_terms_enum::AutomatonTermsEnum;
use crate::index::fields::Fields;
use crate::index::filtered_terms_enum::{FilteredTermsEnum, FilteredTermsEnumBase};
use crate::index::term_state::TermStateEnum;
use crate::index::terms::Terms;
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::util::attribute_source::AttributeSource;
use crate::util::automation::compiled_automaton::CompiledAutomaton;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::Result;
use std::borrow::Cow;

pub trait FilterLeafReader {}
/// Base class for filtering [`Fields`] implementations.
pub struct FilterFields<F>
where
    F: Fields,
{
    /// The underlying Fields instance.
    inner: F,
}
impl<F> FilterFields<F>
where
    F: Fields,
{
    pub fn new(inner: F) -> FilterFields<F> {
        Self { inner }
    }
}
impl<F> Fields for FilterFields<F>
where
    F: Fields,
{
    fn iterator(&self) -> impl Iterator<Item = &String> {
        self.inner.iterator()
    }

    type Terms = F::Terms;

    fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
        self.inner.terms(field)
    }

    fn size(&self) -> Result<i32> {
        self.inner.size()
    }
}

/// Base class for filtering [`Terms`] implementations.
///
/// **NOTE**: If the order of terms and documents is not changed, and if these terms are
/// going to be intersected with automata, you could consider overriding [`Self::intersect`](Terms::intersect) for
/// better performance.
pub struct FilterTerms<T>
where
    T: Terms,
{
    /// The underlying `Terms` instance.
    pub(crate) inner: T,
}

impl<T> FilterTerms<T>
where
    T: Terms,
{
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}
impl<T> Terms for FilterTerms<T>
where
    T: Terms,
{
    type TermsEnum = T::TermsEnum;

    fn iterator(&self) -> Result<Self::TermsEnum> {
        self.inner.iterator()
    }

    type IntersectIter
        = FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>
    where
        Self::TermsEnum: BytesRefIterator,
        AutomatonTermsEnum: FilteredTermsEnumBase;

    fn intersect(
        &self,
        compiled: &mut CompiledAutomaton,
        start_term: Option<BytesRef<Vec<u8>>>,
    ) -> Result<Self::IntersectIter> {
        self.default_intersect(compiled, start_term)
    }

    fn size(&self) -> Result<i64> {
        self.inner.size()
    }

    fn get_sum_total_term_freq(&self) -> Result<i64> {
        self.inner.get_sum_total_term_freq()
    }

    fn get_sum_doc_freq(&self) -> Result<i64> {
        self.inner.get_sum_doc_freq()
    }

    fn get_doc_count(&self) -> Result<i32> {
        self.inner.get_doc_count()
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

    fn get_stats(&self) -> Result<String> {
        self.inner.get_stats()
    }

    fn type_name(&self) -> &'static str {
        "FilterTerms"
    }
}

/// Base class for filtering `TermsEnum` implementations.
pub struct FilterTermsEnum<T>
where
    T: TermsEnum,
{
    terms_enum: T,
}
impl<T> FilterTermsEnum<T>
where
    T: TermsEnum,
{
    pub fn new(terms_enum: T) -> Self {
        Self { terms_enum }
    }
}

impl<T> BytesRefIterator for FilterTermsEnum<T>
where
    T: TermsEnum,
{
    fn next(&mut self) -> Result<Option<Cow<BytesRef<Vec<u8>>>>> {
        self.terms_enum.next()
    }
}

impl<T> TermsEnum for FilterTermsEnum<T>
where
    T: TermsEnum,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        self.terms_enum.attributes()
    }

    fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
        self.terms_enum.seek_exact(term)
    }

    fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        self.terms_enum.seek_ceil(term)
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        self.terms_enum.seek_exact_with_ord(ord)
    }

    fn seek_exact_with_state(
        &mut self,
        term: &BytesRef<Vec<u8>>,
        state: &TermStateEnum,
    ) -> Result<()> {
        self.terms_enum.seek_exact_with_state(term, state)
    }

    fn term(&self) -> Result<Cow<BytesRef<Vec<u8>>>> {
        self.terms_enum.term()
    }

    fn ord(&self) -> Result<i64> {
        self.terms_enum.ord()
    }

    fn doc_freq(&mut self) -> Result<i32> {
        self.terms_enum.doc_freq()
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        self.terms_enum.total_term_freq()
    }

    type PostingsEnum = T::PostingsEnum;

    fn postings_with_flags(
        &mut self,
        reuse: Option<Self::PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnum> {
        self.terms_enum.postings_with_flags(reuse, flags)
    }

    type ImpactsEnum = T::ImpactsEnum;

    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
        self.terms_enum.impacts(flags)
    }

    type TermState = T::TermState;

    fn term_state(&mut self) -> Result<Self::TermState> {
        self.terms_enum.term_state()
    }
}
