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
use crate::core::index::automaton_terms_enum::AutomatonTermsEnum;
use crate::core::index::filtered_terms_enum::{FilteredTermsEnum, FilteredTermsEnumBase};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::terms_enum::{
  EmptyTermsEnum, SeekStatus, TermsEnum, TermsEnumEnum2,
  TermsEnumWithUnsupportedPostingsAndAttributesWithEmpty,
  TermsEnumWithUnsupportedPostingsAndAttributesWithEmptyIntersect,
};
use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

/// Trait representing base term statistics and access.
pub trait Terms {
  type TermsEnum: TermsEnum;
  /// Returns an iterator that will step through all terms. This method will
  /// not return None.
  fn iterator(&self) -> Result<Self::TermsEnum>;

  type IntersectIter: TermsEnum<PostingsEnum = <Self::TermsEnum as TermsEnum>::PostingsEnum>;
  /// Returns a [`TermsEnum`] that iterates over all terms and documents
  /// accepted by the given [`CompiledAutomaton`].
  ///
  /// If `start_term` is provided, the returned enum will only return terms
  /// strictly greater than `start_term`, but you must still call `next()`
  /// first to advance to the first term. The provided `start_term` must
  /// be accepted by the automaton.
  ///
  /// This is an expert-level, low-level API that only works for
  /// [`AutomatonType::NORMAL`](crate::core::util::automation::compiled_automaton::AutomatonType::Normal) compiled automata. To handle any type of
  /// compiled automaton, use
  /// [`CompiledAutomaton::get_terms_enum`](CompiledAutomaton::get_automaton)
  /// instead.
  ///
  /// **Note**: The returned `TermsEnum` does **not** support seeking.
  fn intersect(
    &self,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Self::IntersectIter>;

  fn default_intersect(
    &self,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>>
  where
    Self: Sized,
  {
    let terms_enum = self.iterator()?;
    if start_term.is_some() {
      AutomatonTermsEnum::with_start_term(terms_enum, compiled, start_term)
    } else {
      AutomatonTermsEnum::new(terms_enum, compiled)
    }
  }
  /// Returns the number of terms for this field, or `-1` if this measure
  /// isn't stored by the codec.
  ///
  /// Note that, like other term measures, this value does **not** take
  /// deleted documents into account.
  fn size(&self) -> Result<i64>;

  /// Returns the sum of
  /// [`TermsEnum::total_term_freq`]
  /// for all terms in this field. Note that, like other term measures,
  /// this value does **not** take deleted documents into account.
  fn get_sum_total_term_freq(&self) -> Result<i64>;

  /// Returns the sum of
  /// [`TermsEnum::doc_freq`]
  /// for all terms in this field. Note that, like other term measures,
  /// this value does **not** take deleted documents into account.
  fn get_sum_doc_freq(&self) -> Result<i64>;

  /// Returns the number of documents that have at least one term for this
  /// field. Note that, like other term measures, this value does **not**
  /// take deleted documents into account.
  fn get_doc_count(&self) -> Result<i32>;

  /// Returns `true` if documents in this field store per-document term
  /// frequency
  /// (see [`PostingsEnum::freq`](crate::core::index::postings_enum::PostingsEnum::freq)).
  fn has_freqs(&self) -> bool;

  /// Returns true if documents in this field store offsets.
  fn has_offsets(&self) -> bool;

  /// Returns true if documents in this field store positions.
  fn has_positions(&self) -> bool;

  /// Returns true if documents in this field store payloads.
  fn has_payloads(&self) -> bool;

  /// Returns the smallest term (in lexicographic order) in the field.  
  /// Note that, like other term measures, this does **not** take deleted
  /// documents into account. Returns `None` when there are no terms.
  fn get_min(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    let mut iterator = self.iterator()?;
    match iterator.next()? {
      Some(term) => Ok(Some(Cow::Owned(term.into_owned()))),
      None => Ok(None),
    }
  }

  /// Returns the largest term (in lexicographic order) in the field.  
  /// Note that, like other term measures, this does **not** take deleted
  /// documents into account. Returns `None` when there are no terms.
  fn get_max(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    let size = self.size()?;
    match size.cmp(&0) {
      std::cmp::Ordering::Equal => return Ok(None),
      std::cmp::Ordering::Greater => {
        let seek_result = (|| -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
          let mut iterator = self.iterator()?;
          iterator.seek_exact_with_ord(size - 1)?;
          Ok(Cow::Owned(iterator.term()?.into_owned()))
        })();
        match seek_result {
          Ok(term) => return Ok(Some(term)),
          Err(LuceneError::UnsupportedOperation(_)) => {},
          Err(error) => return Err(error),
        }
      },
      std::cmp::Ordering::Less => {},
    }
    // otherwise: binary search
    let mut iterator = self.iterator()?;
    let v = iterator.next()?;
    if v.is_none() {
      return Ok(None);
    }

    let mut scratch = BytesRefBuilder::new();
    scratch.append_byte(0)?;
    // Iterates over digits:
    loop {
      let mut low = 0;
      let mut high = 256;
      // Binary search current digit to find the highest
      // digit before END:
      while low != high {
        let mid = (((low + high) as u32) >> 1) as i32;
        scratch.set_byte_at(scratch.length() - 1, mid as u8);
        match iterator.seek_ceil(scratch.get_bytes_mut_ref())? {
          SeekStatus::End => {
            if mid == 0 {
              scratch.set_length(scratch.length() - 1);
              return Ok(Some(Cow::Owned(scratch.get_bytes_owner())));
            }
            high = mid;
          },
          _ => {
            if low == mid {
              break;
            }
            low = mid;
          },
        }
      }

      scratch.set_length(scratch.length() + 1);
      scratch.grow(scratch.length())?;
    }
  }

  /// Returns debugging statistics string.
  fn get_stats(&self) -> Result<String> {
    Ok(format!(
      "impl={},size={},docCount={},sumTotalTermFreq={},sumDocFreq={}",
      std::any::type_name::<Self>(),
      self.size()?,
      self.get_doc_count()?,
      self.get_sum_total_term_freq()?,
      self.get_sum_doc_freq()?
    ))
  }
}
impl<T> Terms for Rc<T>
where
  T: Terms,
{
  type TermsEnum = T::TermsEnum;

  fn iterator(&self) -> Result<Self::TermsEnum> {
    (**self).iterator()
  }

  type IntersectIter = T::IntersectIter;

  fn intersect(
    &self,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Self::IntersectIter> {
    (**self).intersect(compiled, start_term)
  }

  fn default_intersect(
    &self,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>>
  where
    Self: Sized,
  {
    (**self).default_intersect(compiled, start_term)
  }

  fn size(&self) -> Result<i64> {
    (**self).size()
  }

  fn get_sum_total_term_freq(&self) -> Result<i64> {
    (**self).get_sum_total_term_freq()
  }

  fn get_sum_doc_freq(&self) -> Result<i64> {
    (**self).get_sum_doc_freq()
  }

  fn get_doc_count(&self) -> Result<i32> {
    (**self).get_doc_count()
  }

  fn has_freqs(&self) -> bool {
    (**self).has_freqs()
  }

  fn has_offsets(&self) -> bool {
    (**self).has_offsets()
  }

  fn has_positions(&self) -> bool {
    (**self).has_positions()
  }

  fn has_payloads(&self) -> bool {
    (**self).has_payloads()
  }

  fn get_min(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    (**self).get_min()
  }

  fn get_max(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    (**self).get_max()
  }

  fn get_stats(&self) -> Result<String> {
    (**self).get_stats()
  }
}

impl<T> Terms for Arc<T>
where
  T: Terms,
{
  type TermsEnum = T::TermsEnum;

  fn iterator(&self) -> Result<Self::TermsEnum> {
    (**self).iterator()
  }

  type IntersectIter = T::IntersectIter;

  fn intersect(
    &self,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Self::IntersectIter> {
    (**self).intersect(compiled, start_term)
  }

  fn default_intersect(
    &self,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>>
  where
    Self: Sized,
  {
    (**self).default_intersect(compiled, start_term)
  }

  fn size(&self) -> Result<i64> {
    (**self).size()
  }

  fn get_sum_total_term_freq(&self) -> Result<i64> {
    (**self).get_sum_total_term_freq()
  }

  fn get_sum_doc_freq(&self) -> Result<i64> {
    (**self).get_sum_doc_freq()
  }

  fn get_doc_count(&self) -> Result<i32> {
    (**self).get_doc_count()
  }

  fn has_freqs(&self) -> bool {
    (**self).has_freqs()
  }

  fn has_offsets(&self) -> bool {
    (**self).has_offsets()
  }

  fn has_positions(&self) -> bool {
    (**self).has_positions()
  }

  fn has_payloads(&self) -> bool {
    (**self).has_payloads()
  }

  fn get_min(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    (**self).get_min()
  }

  fn get_max(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    (**self).get_max()
  }

  fn get_stats(&self) -> Result<String> {
    (**self).get_stats()
  }
}

/// Returns the [`Terms`] index for this field, or [`crate::core::index::terms::Terms::EMPTY`] if it
/// has none.
///
/// Returns:
/// - A `Terms` instance, or an empty instance if the field does not exist
///   in this reader.
///
/// Errors:
/// - Returns an error if an I/O error occurs.
pub(crate) fn get_terms<LR>(reader: &LR, field: &str) -> Result<TermsEnum2Type<LR::Terms>>
where
  LR: LeafReader,
{
  let terms = reader.terms(field)?;
  match terms {
    Some(t) => Ok(TermsWithEmpty::Terms(t)),
    None => Ok(TermsWithEmpty::Empty(EmptyTerms)),
  }
}

pub type TermsEnum2Type<T> = TermsWithEmpty<T>;

pub enum TermsWithEmpty<T> {
  Terms(T),
  Empty(EmptyTerms),
}

impl<T> Terms for TermsWithEmpty<T>
where
  T: Terms,
{
  type TermsEnum = TermsEnumWithUnsupportedPostingsAndAttributesWithEmpty<T::TermsEnum>;

  fn iterator(&self) -> Result<Self::TermsEnum> {
    match self {
      Self::Terms(terms) => terms
        .iterator()
        .map(TermsEnumWithUnsupportedPostingsAndAttributesWithEmpty::WithPostingsAndAttributes),
      Self::Empty(terms) => terms
        .iterator()
        .map(TermsEnumWithUnsupportedPostingsAndAttributesWithEmpty::WithoutPostingsAndAttributes),
    }
  }

  type IntersectIter =
    TermsEnumWithUnsupportedPostingsAndAttributesWithEmptyIntersect<T::IntersectIter>;

  fn intersect(
    &self,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Self::IntersectIter> {
    match self {
      Self::Terms(terms) => terms
        .intersect(compiled, start_term)
        .map(
          TermsEnumWithUnsupportedPostingsAndAttributesWithEmptyIntersect::WithPostingsAndAttributes,
        ),
      Self::Empty(terms) => terms
        .intersect(compiled, start_term)
        .map(
          TermsEnumWithUnsupportedPostingsAndAttributesWithEmptyIntersect::WithoutPostingsAndAttributes,
        ),
    }
  }

  fn size(&self) -> Result<i64> {
    match self {
      Self::Terms(terms) => terms.size(),
      Self::Empty(terms) => terms.size(),
    }
  }

  fn get_sum_total_term_freq(&self) -> Result<i64> {
    match self {
      Self::Terms(terms) => terms.get_sum_total_term_freq(),
      Self::Empty(terms) => terms.get_sum_total_term_freq(),
    }
  }

  fn get_sum_doc_freq(&self) -> Result<i64> {
    match self {
      Self::Terms(terms) => terms.get_sum_doc_freq(),
      Self::Empty(terms) => terms.get_sum_doc_freq(),
    }
  }

  fn get_doc_count(&self) -> Result<i32> {
    match self {
      Self::Terms(terms) => terms.get_doc_count(),
      Self::Empty(terms) => terms.get_doc_count(),
    }
  }

  fn has_freqs(&self) -> bool {
    match self {
      Self::Terms(terms) => terms.has_freqs(),
      Self::Empty(terms) => terms.has_freqs(),
    }
  }

  fn has_offsets(&self) -> bool {
    match self {
      Self::Terms(terms) => terms.has_offsets(),
      Self::Empty(terms) => terms.has_offsets(),
    }
  }

  fn has_positions(&self) -> bool {
    match self {
      Self::Terms(terms) => terms.has_positions(),
      Self::Empty(terms) => terms.has_positions(),
    }
  }

  fn has_payloads(&self) -> bool {
    match self {
      Self::Terms(terms) => terms.has_payloads(),
      Self::Empty(terms) => terms.has_payloads(),
    }
  }

  fn get_min(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match self {
      Self::Terms(terms) => terms.get_min(),
      Self::Empty(terms) => terms.get_min(),
    }
  }

  fn get_max(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match self {
      Self::Terms(terms) => terms.get_max(),
      Self::Empty(terms) => terms.get_max(),
    }
  }

  fn get_stats(&self) -> Result<String> {
    match self {
      Self::Terms(terms) => terms.get_stats(),
      Self::Empty(terms) => terms.get_stats(),
    }
  }
}

#[derive(Default)]
pub struct EmptyTerms;
impl Terms for EmptyTerms {
  type TermsEnum = EmptyTermsEnum;

  fn iterator(&self) -> Result<Self::TermsEnum> {
    Ok(EmptyTermsEnum)
  }

  type IntersectIter
    = FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>
  where
    Self::TermsEnum: BytesRefIterator,
    AutomatonTermsEnum: FilteredTermsEnumBase;

  fn intersect(
    &self,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Self::IntersectIter> {
    self.default_intersect(compiled, start_term)
  }

  fn size(&self) -> Result<i64> {
    Ok(0)
  }

  fn get_sum_total_term_freq(&self) -> Result<i64> {
    Ok(0)
  }

  fn get_sum_doc_freq(&self) -> Result<i64> {
    Ok(0)
  }

  fn get_doc_count(&self) -> Result<i32> {
    Ok(0)
  }

  fn has_freqs(&self) -> bool {
    false
  }

  fn has_offsets(&self) -> bool {
    false
  }

  fn has_positions(&self) -> bool {
    false
  }

  fn has_payloads(&self) -> bool {
    false
  }
}

macro_rules! either_terms {
    ($vis:vis $name:ident => { te: $te:ident, ie: $ie:ident } { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> Terms for $name<$( $T ),+>
        where
            $( $T: Terms ),+
        {
            type TermsEnum     = $te<$( <$T as Terms>::TermsEnum ),+>;
            type IntersectIter = $ie<$( <$T as Terms>::IntersectIter ),+>;


            fn iterator(&self) -> Result<Self::TermsEnum> {
                match self {
                    $(
                        Self::$Variant(inner) => {
                            let it = inner.iterator()?;
                            Ok($te::$Variant(it))
                        }
                    ),+
                }
            }
            fn intersect(
                &self,
                ca: &CompiledAutomaton,
                start: Option<&BytesRef<Vec<u8>>>
            ) -> Result<Self::IntersectIter> {
                match self {
                    $(
                        Self::$Variant(inner) => {
                            let it = inner.intersect(ca, start)?;
                            Ok($ie::$Variant(it))
                        }
                    ),+
                }
            }

            fn size(&self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => inner.size(), )+ }
            }

            fn get_doc_count(&self) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.get_doc_count(), )+ }
            }

            fn get_sum_doc_freq(&self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => inner.get_sum_doc_freq(), )+ }
            }

            fn get_sum_total_term_freq(&self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => inner.get_sum_total_term_freq(), )+ }
            }


            fn has_freqs(&self) -> bool {
                match self { $( Self::$Variant(inner) => inner.has_freqs(), )+ }
            }

            fn has_offsets(&self) -> bool {
                match self { $( Self::$Variant(inner) => inner.has_offsets(), )+ }
            }

            fn has_positions(&self) -> bool {
                match self { $( Self::$Variant(inner) => inner.has_positions(), )+ }
            }

            fn has_payloads(&self) -> bool {
                match self { $( Self::$Variant(inner) => inner.has_payloads(), )+ }
            }

            fn get_min(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
                match self { $( Self::$Variant(inner) => inner.get_min(), )+ }
            }

            fn get_max(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
                match self { $( Self::$Variant(inner) => inner.get_max(), )+ }
            }

            fn get_stats(&self) -> Result<String> {
                match self { $( Self::$Variant(inner) => inner.get_stats(), )+ }
            }
        }
    };
}
either_terms!(
    pub TermsEnum2
    => { te: TermsEnumEnum2, ie: TermsEnumEnum2 }
    { A:A,B:B}
);
pub type TermsTE<T> = <T as Terms>::TermsEnum;
pub type TermsIntersect<T> = <T as Terms>::IntersectIter;

pub type TermsPosting<T> = <TermsTE<T> as TermsEnum>::PostingsEnum;
pub type TermsIntersectPosting<T> = <TermsIntersect<T> as TermsEnum>::PostingsEnum;
