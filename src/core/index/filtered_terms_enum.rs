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
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::util::ToInt;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;
use std::fmt::Debug;

/// Struct for enumerating a subset of all terms.
///
/// Term enumerations are always ordered by [`BytesRef::cmp`] Each term in the
/// enumeration is greater than all that precede it.
///
/// *Please note:* Consumers of this enum cannot call `seek()`, it is forward
/// only; it will return
/// [`UnsupportedOperationError`](LuceneError::unsupported_operation) when a
/// seeking method is called.
pub struct FilteredTermsEnum<T, F>
where
  T: TermsEnum,
  F: FilteredTermsEnumBase,
{
  initial_seek_term: Option<BytesRef<Vec<u8>>>,
  do_seek: bool,
  pub actual_term: Option<BytesRef<Vec<u8>>>,
  pub tenum: T,
  hook: FilteredTermsEnumHook<F>,
}

enum FilteredTermsEnumHook<F>
where
  F: FilteredTermsEnumBase,
{
  Default,
  Filtered(F),
}
impl<T, F> FilteredTermsEnum<T, F>
where
  T: TermsEnum,
  F: FilteredTermsEnumBase,
{
  pub(crate) fn new(tenum: T, sub: F) -> Self {
    Self::with_seek(tenum, true, sub)
  }

  /// Creates a new filtered enumerator with control over initial seeking.
  pub(crate) fn with_seek(tenum: T, start_with_seek: bool, sub: F) -> Self {
    FilteredTermsEnum {
      initial_seek_term: None,
      do_seek: start_with_seek,
      actual_term: None,
      tenum,
      hook: FilteredTermsEnumHook::Filtered(sub),
    }
  }
  pub(crate) fn unfiltered(tenum: T) -> Self {
    FilteredTermsEnum {
      initial_seek_term: None,
      do_seek: false,
      actual_term: None,
      tenum,
      hook: FilteredTermsEnumHook::Default,
    }
  }
  pub(crate) fn set_initial_seek_term(&mut self, term: BytesRef<Vec<u8>>) {
    self.initial_seek_term = Some(term);
  }
  pub fn next_seek_term(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    let sub = match &mut self.hook {
      FilteredTermsEnumHook::Default => {
        return Err(LuceneError::unsupported_operation(
          "unfiltered terms enum has no next seek term",
        ));
      },
      FilteredTermsEnumHook::Filtered(sub) => sub,
    };
    match sub.next_seek_term(Option::from(&self.actual_term)) {
      Ok(v) => Ok(v),
      Err(e) => match e {
        LuceneError::NotImplemented(_) => {
          let mut a = self.initial_seek_term.take().unwrap();
          Ok(Some(Cow::Owned(BytesRef::from_slice(
            std::mem::take(&mut a.bytes),
            a.offset,
            a.length,
          ))))
        },
        _ => Err(e),
      },
    }
  }
}

impl<T, F> BytesRefIterator for FilteredTermsEnum<T, F>
where
  T: TermsEnum,
  F: FilteredTermsEnumBase,
{
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    if matches!(&self.hook, FilteredTermsEnumHook::Default) {
      return self.tenum.next();
    }
    loop {
      if self.do_seek {
        self.do_seek = false;
        let t = self.next_seek_term()?.map(Cow::into_owned);
        debug_assert!(
          self.actual_term.is_none()
            || t.is_none()
            || t
              .as_ref()
              .unwrap()
              .cmp(self.actual_term.as_ref().unwrap())
              .to_int()
              > 0
        );
        if t.is_none() || self.tenum.seek_ceil(t.as_ref().unwrap())? == SeekStatus::End {
          return Ok(None);
        }
        // TODO: avoid copy here?
        self.actual_term = Option::from(self.tenum.term()?.into_owned());
      } else {
        match self.tenum.next()? {
          Some(term) => {
            self.actual_term = Option::from(term.into_owned());
          },
          None => {
            self.actual_term = None;
            return Ok(None);
          },
        };
      }
      // check if term is accepted
      let need_ord = match &self.hook {
        FilteredTermsEnumHook::Default => false,
        FilteredTermsEnumHook::Filtered(sub) => sub.need_ord(),
      };
      let ord = match need_ord {
        true => self.ord()?,
        // padding value
        false => 0,
      };
      let accept_status = match &mut self.hook {
        FilteredTermsEnumHook::Default => AcceptStatus::Yes,
        FilteredTermsEnumHook::Filtered(sub) => {
          sub.accept(self.actual_term.as_ref().unwrap(), ord)?
        },
      };
      match accept_status {
        AcceptStatus::YesAndSeek => {
          self.do_seek = true;
          return Ok(Some(Cow::Borrowed(self.actual_term.as_ref().unwrap())));
        },
        // term accepted, but we need to seek so fall-through
        AcceptStatus::Yes => {
          return Ok(Some(Cow::Borrowed(self.actual_term.as_ref().unwrap())));
        },
        AcceptStatus::NoAndSeek => {
          // invalid term, seek next time
          self.do_seek = true;
        },
        AcceptStatus::End => {
          // we are supposed to end the enum
          return Ok(None);
        },
        // we just iterate again
        AcceptStatus::No => {},
      }
    }
  }
}

