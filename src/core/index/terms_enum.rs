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
use crate::core::codecs::block_term_state::TermStateEnum;
use crate::core::index::BytesRef;
use crate::core::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::core::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::core::index::impacts_enum::{ImpactsEnum, ImpactsEnumEnum2, ImpactsEnumEnum4};
use crate::core::index::postings_enum::{
  FREQS, PostingsEnum, PostingsEnumEnum2, PostingsEnumEnum4,
};
use crate::core::index::terms::{Terms, TermsPosting};
use crate::core::util::attribute_source::AttributeSourceEnum2;
use crate::core::util::attribute_source::{AttributeSource, AttributeSourceEnum4};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::dummy::dummy_attribute_source::DummyAttributeSource;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;

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
  type AttributeSource<'a>: AttributeSource
  where
    Self: 'a;
  type AttributeSourceMut<'a>: AttributeSource
  where
    Self: 'a;
  /// Returns the related attribute source.
  fn attributes(&self) -> Result<Self::AttributeSource<'_>>;
  /// Returns the related attribute source mutably.
  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>>;
  /// Attempts to seek to the exact term.
  ///
  /// Returns `true` if the term is found; `false` if the enum is
  /// unpositioned.
  fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool>;
  /// Two-phase [`seek_exact`](TermsEnum::seek_exact). The first phase
  /// typically calls [`IndexInput::prefetch`](crate::core::store::index_input::IndexInput) on the right range of bytes
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
  /// ⚠️ **Warning:** After calling this method, you **must** call
  /// [`Self::get_prepare_seek_exact_status`] to retrieve the final result,
  /// otherwise the state remains incomplete.
  fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<Option<()>>;
  fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool>;

  /// Seeks to the specified term, if it exists, or to the next (ceiling)
  /// term. Returns `SeekStatus` to indicate whether the exact term was
  /// found, a different term was found, or EOF was hit.
  /// The target term may be before or after the current term.
  /// If this returns `SeekStatus::End`, the enum is unpositioned.
  fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus>;

  /// Seeks to the specified term by ordinal (position) as previously returned
  /// by [`ord()`](TermsEnum::ord). The target ordinal may be before or
  /// after the current ordinal, and must be within bounds.
  fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()>;
  /// Expert: Seeks a specific position by `TermState` previously obtained
  /// from [`term_state()`](TermsEnum::term_state). Callers should
  /// maintain the `TermState` to use this method.
  /// Low-level implementations may position the [`TermsEnum`] without
  /// re-seeking the term dictionary.
  ///
  /// Seeking by `TermState` should only be used if the state was obtained
  /// from the same [`TermsEnum`] instance.
  ///
  /// **NOTE**: Using this method with an incompatible `TermState` might
  /// leave this [`TermsEnum`] in an undefined state. On a segment level,
  /// `TermState` instances are compatible only if the source and target
  /// [`TermsEnum`] operate on the same field. If operating on segment level,
  /// `TermState` instances must not be used across segments.
  ///
  /// **NOTE**: A seek by `TermState` might not restore the
  /// [`AttributeSource`]'s state. [`AttributeSource`] states must be
  /// maintained separately if this method is used.
  ///
  /// - `term`: the term the `TermState` corresponds to
  /// - `state`: the `TermState`
  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    state: &TermStateEnum,
  ) -> Result<()>;

  /// Returns current term. Do not call this when the enum is unpositioned.
  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>>;
  /// Returns ordinal position for the current term.
  /// This is an optional method (the codec may return an error or indicate
  /// unsupported). Do not call this when the enum is unpositioned.
  fn ord(&self) -> Result<i64>;

  /// Returns the number of documents containing the current term.
  /// Do not call this when the enum is unpositioned.
  /// Equivalent to [`SeekStatus::End`] when exhausted.
  fn doc_freq(&mut self) -> Result<i32>;

  /// Returns the total number of occurrences of this term across all
  /// documents (the sum of `freq()` for each doc that has this term).
  ///
  /// Note: like other term measures, this does not take deleted documents
  /// into account.
  fn total_term_freq(&mut self) -> Result<i64>;

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
    self.postings_with_flags(reuse, FREQS as i32)
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
  ///   [`PostingsEnum::FREQS`](FREQS))
  fn postings_with_flags(
    &mut self,
    _reuse: Option<Self::PostingsEnum>,
    _flags: i32,
  ) -> Result<Self::PostingsEnum>;
  type ImpactsEnum: ImpactsEnum;
  /// Return an `ImpactsEnum`.
  ///
  /// See also: [`postings_with_flags`](TermsEnum::postings_with_flags).
  fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum>;

  /// Expert: Returns the [`TermsEnum`]'s internal state to position the enum
  /// without re-seeking the term dictionary.
  ///
  /// **NOTE**: A seek by `TermState` might not capture the
  /// [`AttributeSource`]'s state. Callers must maintain
  /// [`AttributeSource`] states separately.
  ///
  /// See also: `TermState`,
  /// [`seek_exact_with_state`](TermsEnum::seek_exact_with_state).
  fn term_state(&mut self) -> Result<TermStateEnum>;
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

