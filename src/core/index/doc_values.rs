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
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::core::index::binary_doc_values::BinaryDocValuesEnum2;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::numeric_doc_values::{NumericDocValues, NumericDocValuesEnum2};
use crate::core::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::core::index::singleton_sorted_set_doc_values::SingletonSortedSetDocValues;
use crate::core::index::sorted_doc_values::{SortedDocValues, SortedDocValuesEnum2};
use crate::core::index::sorted_doc_values_terms_enum::SortedDocValuesTermsEnum;
use crate::core::index::sorted_numeric_doc_values::{
  SortedNumericDocValues, SortedNumericDocValuesEnum2,
};
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::sorted_set_doc_values_writer::SortedSetDocValuesEnum2;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;

pub type EmptySortedSet = SingletonSortedSetDocValues<EmptySorted>;
/// This struct contains utility methods and constants for DocValues
pub struct DocValues;
impl DocValues {
  /// An empty [`BinaryDocValues`] which returns no documents
  pub fn empty_binary() -> EmptyBinary {
    EmptyBinary::new()
  }
  /// An empty [`NumericDocValues`] which returns no documents
  pub fn empty_numeric() -> EmptyNumeric {
    EmptyNumeric::new()
  }
  /// An empty SortedDocValues which returns empty BytesRef for every document
  pub fn empty_sorted() -> EmptySorted {
    EmptySorted::new()
  }
  /// An empty SortedNumericDocValues which returns zero values for every
  /// document.
  pub fn empty_sorted_numeric() -> Result<SingletonSortedNumericDocValues<EmptyNumeric>> {
    Self::singleton_numeric(Self::empty_numeric())
  }
  /// An empty SortedDocValues which returns empty [`BytesRef`] for every
  /// document.
  pub fn empty_sorted_set() -> Result<SingletonSortedSetDocValues<EmptySorted>> {
    Self::singleton_sorted(Self::empty_sorted())
  }

  /// Returns a multi-valued view over the provided SortedDocValues.
  pub fn singleton_sorted<S>(dv: S) -> Result<SingletonSortedSetDocValues<S>>
  where
    S: SortedDocValues,
  {
    SingletonSortedSetDocValues::new(dv)
  }

  /// Returns a single-valued view of the SortedSetDocValues, if it was
  /// previously wrapped with
  /// [`singleton_sorted`](DocValues::singleton_sorted), or None.
  pub fn unwrap_singleton_sorted<S>(dv: &mut S) -> Result<S::SortedDocValues>
  where
    S: SortedSetDocValues,
  {
    dv.get_sorted_doc_values()
  }

  /// Returns a single-valued view of the SortedNumericDocValues, if it was
  /// previously wrapped with
  /// [`singleton_numeric`](DocValues::singleton_numeric), or None.
  pub fn unwrap_singleton_numeric<SN>(dv: &mut SN) -> Result<SN::NumericDocValues>
  where
    SN: SortedNumericDocValues,
  {
    dv.get_numeric_doc_values()
  }
  /// Returns a multi-valued view over the provided NumericDocValues.
  pub fn singleton_numeric<N>(dv: N) -> Result<SingletonSortedNumericDocValues<N>>
  where
    N: NumericDocValues,
  {
    SingletonSortedNumericDocValues::new(dv)
  }