impl<T, F> TermsEnum for FilteredTermsEnum<T, F>
where
  T: TermsEnum,
  F: FilteredTermsEnumBase,
{
  type AttributeSource<'a>
    = T::AttributeSource<'a>
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = T::AttributeSourceMut<'a>
  where
    Self: 'a;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    self.tenum.attributes()
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    self.tenum.attributes_mut()
  }

  fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
    match &self.hook {
      FilteredTermsEnumHook::Default => self.tenum.seek_exact(term),
      FilteredTermsEnumHook::Filtered(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    match &self.hook {
      FilteredTermsEnumHook::Default => self.tenum.prepare_seek_exact(text),
      FilteredTermsEnumHook::Filtered(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
    match &self.hook {
      FilteredTermsEnumHook::Default => self.tenum.get_prepare_seek_exact_status(target),
      FilteredTermsEnumHook::Filtered(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    match &self.hook {
      FilteredTermsEnumHook::Default => self.tenum.seek_ceil(term),
      FilteredTermsEnumHook::Filtered(_) => Err(LuceneError::unsupported_operation(
        "FilteredTermsEnum::seek_ceil",
      )),
    }
  }

  fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
    match &self.hook {
      FilteredTermsEnumHook::Default => self.tenum.seek_exact_with_ord(ord),
      FilteredTermsEnumHook::Filtered(_) => Err(LuceneError::unsupported_operation(
        "FilteredTermsEnum::seek_exact_with_ord",
      )),
    }
  }

  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    state: &TermStateEnum,
  ) -> Result<()> {
    match &self.hook {
      FilteredTermsEnumHook::Default => self.tenum.seek_exact_with_state(term, state),
      FilteredTermsEnumHook::Filtered(_) => Err(LuceneError::unsupported_operation(
        "FilteredTermsEnum::seek_exact_with_state",
      )),
    }
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    self.tenum.term()
  }

  fn ord(&self) -> Result<i64> {
    self.tenum.ord()
  }

  fn doc_freq(&mut self) -> Result<i32> {
    self.tenum.doc_freq()
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    self.tenum.total_term_freq()
  }

  type PostingsEnum = T::PostingsEnum;

  fn postings_with_flags(
    &mut self,
    reuse: Option<Self::PostingsEnum>,
    flags: i32,
  ) -> Result<Self::PostingsEnum> {
    self.tenum.postings_with_flags(reuse, flags)
  }

  type ImpactsEnum = T::ImpactsEnum;

  fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
    self.tenum.impacts(flags)
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    self.tenum.term_state()
  }
}

/// Return value indicating whether the term should be accepted or the iteration
/// should end. The `*_SEEK` values denote that after handling the current term,
/// the enum should call [`next_seek_term`](FilteredTermsEnum::next_seek_term)
/// and step forward.
///
/// See also:
/// - [`accept`](FilteredTermsEnumBase::accept)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AcceptStatus {
  /// Accept the term and continue.
  Yes,
  /// Accept the term then seek to the next term returned by
  /// `next_seek_term()`.
  YesAndSeek,
  /// Reject the term and continue.
  No,
  /// Reject the term then seek to the next term returned by
  /// `next_seek_term()`.
  NoAndSeek,
  /// Reject the term and terminate enumeration.
  End,
}
pub trait FilteredTermsEnumBase {
  /// Return if term is accepted, not accepted or the iteration should ended
  /// (and possibly seek).
  fn accept(&mut self, term: &BytesRef<Vec<u8>>, ord: i64) -> Result<AcceptStatus>;
  fn next_seek_term(
    &mut self,
    _current: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Err(LuceneError::not_implemented(""))
  }
  fn need_ord(&self) -> bool {
    false
  }
}
