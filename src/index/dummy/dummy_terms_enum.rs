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
use crate::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::index::term_state::TermStateEnum;
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::Result;
use std::borrow::Cow;

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
impl BytesRefIterator for DummyTermsEnum {
    fn next(&mut self) -> Result<Option<Cow<BytesRef<Vec<u8>>>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl TermsEnum for DummyTermsEnum {
    fn attributes(&self) -> Result<&AttributeSource> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn seek_ceil(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn seek_exact_with_state(
        &mut self,
        _term: &BytesRef<Vec<u8>>,
        _state: &TermStateEnum,
    ) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn term(&self) -> Result<Cow<BytesRef<Vec<u8>>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn ord(&self) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn doc_freq(&mut self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type PostingsEnum = DummyPostingsEnum;

    fn postings_with_flags(
        &mut self,
        _reuse: Option<Self::PostingsEnum>,
        _flags: i32,
    ) -> Result<Self::PostingsEnum> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type ImpactsEnum = DummyImpactsEnum;

    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type TermState = TermStateEnum;

    fn term_state(&mut self) -> Result<Self::TermState> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
