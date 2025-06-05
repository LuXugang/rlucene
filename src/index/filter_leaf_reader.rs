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
use std::borrow::Cow;

use crate::index::term_state::TermStateEnum;
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::Result;

pub trait FilterLeafReader {}
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
    fn new(terms_enum: T) -> Self {
        Self { terms_enum }
    }
}

impl<T> BytesRefIterator for FilterTermsEnum<T>
where
    T: TermsEnum,
{
    type AV = T::AV;

    fn next(&mut self) -> Result<Option<Cow<BytesRef<Self::AV>>>> {
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

    fn seek_exact(&mut self, term: &BytesRef<Self::AV>) -> Result<bool> {
        self.terms_enum.seek_exact(term)
    }

    fn seek_ceil(&mut self, term: &BytesRef<Self::AV>) -> Result<SeekStatus> {
        self.terms_enum.seek_ceil(term)
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        self.terms_enum.seek_exact_with_ord(ord)
    }

    fn seek_exact_with_state(
        &mut self,
        term: &BytesRef<Self::AV>,
        state: &TermStateEnum,
    ) -> Result<()> {
        self.terms_enum.seek_exact_with_state(term, state)
    }

    fn term(&self) -> Result<Cow<BytesRef<Self::AV>>> {
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
    type PostingsEnumRet = T::PostingsEnumRet;

    fn postings_with_flags(
        &mut self,
        reuse: Option<Self::PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnumRet> {
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
