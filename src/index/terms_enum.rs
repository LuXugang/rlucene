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
use std::marker::PhantomData;

use crate::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::index::impacts_enum::ImpactsEnum;
use crate::index::postings_enum::{postings_enum_util, PostingsEnum};
use crate::index::term_state::{TermState, TermStateEnum};
use crate::index::BytesRef;
use crate::store::IndexInput;
use crate::util::access::AccessVec;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};

/// Iterator to seek [`seek_ceil(BytesRef)`](TermsEnum::seek_ceil),
/// [`seek_exact(BytesRef)`](TermsEnum::seek_exact) or step through
/// [`next`](BytesRefIterator::next) terms to obtain frequency information
/// [`doc_freq`](TermsEnum::doc_freq), [`PostingsEnum`] or [`ImpactsEnum`] for
/// the current term [`postings`](TermsEnum::postings).
///
/// Term enumerations are always ordered by `BytesRef::compare_to`, which is
/// Unicode sort order if the terms are UTF-8 bytes. Each term in the
/// enumeration is greater than the one before it.
///
/// The `TermsEnum` is unpositioned when you first obtain it, and you must first
/// successfully call [`next()`](BytesRefIterator::next) or one of the `seek`
/// methods.
pub trait TermsEnum<AV>: BytesRefIterator<AV>
where
    AV: AccessVec<u8>,
{
    /// Returns the related attribute source.
    fn attributes(&self) -> Result<&AttributeSource>;

    /// Attempts to seek to the exact term.
    ///
    /// Returns `true` if the term is found; `false` if the enum is
    /// unpositioned.
    fn seek_exact(&mut self, term: &BytesRef<AV>) -> Result<bool> {
        Ok(self.seek_ceil(term)? == SeekStatus::Found)
    }

    /// Two-phase [`seek_exact`](TermsEnum::seek_exact). The first phase
    /// typically calls [`IndexInput::prefetch`] on the right range of bytes
    /// under the hood, while the second phase
    /// [`see.exact`](TermsEnum::seek_exact) actually seeks the term within
    /// these bytes. This can be used to parallelize I/O across multiple
    /// terms by calling [`prepare_seek_exact`](TermsEnum::prepare_seek_exact)
    /// on multiple terms enums before calling `IOBooleanSupplier::get()`.
    ///
    /// **NOTE**: It is illegal to call other methods on this [`TermsEnum`]
    /// after calling this method until
    /// [`seek_exact`](TermsEnum::seek_exact) is called.
    ///
    /// **NOTE**: This may return `None` if this [`TermsEnum`] can identify that
    /// the term may not exist without performing any I/O.
    fn prepare_seek_exact<'a>(
        &'a mut self,
        text: BytesRef<AV>,
    ) -> Result<impl FnMut() -> Result<bool> + 'a>
    where
        AV: 'a,
    {
        Ok(move || self.seek_exact(&text))
    }

    /// Seeks to the specified term, if it exists, or to the next (ceiling)
    /// term. Returns `SeekStatus` to indicate whether the exact term was
    /// found, a different term was found, or EOF was hit.
    /// The target term may be before or after the current term.
    /// If this returns `SeekStatus::End`, the enum is unpositioned.
    fn seek_ceil(&mut self, term: &BytesRef<AV>) -> Result<SeekStatus>;

    /// Seeks to the specified term by ordinal (position) as previously returned
    /// by [`ord()`](TermsEnum::ord). The target ordinal may be before or
    /// after the current ordinal, and must be within bounds.
    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()>;
    /// Expert: Seeks a specific position by [`TermState`] previously obtained
    /// from [`term_state()`](TermsEnum::term_state). Callers should
    /// maintain the [`TermState`] to use this method.
    /// Low-level implementations may position the [`TermsEnum`] without
    /// re-seeking the term dictionary.
    ///
    /// Seeking by [`TermState`] should only be used if the state was obtained
    /// from the same [`TermsEnum`] instance.
    ///
    /// **NOTE**: Using this method with an incompatible [`TermState`] might
    /// leave this [`TermsEnum`] in an undefined state. On a segment level,
    /// [`TermState`] instances are compatible only if the source and target
    /// [`TermsEnum`] operate on the same field. If operating on segment level,
    /// [`TermState`] instances must not be used across segments.
    ///
    /// **NOTE**: A seek by [`TermState`] might not restore the
    /// [`AttributeSource`]'s state. [`AttributeSource`] states must be
    /// maintained separately if this method is used.
    ///
    /// - `term`: the term the [`TermState`] corresponds to
    /// - `state`: the [`TermState`]
    fn seek_exact_with_state(&mut self, term: &BytesRef<AV>, state: &TermStateEnum) -> Result<()>;

    /// Returns current term. Do not call this when the enum is unpositioned.
    fn term(&self) -> Result<Cow<BytesRef<AV>>> {
        Err(LuceneError::need_implemented("this method need implement"))
    }
    /// Returns ordinal position for the current term.
    /// This is an optional method (the codec may return an error or indicate
    /// unsupported). Do not call this when the enum is unpositioned.
    fn ord(&self) -> Result<i64>;

    /// Returns the number of documents containing the current term.
    /// Do not call this when the enum is unpositioned.
    /// Equivalent to [`SeekStatus::End`] when exhausted.
    fn doc_freq(&self) -> Result<i32>;

    /// Returns the total number of occurrences of this term across all
    /// documents (the sum of `freq()` for each doc that has this term).
    ///
    /// Note: like other term measures, this does not take deleted documents
    /// into account.
    fn total_term_freq(&self) -> Result<i64>;

    type PostingsEnum: PostingsEnum;
    /// Get [`PostingsEnum`] for the current term. Do not call this when the
    /// enum is unpositioned. This method will not return `None`.
    ///
    /// **NOTE**: The returned iterator may include deleted documents.
    /// Deleted documents must be checked separately.
    ///
    /// Use this method if you only require documents and frequencies,
    /// and do not need any proximity data.
    /// This is equivalent to [`postings(reuse,
    /// PostingsEnum::FREQS)`](TermsEnum::postings_with_flags).
    ///
    /// - `reuse`: a prior [`PostingsEnum`] for possible reuse See also:
    ///   `postings_with_flags`.
    fn postings(&mut self, reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
        self.postings_with_flags(reuse, postings_enum_util::FREQS as i32)
    }

    /// Get [`PostingsEnum`] for the current term, with control over whether
    /// freqs, positions, offsets or payloads are required. Do not call this
    /// when the enum is unpositioned. This method will not return `None`.
    ///
    /// **NOTE**: The returned iterator may include deleted documents,
    /// so deleted documents must be checked on top of the [`PostingsEnum`].
    ///
    /// - `reuse`: a prior [`PostingsEnum`] for possible reuse
    /// - `flags`: specifies which optional per-document values you require (see
    ///   [`PostingsEnum::FREQS`](postings_enum_util::FREQS))
    fn postings_with_flags(
        &mut self,
        reuse: Option<Self::PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnum>;
    type ImpactsEnum: ImpactsEnum;
    /// Return an `ImpactsEnum`.
    ///
    /// See also: [`postings_with_flags`](TermsEnum::postings_with_flags).
    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum>;

    type TermState: TermState;
    /// Expert: Returns the [`TermsEnum`]'s internal state to position the enum
    /// without re-seeking the term dictionary.
    ///
    /// **NOTE**: A seek by [`TermState`] might not capture the
    /// [`AttributeSource`]'s state. Callers must maintain
    /// [`AttributeSource`] states separately.
    ///
    /// See also: [`TermState`],
    /// [`seek_exact_with_state`](TermsEnum::seek_exact_with_state).
    fn term_state(&self) -> Result<Self::TermState>;
}
pub struct TermsEnumEmpty<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    _phantom1: PhantomData<I>,
    _phantom2: PhantomData<AV>,
}