pub enum PrepareSeekStatus {
  Pending,
  Found,
  NotFound,
}
#[derive(Default)]
pub struct EmptyTermsEnum;
impl BytesRefIterator for EmptyTermsEnum {
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(None)
  }
}

impl TermsEnum for EmptyTermsEnum {
  type AttributeSource<'a>
    = &'a DummyAttributeSource
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = &'a mut DummyAttributeSource
  where
    Self: 'a;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
    Ok(self.seek_ceil(term)? == SeekStatus::Found)
  }

  fn prepare_seek_exact(&mut self, _text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_prepare_seek_exact_status(&mut self, _target: &BytesRef<Vec<u8>>) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_ceil(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    Ok(SeekStatus::End)
  }

  fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
    Ok(())
  }

  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    _state: &TermStateEnum,
  ) -> Result<()> {
    if !self.seek_exact(term)? {
      return Err(LuceneError::illegal_argument(format!(
        "term= {term} does not exist"
      )));
    }
    Ok(())
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  fn ord(&self) -> Result<i64> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  fn doc_freq(&mut self) -> Result<i32> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  type PostingsEnum = DummyPostingsEnum;

  fn postings_with_flags(
    &mut self,
    _reuse: Option<Self::PostingsEnum>,
    _flags: i32,
  ) -> Result<Self::PostingsEnum> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  type ImpactsEnum = DummyImpactsEnum;

  fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }
}
pub struct EmptyTermsEnumTermsWrapper<T> {
  in_: T,
}
impl<T> EmptyTermsEnumTermsWrapper<T> {
  pub fn new(in_: T) -> EmptyTermsEnumTermsWrapper<T> {
    Self { in_ }
  }
}

impl<T> BytesRefIterator for EmptyTermsEnumTermsWrapper<T>
where
  T: Terms,
{
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(None)
  }
}

impl<T> TermsEnum for EmptyTermsEnumTermsWrapper<T>
where
  T: Terms,
{
  type AttributeSource<'a>
    = &'a DummyAttributeSource
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = &'a mut DummyAttributeSource
  where
    Self: 'a;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
    Ok(self.seek_ceil(term)? == SeekStatus::Found)
  }

  fn prepare_seek_exact(&mut self, _text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_prepare_seek_exact_status(&mut self, _target: &BytesRef<Vec<u8>>) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_ceil(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    Ok(SeekStatus::End)
  }

  fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
    Ok(())
  }

  fn seek_exact_with_state(
    &mut self,
    _term: &BytesRef<Vec<u8>>,
    _state: &TermStateEnum,
  ) -> Result<()> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  fn ord(&self) -> Result<i64> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  fn doc_freq(&mut self) -> Result<i32> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  type PostingsEnum = TermsPosting<T>;

  fn postings_with_flags(
    &mut self,
    _reuse: Option<Self::PostingsEnum>,
    _flags: i32,
  ) -> Result<Self::PostingsEnum> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  type ImpactsEnum = DummyImpactsEnum;

  fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    Err(LuceneError::illegal_state(
      "this method should never be called",
    ))
  }
}

