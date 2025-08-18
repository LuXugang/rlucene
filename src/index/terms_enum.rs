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

use crate::index::BytesRef;
use crate::index::impacts_enum::{EitherImpactsEnum, ImpactsEnum};
use crate::index::postings_enum::{EitherPostingsEnum, PostingsEnum, postings_enum_util};
use crate::index::term_state::{EitherTermState, TermState, TermStateEnum};
use crate::util::attribute_source::AttributeSource;
use crate::util::attribute_source::EitherAttributeSource;
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
pub trait TermsEnum: BytesRefIterator {
    type AttributeSource: AttributeSource;
    /// Returns the related attribute source.
    fn attributes(&self) -> Result<Self::AttributeSource> {
        Err(LuceneError::need_implemented(""))
    }
    /// Attempts to seek to the exact term.
    ///
    /// Returns `true` if the term is found; `false` if the enum is
    /// unpositioned.
    fn seek_exact(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<bool> {
        Err(LuceneError::need_implemented(""))
    }
    /// Two-phase [`seek_exact`](TermsEnum::seek_exact). The first phase
    /// typically calls [`IndexInput::prefetch`](crate::store::index_input::IndexInput) on the right range of bytes
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
    fn prepare_seek_exact(&mut self, _text: &BytesRef<Vec<u8>>) -> Result<bool> {
        Err(LuceneError::need_implemented(""))
    }

    /// Seeks to the specified term, if it exists, or to the next (ceiling)
    /// term. Returns `SeekStatus` to indicate whether the exact term was
    /// found, a different term was found, or EOF was hit.
    /// The target term may be before or after the current term.
    /// If this returns `SeekStatus::End`, the enum is unpositioned.
    fn seek_ceil(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        Err(LuceneError::need_implemented(""))
    }

    /// Seeks to the specified term by ordinal (position) as previously returned
    /// by [`ord()`](TermsEnum::ord). The target ordinal may be before or
    /// after the current ordinal, and must be within bounds.
    fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
        Err(LuceneError::need_implemented(""))
    }
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
    fn seek_exact_with_state(
        &mut self,
        _term: &BytesRef<Vec<u8>>,
        _state: &TermStateEnum,
    ) -> Result<()> {
        Err(LuceneError::need_implemented(""))
    }

    /// Returns current term. Do not call this when the enum is unpositioned.
    fn term(&self) -> Result<Cow<BytesRef<Vec<u8>>>> {
        Err(LuceneError::need_implemented(""))
    }
    /// Returns ordinal position for the current term.
    /// This is an optional method (the codec may return an error or indicate
    /// unsupported). Do not call this when the enum is unpositioned.
    fn ord(&self) -> Result<i64> {
        Err(LuceneError::need_implemented(""))
    }

    /// Returns the number of documents containing the current term.
    /// Do not call this when the enum is unpositioned.
    /// Equivalent to [`SeekStatus::End`] when exhausted.
    fn doc_freq(&mut self) -> Result<i32> {
        Err(LuceneError::need_implemented(""))
    }

