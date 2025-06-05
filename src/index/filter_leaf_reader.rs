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
use crate::util::either_enums::{EitherImpactsEnum, EitherPostingsEnum};
use crate::util::error::lucene_error::{LuceneError, Result};

pub trait FilterLeafReader {}

pub struct FilterTermsEnum<T, S>
where
    T: TermsEnum<AV = Vec<u8>>,
    S: TermsEnum<AV = Vec<u8>>,
{
    terms_enum: T,
    sub: S,
}
impl<T, S> FilterTermsEnum<T, S>
where
    T: TermsEnum<AV = Vec<u8>>,
    S: TermsEnum<AV = Vec<u8>>,
{
    fn new(terms_enum: T, sub: S) -> Self {
        Self { terms_enum, sub }
    }
}

impl<T, S> BytesRefIterator for FilterTermsEnum<T, S>
where
    S: TermsEnum<AV = Vec<u8>>,
    T: TermsEnum<AV = Vec<u8>>,
{
    type AV = Vec<u8>;

    fn next(&mut self) -> Result<Option<Cow<BytesRef<Self::AV>>>> {
        match self.sub.next() {
            Ok(v) => Ok(v),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => self.terms_enum.next(),
                _ => Err(e),
            },
        }
    }
}

impl<T, S> TermsEnum for FilterTermsEnum<T, S>
where
    T: TermsEnum<AV = S::AV>,
    S: TermsEnum<AV = Vec<u8>>,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        self.terms_enum.attributes()
    }

    fn seek_exact(&mut self, term: &BytesRef<Self::AV>) -> Result<bool> {
        match self.sub.seek_exact(term) {
            Ok(v) => Ok(v),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => self.terms_enum.seek_exact(term),
                _ => Err(e),
            },
        }
    }

    fn seek_ceil(&mut self, term: &BytesRef<Self::AV>) -> Result<SeekStatus> {
        match self.sub.seek_ceil(term) {
            Ok(v) => Ok(v),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => self.terms_enum.seek_ceil(term),
                _ => Err(e),
            },
        }
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        match self.sub.seek_exact_with_ord(ord) {
            Ok(_) => Ok(()),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => self.terms_enum.seek_exact_with_ord(ord),
                _ => Err(e),
            },
        }
    }

    fn seek_exact_with_state(
        &mut self,
        term: &BytesRef<Self::AV>,
        state: &TermStateEnum,
    ) -> Result<()> {
        todo!()
    }

    fn term(&self) -> Result<Cow<BytesRef<Self::AV>>> {
        match self.sub.term() {
            Ok(v) => Ok(v),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => self.terms_enum.term(),
                _ => Err(e),
            },
        }
    }

    fn ord(&self) -> Result<i64> {
        match self.sub.ord() {
            Ok(v) => Ok(v),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => self.terms_enum.ord(),
                _ => Err(e),
            },
        }
    }

    fn doc_freq(&mut self) -> Result<i32> {
        match self.sub.doc_freq() {
            Ok(v) => Ok(v),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => self.terms_enum.doc_freq(),
                _ => Err(e),
            },
        }
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        match self.sub.total_term_freq() {
            Ok(v) => Ok(v),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => self.terms_enum.total_term_freq(),
                _ => Err(e),
            },
        }
    }

    type PostingsEnum = EitherPostingsEnum<T::PostingsEnum, S::PostingsEnum>;
    type PostingsEnumRet = EitherPostingsEnum<T::PostingsEnumRet, S::PostingsEnumRet>;

    fn postings_with_flags(
        &mut self,
        reuse: Option<Self::PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnumRet> {
        match reuse {
            Some(EitherPostingsEnum::S(s)) => Ok(EitherPostingsEnum::S(
                self.sub.postings_with_flags(Some(s), flags)?,
            )),
            Some(EitherPostingsEnum::T(t)) => Ok(EitherPostingsEnum::T(
                self.terms_enum.postings_with_flags(Some(t), flags)?,
            )),
            None => {
                return match self.sub.postings_with_flags(None, flags) {
                    Ok(v) => Ok(EitherPostingsEnum::S(v)),
                    Err(e) => match e {
                        LuceneError::NotImplemented(_) => {
                            let postings = self.terms_enum.postings_with_flags(None, flags)?;
                            Ok(EitherPostingsEnum::T(postings))
                        },
                        _ => Err(e),
                    },
                };
            },
        }
    }

    type ImpactsEnum = EitherImpactsEnum<T::ImpactsEnum, S::ImpactsEnum>;

    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
        todo!()
    }

    type TermState = T::TermState;

    fn term_state(&mut self) -> Result<Self::TermState> {
        todo!()
    }
}
