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
use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};

use crate::index::term_state::{TermState, TermStateEnum};
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::either_enums::EitherTermState;
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
pub struct BaseTermsEnum<S>
where
    S: TermsEnum,
{
    atts: AttributeSource,
    sub: S,
}
impl<S> BaseTermsEnum<S>
where
    S: TermsEnum,
{
    pub fn new(sub: S) -> Self {
        Self {
            atts: AttributeSource::new(),
            sub,
        }
    }
}

impl<S> BytesRefIterator for BaseTermsEnum<S> where S: TermsEnum {}

impl<S> TermsEnum for BaseTermsEnum<S>
where
    S: TermsEnum,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        match self.sub.attributes() {
            Ok(v) => Ok(v),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => Ok(&self.atts),
                _ => Err(e),
            },
        }
    }

    fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
        match self.sub.seek_exact(term) {
            Ok(v) => Ok(v),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => Ok(self.seek_ceil(term)? == SeekStatus::Found),
                _ => Err(e),
            },
        }
    }

    fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<bool> {
        match self.sub.prepare_seek_exact(text) {
            Ok(v) => Ok(v),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => self.seek_exact(text),
                _ => Err(e),
            },
        }
    }

    fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        self.sub.seek_ceil(term)
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        self.sub.seek_exact_with_ord(ord)
    }

    fn seek_exact_with_state(
        &mut self,
        term: &BytesRef<Vec<u8>>,
        state: &TermStateEnum,
    ) -> Result<()> {
        match self.sub.seek_exact_with_state(term, state) {
            Ok(v) => Ok(v),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => {
                    if !self.seek_exact(term)? {
                        return Err(LuceneError::illegal_argument(format!(
                            "term= {} does not exist",
                            term
                        )));
                    };
                    Ok(())
                },
                _ => Err(e),
            },
        }
    }

    fn term(&self) -> Result<Cow<BytesRef<Vec<u8>>>> {
        self.sub.term()
    }

    fn ord(&self) -> Result<i64> {
        self.sub.ord()
    }

    fn doc_freq(&mut self) -> Result<i32> {
        self.sub.doc_freq()
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        self.sub.total_term_freq()
    }

    type PostingsEnum = S::PostingsEnum;

    fn postings(&mut self, reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
        self.sub.postings(reuse)
    }

    fn postings_with_flags(
        &mut self,
        reuse: Option<Self::PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnum> {
        self.sub.postings_with_flags(reuse, flags)
    }

    type ImpactsEnum = S::ImpactsEnum;

    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
        self.sub.impacts(flags)
    }

    type TermState = EitherTermState<TermStateImpl1, S::TermState>;

    fn term_state(&mut self) -> Result<Self::TermState> {
        match self.sub.term_state() {
            Ok(v) => Ok(EitherTermState::S(v)),
            Err(e) => match e {
                LuceneError::NotImplemented(_) => Ok(EitherTermState::F(TermStateImpl1)),
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
