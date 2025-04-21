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
use crate::index::impacts_enum::ImpactsEnumEnum;
use crate::index::postings_enum::{PostingsEnum, PostingsEnums};
use crate::index::term_state::{TermState, TermStateEnum};
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::terms_enums::TermsEnums;
use crate::index::BytesRef;
use crate::store::IndexInput;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};

/// A base `TermsEnum` that provides default implementations for:
///
/// - [`attributes()`](BaseTermsEnum::attributes)
/// - [`term_state()`](BaseTermsEnum::term_state)
/// - [`seek_exact(&BytesRef)`](BaseTermsEnum::seek_exact)
/// - [`seek_exact_with_state(&BytesRef, &TermState)`](BaseTermsEnum::seek_exact_with_state)
///
/// In some cases, the default implementation may be slow and consume large amounts of memory,
/// so subclasses SHOULD provide their own implementation if possible.
pub struct BaseTermsEnum<I>
where
    I: IndexInput,
{
    atts: AttributeSource,
    sub_terms_enum: TermsEnums<I>,
}
impl<I> BaseTermsEnum<I>
where
    I: IndexInput,
{
    pub fn new(sub_terms_enum: TermsEnums<I>) -> Self {
        Self {
            atts: AttributeSource::new(),
            sub_terms_enum,
        }
    }
}

impl<I> BytesRefIterator for BaseTermsEnum<I>
where
    I: IndexInput,
{
    fn next(&mut self) -> Result<Option<Cow<BytesRef<Vec<u8>>>>> {
        self.sub_terms_enum.next()
    }
}

impl<I> TermsEnum for BaseTermsEnum<I>
where
    I: IndexInput,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        // TODO: 参考BaseTermsEnum中prepare_seek_exact方法 来选择使用父或子的实现
        Ok(&self.atts)
    }

    fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        self.sub_terms_enum.seek_ceil(term)
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        self.sub_terms_enum.seek_exact_with_ord(ord)
    }

    fn seek_exact_with_state(
        &mut self,
        term: &BytesRef<Vec<u8>>,
        _state: &TermStateEnum,
    ) -> Result<()> {
        if self.seek_exact(term)? {
            return Err(LuceneError::illegal_argument(format!(
                "term= {} does not exist",
                term
            )));
        };
        Ok(())
    }

    fn term(&self) -> Result<BytesRef<Vec<u8>>> {
        self.sub_terms_enum.term()
    }

    fn ord(&self) -> Result<i64> {
        self.sub_terms_enum.ord()
    }

    fn doc_freq(&self) -> Result<i32> {
        self.sub_terms_enum.doc_freq()
    }

    fn total_term_freq(&self) -> Result<i64> {
        self.sub_terms_enum.total_term_freq()
    }

    type PostingsEnum = PostingsEnums<I>;

    fn postings_with_flags(
        &mut self,
        reuse: Option<impl PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnum> {
        self.sub_terms_enum.postings_with_flags(reuse, flags)
    }

    type ImpactsEnum = ImpactsEnumEnum;

    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
        self.sub_terms_enum.impacts(flags)
    }

    type TermState = TermStateEnum;

    fn term_state(&self) -> Result<Self::TermState> {
        let sub = self.sub_terms_enum.term_state();
        match sub {
            Ok(s) => Ok(s),
            Err(e) => match e {
                // sub_terms_enum's invalid error,
                // it means sub_terms_enum uses the return value of BaseTermsEnum's term_state
                LuceneError::NotImplemented(_) => Ok(TermStateEnum::Impl1(TermStateImpl1)),
                // sub_terms_enum's valid error
                _ => Err(e),
            },
        }
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
