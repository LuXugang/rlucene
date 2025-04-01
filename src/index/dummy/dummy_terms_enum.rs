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
use crate::index::dummy::dummy_io_boolean_supplier::DummyIOBooleanSupplier;
use crate::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::index::postings_enum::PostingsEnum;
use crate::index::term_state::{TermState, TermStateEnum};
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::util::access::Shared;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::rc::Rc;

pub struct DummyTermsEnum {
    atts: AttributeSource,
}
impl<S> BytesRefIterator<S> for DummyTermsEnum
where
    S: Shared<BytesRef>,
{
    fn next(&mut self) -> Result<Option<S>> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }
}

impl<S> TermsEnum<S> for DummyTermsEnum
where
    S: Shared<BytesRef>,
{
    fn attributes(&self) -> &AttributeSource {
        debug_assert!(false, "should never be called");
        &self.atts
    }

    fn prepare_seek_exact(
        &mut self,
        _term: Rc<BytesRef>,
    ) -> Result<Option<Self::IOBooleanSupplierType>> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    type IOBooleanSupplierType = DummyIOBooleanSupplier;

    fn seek_ceil(&mut self, _term: &BytesRef) -> Result<SeekStatus> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn seek_exact_by_ord(&mut self, _ord: i64) -> Result<()> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn seek_exact_with_state(&mut self, _term: &BytesRef, _state: &impl TermState) -> Result<()> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn term(&self) -> Result<BytesRef> {
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

    fn postings_with_flags(
        &mut self,
        _reuse: &Option<impl PostingsEnum>,
        _flags: i32,
    ) -> Result<Self::PostingsEnumType> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    type PostingsEnumType = DummyPostingsEnum;

    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnumType> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    type ImpactsEnumType = DummyImpactsEnum;

    fn term_state(&self) -> Result<Self::TermStateType> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    type TermStateType = TermStateEnum;
}