impl<T> TermsEnum for &mut T
where
  T: TermsEnum,
{
  type AttributeSource<'a>
    = T::AttributeSource<'a>
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = T::AttributeSourceMut<'a>
  where
    Self: 'a;
  type PostingsEnum = T::PostingsEnum;
  type ImpactsEnum = T::ImpactsEnum;

  #[inline]
  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    (**self).attributes()
  }

  #[inline]
  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    (**self).attributes_mut()
  }

  #[inline]
  fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
    (**self).seek_exact(term)
  }

  #[inline]
  fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    (**self).prepare_seek_exact(text)
  }

  #[inline]
  fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
    (**self).get_prepare_seek_exact_status(target)
  }

  #[inline]
  fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    (**self).seek_ceil(term)
  }

  #[inline]
  fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
    (**self).seek_exact_with_ord(ord)
  }

  #[inline]
  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    state: &TermStateEnum,
  ) -> Result<()> {
    (**self).seek_exact_with_state(term, state)
  }

  #[inline]
  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    (**self).term()
  }

  #[inline]
  fn ord(&self) -> Result<i64> {
    (**self).ord()
  }

  #[inline]
  fn doc_freq(&mut self) -> Result<i32> {
    (**self).doc_freq()
  }

  #[inline]
  fn total_term_freq(&mut self) -> Result<i64> {
    (**self).total_term_freq()
  }

  #[inline]
  fn postings(&mut self, reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
    (**self).postings(reuse)
  }

  #[inline]
  fn postings_with_flags(
    &mut self,
    reuse: Option<Self::PostingsEnum>,
    flags: i32,
  ) -> Result<Self::PostingsEnum> {
    (**self).postings_with_flags(reuse, flags)
  }

  #[inline]
  fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
    (**self).impacts(flags)
  }

  #[inline]
  fn term_state(&mut self) -> Result<TermStateEnum> {
    (**self).term_state()
  }
}

pub enum TermsEnumWithUnsupportedPostings2<A, B> {
  A(A),
  B(B),
}

impl<A, B> BytesRefIterator for TermsEnumWithUnsupportedPostings2<A, B>
where
  A: TermsEnum,
  B: TermsEnum,
{
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match self {
      Self::A(terms) => terms.next(),
      Self::B(terms) => terms.next(),
    }
  }

  fn set_next(&mut self) -> Result<bool> {
    match self {
      Self::A(terms) => terms.set_next(),
      Self::B(terms) => terms.set_next(),
    }
  }
}

