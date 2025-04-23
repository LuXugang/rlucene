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

use crate::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::index::postings_enum::PostingsEnum;
use crate::index::term_state::TermStateEnum;
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::util::access::AccessVec;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};

pub struct DummyTermsEnum {
    atts: AttributeSource,
}
impl Default for DummyTermsEnum {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyTermsEnum {
    pub fn new() -> Self {
        Self {
            atts: AttributeSource::new(),
        }
    }
}
impl<AV> BytesRefIterator<AV> for DummyTermsEnum
where
    AV: AccessVec<u8>,
{
    fn next(&mut self) -> Result<Option<Cow<BytesRef<AV>>>> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }
}

impl<AV> TermsEnum<AV> for DummyTermsEnum
where
    AV: AccessVec<u8>,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        debug_assert!(false, "should never be called");
        Ok(&self.atts)
    }

    fn seek_ceil(&mut self, term: &BytesRef<AV>) -> Result<SeekStatus> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn seek_exact_with_state(&mut self, term: &BytesRef<AV>, state: &TermStateEnum) -> Result<()> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn term(&self) -> Result<Cow<BytesRef<AV>>> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn ord(&self) -> Result<i64> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn doc_freq(&self) -> Result<i32> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn total_term_freq(&self) -> Result<i64> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    type PostingsEnum = DummyPostingsEnum;

    fn postings_with_flags(
        &mut self,
        _reuse: Option<impl PostingsEnum>,
        _flags: i32,
    ) -> Result<Self::PostingsEnum> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    type ImpactsEnum = DummyImpactsEnum;

    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    type TermState = TermStateEnum;

    fn term_state(&self) -> Result<Self::TermState> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }
}
