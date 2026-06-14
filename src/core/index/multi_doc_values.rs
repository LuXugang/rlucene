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
use crate::core::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::core::index::BytesRef;
use crate::core::index::binary_doc_values::{BinaryDocValues, BinaryDocValuesEnum2};
use crate::core::index::composite_reader::{CompositeReader, get_context};
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::doc_values::{DocValues, EmptyNumeric, EmptySorted, EmptySortedSet};
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::index_reader::CacheHelper;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::{
  LRBinaryDocValues, LRNormNumericDocValues, LRNumericDocValues, LRSortedDocValues,
  LRSortedNumericDocValues, LRSortedSetDocValues, LeafReader,
};
use crate::core::index::numeric_doc_values::{NumericDocValues, NumericDocValuesEnum2};
use crate::core::index::ordinal_map::OrdinalMap;
use crate::core::index::reader_util::ReaderUtil;
use crate::core::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::core::index::sorted_doc_values::{SortedDocValues, SortedDocValuesEnum2};
use crate::core::index::sorted_doc_values_terms_enum::SortedDocValuesTermsEnum;
use crate::core::index::sorted_numeric_doc_values::{
  SortedNumericDocValues, SortedNumericDocValuesEnum2,
};
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::sorted_set_doc_values_terms_enum::SortedSetDocValuesTermsEnum;
use crate::core::index::sorted_set_doc_values_writer::SortedSetDocValuesEnum2;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;

use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::PackedInts;
use std::borrow::Cow;
use std::sync::Arc;

/// A wrapper for `CompositeIndexReader` providing access to `DocValues`.
///
/// **NOTE**: for multi readers, you'll get better performance by gathering the
/// sub readers using `IndexReader::get_context` to get the atomic leaves and
/// then operate per-`LeafReader` instead of using this type.
///
/// **NOTE**: This is very costly.
pub struct MultiDocValues;

pub type MultiNormNumericDocValues<CR> = NumericDocValuesEnum2<
  LRNormNumericDocValues<<CR as CompositeReader>::LeafReader>,
  NumericDocValuesImpl<CR>,
>;
pub type MultiNumericDocValues<CR> = NumericDocValuesEnum2<
  LRNumericDocValues<<CR as CompositeReader>::LeafReader>,
  NumericDocValuesImpl1<CR>,
>;
pub type MultiBinaryDocValues<CR> = BinaryDocValuesEnum2<
  LRBinaryDocValues<<CR as CompositeReader>::LeafReader>,
  BinaryDocValuesImpl<CR>,
>;
pub type MultiSortedNumericDocValues<CR> = SortedNumericDocValuesEnum2<
  LRSortedNumericDocValues<<CR as CompositeReader>::LeafReader>,
  SortedNumericDocValuesImpl<CR>,
>;
pub type MultiSortedDocValuesType<CR> = SortedDocValuesEnum2<
  LRSortedDocValues<<CR as CompositeReader>::LeafReader>,
  MultiSortedDocValues<
    SortedDocValuesEnum2<LRSortedDocValues<<CR as CompositeReader>::LeafReader>, EmptySorted>,
  >,
>;
pub type MultiSortedSetDocValuesType<CR> = SortedSetDocValuesEnum2<
  LRSortedSetDocValues<<CR as CompositeReader>::LeafReader>,
  MultiSortedSetDocValues<
    SortedSetDocValuesEnum2<
      LRSortedSetDocValues<<CR as CompositeReader>::LeafReader>,
      EmptySortedSet,
    >,
  >,
>;

impl MultiDocValues {
  ///  Returns a NumericDocValues for a reader's norms (potentially merging on-the-fly).
  pub fn get_norm_values<CR>(
    reader: CR,
    field: &str,
  ) -> Result<Option<MultiNormNumericDocValues<CR>>>
  where
    CR: CompositeReader,
  {
    let reader = get_context(reader)?;
    let leaves = reader.leaves()?;
    let size = leaves.len();

    if size == 0 {
      return Ok(None);
    } else if size == 1 {
      return match leaves[0].reader().get_norm_values(field)? {
        Some(v) => Ok(Some(MultiNormNumericDocValues::A(v))),
        None => Ok(None),
      };
    }
    // Check if any of the leaf reader which has this field has norms.
    let mut norm_found = false;
    for leaf in leaves.iter() {
      if let Some(info) = leaf.reader().get_field_infos()?.field_info_by_name(field)
        && info.has_norms()
      {
        norm_found = true;
        break;
      }
    }

    if !norm_found {
      return Ok(None);
    }
    Ok(Some(MultiNormNumericDocValues::B(
      NumericDocValuesImpl::new(reader, field.to_string()),
    )))
  }

