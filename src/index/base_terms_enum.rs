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
use std::fmt::{Debug, Display, Formatter};

use crate::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::index::term_state::{TermState, TermStateEnum};
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};

/// A base `TermsEnum` that provides default implementations for:
///
/// - [`attributes()`](BaseTermsEnum::attributes)
/// - [`term_state()`](BaseTermsEnum::term_state)
/// - [`seek_exact(&BytesRef)`](BaseTermsEnum::seek_exact)
/// - [`seek_exact_with_state(&BytesRef,
///   &TermState)`](BaseTermsEnum::seek_exact_with_state)
///
/// In some cases, the default implementation may be slow and consume large
/// amounts of memory, so subclasses SHOULD provide their own implementation if
/// possible.
pub struct BaseTermsEnum<S>
where
    S: TermsEnum,
{
    atts: AttributeSource,
    sub: S,
}
impl<S> BaseTermsEnum<S>
where
    S: TermsEnum,
{
    pub fn new(sub: S) -> Self {
        Self {
            atts: AttributeSource::new(),
            sub,
        }
    }
}

impl<S> BytesRefIterator for BaseTermsEnum<S>
where
    S: TermsEnum,
{
    type AV = S::AV;
}

impl<S> TermsEnum for BaseTermsEnum<S>
where
    S: TermsEnum,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        Ok(&self.atts)
    }

    fn seek_exact(&mut self, term: &BytesRef<Self::AV>) -> Result<bool> {
        Ok(self.seek_ceil(term)? == SeekStatus::Found)
    }

    fn prepare_seek_exact(&mut self, text: &BytesRef<Self::AV>) -> Result<bool> {
        self.seek_exact(text)
    }

    fn seek_ceil(&mut self, term: &BytesRef<Self::AV>) -> Result<SeekStatus> {
        self.sub.seek_ceil(term)
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        self.sub.seek_exact_with_ord(ord)
    }

    fn seek_exact_with_state(
        &mut self,
        term: &BytesRef<Self::AV>,
        _state: &TermStateEnum,
    ) -> Result<()> {
        if !self.seek_exact(term)? {
            return Err(LuceneError::illegal_argument(format!(
                "term= {} does not exist",
                term
            )));
        };
        Ok(())
    }

    fn term(&self) -> Result<Cow<BytesRef<Self::AV>>> {
        self.sub.term()
    }

    fn ord(&self) -> Result<i64> {
        self.sub.ord()
    }

    fn doc_freq(&mut self) -> Result<i32> {
        self.sub.doc_freq()
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        self.sub.total_term_freq()
    }

    type PostingsEnum = DummyPostingsEnum;

    fn postings(&mut self, reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
        todo!()
    }

    fn postings_with_flags(
        &mut self,
        _reuse: Option<Self::PostingsEnum>,
        _flags: i32,
    ) -> Result<Self::PostingsEnum> {
        Err(LuceneError::need_implemented(""))
    }

    type ImpactsEnum = DummyImpactsEnum;

    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
        Err(LuceneError::need_implemented(""))
    }

    type TermState = TermStateImpl1;

    fn term_state(&mut self) -> Result<Self::TermState> {
        Ok(TermStateImpl1)
    }
}
#[derive(Debug, Clone)]
pub struct TermStateImpl1;
impl Display for TermStateImpl1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BaseTermsEnum#TermState",)
    }
}
impl TermState for TermStateImpl1 {
    fn copy_from(&mut self, _other: &TermStateEnum) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }
}