  fn check_field<LR>(reader: &LR, field: &str, expected: &[DocValuesType]) -> Result<()>
  where
    LR: LeafReader,
  {
    if let Some(fi) = reader.get_field_infos()?.field_info_by_name(field) {
      let actual = *fi.get_doc_values_type();
      if !expected.contains(&actual) {
        let expected_str = if expected.len() == 1 {
          format!("={}", expected[0])
        } else {
          format!("one of {expected:?}")
        };
        return Err(LuceneError::illegal_state(format!(
          "unexpected docvalues type {actual} for field '{field}' (expected {expected_str}). Re-index with correct docvalues type."
        )));
      }
    }
    Ok(())
  }
  /// Returns `NumericDocValues` for the field, or [`Self::empty_numeric()`] if it has none.
  ///
  /// # Returns
  ///
  /// A `NumericDocValues` instance, or an empty instance if `field` does not exist in this reader.
  ///
  /// # Error
  ///
  /// - [`LuceneError::IllegalState`] if `field` exists but was not indexed with doc values.  
  /// - [`LuceneError::IllegalState`] if `field` has doc values but the type is not [`DocValuesType::Numeric`].  
  pub fn get_numeric<LR>(reader: &LR, field: &str) -> Result<Numeric<LR>>
  where
    LR: LeafReader,
  {
    match reader.get_numeric_doc_values(field)? {
      Some(dv) => Ok(NumericDocValuesEnum2::A(dv)),
      None => {
        Self::check_field(reader, field, &[DocValuesType::Numeric])?;
        Ok(NumericDocValuesEnum2::B(Self::empty_numeric()))
      },
    }
  }
  /// Returns `BinaryDocValues` for the field, or [`Self::empty_binary()`] if it has none.
  ///
  /// # Returns
  ///
  /// A `BinaryDocValues` instance, or an empty instance if `field` does not exist in this reader.
  ///
  /// # Error
  ///
  /// - [`LuceneError::IllegalState`] if `field` exists but was not indexed with doc values.  
  /// - [`LuceneError::IllegalState`] if `field` has doc values but the type is not [`DocValuesType::Binary`].  
  pub fn get_binary<LR>(reader: &LR, field: &str) -> Result<Binary<LR>>
  where
    LR: LeafReader,
  {
    match reader.get_binary_doc_values(field)? {
      Some(dv) => Ok(BinaryDocValuesEnum2::A(dv)),
      None => {
        Self::check_field(reader, field, &[DocValuesType::Binary])?;
        Ok(BinaryDocValuesEnum2::B(Self::empty_binary()))
      },
    }
  }
  /// Returns `SortedDocValues` for the field, or [`Self::empty_sorted()`] if it has none.
  ///
  /// # Returns
  ///
  /// A `SortedDocValues` instance, or an empty instance if `field` does not exist in this reader.
  ///
  /// # Error
  ///
  /// - [`LuceneError::IllegalState`] if `field` exists but was not indexed with doc values.  
  /// - [`LuceneError::IllegalState`] if `field` has doc values but the type is not [`DocValuesType::Sorted`].  
  pub fn get_sorted<LR>(reader: &LR, field: &str) -> Result<Sorted<LR>>
  where
    LR: LeafReader,
  {
    match reader.get_sorted_doc_values(field)? {
      Some(dv) => Ok(SortedDocValuesEnum2::A(dv)),
      None => {
        Self::check_field(reader, field, &[DocValuesType::Sorted])?;
        Ok(SortedDocValuesEnum2::B(Self::empty_sorted()))
      },
    }
  }
  /// Returns `SortedNumericDocValues` for the field, or [`Self::empty_sorted_numeric()`] if it has none.
  ///
  /// # Returns
  ///
  /// A `SortedNumericDocValues` instance, or an empty instance if `field` does not exist in this reader.
  ///
  /// # Error
  ///
  /// - [`LuceneError::IllegalState`] if `field` exists but was not indexed with doc values.  
  /// - [`LuceneError::IllegalState`] if `field` has doc values but the type is not [`DocValuesType::SortedNumeric`] or [`DocValuesType::Numeric`].  
  pub fn get_sorted_numeric<LR>(reader: &LR, field: &str) -> Result<SortedNumeric<LR>>
  where
    LR: LeafReader,
  {
    match reader.get_sorted_numeric_doc_values(field)? {
      Some(dv) => Ok(SortedNumericDocValuesEnum2::A(dv)),
      None => {
        let v = match reader.get_numeric_doc_values(field)? {
          Some(single) => SortedNumericDocValuesEnum2::A(Self::singleton_numeric(single)?),
          None => {
            Self::check_field(reader, field, &[DocValuesType::SortedNumeric])?;
            SortedNumericDocValuesEnum2::B(Self::empty_sorted_numeric()?)
          },
        };
        Ok(SortedNumericDocValuesEnum2::B(v))
      },
    }
  }
  /// Returns `SortedSetDocValues` for the field, or [`Self::empty_sorted_set()`] if it has none.
  ///
  /// # Returns
  ///
  /// A `SortedSetDocValues` instance, or an empty instance if `field` does not exist in this reader.
  ///
  /// # Error
  ///
  /// - [`LuceneError::IllegalState`] if `field` exists but was not indexed with doc values.  
  /// - [`LuceneError::IllegalState`] if `field` has doc values but the type is not [`DocValuesType::SortedSet`] or [`DocValuesType::Sorted`].  
  pub fn get_sorted_set<LR>(reader: &LR, field: &str) -> Result<SortedSet<LR>>
  where
    LR: LeafReader,
  {
    match reader.get_sorted_set_doc_values(field)? {
      Some(dv) => Ok(SortedSetDocValuesEnum2::A(dv)),
      None => {
        let v = match reader.get_sorted_doc_values(field)? {
          Some(sorted) => SortedSetDocValuesEnum2::A(Self::singleton_sorted(sorted)?),
          None => {
            Self::check_field(
              reader,
              field,
              &[DocValuesType::Sorted, DocValuesType::SortedSet],
            )?;
            SortedSetDocValuesEnum2::B(Self::empty_sorted_set()?)
          },
        };
        Ok(SortedSetDocValuesEnum2::B(v))
      },
    }
  }