  /// Returns a NumericDocValues for a reader's docvalues (potentially merging on-the-fly)
  pub fn get_numeric_values<CR>(
    reader: CR,
    field: &str,
  ) -> Result<Option<MultiNumericDocValues<CR>>>
  where
    CR: CompositeReader,
  {
    let reader = get_context(reader)?;
    let leaves = reader.leaves()?;
    let size = leaves.len();

    if size == 0 {
      return Ok(None);
    } else if size == 1 {
      return match leaves[0].reader().get_numeric_doc_values(field)? {
        Some(v) => Ok(Some(MultiNumericDocValues::A(v))),
        None => Ok(None),
      };
    }

    let mut any_real = false;
    for leaf in leaves.iter() {
      if let Some(info) = leaf.reader().get_field_infos()?.field_info_by_name(field)
        && *info.get_doc_values_type() == DocValuesType::Numeric
      {
        any_real = true;
        break;
      }
    }

    if !any_real {
      return Ok(None);
    }

    Ok(Some(MultiNumericDocValues::B(NumericDocValuesImpl1::new(
      reader,
      field.to_string(),
    ))))
  }

  /// Returns a BinaryDocValues for a reader's docvalues (potentially merging on-the-fly)
  pub fn get_binary_values<CR>(reader: CR, field: &str) -> Result<Option<MultiBinaryDocValues<CR>>>
  where
    CR: CompositeReader,
  {
    let reader = get_context(reader)?;
    let leaves = reader.leaves()?;
    let size = leaves.len();

    if size == 0 {
      return Ok(None);
    } else if size == 1 {
      return match leaves[0].reader().get_binary_doc_values(field)? {
        Some(v) => Ok(Some(MultiBinaryDocValues::A(v))),
        None => Ok(None),
      };
    }

    let mut any_real = false;
    for leaf in leaves.iter() {
      if let Some(info) = leaf.reader().get_field_infos()?.field_info_by_name(field)
        && *info.get_doc_values_type() == DocValuesType::Binary
      {
        any_real = true;
        break;
      }
    }

    if !any_real {
      return Ok(None);
    }

    Ok(Some(MultiBinaryDocValues::B(BinaryDocValuesImpl::new(
      reader,
      field.to_string(),
    ))))
  }
  pub fn get_sorted_numeric_values<CR>(
    reader: CR,
    field: &str,
  ) -> Result<Option<MultiSortedNumericDocValues<CR>>>
  where
    CR: CompositeReader,
  {
    let reader = get_context(reader)?;
    let leaves = reader.leaves()?;
    let size = leaves.len();

    if size == 0 {
      return Ok(None);
    } else if size == 1 {
      return match leaves[0].reader().get_sorted_numeric_doc_values(field)? {
        Some(v) => Ok(Some(MultiSortedNumericDocValues::A(v))),
        None => Ok(None),
      };
    }

    let mut any_real = false;
    let mut values = Vec::with_capacity(size);
    let mut total_cost = 0i64;

    for leaf in leaves.iter() {
      let v = leaf.reader().get_sorted_numeric_doc_values(field)?;
      let dv = match v {
        Some(v) => {
          any_real = true;
          SortedNumericDocValuesEnum2::B(v)
        },
        None => SortedNumericDocValuesEnum2::A(DocValues::empty_sorted_numeric()?),
      };

      total_cost += dv.cost()?;
      values.push(dv);
    }

    if !any_real {
      return Ok(None);
    }

    Ok(Some(MultiSortedNumericDocValues::B(
      SortedNumericDocValuesImpl::new(reader, values, field.to_string(), total_cost),
    )))
  }
  /// Returns a [`SortedDocValues`] for a reader's docvalues (potentially doing extremely slow things).
  ///
  /// This is an extremely slow way to access sorted values. Instead, access them per-segment with
  /// [`LeafReader::get_sorted_doc_values`].
  pub fn get_sorted_values<CR>(r: CR, field: &str) -> Result<Option<MultiSortedDocValuesType<CR>>>
  where
    CR: CompositeReader,
  {
    let max_doc = r.max_doc()?;
    let reader = get_context(r)?;
    let leaves = reader.leaves()?;
    let size = leaves.len();

    if size == 0 {
      return Ok(None);
    } else if size == 1 {
      return match leaves[0].reader().get_sorted_doc_values(field)? {
        Some(v) => Ok(Some(MultiSortedDocValuesType::<CR>::A(v))),
        None => Ok(None),
      };
    }

    let mut any_real = false;
    let mut values = Vec::with_capacity(size);
    let mut starts: Vec<usize> = Vec::with_capacity(size + 1);
    let mut total_cost: i64 = 0;

    for ctx in leaves.iter() {
      let v = match ctx.reader().get_sorted_doc_values(field)? {
        Some(s) => {
          any_real = true;
          total_cost += s.cost()?;
          SortedDocValuesEnum2::A(s)
        },
        None => SortedDocValuesEnum2::B(DocValues::empty_sorted()),
      };

      values.push(v);
      starts.push(ctx.doc_base);
    }

    starts.push(max_doc as usize);

    if !any_real {
      Ok(None)
    } else {
      let owner = reader
        .reader()
        .get_reader_cache_helper()?
        .map(|helper| helper.get_key());

      let mapping =
        OrdinalMap::build_from_sorted(owner, values.as_mut_slice(), PackedInts::DEFAULT)?;

      Ok(Some(MultiSortedDocValuesType::<CR>::B(
        MultiSortedDocValues::new(starts, values, mapping, total_cost),
      )))
    }
  }
  /// Returns a [`SortedSetDocValues`] for a reader's docvalues (potentially doing extremely slow
  /// things).
  ///
  /// This is an extremely slow way to access sorted values. Instead, access them per-segment with
  /// [`LeafReader::get_sorted_set_doc_values`].
  pub fn get_sorted_set_values<CR>(
    r: CR,
    field: &str,
  ) -> Result<Option<MultiSortedSetDocValuesType<CR>>>
  where
    CR: CompositeReader,
  {
    let max_doc = r.max_doc()?;
    let reader = get_context(r)?;
    let leaves = reader.leaves()?;
    let size = leaves.len();

    if size == 0 {
      return Ok(None);
    } else if size == 1 {
      return match leaves[0].reader().get_sorted_set_doc_values(field)? {
        Some(v) => Ok(Some(MultiSortedSetDocValuesType::<CR>::A(v))),
        None => Ok(None),
      };
    }

    let mut any_real = false;
    let mut values = Vec::with_capacity(size);
    let mut starts: Vec<usize> = Vec::with_capacity(size + 1);
    let mut total_cost: i64 = 0;

    for ctx in leaves.iter() {
      let v = match ctx.reader().get_sorted_set_doc_values(field)? {
        Some(s) => {
          any_real = true;
          total_cost += s.cost()?;
          SortedSetDocValuesEnum2::A(s)
        },
        None => SortedSetDocValuesEnum2::B(DocValues::empty_sorted_set()?),
      };

      values.push(v);
      starts.push(ctx.doc_base);
    }

    starts.push(max_doc as usize);

    if !any_real {
      Ok(None)
    } else {
      let owner = reader
        .reader()
        .get_reader_cache_helper()?
        .map(|helper| helper.get_key());

      let mapping =
        OrdinalMap::build_from_sorted_set(owner, values.as_mut_slice(), PackedInts::DEFAULT)?;

      Ok(Some(MultiSortedSetDocValuesType::<CR>::B(
        MultiSortedSetDocValues::new(values, starts, mapping, total_cost),
      )))
    }
  }
}
/// Implements SortedDocValues over n subs, using an OrdinalMap
pub struct MultiSortedDocValues<S>
where
  S: SortedDocValues,
{
  /// docbase for each leaf: parallel with `values`
  pub doc_starts: Arc<Vec<usize>>,
  /// leaf values
  pub values: Vec<S>,
  /// ordinal map mapping ords from `values` to global ord space
  pub mapping: Arc<OrdinalMap>,

  total_cost: i64,
  next_leaf: usize,
  current_values: Option<usize>,
  current_doc_start: i32,
  doc_id: i32,
}

