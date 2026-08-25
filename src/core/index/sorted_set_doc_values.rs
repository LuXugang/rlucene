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

use crate::core::index::BytesRef;
use crate::core::index::automaton_terms_enum::AutomatonTermsEnum;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::single_terms_enum::SingleTermsEnum;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::sorted_set_doc_values_terms_enum::SortedSetDocValuesTermsEnum;
use crate::core::index::terms_enum::{
  EmptyTermsEnum, TermsEnum, TermsEnumWithUnsupportedFirstPostings,
};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::ToInt;
use crate::core::util::automation::compiled_automaton::{AutomatonType, CompiledAutomaton};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

/// A multi-valued version of
/// [`SortedDocValues`].
///
/// Per-Document values in a [`SortedSetDocValues`] are deduplicated,
/// dereferenced, and sorted into a dictionary of unique values. A pointer to
/// the dictionary value (ordinal) can be retrieved for each document. Ordinals
/// are dense and in increasing sorted order.
pub trait SortedSetDocValues: DocValuesIterator {
  /// Returns the next ordinal for the current document. It is illegal to call
  /// this method after
  /// [`advance_exact`](DocValuesIterator::advance_exact) returned
  /// `false`. It is illegal to call this more than
  /// [`doc_value_count()`](SortedSetDocValues::doc_value_count) times for the
  /// currently-positioned doc.
  ///
  /// # Returns
  /// Next ordinal for the document. Ordinals are dense, start at 0, then
  /// increment by 1 for the next value in sorted order.
  fn next_ord(&mut self) -> Result<i64>;

  /// Retrieves the number of unique ords for the current document. This must
  /// always be greater than zero. It is illegal to call this method after
  /// [`advance_exact`](DocValuesIterator::advance_exact) returned
  /// `false`.
  fn doc_value_count(&mut self) -> Result<i32>;