impl<A, B> TermsEnum for TermsEnumWithUnsupportedPostings2<A, B>
where
  A: TermsEnum,
  B: TermsEnum,
{
  type AttributeSource<'a>
    = AttributeSourceEnum2<A::AttributeSource<'a>, B::AttributeSource<'a>>
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = AttributeSourceEnum2<A::AttributeSourceMut<'a>, B::AttributeSourceMut<'a>>
  where
    Self: 'a;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    match self {
      Self::A(terms) => terms.attributes().map(AttributeSourceEnum2::A),
      Self::B(terms) => terms.attributes().map(AttributeSourceEnum2::B),
    }
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    match self {
      Self::A(terms) => terms.attributes_mut().map(AttributeSourceEnum2::A),
      Self::B(terms) => terms.attributes_mut().map(AttributeSourceEnum2::B),
    }
  }

  fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
    match self {
      Self::A(terms) => terms.seek_exact(term),
      Self::B(terms) => terms.seek_exact(term),
    }
  }

  fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    match self {
      Self::A(terms) => terms.prepare_seek_exact(text),
      Self::B(terms) => terms.prepare_seek_exact(text),
    }
  }

  fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
    match self {
      Self::A(terms) => terms.get_prepare_seek_exact_status(target),
      Self::B(terms) => terms.get_prepare_seek_exact_status(target),
    }
  }

  fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    match self {
      Self::A(terms) => terms.seek_ceil(term),
      Self::B(terms) => terms.seek_ceil(term),
    }
  }

  fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
    match self {
      Self::A(terms) => terms.seek_exact_with_ord(ord),
      Self::B(terms) => terms.seek_exact_with_ord(ord),
    }
  }

  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    state: &TermStateEnum,
  ) -> Result<()> {
    match self {
      Self::A(terms) => terms.seek_exact_with_state(term, state),
      Self::B(terms) => terms.seek_exact_with_state(term, state),
    }
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    match self {
      Self::A(terms) => terms.term(),
      Self::B(terms) => terms.term(),
    }
  }

  fn ord(&self) -> Result<i64> {
    match self {
      Self::A(terms) => terms.ord(),
      Self::B(terms) => terms.ord(),
    }
  }

  fn doc_freq(&mut self) -> Result<i32> {
    match self {
      Self::A(terms) => terms.doc_freq(),
      Self::B(terms) => terms.doc_freq(),
    }
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    match self {
      Self::A(terms) => terms.total_term_freq(),
      Self::B(terms) => terms.total_term_freq(),
    }
  }

  type PostingsEnum = A::PostingsEnum;

  fn postings(&mut self, reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
    match self {
      Self::A(terms) => terms.postings(reuse),
      Self::B(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn postings_with_flags(
    &mut self,
    reuse: Option<Self::PostingsEnum>,
    flags: i32,
  ) -> Result<Self::PostingsEnum> {
    match self {
      Self::A(terms) => terms.postings_with_flags(reuse, flags),
      Self::B(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  type ImpactsEnum = A::ImpactsEnum;

  fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
    match self {
      Self::A(terms) => terms.impacts(flags),
      Self::B(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    match self {
      Self::A(terms) => terms.term_state(),
      Self::B(terms) => terms.term_state(),
    }
  }
}

pub enum TermsEnumWithUnsupportedFirstPostings4<A, B, C, D> {
  A(A),
  B(B),
  C(C),
  D(D),
}

impl<A, B, C, D> BytesRefIterator for TermsEnumWithUnsupportedFirstPostings4<A, B, C, D>
where
  A: TermsEnum,
  B: TermsEnum,
  C: TermsEnum,
  D: TermsEnum,
{
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match self {
      Self::A(terms) => terms.next(),
      Self::B(terms) => terms.next(),
      Self::C(terms) => terms.next(),
      Self::D(terms) => terms.next(),
    }
  }

  fn set_next(&mut self) -> Result<bool> {
    match self {
      Self::A(terms) => terms.set_next(),
      Self::B(terms) => terms.set_next(),
      Self::C(terms) => terms.set_next(),
      Self::D(terms) => terms.set_next(),
    }
  }
}

impl<A, B, C, D> TermsEnum for TermsEnumWithUnsupportedFirstPostings4<A, B, C, D>
where
  A: TermsEnum,
  B: TermsEnum,
  C: TermsEnum<PostingsEnum = B::PostingsEnum, ImpactsEnum = B::ImpactsEnum>,
  D: TermsEnum<PostingsEnum = B::PostingsEnum, ImpactsEnum = B::ImpactsEnum>,
{
  type AttributeSource<'a>
    = AttributeSourceEnum4<
    A::AttributeSource<'a>,
    B::AttributeSource<'a>,
    C::AttributeSource<'a>,
    D::AttributeSource<'a>,
  >
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = AttributeSourceEnum4<
    A::AttributeSourceMut<'a>,
    B::AttributeSourceMut<'a>,
    C::AttributeSourceMut<'a>,
    D::AttributeSourceMut<'a>,
  >
  where
    Self: 'a;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    match self {
      Self::A(terms) => terms.attributes().map(AttributeSourceEnum4::A),
      Self::B(terms) => terms.attributes().map(AttributeSourceEnum4::B),
      Self::C(terms) => terms.attributes().map(AttributeSourceEnum4::C),
      Self::D(terms) => terms.attributes().map(AttributeSourceEnum4::D),
    }
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    match self {
      Self::A(terms) => terms.attributes_mut().map(AttributeSourceEnum4::A),
      Self::B(terms) => terms.attributes_mut().map(AttributeSourceEnum4::B),
      Self::C(terms) => terms.attributes_mut().map(AttributeSourceEnum4::C),
      Self::D(terms) => terms.attributes_mut().map(AttributeSourceEnum4::D),
    }
  }

  fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
    match self {
      Self::A(terms) => terms.seek_exact(term),
      Self::B(terms) => terms.seek_exact(term),
      Self::C(terms) => terms.seek_exact(term),
      Self::D(terms) => terms.seek_exact(term),
    }
  }

  fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    match self {
      Self::A(terms) => terms.prepare_seek_exact(text),
      Self::B(terms) => terms.prepare_seek_exact(text),
      Self::C(terms) => terms.prepare_seek_exact(text),
      Self::D(terms) => terms.prepare_seek_exact(text),
    }
  }

  fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
    match self {
      Self::A(terms) => terms.get_prepare_seek_exact_status(target),
      Self::B(terms) => terms.get_prepare_seek_exact_status(target),
      Self::C(terms) => terms.get_prepare_seek_exact_status(target),
      Self::D(terms) => terms.get_prepare_seek_exact_status(target),
    }
  }

  fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    match self {
      Self::A(terms) => terms.seek_ceil(term),
      Self::B(terms) => terms.seek_ceil(term),
      Self::C(terms) => terms.seek_ceil(term),
      Self::D(terms) => terms.seek_ceil(term),
    }
  }

  fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
    match self {
      Self::A(terms) => terms.seek_exact_with_ord(ord),
      Self::B(terms) => terms.seek_exact_with_ord(ord),
      Self::C(terms) => terms.seek_exact_with_ord(ord),
      Self::D(terms) => terms.seek_exact_with_ord(ord),
    }
  }

  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    state: &TermStateEnum,
  ) -> Result<()> {
    match self {
      Self::A(terms) => terms.seek_exact_with_state(term, state),
      Self::B(terms) => terms.seek_exact_with_state(term, state),
      Self::C(terms) => terms.seek_exact_with_state(term, state),
      Self::D(terms) => terms.seek_exact_with_state(term, state),
    }
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    match self {
      Self::A(terms) => terms.term(),
      Self::B(terms) => terms.term(),
      Self::C(terms) => terms.term(),
      Self::D(terms) => terms.term(),
    }
  }

  fn ord(&self) -> Result<i64> {
    match self {
      Self::A(terms) => terms.ord(),
      Self::B(terms) => terms.ord(),
      Self::C(terms) => terms.ord(),
      Self::D(terms) => terms.ord(),
    }
  }

  fn doc_freq(&mut self) -> Result<i32> {
    match self {
      Self::A(terms) => terms.doc_freq(),
      Self::B(terms) => terms.doc_freq(),
      Self::C(terms) => terms.doc_freq(),
      Self::D(terms) => terms.doc_freq(),
    }
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    match self {
      Self::A(terms) => terms.total_term_freq(),
      Self::B(terms) => terms.total_term_freq(),
      Self::C(terms) => terms.total_term_freq(),
      Self::D(terms) => terms.total_term_freq(),
    }
  }

  type PostingsEnum = B::PostingsEnum;

  fn postings(&mut self, reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
    match self {
      Self::A(_) => Err(LuceneError::unsupported_operation("")),
      Self::B(terms) => terms.postings(reuse),
      Self::C(terms) => terms.postings(reuse),
      Self::D(terms) => terms.postings(reuse),
    }
  }

  fn postings_with_flags(
    &mut self,
    reuse: Option<Self::PostingsEnum>,
    flags: i32,
  ) -> Result<Self::PostingsEnum> {
    match self {
      Self::A(_) => Err(LuceneError::unsupported_operation("")),
      Self::B(terms) => terms.postings_with_flags(reuse, flags),
      Self::C(terms) => terms.postings_with_flags(reuse, flags),
      Self::D(terms) => terms.postings_with_flags(reuse, flags),
    }
  }

  type ImpactsEnum = B::ImpactsEnum;

  fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
    match self {
      Self::A(_) => Err(LuceneError::unsupported_operation("")),
      Self::B(terms) => terms.impacts(flags),
      Self::C(terms) => terms.impacts(flags),
      Self::D(terms) => terms.impacts(flags),
    }
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    match self {
      Self::A(terms) => terms.term_state(),
      Self::B(terms) => terms.term_state(),
      Self::C(terms) => terms.term_state(),
      Self::D(terms) => terms.term_state(),
    }
  }
}