impl<S> MultiSortedDocValues<S>
where
  S: SortedDocValues,
{
  pub fn new<T, O>(doc_starts: T, values: Vec<S>, mapping: O, total_cost: i64) -> Self
  where
    T: Into<Arc<Vec<usize>>>,
    O: Into<Arc<OrdinalMap>>,
  {
    let doc_starts = doc_starts.into();
    let mapping = mapping.into();
    Self {
      doc_starts,
      values,
      mapping,
      total_cost,
      next_leaf: 0,
      current_values: None,
      current_doc_start: 0,
      doc_id: -1,
    }
  }
}

impl<S> DocValuesIterator for MultiSortedDocValues<S>
where
  S: SortedDocValues,
{
  fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
    if target_doc_id < self.doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "can only advance beyond current document: on docID={} but targetDocID={}",
        self.doc_id, target_doc_id
      )));
    }

    let reader_index = ReaderUtil::sub_index(target_doc_id as usize, &self.doc_starts);
    if reader_index < 0 {
      return Err(LuceneError::illegal_state("reader_index should be >= 0"));
    }
    let reader_index = reader_index as usize;
    if reader_index >= self.next_leaf {
      if reader_index == self.values.len() {
        return Err(LuceneError::illegal_argument(format!(
          "Out of range: {}",
          target_doc_id
        )));
      }
      self.current_doc_start = self.doc_starts[reader_index] as i32;
      self.current_values = Some(reader_index);
      self.next_leaf = reader_index + 1;
    }

    self.doc_id = target_doc_id;

    let idx = match self.current_values {
      None => return Ok(false),
      Some(i) => i,
    };

    // delegate to leaf-level advanceExact()
    let exists = self.values[idx].advance_exact(target_doc_id - self.current_doc_start)?;

    Ok(exists)
  }
}