    /// Returns the total number of occurrences of this term across all
    /// documents (the sum of `freq()` for each doc that has this term).
    ///
    /// Note: like other term measures, this does not take deleted documents
    /// into account.
    fn total_term_freq(&mut self) -> Result<i64> {
        Err(LuceneError::need_implemented(""))
    }

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
        _reuse: Option<Self::PostingsEnum>,
        _flags: i32,
    ) -> Result<Self::PostingsEnum> {
        Err(LuceneError::need_implemented(""))
    }
    type ImpactsEnum: ImpactsEnum;
    /// Return an `ImpactsEnum`.
    ///
    /// See also: [`postings_with_flags`](TermsEnum::postings_with_flags).
    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
        Err(LuceneError::need_implemented(""))
    }

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
    fn term_state(&mut self) -> Result<Self::TermState> {
        Err(LuceneError::need_implemented(""))
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

// TermsEnum
pub enum EitherTermsEnum<F, S> {
    F(F),
    S(S),
}

impl<F, S> BytesRefIterator for EitherTermsEnum<F, S>
where
    F: TermsEnum,
    S: TermsEnum,
{
}

impl<F, S> TermsEnum for EitherTermsEnum<F, S>
where
    F: TermsEnum,
    S: TermsEnum,
{
    type AttributeSource = EitherAttributeSource<F::AttributeSource, S::AttributeSource>;

    fn attributes(&self) -> Result<Self::AttributeSource> {
        match self {
            EitherTermsEnum::F(t) => Ok(EitherAttributeSource::F(t.attributes()?)),
            EitherTermsEnum::S(s) => Ok(EitherAttributeSource::S(s.attributes()?)),
        }
    }

    fn seek_exact(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<bool> {
        match self {
            EitherTermsEnum::F(t) => t.seek_exact(_term),
            EitherTermsEnum::S(s) => s.seek_exact(_term),
        }
    }

    fn prepare_seek_exact(&mut self, _text: &BytesRef<Vec<u8>>) -> Result<bool> {
        match self {
            EitherTermsEnum::F(t) => t.prepare_seek_exact(_text),
            EitherTermsEnum::S(s) => s.prepare_seek_exact(_text),
        }
    }

    fn seek_ceil(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        match self {
            EitherTermsEnum::F(t) => t.seek_ceil(_term),
            EitherTermsEnum::S(s) => s.seek_ceil(_term),
        }
    }

    fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
        match self {
            EitherTermsEnum::F(t) => t.seek_exact_with_ord(_ord),
            EitherTermsEnum::S(s) => s.seek_exact_with_ord(_ord),
        }
    }

    fn seek_exact_with_state(
        &mut self,
        _term: &BytesRef<Vec<u8>>,
        _state: &TermStateEnum,
    ) -> Result<()> {
        match self {
            EitherTermsEnum::F(t) => t.seek_exact_with_state(_term, _state),
            EitherTermsEnum::S(s) => s.seek_exact_with_state(_term, _state),
        }
    }

    fn term(&self) -> Result<Cow<BytesRef<Vec<u8>>>> {
        match self {
            EitherTermsEnum::F(t) => t.term(),
            EitherTermsEnum::S(s) => s.term(),
        }
    }

    fn ord(&self) -> Result<i64> {
        match self {
            EitherTermsEnum::F(t) => t.ord(),
            EitherTermsEnum::S(s) => s.ord(),
        }
    }

    fn doc_freq(&mut self) -> Result<i32> {
        match self {
            EitherTermsEnum::F(t) => t.doc_freq(),
            EitherTermsEnum::S(s) => s.doc_freq(),
        }
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        match self {
            EitherTermsEnum::F(t) => t.total_term_freq(),
            EitherTermsEnum::S(s) => s.total_term_freq(),
        }
    }

    type PostingsEnum = EitherPostingsEnum<F::PostingsEnum, S::PostingsEnum>;

    fn postings(&mut self, reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
        match self {
            EitherTermsEnum::F(t) => match reuse {
                Some(EitherPostingsEnum::F(v)) => {
                    let postings_enum = t.postings(Some(v))?;
                    Ok(EitherPostingsEnum::F(postings_enum))
                },
                None => {
                    let postings_enum = t.postings(None)?;
                    Ok(EitherPostingsEnum::F(postings_enum))
                },
                _ => Err(LuceneError::illegal_state(
                    "EitherTermsEnum::F expected EitherPostingsEnum::F for reuse".to_string(),
                )),
            },
            EitherTermsEnum::S(s) => match reuse {
                Some(EitherPostingsEnum::S(v)) => {
                    let postings_enum = s.postings(Some(v))?;
                    Ok(EitherPostingsEnum::S(postings_enum))
                },
                None => {
                    let postings_enum = s.postings(None)?;
                    Ok(EitherPostingsEnum::S(postings_enum))
                },
                _ => Err(LuceneError::illegal_state(
                    "EitherTermsEnum::S expected EitherPostingsEnum::S for reuse".to_string(),
                )),
            },
        }
    }

    fn postings_with_flags(
        &mut self,
        reuse: Option<Self::PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnum> {
        match self {
            EitherTermsEnum::F(t) => match reuse {
                Some(EitherPostingsEnum::F(v)) => {
                    let postings_enum = t.postings_with_flags(Some(v), flags)?;
                    Ok(EitherPostingsEnum::F(postings_enum))
                },
                None => {
                    let postings_enum = t.postings_with_flags(None, flags)?;
                    Ok(EitherPostingsEnum::F(postings_enum))
                },
                _ => Err(LuceneError::illegal_state(
                    "EitherTermsEnum::F expected EitherPostingsEnum::F for reuse".to_string(),
                )),
            },
            EitherTermsEnum::S(s) => match reuse {
                Some(EitherPostingsEnum::S(v)) => {
                    let postings_enum = s.postings_with_flags(Some(v), flags)?;
                    Ok(EitherPostingsEnum::S(postings_enum))
                },
                None => {
                    let postings_enum = s.postings_with_flags(None, flags)?;
                    Ok(EitherPostingsEnum::S(postings_enum))
                },
                _ => Err(LuceneError::illegal_state(
                    "EitherTermsEnum::S expected EitherPostingsEnum::S for reuse".to_string(),
                )),
            },
        }
    }

    type ImpactsEnum = EitherImpactsEnum<F::ImpactsEnum, S::ImpactsEnum>;

    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
        match self {
            EitherTermsEnum::F(t) => {
                let impacts_enum = t.impacts(flags)?;
                Ok(EitherImpactsEnum::F(impacts_enum))
            },
            EitherTermsEnum::S(s) => {
                let impacts_enum = s.impacts(flags)?;
                Ok(EitherImpactsEnum::S(impacts_enum))
            },
        }
    }

    type TermState = EitherTermState<F::TermState, S::TermState>;

    fn term_state(&mut self) -> Result<Self::TermState> {
        match self {
            EitherTermsEnum::F(t) => {
                let term_state = t.term_state()?;
                Ok(EitherTermState::F(term_state))
            },
            EitherTermsEnum::S(s) => {
                let term_state = s.term_state()?;
                Ok(EitherTermState::S(term_state))
            },
        }
    }
}