  /// Returns `true` if the specified docvalues fields have not been updated
  pub fn is_cacheable<LR>(ctx: &LeafReaderContext<LR>, fields: &[String]) -> Result<bool>
  where
    LR: LeafReader,
  {
    for field in fields {
      if let Some(fi) = ctx.reader().get_field_infos()?.field_info_by_name(field)
        && fi.get_doc_values_gen() > -1
      {
        return Ok(false);
      }
    }
    Ok(true)
  }
}
pub type Numeric<LR> = NumericDocValuesEnum2<<LR as LeafReader>::NumericDocValues, EmptyNumeric>;
pub type Binary<LR> = BinaryDocValuesEnum2<<LR as LeafReader>::BinaryDocValues, EmptyBinary>;
pub type Sorted<LR> = SortedDocValuesEnum2<<LR as LeafReader>::SortedDocValues, EmptySorted>;
pub type SortedNumeric<LR> = SortedNumericDocValuesEnum2<
  <LR as LeafReader>::SortedNumericDocValues,
  SortedNumericDocValuesEnum2<
    SingletonSortedNumericDocValues<<LR as LeafReader>::NumericDocValues>,
    SingletonSortedNumericDocValues<EmptyNumeric>,
  >,
>;
pub type SortedSet<LR> = SortedSetDocValuesEnum2<
  <LR as LeafReader>::SortedSetDocValues,
  SortedSetDocValuesEnum2<
    SingletonSortedSetDocValues<<LR as LeafReader>::SortedDocValues>,
    EmptySortedSet,
  >,
>;
/// An empty [`BinaryDocValues`] which returns no documents  */
pub struct EmptyBinary {
  doc: i32,
  bytes: BytesRef<Vec<u8>>,
}
impl Default for EmptyBinary {
  fn default() -> Self {
    Self::new()
  }
}
impl EmptyBinary {
  fn new() -> Self {
    Self {
      doc: -1,
      bytes: BytesRef::default(),
    }
  }
}

impl DocIdSetIterator for EmptyBinary {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.doc = NO_MORE_DOCS;
    Ok(self.doc)
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    self.doc = NO_MORE_DOCS;
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok(0)
  }
}

impl DocValuesIterator for EmptyBinary {
  fn advance_exact(&mut self, target: i32) -> Result<bool> {
    self.doc = target;
    Ok(false)
  }
}
impl BinaryDocValues for EmptyBinary {
  fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    debug_assert!(
      false,
      "EmptyBinary::binary_value() should not be called, as it is an empty iterator"
    );
    Ok(Cow::Borrowed(&self.bytes))
  }
}
/// An empty [`NumericDocValues`] which returns no documents  */
pub struct EmptyNumeric {
  doc: i32,
}
impl Default for EmptyNumeric {
  fn default() -> Self {
    Self::new()
  }
}

impl EmptyNumeric {
  fn new() -> Self {
    Self { doc: -1 }
  }
}

impl DocValuesIterator for EmptyNumeric {
  fn advance_exact(&mut self, target: i32) -> Result<bool> {
    self.doc = target;
    Ok(false)
  }
}

impl DocIdSetIterator for EmptyNumeric {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.doc = NO_MORE_DOCS;
    Ok(self.doc)
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    self.doc = NO_MORE_DOCS;
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok(0)
  }
}

impl NumericDocValues for EmptyNumeric {
  fn long_value(&mut self) -> Result<i64> {
    debug_assert!(false);
    Ok(0)
  }
}

/// An empty SortedDocValues which returns empty [`BytesRef`] for every
/// document.
pub struct EmptySorted {
  doc: i32,
  empty: BytesRef<Vec<u8>>,
}

impl Default for EmptySorted {
  fn default() -> Self {
    Self::new()
  }
}

impl EmptySorted {
  fn new() -> Self {
    Self {
      doc: -1,
      empty: BytesRef::default(),
    }
  }
}

impl DocValuesIterator for EmptySorted {
  fn advance_exact(&mut self, target: i32) -> Result<bool> {
    self.doc = target;
    Ok(false)
  }
}

impl DocIdSetIterator for EmptySorted {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.doc = NO_MORE_DOCS;
    Ok(self.doc)
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    self.doc = NO_MORE_DOCS;
    Ok(NO_MORE_DOCS)
  }

  fn cost(&self) -> Result<i64> {
    Ok(0)
  }
}

impl SortedDocValues for EmptySorted {
  fn ord_value(&mut self) -> Result<i32> {
    debug_assert!(
      false,
      "EmptySorted should not be called, as it is an empty iterator"
    );
    Ok(-1)
  }

  fn lookup_ord(&mut self, _ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    Ok(Cow::Owned(std::mem::take(&mut self.empty)))
  }

  fn get_value_count(&self) -> Result<i32> {
    Ok(0)
  }

  type TermsEnum<'a> = SortedDocValuesTermsEnum<&'a mut Self>;

  fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
    self.default_terms_enum()
  }
}