impl<S> DocIdSetIterator for MultiSortedDocValues<S>
where
  S: SortedDocValues,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    loop {
      while self.current_values.is_none() {
        if self.next_leaf == self.values.len() {
          self.doc_id = NO_MORE_DOCS;
          return Ok(self.doc_id);
        }
        self.current_doc_start = self.doc_starts[self.next_leaf] as i32;
        self.current_values = Some(self.next_leaf);
        self.next_leaf += 1;
      }

      let new_doc_id = self.values[*self.current_values.as_ref().unwrap()].next_doc()?;

      if new_doc_id == NO_MORE_DOCS {
        self.current_values = None;
      } else {
        self.doc_id = self.current_doc_start + new_doc_id;
        return Ok(self.doc_id);
      }
    }
  }

  fn advance(&mut self, target_doc_id: i32) -> Result<i32> {
    if target_doc_id <= self.doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "can only advance beyond current document: on docID={} but targetDocID={}",
        self.doc_id, target_doc_id
      )));
    }

    let reader_index = ReaderUtil::sub_index(target_doc_id as usize, &self.doc_starts);
    if reader_index < 0 {
      return Err(LuceneError::illegal_state("reader_index should be >= 0"));
    }
    let reader_index = reader_index as usize;
    if reader_index >= self.next_leaf {
      if reader_index == self.values.len() {
        self.current_values = None;
        self.doc_id = NO_MORE_DOCS;
        return Ok(self.doc_id);
      }
      self.current_doc_start = self.doc_starts[reader_index] as i32;
      self.current_values = Some(reader_index);
      self.next_leaf = reader_index + 1;
    }

    let idx = *self.current_values.as_ref().unwrap();
    let new_doc_id = self.values[idx].advance(target_doc_id - self.current_doc_start)?;

    if new_doc_id == NO_MORE_DOCS {
      self.current_values = None;
      self.next_doc()
    } else {
      self.doc_id = self.current_doc_start + new_doc_id;
      Ok(self.doc_id)
    }
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.total_cost)
  }
}

impl<S> SortedDocValues for MultiSortedDocValues<S>
where
  S: SortedDocValues,
{
  fn ord_value(&mut self) -> Result<i32> {
    let seg_idx = match self.current_values {
      Some(i) => i,
      None => return Err(LuceneError::illegal_state("current_values is None")),
    };

    let local_ord = self.values[seg_idx].ord_value()? as usize;

    let global_ord = self
      .mapping
      .get_global_ords(self.next_leaf - 1)
      .get(local_ord)?;

    Ok(global_ord as i32)
  }

  fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    let sub_index: usize = self
      .mapping
      .get_first_segment_number(ord as usize)?
      .try_convert()?;
    let segment_ord = self
      .mapping
      .get_first_segment_ord(ord as usize)?
      .try_convert()?;
    self.values[sub_index].lookup_ord(segment_ord)
  }

  fn get_value_count(&self) -> Result<i32> {
    self.mapping.get_value_count().try_convert()
  }

  type TermsEnum<'a>
    = SortedDocValuesTermsEnum<&'a mut Self>
  where
    S: 'a;

  fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
    self.default_terms_enum()
  }
}

/// Implements SortedSetDocValues over N subs, using an OrdinalMap.
pub struct MultiSortedSetDocValues<T>
where
  T: SortedSetDocValues,
{
  /// docbase for each leaf: parallel with `values`
  pub doc_starts: Arc<Vec<usize>>,

  /// leaf values
  pub values: Vec<T>,

  /// ordinal map mapping ords from `values` to global ord space
  pub mapping: Arc<OrdinalMap>,

  total_cost: i64,
  next_leaf: usize,
  current_values: Option<usize>,
  current_doc_start: i32,
  doc_id: i32,
}

impl<T> MultiSortedSetDocValues<T>
where
  T: SortedSetDocValues,
{
  pub fn new<V, R>(values: Vec<T>, doc_starts: R, mapping: V, total_cost: i64) -> Self
  where
    V: Into<Arc<OrdinalMap>>,
    R: Into<Arc<Vec<usize>>>,
  {
    let doc_starts = doc_starts.into();
    let mapping = mapping.into();
    debug_assert_eq!(doc_starts.len(), values.len() + 1);
    Self {
      doc_starts,
      values,
      mapping,
      total_cost,
      next_leaf: 0,
      current_values: None,
      current_doc_start: 0,
      doc_id: -1,
    }
  }
}