macro_rules! define_terms_enum_enum {
    (
        $enum_name:ident,
        $attr_enum:ident,
        $postings_enum:ident,
        $impacts_enum:ident,
        [$($V:ident),+ $(,)?]
    ) => {
        pub enum $enum_name<$($V),+> {
            $($V($V)),+
        }

        impl<$($V),+> BytesRefIterator for $enum_name<$($V),+>
        where
            $($V: TermsEnum,)+
        {
            fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
                match self {
                    $(Self::$V(t) => t.next(),)+
                }
            }

            fn set_next(&mut self) -> Result<bool> {
                match self {
                    $(Self::$V(t) => t.set_next(),)+
                }
            }
        }

        impl<$($V),+> TermsEnum for $enum_name<$($V),+>
        where
            $($V: TermsEnum,)+
        {
            type AttributeSource<'a> = $attr_enum<$($V::AttributeSource<'a>),+> where Self: 'a;
            type AttributeSourceMut<'a> = $attr_enum<$($V::AttributeSourceMut<'a>),+> where Self: 'a;

            fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
                match self {
                    $(Self::$V(t) => Ok($attr_enum::$V(t.attributes()?)),)+
                }
            }

            fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
                match self {
                    $(Self::$V(t) => Ok($attr_enum::$V(t.attributes_mut()?)),)+
                }
            }

            fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
                match self {
                    $(Self::$V(t) => t.seek_exact(term),)+
                }
            }

            fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
                match self {
                    $(Self::$V(t) => t.prepare_seek_exact(text),)+
                }
            }

            fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
                match self {
                    $(Self::$V(t) => t.get_prepare_seek_exact_status(target),)+
                }
            }

            fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
                match self {
                    $(Self::$V(t) => t.seek_ceil(term),)+
                }
            }

            fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.seek_exact_with_ord(ord),)+
                }
            }

            fn seek_exact_with_state(
                &mut self,
                term: &BytesRef<Vec<u8>>,
                state: &TermStateEnum,
            ) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.seek_exact_with_state(term, state),)+
                }
            }

            fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
                match self {
                    $(Self::$V(t) => t.term(),)+
                }
            }

            fn ord(&self) -> Result<i64> {
                match self {
                    $(Self::$V(t) => t.ord(),)+
                }
            }

            fn doc_freq(&mut self) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.doc_freq(),)+
                }
            }

            fn total_term_freq(&mut self) -> Result<i64> {
                match self {
                    $(Self::$V(t) => t.total_term_freq(),)+
                }
            }

            type PostingsEnum = $postings_enum<$($V::PostingsEnum),+>;

            fn postings(&mut self, reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
                match self {
                    $(
                        Self::$V(t) => match reuse {
                            Some($postings_enum::$V(v)) => Ok($postings_enum::$V(t.postings(Some(v))?)),
                            None => Ok($postings_enum::$V(t.postings(None)?)),
                            _ => Ok($postings_enum::$V(t.postings(None)?)),
                        },
                    )+
                }
            }

            fn postings_with_flags(
                &mut self,
                reuse: Option<Self::PostingsEnum>,
                flags: i32,
            ) -> Result<Self::PostingsEnum> {
                match self {
                    $(
                        Self::$V(t) => match reuse {
                            Some($postings_enum::$V(v)) => Ok($postings_enum::$V(t.postings_with_flags(Some(v), flags)?)),
                            None => Ok($postings_enum::$V(t.postings_with_flags(None, flags)?)),
                            _ => Ok($postings_enum::$V(t.postings_with_flags(None, flags)?)),
                        },
                    )+
                }
            }

            type ImpactsEnum = $impacts_enum<$($V::ImpactsEnum),+>;

            fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
                match self {
                    $(Self::$V(t) => Ok($impacts_enum::$V(t.impacts(flags)?)),)+
                }
            }

            fn term_state(&mut self) -> Result<TermStateEnum> {
                match self {
                    $(Self::$V(t) => t.term_state(),)+
                }
            }
        }
    };
}
define_terms_enum_enum!(
  TermsEnumEnum2,
  AttributeSourceEnum2,
  PostingsEnumEnum2,
  ImpactsEnumEnum2,
  [A, B]
);
define_terms_enum_enum!(
  TermsEnumEnum4,
  AttributeSourceEnum4,
  PostingsEnumEnum4,
  ImpactsEnumEnum4,
  [A, B, C, D]
);