impl<I, AV> BytesRefIterator<AV> for TermsEnumEmpty<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    fn next(&mut self) -> Result<Option<Cow<BytesRef<AV>>>> {
        Ok(None)
    }
}

impl<I, AV> TermsEnum<AV> for TermsEnumEmpty<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        todo!()
    }

    fn seek_ceil(&mut self, _term: &BytesRef<AV>) -> Result<SeekStatus> {
        Ok(SeekStatus::End)
    }

    fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
        Ok(())
    }

    fn seek_exact_with_state(
        &mut self,
        _term: &BytesRef<AV>,
        _state: &TermStateEnum,
    ) -> Result<()> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }

    fn term(&self) -> Result<Cow<BytesRef<AV>>> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }

    fn ord(&self) -> Result<i64> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }

    fn doc_freq(&self) -> Result<i32> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }

    fn total_term_freq(&self) -> Result<i64> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }

    type PostingsEnum = DummyPostingsEnum;

    fn postings_with_flags(
        &mut self,
        _reuse: Option<Self::PostingsEnum>,
        _flags: i32,
    ) -> Result<Self::PostingsEnum> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }

    type ImpactsEnum = DummyImpactsEnum;

    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }

    type TermState = TermStateEnum;

    fn term_state(&self) -> Result<Self::TermState> {
        Err(LuceneError::not_implemented(
            "this method should never be called",
        ))
    }
}
/// Represents returned result from `seek_ceil`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekStatus {
    /// The term was not found, and the end of iteration was hit.
    End,
    /// The precise term was found.
    Found,
    /// A different term was found after the requested term.
    NotFound,
}
