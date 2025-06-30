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