impl<T> DocValuesIterator for MultiSortedSetDocValues<T>
where
  T: SortedSetDocValues,
{
  fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
    if target_doc_id < self.doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "can only advance beyond current document: on docID={} but targetDocID={}",
        self.doc_id, target_doc_id
      )));
    }

    let reader_index = ReaderUtil::sub_index(target_doc_id as usize, &self.doc_starts);
    if reader_index < 0 {
      return Err(LuceneError::illegal_state("reader_index should be >= 0"));
    }
    let reader_index = reader_index as usize;

    if reader_index >= self.next_leaf {
      if reader_index == self.values.len() {
        return Err(LuceneError::illegal_argument(format!(
          "Out of range: {}",
          target_doc_id
        )));
      }
      self.current_doc_start = self.doc_starts[reader_index] as i32;
      self.current_values = Some(reader_index);
      self.next_leaf = reader_index + 1;
    }

    self.doc_id = target_doc_id;

    let idx = match self.current_values {
      None => return Ok(false),
      Some(i) => i,
    };

    self.values[idx].advance_exact(target_doc_id - self.current_doc_start)
  }
}

impl<T> DocIdSetIterator for MultiSortedSetDocValues<T>
where
  T: SortedSetDocValues,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    loop {
      while self.current_values.is_none() {
        if self.next_leaf == self.values.len() {
          self.doc_id = NO_MORE_DOCS;
          return Ok(self.doc_id);
        }

        self.current_doc_start = self.doc_starts[self.next_leaf] as i32;
        self.current_values = Some(self.next_leaf);
        self.next_leaf += 1;
      }

      let idx = *self.current_values.as_ref().unwrap();
      let new_doc_id = self.values[idx].next_doc()?;

      if new_doc_id == NO_MORE_DOCS {
        self.current_values = None;
      } else {
        self.doc_id = self.current_doc_start + new_doc_id;
        return Ok(self.doc_id);
      }
    }
  }

  fn advance(&mut self, target_doc_id: i32) -> Result<i32> {
    if target_doc_id <= self.doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "can only advance beyond current document: on docID={} but targetDocID={}",
        self.doc_id, target_doc_id
      )));
    }

    let reader_index = ReaderUtil::sub_index(target_doc_id as usize, &self.doc_starts);
    if reader_index < 0 {
      return Err(LuceneError::illegal_state("reader_index should be >= 0"));
    }
    let reader_index = reader_index as usize;

    if reader_index >= self.next_leaf {
      if reader_index == self.values.len() {
        self.current_values = None;
        self.doc_id = NO_MORE_DOCS;
        return Ok(self.doc_id);
      }

      self.current_doc_start = self.doc_starts[reader_index] as i32;
      self.current_values = Some(reader_index);
      self.next_leaf = reader_index + 1;
    }

    let idx = *self.current_values.as_ref().unwrap();
    let new_doc_id = self.values[idx].advance(target_doc_id - self.current_doc_start)?;

    if new_doc_id == NO_MORE_DOCS {
      self.current_values = None;
      self.next_doc()
    } else {
      self.doc_id = self.current_doc_start + new_doc_id;
      Ok(self.doc_id)
    }
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.total_cost)
  }
}

impl<T> SortedSetDocValues for MultiSortedSetDocValues<T>
where
  T: SortedSetDocValues,
{
  fn next_ord(&mut self) -> Result<i64> {
    let idx = match self.current_values {
      Some(i) => i,
      None => return Err(LuceneError::illegal_state("current_values is None")),
    };

    let segment_ord = self.values[idx].next_ord()? as usize;
    let global = self
      .mapping
      .get_global_ords(self.next_leaf - 1)
      .get(segment_ord)?;

    Ok(global)
  }

  fn doc_value_count(&mut self) -> Result<i32> {
    let idx = self
      .current_values
      .ok_or_else(|| LuceneError::illegal_state("current_values is None"))?;
    self.values[idx].doc_value_count()
  }

  fn lookup_ord(&mut self, ord: i64) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    let sub_index: usize = self
      .mapping
      .get_first_segment_number(ord as usize)?
      .try_convert()?;
    let segment_ord = self.mapping.get_first_segment_ord(ord as usize)?;
    self.values[sub_index].lookup_ord(segment_ord)
  }

  fn get_value_count(&self) -> Result<i64> {
    Ok(self.mapping.get_value_count())
  }

  type TermsEnum<'a>
    = SortedSetDocValuesTermsEnum<&'a mut Self>
  where
    T: 'a;

  fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
    self.default_terms_enum()
  }

  type SortedDocValues = DummySortedDocValues;
}

pub struct NumericDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  next_leaf: usize,
  current_values: Option<LRNormNumericDocValues<CR::LeafReader>>,
  reader: CompositeReaderContext<CR>,
  doc_id: i32,
  field: String,
  current_doc_base: usize,
}
impl<CR> NumericDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  pub fn new(reader: CompositeReaderContext<CR>, field: String) -> Self {
    Self {
      next_leaf: 0,
      current_values: None,
      reader,
      doc_id: -1,
      field,
      current_doc_base: 0,
    }
  }
}

impl<CR> DocValuesIterator for NumericDocValuesImpl<CR> where CR: CompositeReader {}

