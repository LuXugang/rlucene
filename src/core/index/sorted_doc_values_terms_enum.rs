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
use crate::core::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::core::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::dummy::dummy_attribute_source::DummyAttributeSource;

use crate::core::index::ord_term_state::OrdTermState;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// Implements a [`TermsEnum`](TermsEnum) wrapping a provided
/// [`SortedDocValues`](SortedDocValues).
use std::borrow::Cow;

/// Implements a [`TermsEnum`] wrapping a provided [`SortedDocValues`].
pub struct SortedDocValuesTermsEnum<'a, S>
where
    S: SortedDocValues,
{
    values: &'a mut S,
    current_ord: i32,
    scratch: BytesRefBuilder<Vec<u8>>,
}

impl<'a, S> SortedDocValuesTermsEnum<'a, S>
where
    S: SortedDocValues,
{
    /// Creates a new TermsEnum over the provided values.
    pub fn new(values: &'a mut S) -> SortedDocValuesTermsEnum<'a, S> {
        SortedDocValuesTermsEnum {
            values,
            current_ord: -1,
            scratch: BytesRefBuilder::new(),
        }
    }
}

impl<'a, S> BytesRefIterator for SortedDocValuesTermsEnum<'a, S>
where
    S: SortedDocValues,
{
    fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        self.current_ord += 1;
        if self.current_ord >= self.values.get_value_count()? {
            return Ok(None);
        }
        let term = self.values.lookup_ord(self.current_ord)?;
        self.scratch.copy_bytes_with_ref(term.as_ref());
        Ok(Some(Cow::Borrowed(self.scratch.get_bytes_ref())))
    }
}

impl<'a, S> TermsEnum for SortedDocValuesTermsEnum<'a, S>
where
    S: SortedDocValues,
{
    type AttributeSource = DummyAttributeSource;

    fn attributes(&self) -> Result<Self::AttributeSource> {
        Err(LuceneError::not_implemented(""))
    }

    fn seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<bool> {
        let ord = self.values.lookup_term(text)?;
        if ord >= 0 {
            self.current_ord = ord;
            self.scratch.copy_bytes_with_ref(text);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn prepare_seek_exact(&mut self, _text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
        Err(LuceneError::not_implemented(""))
    }

    fn get_prepare_seek_exact_status(&mut self, _target: &BytesRef<Vec<u8>>) -> Result<bool> {
        Err(LuceneError::not_implemented(""))
    }

    fn seek_ceil(&mut self, text: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        let ord = self.values.lookup_term(text)?;
        if ord >= 0 {
            self.current_ord = ord;
            self.scratch.copy_bytes_with_ref(text);
            Ok(SeekStatus::Found)
        } else {
            self.current_ord = -ord - 1;
            if self.current_ord == self.values.get_value_count()? {
                Ok(SeekStatus::End)
            } else {
                let next_term = self.values.lookup_ord(self.current_ord)?;
                self.scratch.copy_bytes_with_ref(next_term.as_ref());
                Ok(SeekStatus::NotFound)
            }
        }
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        debug_assert!(
            ord >= 0 && ord < self.values.get_value_count()? as i64,
            "ord out of range: {ord}"
        );
        self.current_ord = ord as i32;
        let term = self.values.lookup_ord(self.current_ord)?;
        self.scratch.copy_bytes_with_ref(term.as_ref());
        Ok(())
    }

    fn seek_exact_with_state(
        &mut self,
        _term: &BytesRef<Vec<u8>>,
        state: &Self::TermState,
    ) -> Result<()> {
        self.seek_exact_with_ord(state.ord)
    }

    fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        Ok(Cow::Borrowed(self.scratch.get_bytes_ref()))
    }

    fn ord(&self) -> Result<i64> {
        Ok(self.current_ord as i64)
    }

    fn doc_freq(&mut self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }

    type PostingsEnum = DummyPostingsEnum;

    fn postings(&mut self, _reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
        Err(LuceneError::unsupported_operation(""))
    }

    type ImpactsEnum = DummyImpactsEnum;

    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
        Err(LuceneError::unsupported_operation(""))
    }

    type TermState = OrdTermState;

    fn term_state(&mut self) -> Result<Self::TermState> {
        Ok(OrdTermState {
            ord: self.current_ord as i64,
            ..OrdTermState::new()
        })
    }
}
