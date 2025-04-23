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
use std::fmt::{Debug, Display, Formatter};

use crate::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::index::impacts_enum::ImpactsEnumEnum;
use crate::index::postings_enum::PostingsEnum;
use crate::index::term_state::{TermState, TermStateEnum};
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::util::access::AccessVec;
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
pub struct BaseTermsEnum {
    atts: AttributeSource,
}
impl Default for BaseTermsEnum {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseTermsEnum {
    pub fn new() -> Self {
        Self {
            atts: AttributeSource::new(),
        }
    }
}

impl<AV> BytesRefIterator<AV> for BaseTermsEnum where AV: AccessVec<u8> {}

impl<AV> TermsEnum<AV> for BaseTermsEnum
where
    AV: AccessVec<u8>,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        // TODO: 参考BaseTermsEnum中prepare_seek_exact方法
        // 来选择使用父或子的实现
        Ok(&self.atts)
    }

    fn seek_ceil(&mut self, term: &BytesRef<AV>) -> Result<SeekStatus> {
        Err(LuceneError::need_implemented(""))
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        Err(LuceneError::need_implemented(""))
    }

    fn seek_exact_with_state(&mut self, term: &BytesRef<AV>, state: &TermStateEnum) -> Result<()> {
        if !self.seek_exact(term)? {
            return Err(LuceneError::illegal_argument(format!(
                "term= {} does not exist",
                term
            )));
        };
        Ok(())
    }

    fn ord(&self) -> Result<i64> {
        Err(LuceneError::need_implemented(""))
    }

    fn doc_freq(&self) -> Result<i32> {
        Err(LuceneError::need_implemented(""))
    }

    fn total_term_freq(&self) -> Result<i64> {
        Err(LuceneError::need_implemented(""))
    }

    type PostingsEnum = DummyPostingsEnum;

    fn postings_with_flags(
        &mut self,
        _reuse: Option<impl PostingsEnum>,
        _flags: i32,
    ) -> Result<Self::PostingsEnum> {
        Err(LuceneError::need_implemented(""))
    }

    type ImpactsEnum = ImpactsEnumEnum;

    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
        Err(LuceneError::need_implemented(""))
    }

    type TermState = TermStateImpl1;

    fn term_state(&self) -> Result<Self::TermState> {
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