impl<CR> DocIdSetIterator for NumericDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    let leaves = self.reader.leaves()?;
    loop {
      if self.current_values.is_none() {
        if self.next_leaf == leaves.len() {
          self.doc_id = NO_MORE_DOCS;
          return Ok(self.doc_id);
        }

        let leaf = &leaves[self.next_leaf];
        self.current_doc_base = leaf.doc_base;
        self.current_values = leaf.reader().get_norm_values(&self.field)?;

        self.next_leaf += 1;
        continue;
      }

      let new_doc_id = self.current_values.as_mut().unwrap().next_doc()?;

      if new_doc_id == NO_MORE_DOCS {
        self.current_values = None;
      } else {
        self.doc_id = self.current_doc_base as i32 + new_doc_id;
        return Ok(self.doc_id);
      }
    }
  }

  fn advance(&mut self, target_doc_id: i32) -> Result<i32> {
    let leaves = self.reader.leaves()?;
    if target_doc_id <= self.doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "can only advance beyond current document: on docID={} but targetDocID={}",
        self.doc_id, target_doc_id
      )));
    }

    let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

    if reader_index >= self.next_leaf {
      if reader_index == leaves.len() {
        self.current_values = None;
        self.doc_id = NO_MORE_DOCS;
        return Ok(self.doc_id);
      }

      let leaf = &leaves[reader_index];
      self.current_doc_base = leaf.doc_base;
      self.current_values = leaf.reader().get_norm_values(&self.field)?;

      if self.current_values.is_none() {
        return self.next_doc();
      }

      self.next_leaf = reader_index + 1;
    }

    let new_doc_id = self
      .current_values
      .as_mut()
      .unwrap()
      .advance(target_doc_id - self.current_doc_base as i32)?;

    if new_doc_id == NO_MORE_DOCS {
      self.current_values = None;
      self.next_doc()
    } else {
      self.doc_id = self.current_doc_base as i32 + new_doc_id;
      Ok(self.doc_id)
    }
  }

  fn cost(&self) -> Result<i64> {
    Ok(0)
  }
}

impl<CR> NumericDocValues for NumericDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  fn long_value(&mut self) -> Result<i64> {
    match self.current_values {
      Some(ref mut values) => values.long_value(),
      None => Err(LuceneError::illegal_state("current_values is none")),
    }
  }
}

pub struct NumericDocValuesImpl1<CR>
where
  CR: CompositeReader,
{
  next_leaf: usize,
  current_values: Option<LRNumericDocValues<CR::LeafReader>>,
  reader: CompositeReaderContext<CR>,
  doc_id: i32,
  field: String,
  current_doc_base: usize,
}

impl<CR> NumericDocValuesImpl1<CR>
where
  CR: CompositeReader,
{
  pub fn new(reader: CompositeReaderContext<CR>, field: String) -> Self {
    Self {
      next_leaf: 0,
      current_values: None,
      reader,
      doc_id: -1,
      field,
      current_doc_base: 0,
    }
  }
}

impl<CR> DocValuesIterator for NumericDocValuesImpl1<CR>
where
  CR: CompositeReader,
{
  fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
    let leaves = self.reader.leaves()?;
    if target_doc_id < self.doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "can only advance beyond current document: on docID={} but targetDocID={}",
        self.doc_id, target_doc_id
      )));
    }

    let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

    if reader_index >= self.next_leaf {
      if reader_index == leaves.len() {
        return Err(LuceneError::illegal_argument(format!(
          "Out of range: {}",
          target_doc_id
        )));
      }

      let leaf = &leaves[reader_index];
      self.current_doc_base = leaf.doc_base;
      self.current_values = leaf.reader().get_numeric_doc_values(&self.field)?;
      self.next_leaf = reader_index + 1;
    }

    self.doc_id = target_doc_id;

    match self.current_values {
      None => Ok(false),
      Some(ref mut v) => v.advance_exact(target_doc_id - self.current_doc_base as i32),
    }
  }
}

