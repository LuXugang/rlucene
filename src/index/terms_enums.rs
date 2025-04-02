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
use crate::codecs::lucene90_doc_values_producer::TermsDict;
use crate::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::index::impacts_enum::ImpactsEnumEnum;
use crate::index::postings_enum::{PostingsEnum, PostingsEnums};
use crate::index::sorted_doc_values_terms_enum::SortedDocValuesTermsEnum;
use crate::index::term_state::TermStateEnum;
use crate::index::terms_enum::{SeekStatus, TermsEnum, TermsEnumEmpty};
use crate::index::BytesRef;
use crate::store::IndexInput;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::Result;
use crate::util::io_boolean_supplier::IOBooleanSupplierEnum;

pub enum TermsEnums<I>
where
    I: IndexInput,
{
    Dummy(DummyTermsEnum),
    Empty(TermsEnumEmpty<I>),
    SortedDocValues(SortedDocValuesTermsEnum<I>),
    TermsDict(TermsDict<I>),
}

impl<I> BytesRefIterator for TermsEnums<I>
where
    I: IndexInput,
{
    fn next(&mut self) -> Result<Option<BytesRef>> {
        todo!()
    }

    fn next_ref(&mut self) -> Result<Option<&BytesRef>> {
        todo!()
    }
}

impl<I> TermsEnum for TermsEnums<I>
where
    I: IndexInput,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        todo!()
    }

    fn seek_exact(&mut self, term: &BytesRef) -> Result<bool> {
        todo!()
    }

    fn prepare_seek_exact(
        &mut self,
        term: &BytesRef,
    ) -> Result<Option<Self::IOBooleanSupplierType>> {
        todo!()
    }

    type IOBooleanSupplierType = IOBooleanSupplierEnum<I>;

    fn seek_ceil(&mut self, term: &BytesRef) -> Result<SeekStatus> {
        todo!()
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        todo!()
    }

    fn seek_exact_with_state(&mut self, term: &BytesRef, state: &TermStateEnum) -> Result<()> {
        todo!()
    }

    fn term(&self) -> Result<BytesRef> {
        todo!()
    }

    fn term_ref(&self) -> Result<&BytesRef> {
        todo!()
    }

    fn ord(&self) -> Result<i64> {
        todo!()
    }

    fn doc_freq(&self) -> Result<i32> {
        todo!()
    }

    fn total_term_freq(&self) -> Result<i64> {
        todo!()
    }

    fn postings(&mut self, reuse: Option<impl PostingsEnum>) -> Result<Self::PostingsEnumType> {
        todo!()
    }

    fn postings_with_flags(
        &mut self,
        reuse: &Option<impl PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnumType> {
        todo!()
    }

    type PostingsEnumType = PostingsEnums;

    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnumType> {
        todo!()
    }

    type ImpactsEnumType = ImpactsEnumEnum;

    fn term_state(&self) -> Result<Self::TermStateType> {
        todo!()
    }

    type TermStateType = TermStateEnum;
}
