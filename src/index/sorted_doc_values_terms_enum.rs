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
use crate::codecs::doc_values_enum::doc_values::SortedDocValuesEnum;
use crate::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::index::ord_term_state::OrdTermState;
use crate::index::postings_enum::PostingsEnum;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::term_state::TermStateEnum;
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::store::IndexInput;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;

/// Implements a [`TermsEnum`](TermsEnum) wrapping a provided [`SortedDocValues`](SortedDocValues).
pub struct SortedDocValuesTermsEnum<I>
where
    I: IndexInput,
{
    values: SortedDocValuesEnum<I>,
    current_ord: i32,
    bytes: BytesRef,
}

impl<I> SortedDocValuesTermsEnum<I>
where
    I: IndexInput,
{
    /// Creates a new TermsEnum over the provided values.
    pub fn new(values: SortedDocValuesEnum<I>) -> Self {
        Self {
            values,
            current_ord: -1,
            bytes: BytesRef::new(),
        }
    }
}

impl<I> BytesRefIterator for SortedDocValuesTermsEnum<I>
where
    I: IndexInput,
{
    fn next(&mut self) -> Result<Option<Cow<BytesRef>>> {
        self.current_ord += 1;
        if self.current_ord >= self.values.get_value_count()? {
            Ok(None)
        } else {
            self.bytes = self.values.lookup_ord(self.current_ord)?;
            Ok(Some(Cow::Borrowed(&self.bytes)))
        }
    }
}

impl<I> TermsEnum for SortedDocValuesTermsEnum<I>
where
    I: IndexInput,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        Err(LuceneError::not_implemented(""))
    }

    fn seek_exact(&mut self, text: &BytesRef) -> Result<bool> {
        let ord = self.values.lookup_term(text)?;
        if ord >= 0 {
            self.current_ord = ord;
            self.bytes = text.clone();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn seek_ceil(&mut self, text: &BytesRef) -> Result<SeekStatus> {
        let ord = self.values.lookup_term(text)?;
        if ord >= 0 {
            self.current_ord = ord;
            self.bytes = text.clone();
            Ok(SeekStatus::Found)
        } else {
            self.current_ord = -ord - 1;
            if self.current_ord == self.values.get_value_count()? {
                Ok(SeekStatus::End)
            } else {
                // TODO: hmm, can we avoid this extra lookup?
                self.bytes = self.values.lookup_ord(self.current_ord)?.clone();
                Ok(SeekStatus::NotFound)
            }
        }
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        debug_assert!(ord >= 0 && ord < self.values.get_value_count()? as i64);
        self.current_ord = ord as i32;
        self.bytes = self.values.lookup_ord(self.current_ord)?.clone();
        Ok(())
    }

    fn seek_exact_with_state(&mut self, _term: &BytesRef, state: &TermStateEnum) -> Result<()> {
        debug_assert!({ matches!(state, TermStateEnum::Ord(_)) });
        match state {
            TermStateEnum::Ord(ord_term_state) => self.seek_exact_with_ord(ord_term_state.ord)?,
            _ => return Err(LuceneError::illegal_state("state should be OrdTermState")),
        }
        Ok(())
    }

    fn term(&self) -> Result<BytesRef> {
        Ok(self.bytes.clone())
    }

    fn ord(&self) -> Result<i64> {
        Ok(self.current_ord as i64)
    }

    fn doc_freq(&self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn total_term_freq(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }

    type PostingsEnumType = DummyPostingsEnum;

    fn postings_with_flags(
        &mut self,
        _reuse: &Option<impl PostingsEnum>,
        _flags: i32,
    ) -> Result<Self::PostingsEnumType> {
        Err(LuceneError::unsupported_operation(""))
    }

    type ImpactsEnumType = DummyImpactsEnum;

    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnumType> {
        Err(LuceneError::unsupported_operation(""))
    }

    type TermStateType = TermStateEnum;

    fn term_state(&self) -> Result<Self::TermStateType> {
        let mut state = OrdTermState::new();
        state.ord = self.current_ord as i64;
        Ok(TermStateEnum::Ord(state))
    }
}