impl<CR> DocIdSetIterator for NumericDocValuesImpl1<CR>
where
  CR: CompositeReader,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    let leaves = self.reader.leaves()?;
    loop {
      while self.current_values.is_none() {
        if self.next_leaf == leaves.len() {
          self.doc_id = NO_MORE_DOCS;
          return Ok(self.doc_id);
        }
        let leaf = &leaves[self.next_leaf];
        self.current_doc_base = leaf.doc_base;
        self.current_values = leaf.reader().get_numeric_doc_values(&self.field)?;
        self.next_leaf += 1;
      }

      let new_doc_id = self.current_values.as_mut().unwrap().next_doc()?;

      if new_doc_id == NO_MORE_DOCS {
        self.current_values = None;
      } else {
        self.doc_id = self.current_doc_base as i32 + new_doc_id;
        return Ok(self.doc_id);
      }
    }
  }

  fn advance(&mut self, target_doc_id: i32) -> Result<i32> {
    let leaves = self.reader.leaves()?;
    if target_doc_id <= self.doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "can only advance beyond current document: on docID={} but targetDocID={}",
        self.doc_id, target_doc_id
      )));
    }

    let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

    if reader_index >= self.next_leaf {
      if reader_index == leaves.len() {
        self.current_values = None;
        self.doc_id = NO_MORE_DOCS;
        return Ok(self.doc_id);
      }
      let leaf = &leaves[reader_index];
      self.current_doc_base = leaf.doc_base;
      self.current_values = leaf.reader().get_numeric_doc_values(&self.field)?;
      self.next_leaf = reader_index + 1;

      if self.current_values.is_none() {
        return self.next_doc();
      }
    }

    let new_doc_id = self
      .current_values
      .as_mut()
      .unwrap()
      .advance(target_doc_id - self.current_doc_base as i32)?;

    if new_doc_id == NO_MORE_DOCS {
      self.current_values = None;
      self.next_doc()
    } else {
      self.doc_id = self.current_doc_base as i32 + new_doc_id;
      Ok(self.doc_id)
    }
  }

  fn cost(&self) -> Result<i64> {
    Ok(0)
  }
}

impl<CR> NumericDocValues for NumericDocValuesImpl1<CR>
where
  CR: CompositeReader,
{
  fn long_value(&mut self) -> Result<i64> {
    match self.current_values {
      Some(ref mut values) => values.long_value(),
      None => Err(LuceneError::illegal_state("current_values is none")),
    }
  }
}

pub struct BinaryDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  next_leaf: usize,
  current_values: Option<LRBinaryDocValues<CR::LeafReader>>,
  reader: CompositeReaderContext<CR>,
  doc_id: i32,
  field: String,
  current_doc_base: usize,
}