  /// Retrieves the value for the specified ordinal. The returned [`BytesRef`]
  /// may be re-used across calls to `lookup_ord`, so make sure to
  /// [`BytesRef::deep_copy_of`] it if you want to keep it around.
  ///
  /// # Arguments
  /// * `ord` - Ordinal to lookup
  ///
  /// See also: [`next_ord`](SortedSetDocValues::next_ord)
  fn lookup_ord(&mut self, _ord: i64) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    Err(LuceneError::need_implemented("this method not implement"))
  }
  /// Returns the number of unique values.
  ///
  /// # Returns
  /// Number of unique values in this [`SortedDocValues`]. This is also
  /// equivalent to one plus the maximum ordinal.
  fn get_value_count(&self) -> Result<i64> {
    Err(LuceneError::need_implemented("this method not implement"))
  }
  /// If `key` exists, returns its ordinal, else returns `-insertion_point -
  /// 1`, like `[T]::binary_search`.
  ///
  /// # Arguments
  /// * `key` - Key to look up
  ///
  /// # Returns
  /// * Ordinal of the key if found, otherwise `-insertion_point - 1`
  fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i64> {
    let mut low = 0;
    let mut high = self.get_value_count()? - 1;

    while low <= high {
      let mid = (low + high) >> 1;
      let term = self.lookup_ord(mid)?;
      let cmp = term.as_ref().cmp(key).to_int();
      if cmp < 0 {
        low = mid + 1;
      } else if cmp > 0 {
        high = mid - 1;
      } else {
        return Ok(mid); // key found
      }
    }
    Ok(-(low + 1)) // key not found
  }
  type TermsEnum<'a>: TermsEnum
  where
    Self: 'a;
  /// Returns a [`TermsEnum`] over the
  /// values. The enum supports
  /// [`TermsEnum::ord()`] and
  /// [`TermsEnum::seek_exact_with_ord()`].
  fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>>;

  fn default_terms_enum(&mut self) -> Result<SortedSetDocValuesTermsEnum<&mut Self>>
  where
    Self: Sized,
  {
    Ok(SortedSetDocValuesTermsEnum::new(self))
  }

  /// Returns a [`TermsEnum`] over the values, filtered by a
  /// [`CompiledAutomaton`]. The enum supports [`TermsEnum::ord`].
  #[allow(clippy::type_complexity)]
  fn intersect(
    &mut self,
    automaton: &CompiledAutomaton,
  ) -> Result<TermsEnumWithUnsupportedFirstPostings<Self::TermsEnum<'_>>>
  where
    Self: Sized,
  {
    let terms_enum = self.terms_enum()?;
    match automaton.type_ {
      AutomatonType::None => Ok(TermsEnumWithUnsupportedFirstPostings::None(EmptyTermsEnum)),
      AutomatonType::All => Ok(TermsEnumWithUnsupportedFirstPostings::All(terms_enum)),
      AutomatonType::Single => Ok(TermsEnumWithUnsupportedFirstPostings::Single(
        SingleTermsEnum::new(
          terms_enum,
          automaton.term.clone().ok_or_else(|| {
            LuceneError::illegal_state("term must exist for AutomatonType::Single")
          })?,
        ),
      )),
      AutomatonType::Normal => Ok(TermsEnumWithUnsupportedFirstPostings::Normal(
        AutomatonTermsEnum::new(terms_enum, automaton)?,
      )),
    }
  }

  fn is_single_valued(&self) -> bool {
    false
  }
  type SortedDocValues: SortedDocValues;
  fn get_sorted_doc_values(&mut self) -> Result<Self::SortedDocValues> {
    Err(LuceneError::unsupported_operation(""))
  }
}
impl<S> SortedSetDocValues for &mut S
where
  S: SortedSetDocValues,
{
  fn next_ord(&mut self) -> Result<i64> {
    (**self).next_ord()
  }

  fn doc_value_count(&mut self) -> Result<i32> {
    (**self).doc_value_count()
  }

  fn lookup_ord(&mut self, ord: i64) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    (**self).lookup_ord(ord)
  }

  fn get_value_count(&self) -> Result<i64> {
    (**self).get_value_count()
  }

  fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i64> {
    (**self).lookup_term(key)
  }

  type TermsEnum<'a>
    = <S as SortedSetDocValues>::TermsEnum<'a>
  where
    Self: 'a;

  fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
    (**self).terms_enum()
  }

  fn is_single_valued(&self) -> bool {
    (**self).is_single_valued()
  }

  type SortedDocValues = S::SortedDocValues;

  fn get_sorted_doc_values(&mut self) -> Result<Self::SortedDocValues> {
    (**self).get_sorted_doc_values()
  }
}

impl<S> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for Rc<RefCell<S>> where
  S: DocIdSetIterator
{
}
impl<S> DocIdSetIterator for Rc<RefCell<S>>
where
  S: DocIdSetIterator,
{
  fn doc_id(&self) -> i32 {
    self.borrow().doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.borrow_mut().next_doc()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.borrow_mut().advance(target)
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.borrow_mut().slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    self.borrow().cost()
  }
}

impl<S> DocValuesIterator for Rc<RefCell<S>>
where
  S: DocValuesIterator,
{
  fn advance_exact(&mut self, target: i32) -> Result<bool> {
    self.borrow_mut().advance_exact(target)
  }
}

impl<S> SortedSetDocValues for Rc<RefCell<S>>
where
  S: SortedSetDocValues,
{
  fn next_ord(&mut self) -> Result<i64> {
    self.borrow_mut().next_ord()
  }

  fn doc_value_count(&mut self) -> Result<i32> {
    self.borrow_mut().doc_value_count()
  }

  fn lookup_ord(&mut self, ord: i64) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    Ok(Cow::Owned(self.borrow_mut().lookup_ord(ord)?.into_owned()))
  }

  fn get_value_count(&self) -> Result<i64> {
    self.borrow().get_value_count()
  }

  fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i64> {
    self.borrow_mut().lookup_term(key)
  }

  type TermsEnum<'a>
    = SortedSetDocValuesTermsEnum<&'a mut Self>
  where
    Self: 'a;

  fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
    self.default_terms_enum()
  }

  fn is_single_valued(&self) -> bool {
    self.borrow().is_single_valued()
  }

  type SortedDocValues = crate::core::index::doc_values::EmptySorted;
}