impl<CR> BinaryDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  pub fn new(reader: CompositeReaderContext<CR>, field: String) -> Self {
    Self {
      next_leaf: 0,
      current_values: None,
      reader,
      doc_id: -1,
      field,
      current_doc_base: 0,
    }
  }
}
impl<CR> DocValuesIterator for BinaryDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
    let leaves = self.reader.leaves()?;
    if target_doc_id < self.doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "can only advance beyond current document: on docID={} but targetDocID={}",
        self.doc_id, target_doc_id
      )));
    }

    let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

    if reader_index >= self.next_leaf {
      if reader_index == leaves.len() {
        return Err(LuceneError::illegal_argument(format!(
          "Out of range: {}",
          target_doc_id
        )));
      }

      let leaf = &leaves[reader_index];
      self.current_doc_base = leaf.doc_base;
      self.current_values = leaf.reader().get_binary_doc_values(&self.field)?;
      self.next_leaf = reader_index + 1;
    }

    self.doc_id = target_doc_id;

    match self.current_values {
      None => Ok(false),
      Some(ref mut v) => v.advance_exact(target_doc_id - self.current_doc_base as i32),
    }
  }
}
impl<CR> DocIdSetIterator for BinaryDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    let leaves = self.reader.leaves()?;
    loop {
      while self.current_values.is_none() {
        if self.next_leaf == leaves.len() {
          self.doc_id = NO_MORE_DOCS;
          return Ok(self.doc_id);
        }

        let leaf = &leaves[self.next_leaf];
        self.current_doc_base = leaf.doc_base;
        self.current_values = leaf.reader().get_binary_doc_values(&self.field)?;
        self.next_leaf += 1;
      }

      let new_doc_id = self.current_values.as_mut().unwrap().next_doc()?;

      if new_doc_id == NO_MORE_DOCS {
        self.current_values = None;
      } else {
        self.doc_id = self.current_doc_base as i32 + new_doc_id;
        return Ok(self.doc_id);
      }
    }
  }

  fn advance(&mut self, target_doc_id: i32) -> Result<i32> {
    let leaves = self.reader.leaves()?;
    if target_doc_id <= self.doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "can only advance beyond current document: on docID={} but targetDocID={}",
        self.doc_id, target_doc_id
      )));
    }

    let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

    if reader_index >= self.next_leaf {
      if reader_index == leaves.len() {
        self.current_values = None;
        self.doc_id = NO_MORE_DOCS;
        return Ok(self.doc_id);
      }

      let leaf = &leaves[reader_index];
      self.current_doc_base = leaf.doc_base;
      self.current_values = leaf.reader().get_binary_doc_values(&self.field)?;
      self.next_leaf = reader_index + 1;

      if self.current_values.is_none() {
        return self.next_doc();
      }
    }

    let new_doc_id = self
      .current_values
      .as_mut()
      .unwrap()
      .advance(target_doc_id - self.current_doc_base as i32)?;

    if new_doc_id == NO_MORE_DOCS {
      self.current_values = None;
      self.next_doc()
    } else {
      self.doc_id = self.current_doc_base as i32 + new_doc_id;
      Ok(self.doc_id)
    }
  }

  fn cost(&self) -> Result<i64> {
    Ok(0)
  }
}
impl<CR> BinaryDocValues for BinaryDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    match self.current_values {
      Some(ref mut values) => values.binary_value(),
      None => Err(LuceneError::illegal_state("current_values is none")),
    }
  }
}
pub struct SortedNumericDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  next_leaf: usize,
  current_values_index: Option<usize>,
  values: Vec<
    SortedNumericDocValuesEnum2<
      SingletonSortedNumericDocValues<EmptyNumeric>,
      LRSortedNumericDocValues<CR::LeafReader>,
    >,
  >,
  reader: CompositeReaderContext<CR>,
  doc_id: i32,
  field: String,
  current_doc_base: usize,
  final_total_cost: i64,
}
impl<CR> SortedNumericDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  pub fn new(
    reader: CompositeReaderContext<CR>,
    values: Vec<
      SortedNumericDocValuesEnum2<
        SingletonSortedNumericDocValues<EmptyNumeric>,
        LRSortedNumericDocValues<CR::LeafReader>,
      >,
    >,
    field: String,
    total_cost: i64,
  ) -> Self {
    Self {
      next_leaf: 0,
      current_values_index: None,
      values,
      reader,
      doc_id: -1,
      field,
      current_doc_base: 0,
      final_total_cost: total_cost,
    }
  }
}
impl<CR> DocValuesIterator for SortedNumericDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
    let leaves = self.reader.leaves()?;
    if target_doc_id < self.doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "can only advance beyond current document: on docID={} but targetDocID={}",
        self.doc_id, target_doc_id
      )));
    }

    let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

    if reader_index >= self.next_leaf {
      if reader_index == leaves.len() {
        return Err(LuceneError::illegal_argument(format!(
          "Out of range: {}",
          target_doc_id
        )));
      }

      let leaf = &leaves[reader_index];
      self.current_doc_base = leaf.doc_base;
      self.current_values_index = Some(reader_index);
      self.next_leaf = reader_index + 1;
    }

    self.doc_id = target_doc_id;
    match self.current_values_index {
      None => Ok(false),
      Some(current_values) => {
        let current_values = &mut self.values[current_values];
        current_values.advance_exact(target_doc_id - self.current_doc_base as i32)
      },
    }
  }
}
impl<CR> DocIdSetIterator for SortedNumericDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    let leaves = self.reader.leaves()?;
    loop {
      if self.current_values_index.is_none() {
        if self.next_leaf == leaves.len() {
          self.doc_id = NO_MORE_DOCS;
          return Ok(self.doc_id);
        }

        let leaf = &leaves[self.next_leaf];
        self.current_doc_base = leaf.doc_base;
        self.current_values_index = Some(self.next_leaf);
        self.next_leaf += 1;
      }

      let new_doc = self.values[*self.current_values_index.as_ref().unwrap()].next_doc()?;

      if new_doc == NO_MORE_DOCS {
        self.current_values_index = None;
      } else {
        self.doc_id = self.current_doc_base as i32 + new_doc;
        return Ok(self.doc_id);
      }
    }
  }

  fn advance(&mut self, target_doc_id: i32) -> Result<i32> {
    let leaves = self.reader.leaves()?;
    if target_doc_id <= self.doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "can only advance beyond current document: on docID={} but targetDocID={}",
        self.doc_id, target_doc_id
      )));
    }

    let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

    if reader_index >= self.next_leaf {
      if reader_index == leaves.len() {
        self.current_values_index = None;
        self.doc_id = NO_MORE_DOCS;
        return Ok(self.doc_id);
      }

      let leaf = &leaves[reader_index];
      self.current_doc_base = leaf.doc_base;
      self.current_values_index = Some(reader_index);
      self.next_leaf = reader_index + 1;
    }

    let new_doc = self.values[*self.current_values_index.as_ref().unwrap()]
      .advance(target_doc_id - self.current_doc_base as i32)?;

    if new_doc == NO_MORE_DOCS {
      self.current_values_index = None;
      self.next_doc()
    } else {
      self.doc_id = self.current_doc_base as i32 + new_doc;
      Ok(self.doc_id)
    }
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.final_total_cost)
  }
}
impl<CR> SortedNumericDocValues for SortedNumericDocValuesImpl<CR>
where
  CR: CompositeReader,
{
  fn doc_value_count(&mut self) -> Result<i32> {
    match self.current_values_index {
      Some(ref v) => self.values[*v].doc_value_count(),
      None => Err(LuceneError::illegal_state("current_values is none")),
    }
  }

  fn next_value(&mut self) -> Result<i64> {
    match self.current_values_index {
      Some(ref v) => self.values[*v].next_value(),
      None => Err(LuceneError::illegal_state("current_values is none")),
    }
  }

  type NumericDocValues = DummyNumericDocValues;
}
