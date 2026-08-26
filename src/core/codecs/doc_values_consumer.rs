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
use crate::core::codecs::doc_values_producer::DocValuesProducer;
use crate::core::codecs::dummy::dummy_binary_doc_values::DummyBinaryDocValues;
use crate::core::codecs::dummy::dummy_doc_values_skipper::DummyDocValuesSkipper;
use crate::core::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::core::codecs::dummy::dummy_sorted_numeric_doc_values::DummySortedNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_set_doc_values::DummySortedSetDocValues;
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::core::index::doc_values::{
  DocValues, NumericDocValuesWithEmpty, SortedDocValuesWithEmpty, SortedNumericDocValuesWithEmpty,
};
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::core::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::filtered_terms_enum::{
  AcceptStatus, FilteredTermsEnum, FilteredTermsEnumBase,
};
use crate::core::index::merge_state::{DocMap, MergeStateAccess};
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::ordinal_map::{OrdinalMap, SegmentToGlobalOrds};
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::core::index::singleton_sorted_set_doc_values::SingletonSortedSetDocValues;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::sorted_numeric_doc_values::{
  SingletonOrMultiSortedNumericDocValuesEnum, SortedNumericDocValues,
};
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::sorted_set_doc_values_writer::{
  SingletonOrMultiSortedSetDocValuesEnum, SortedSetDocValuesEnum2, SortedSetDocValuesWithEmpty,
};
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::index::{BytesRef, DocIDMerger, DocIDMergerEnum, Sub, SubBase, of};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::store::directory::Directory;
use crate::core::util::bits::Bits;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::dummy::dummy_attribute_source::DummyAttributeSource;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::long_bit_set::LongBitSet;
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::PackedInts;
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

pub trait DocValuesConsumer: Closeable {
  type IndexOutput: crate::core::store::IndexOutput;

  fn add_numeric_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer;
  fn add_binary_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer;
  fn add_sorted_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer;
  fn add_sorted_numeric_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer;
  fn add_sorted_set_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer;
  fn merge<D1, D2, MS>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    merge_state: &MS,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    MS: MergeStateAccess,
  {
    for producer in merge_state.doc_values_producers().iter().flatten() {
      producer.check_integrity()?;
    }
    for merge_field_info in merge_state.merge_field_infos().iter() {
      let dv_type = merge_field_info.get_doc_values_type();

      if *dv_type == DocValuesType::None {
        continue;
      }

      match *dv_type {
        DocValuesType::Numeric => {
          self.merge_numeric_field(write_state, segment_info, merge_field_info, merge_state)?;
        },
        DocValuesType::Binary => {
          self.merge_binary_field(write_state, segment_info, merge_field_info, merge_state)?;
        },
        DocValuesType::Sorted => {
          self.merge_sorted_field(write_state, segment_info, merge_field_info, merge_state)?;
        },
        DocValuesType::SortedSet => {
          self.merge_sorted_set_field(write_state, segment_info, merge_field_info, merge_state)?;
        },
        DocValuesType::SortedNumeric => {
          self.merge_sorted_numeric_field(
            write_state,
            segment_info,
            merge_field_info,
            merge_state,
          )?;
        },

        _ => return Err(LuceneError::illegal_state(format!("type= {}", dv_type))),
      }
    }

    Ok(())
  }

  fn merge_numeric_field<D1, D2, MS>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    merge_field_info: &Arc<FieldInfo>,
    merge_state: &MS,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    MS: MergeStateAccess,
  {
    let producer = EmptyDocValuesProducerMerge1 {
      merge_field_info: merge_field_info.clone(),
      merge_state,
    };
    self.add_numeric_field(write_state, segment_info, merge_field_info, &producer)?;
    Ok(())
  }
  fn merge_binary_field<D1, D2, MS>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    merge_field_info: &Arc<FieldInfo>,
    merge_state: &MS,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    MS: MergeStateAccess,
  {
    let producer = EmptyDocValuesProducerMerge2 {
      merge_field_info: merge_field_info.clone(),
      merge_state,
    };
    self.add_binary_field(write_state, segment_info, merge_field_info, &producer)
  }
  fn merge_sorted_numeric_field<D1, D2, MS>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    merge_field_info: &Arc<FieldInfo>,
    merge_state: &MS,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    MS: MergeStateAccess,
  {
    let producer = EmptyDocValuesProducerMerge3 {
      merge_field_info: merge_field_info.clone(),
      merge_state,
    };
    self.add_sorted_numeric_field(write_state, segment_info, merge_field_info, &producer)
  }
  fn merge_sorted_field<D1, D2, MS>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field_info: &Arc<FieldInfo>,
    merge_state: &MS,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    MS: MergeStateAccess,
  {
    let mut to_merge = Vec::with_capacity(merge_state.doc_values_producers().len());

    for i in 0..merge_state.doc_values_producers().len() {
      let mut values = None;

      if let Some(doc_values_producer) = &merge_state.doc_values_producers()[i]
        && let Some(reader_field_info) =
          merge_state.field_infos()[i].field_info_by_name(&field_info.name)?
        && *reader_field_info.get_doc_values_type() == DocValuesType::Sorted
      {
        values = Some(SortedDocValuesWithEmpty::A(
          doc_values_producer.get_sorted(&reader_field_info)?,
        ));
      }
      let values = match values {
        Some(values) => values,
        None => SortedDocValuesWithEmpty::B(DocValues::empty_sorted()),
      };
      to_merge.push(values);
    }

    let num_readers = to_merge.len();
    // step 1: iterate thru each sub and mark terms still in use
    let mut live_terms = Vec::with_capacity(num_readers);
    let mut weights: Vec<i64> = vec![0; num_readers];

    let mut remaining = to_merge.as_mut_slice();
    let mut sub = 0;
    while let Some((dvs, rest)) = remaining.split_first_mut() {
      remaining = rest;
      let live_docs_opt = merge_state.live_docs()[sub].as_ref();

      match live_docs_opt {
        None => {
          let value_count = dvs.get_value_count()?;
          weights[sub] = value_count as i64;
          let terms_enum = FilteredTermsEnum::unfiltered(dvs.terms_enum()?);
          live_terms.push(terms_enum);
        },
        Some(live_docs) => {
          let value_count = dvs.get_value_count()? as usize;
          let mut bitset = LongBitSet::new(value_count)?;

          loop {
            let doc_id = dvs.next_doc()?;
            if doc_id == NO_MORE_DOCS {
              break;
            }
            if live_docs.get(doc_id as usize)? {
              let ord = dvs.ord_value()?;
              if ord >= 0 {
                bitset.set(ord as usize);
              }
            }
          }

          let cardinality = bitset.cardinality();
          weights[sub] = cardinality as i64;
          let terms_enum = BitsFilteredTermsEnum::new(dvs.terms_enum()?, bitset);
          live_terms.push(terms_enum);
        },
      }
      sub += 1;
    }
    // step 2: create ordinal map (this conceptually does the "merging")
    let ordinal_map = OrdinalMap::build(None, live_terms, &weights, PackedInts::COMPACT)?;
    let producer = EmptyDocValuesProducerMerge4 {
      field_info: field_info.clone(),
      merge_state,
      map: Rc::new(ordinal_map),
    };
    self.add_sorted_field(write_state, segment_info, field_info, &producer)
  }
  fn merge_sorted_set_field<D1, D2, MS>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    merge_field_info: &Arc<FieldInfo>,
    merge_state: &MS,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    MS: MergeStateAccess,
  {
    let mut to_merge = Vec::with_capacity(merge_state.doc_values_producers().len());

    for i in 0..merge_state.doc_values_producers().len() {
      let mut values = None;

      if let Some(doc_values_producer) = &merge_state.doc_values_producers()[i]
        && let Some(field_info) =
          merge_state.field_infos()[i].field_info_by_name(&merge_field_info.name)?
        && *field_info.get_doc_values_type() == DocValuesType::SortedSet
      {
        values = Some(SortedSetDocValuesEnum2::A(
          doc_values_producer.get_sorted_set(&field_info)?,
        ));
      }

      let values = match values {
        Some(values) => values,
        None => SortedSetDocValuesEnum2::B(DocValues::empty_sorted_set()?),
      };
      to_merge.push(values);
    }

    // step 1: iterate thru each sub and mark terms still in use
    let num_readers = to_merge.len();
    let mut live_terms = Vec::with_capacity(num_readers);
    let mut weights: Vec<i64> = vec![0; num_readers];

    for (sub, dv) in to_merge.iter_mut().enumerate() {
      let live_docs_opt = merge_state.live_docs()[sub].as_ref();

      match live_docs_opt {
        None => {
          let value_count = dv.get_value_count()?;
          weights[sub] = value_count;
          let terms_enum = FilteredTermsEnum::unfiltered(dv.terms_enum()?);
          live_terms.push(terms_enum);
        },
        Some(live_docs) => {
          let value_count = dv.get_value_count()? as usize;
          let mut bitset = LongBitSet::new(value_count)?;

          loop {
            let doc_id = dv.next_doc()?;
            if doc_id == NO_MORE_DOCS {
              break;
            }
            if live_docs.get(doc_id as usize)? {
              let count = dv.doc_value_count()?;
              for _ in 0..count {
                let ord = dv.next_ord()?;
                bitset.set(ord as usize);
              }
            }
          }

          let cardinality = bitset.cardinality();
          weights[sub] = cardinality as i64;

          let terms_enum = BitsFilteredTermsEnum::new(dv.terms_enum()?, bitset);
          live_terms.push(terms_enum);
        },
      }
    }

    // step 2: create ordinal map (this conceptually does the "merging")
    let ordinal_map = OrdinalMap::build(None, live_terms, &weights, PackedInts::COMPACT)?;
    let v = EmptyDocValuesProducerMerge5 {
      merge_field_info: merge_field_info.clone(),
      merge_state,
      map: Rc::new(ordinal_map),
    };
    self.add_sorted_set_field(write_state, segment_info, merge_field_info, &v)
  }
}
pub struct BitsFilteredTermsEnum {
  live_terms: LongBitSet,
}
impl BitsFilteredTermsEnum {
  fn new<TE>(in_: TE, live_terms: LongBitSet) -> FilteredTermsEnum<TE, Self>
  where
    TE: TermsEnum,
  {
    let sub = Self { live_terms };
    FilteredTermsEnum::with_seek(in_, false, sub)
  }
}
impl FilteredTermsEnumBase for BitsFilteredTermsEnum {
  fn accept(&mut self, _term: &BytesRef<Vec<u8>>, ord: i64) -> Result<AcceptStatus> {
    if self.live_terms.get(ord as usize) {
      Ok(AcceptStatus::Yes)
    } else {
      Ok(AcceptStatus::No)
    }
  }

  fn need_ord(&self) -> bool {
    true
  }
}

// 1. NumericDocValues
/// Tracks state of one numeric sub-reader that we are merging.
pub(crate) struct NumericDocValuesSub<N, DM> {
  values: N,
  doc_map: Rc<DM>,
}

impl<N, DM> NumericDocValuesSub<N, DM>
where
  N: DocIdSetIterator,
{
  fn new(doc_map: Rc<DM>, values: N) -> Self {
    debug_assert!(values.doc_id() == -1);
    NumericDocValuesSub { values, doc_map }
  }
}
impl<N, DM> SubBase for NumericDocValuesSub<N, DM>
where
  N: NumericDocValues,
  DM: DocMap,
{
  fn next_doc(&mut self) -> Result<i32> {
    self.values.next_doc()
  }

  type DocMap = DM;

  fn get_doc_map(&self) -> Result<&Self::DocMap> {
    Ok(&self.doc_map)
  }
}

pub struct NumericDocValuesMerge<N, DM> {
  doc_id: i32,
  current: Option<usize>,
  doc_id_merger: DocIDMergerEnum<NumericDocValuesSub<N, DM>>,
  final_cost: i64,
}

impl<N, DM> DocValuesIterator for NumericDocValuesMerge<N, DM>
where
  N: NumericDocValues,
  DM: DocMap,
{
  fn advance_exact(&mut self, _target: i32) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl<N, DM> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for NumericDocValuesMerge<N, DM>
where
  N: NumericDocValues,
  DM: DocMap,
{
}
impl<N, DM> DocIdSetIterator for NumericDocValuesMerge<N, DM>
where
  N: NumericDocValues,
  DM: DocMap,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.current = self.doc_id_merger.next()?;
    match &self.current {
      Some(current) => {
        let v = self.doc_id_merger.get_subs()[*current].mapped_doc_id;
        self.doc_id = v;
        Ok(self.doc_id)
      },
      None => {
        self.doc_id = NO_MORE_DOCS;
        Ok(NO_MORE_DOCS)
      },
    }
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.final_cost)
  }
}

impl<N, DM> NumericDocValues for NumericDocValuesMerge<N, DM>
where
  N: NumericDocValues,
  DM: DocMap,
{
  fn long_value(&mut self) -> Result<i64> {
    match self.current {
      Some(ref current) => {
        let v = &mut self.doc_id_merger.get_subs_mut()[*current];
        v.sub.values.long_value()
      },
      None => Err(LuceneError::unreachable("")),
    }
  }
}
pub(crate) struct EmptyDocValuesProducerMerge1<'a, MS> {
  merge_field_info: Arc<FieldInfo>,
  merge_state: &'a MS,
}

impl<MS> CloseableRef for EmptyDocValuesProducerMerge1<'_, MS> {}

impl<MS> DocValuesProducer for EmptyDocValuesProducerMerge1<'_, MS>
where
  MS: MergeStateAccess,
{
  type NumericDocValues = NumericDocValuesMerge<
    <MS::DocValuesProducer as DocValuesProducer>::NumericDocValues,
    MS::DocMap,
  >;

  fn get_numeric(&self, field_info: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
    if !Arc::ptr_eq(field_info, &self.merge_field_info) {
      return Err(LuceneError::illegal_argument("wrong fieldInfo"));
    }

    let mut subs = vec![];
    debug_assert!(
      self.merge_state.doc_maps().len() == self.merge_state.doc_values_producers().len()
    );
    for i in 0..self.merge_state.doc_values_producers().len() {
      let mut values = None;
      let doc_values_producer_opt = &self.merge_state.doc_values_producers()[i];
      if let Some(doc_values_producer) = doc_values_producer_opt {
        let reader_field_info =
          self.merge_state.field_infos()[i].field_info_by_name(&self.merge_field_info.name)?;
        if let Some(reader_field_info) = &reader_field_info
          && *reader_field_info.get_doc_values_type() == DocValuesType::Numeric
        {
          values = Some(doc_values_producer.get_numeric(reader_field_info)?);
        }
      }

      if let Some(values) = values {
        let doc_map = self.merge_state.doc_maps()[i].clone();
        subs.push(Sub::new(NumericDocValuesSub::new(doc_map, values)));
      }
    }
    merge_numeric_values(subs, self.merge_state.needs_index_sort())
  }

  type BinaryDocValues = DummyBinaryDocValues;
  type SortedDocValues = DummySortedDocValues;

  type SortedNumericDocValues = DummySortedNumericDocValues;
  type SortedSetDocValues = DummySortedSetDocValues;
  type DocValuesSkipper = DummyDocValuesSkipper;
}
// 2. BinaryDocValues
/// Tracks state of one binary sub-reader that we are merging.
struct BinaryDocValuesSub<B, DM> {
  values: B,
  doc_map: Rc<DM>,
}

impl<B, DM> BinaryDocValuesSub<B, DM>
where
  B: DocIdSetIterator,
{
  fn new(doc_map: Rc<DM>, values: B) -> Self {
    debug_assert!(values.doc_id() == -1);
    BinaryDocValuesSub { values, doc_map }
  }
}

impl<B, DM> SubBase for BinaryDocValuesSub<B, DM>
where
  B: BinaryDocValues,
  DM: DocMap,
{
  fn next_doc(&mut self) -> Result<i32> {
    self.values.next_doc()
  }

  type DocMap = DM;

  fn get_doc_map(&self) -> Result<&Self::DocMap> {
    Ok(&self.doc_map)
  }
}

pub struct BinaryDocValuesMerge<B, DM> {
  doc_id: i32,
  current: Option<usize>,
  doc_id_merger: DocIDMergerEnum<BinaryDocValuesSub<B, DM>>,
  final_cost: i64,
}

impl<B, DM> DocValuesIterator for BinaryDocValuesMerge<B, DM>
where
  B: BinaryDocValues,
  DM: DocMap,
{
  fn advance_exact(&mut self, _target: i32) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl<B, DM> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for BinaryDocValuesMerge<B, DM>
where
  B: BinaryDocValues,
  DM: DocMap,
{
}
impl<B, DM> DocIdSetIterator for BinaryDocValuesMerge<B, DM>
where
  B: BinaryDocValues,
  DM: DocMap,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.current = self.doc_id_merger.next()?;
    match &self.current {
      Some(current) => {
        let mapped_doc_id = self.doc_id_merger.get_subs()[*current].mapped_doc_id;
        self.doc_id = mapped_doc_id;
        Ok(self.doc_id)
      },
      None => {
        self.doc_id = NO_MORE_DOCS;
        Ok(NO_MORE_DOCS)
      },
    }
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.final_cost)
  }
}

impl<B, DM> BinaryDocValues for BinaryDocValuesMerge<B, DM>
where
  B: BinaryDocValues,
  DM: DocMap,
{
  fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    match self.current {
      Some(ref current) => {
        let v = &mut self.doc_id_merger.get_subs_mut()[*current].sub;
        v.values.binary_value()
      },
      None => Err(LuceneError::unreachable("")),
    }
  }
}
pub(crate) struct EmptyDocValuesProducerMerge2<'a, MS> {
  merge_field_info: Arc<FieldInfo>,
  merge_state: &'a MS,
}

impl<MS> CloseableRef for EmptyDocValuesProducerMerge2<'_, MS> {}

impl<MS> DocValuesProducer for EmptyDocValuesProducerMerge2<'_, MS>
where
  MS: MergeStateAccess,
{
  type NumericDocValues = DummyNumericDocValues;
  type BinaryDocValues =
    BinaryDocValuesMerge<<MS::DocValuesProducer as DocValuesProducer>::BinaryDocValues, MS::DocMap>;

  fn get_binary(&self, field_info: &Arc<FieldInfo>) -> Result<Self::BinaryDocValues> {
    if !Arc::ptr_eq(field_info, &self.merge_field_info) {
      return Err(LuceneError::illegal_argument("wrong fieldInfo"));
    }

    let mut subs = vec![];
    let mut cost = 0;
    debug_assert!(
      self.merge_state.doc_maps().len() == self.merge_state.doc_values_producers().len()
    );

    for i in 0..self.merge_state.doc_values_producers().len() {
      let mut values = None;
      let doc_values_producer_opt = &self.merge_state.doc_values_producers()[i];

      if let Some(doc_values_producer) = doc_values_producer_opt {
        let reader_field_info =
          self.merge_state.field_infos()[i].field_info_by_name(&self.merge_field_info.name)?;
        if let Some(reader_field_info) = &reader_field_info
          && *reader_field_info.get_doc_values_type() == DocValuesType::Binary
        {
          values = Some(doc_values_producer.get_binary(reader_field_info)?);
        }
      }

      if let Some(values) = values {
        cost += values.cost()?;
        let doc_map = self.merge_state.doc_maps()[i].clone();
        subs.push(Sub::new(BinaryDocValuesSub::new(doc_map, values)));
      }
    }
    let doc_id_merger = of(subs, self.merge_state.needs_index_sort())?;
    let doc_value = BinaryDocValuesMerge {
      doc_id: -1,
      current: None,
      doc_id_merger,
      final_cost: cost,
    };
    Ok(doc_value)
  }

  type SortedDocValues = DummySortedDocValues;

  type SortedNumericDocValues = DummySortedNumericDocValues;
  type SortedSetDocValues = DummySortedSetDocValues;
  type DocValuesSkipper = DummyDocValuesSkipper;
}
// 3. SortedNumericDocValues
/// Tracks state of one sorted numeric sub-reader that we are merging.
struct SortedNumericDocValuesSub<SN, DM> {
  values: SN,
  doc_map: Rc<DM>,
}

impl<SN, DM> SortedNumericDocValuesSub<SN, DM>
where
  SN: DocIdSetIterator,
{
  fn new(doc_map: Rc<DM>, values: SN) -> Self {
    debug_assert!(values.doc_id() == -1);
    SortedNumericDocValuesSub { values, doc_map }
  }
}

impl<SN, DM> SubBase for SortedNumericDocValuesSub<SN, DM>
where
  SN: SortedNumericDocValues,
  DM: DocMap,
{
  fn next_doc(&mut self) -> Result<i32> {
    self.values.next_doc()
  }

  type DocMap = DM;

  fn get_doc_map(&self) -> Result<&Self::DocMap> {
    Ok(&self.doc_map)
  }
}

pub struct SortedNumericDocValuesMerge<SN, DM> {
  doc_id: i32,
  current_sub: Option<usize>,
  doc_id_merger: DocIDMergerEnum<SortedNumericDocValuesSub<SN, DM>>,
  final_cost: i64,
}

impl<SN, DM> DocValuesIterator for SortedNumericDocValuesMerge<SN, DM>
where
  SN: SortedNumericDocValues,
  DM: DocMap,
{
  fn advance_exact(&mut self, _target: i32) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl<SN, DM> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for SortedNumericDocValuesMerge<SN, DM>
where
  SN: SortedNumericDocValues,
  DM: DocMap,
{
}
impl<SN, DM> DocIdSetIterator for SortedNumericDocValuesMerge<SN, DM>
where
  SN: SortedNumericDocValues,
  DM: DocMap,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.current_sub = self.doc_id_merger.next()?;
    match self.current_sub {
      Some(ref current) => {
        let v = self.doc_id_merger.get_subs()[*current].mapped_doc_id;
        self.doc_id = v;
        Ok(self.doc_id)
      },
      None => {
        self.doc_id = NO_MORE_DOCS;
        Ok(NO_MORE_DOCS)
      },
    }
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.final_cost)
  }
}

impl<SN, DM> SortedNumericDocValues for SortedNumericDocValuesMerge<SN, DM>
where
  SN: SortedNumericDocValues,
  DM: DocMap,
{
  fn next_value(&mut self) -> Result<i64> {
    match self.current_sub {
      Some(ref current) => {
        let v = &mut self.doc_id_merger.get_subs_mut()[*current].sub;
        v.values.next_value()
      },
      None => Err(LuceneError::unreachable("")),
    }
  }

  fn doc_value_count(&mut self) -> Result<i32> {
    match self.current_sub {
      Some(ref current) => {
        let v = &mut self.doc_id_merger.get_subs_mut()[*current];
        v.sub.values.doc_value_count()
      },
      None => Err(LuceneError::unreachable("")),
    }
  }

  type NumericDocValues = DummyNumericDocValues;
}
pub(crate) struct EmptyDocValuesProducerMerge3<'a, MS> {
  merge_field_info: Arc<FieldInfo>,
  merge_state: &'a MS,
}

impl<MS> CloseableRef for EmptyDocValuesProducerMerge3<'_, MS> {}

pub type MergeSortedNumeric<P> = <P as DocValuesProducer>::SortedNumericDocValues;
pub type MergeNumeric<P> = <MergeSortedNumeric<P> as SortedNumericDocValues>::NumericDocValues;
impl<MS> DocValuesProducer for EmptyDocValuesProducerMerge3<'_, MS>
where
  MS: MergeStateAccess,
{
  type NumericDocValues = DummyNumericDocValues;
  type BinaryDocValues = DummyBinaryDocValues;
  type SortedDocValues = DummySortedDocValues;
  type SortedNumericDocValues = SingletonOrMultiSortedNumericDocValuesEnum<
    SingletonSortedNumericDocValues<
      NumericDocValuesMerge<
        NumericDocValuesWithEmpty<MergeNumeric<MS::DocValuesProducer>>,
        MS::DocMap,
      >,
    >,
    SortedNumericDocValuesMerge<
      SortedNumericDocValuesWithEmpty<MergeSortedNumeric<MS::DocValuesProducer>>,
      MS::DocMap,
    >,
  >;

  fn get_sorted_numeric(
    &self,
    field_info: &Arc<FieldInfo>,
  ) -> Result<Self::SortedNumericDocValues> {
    if !Arc::ptr_eq(field_info, &self.merge_field_info) {
      return Err(LuceneError::illegal_argument("wrong FieldInfo"));
    }
    // We must make new iterators + DocIDMerger for each iterator:
    let mut subs = vec![];
    let mut cost = 0;
    let mut all_singletons = true;

    for i in 0..self.merge_state.doc_values_producers().len() {
      let mut values = None;
      let doc_values_producer_opt = &self.merge_state.doc_values_producers()[i];
      if let Some(doc_values_producer) = doc_values_producer_opt {
        let reader_field_info =
          self.merge_state.field_infos()[i].field_info_by_name(&self.merge_field_info.name)?;
        if let Some(reader_field_info) = reader_field_info
          && *reader_field_info.get_doc_values_type() == DocValuesType::SortedNumeric
        {
          values = Some(SortedNumericDocValuesWithEmpty::A(
            doc_values_producer.get_sorted_numeric(&reader_field_info)?,
          ));
        }
      }

      let values = match values {
        Some(values) => values,
        None => SortedNumericDocValuesWithEmpty::B(DocValues::empty_sorted_numeric()?),
      };
      cost += values.cost()?;
      if all_singletons && !values.is_single_valued() {
        all_singletons = false;
      }
      let doc_map = self.merge_state.doc_maps()[i].clone();
      subs.push(Sub::new(SortedNumericDocValuesSub::new(doc_map, values)));
    }

    if all_singletons {
      // All subs are single-valued.
      // We specialize for that case since it makes it easier for codecs
      // to optimize for single-valued fields.
      let mut single_valued_subs = vec![];
      for mut sub in subs {
        let single_valued_values = sub.sub.values.get_numeric_doc_values()?;
        single_valued_subs.push(Sub::new(NumericDocValuesSub::new(
          sub.sub.doc_map.clone(),
          single_valued_values,
        )));
      }
      let dv = merge_numeric_values(single_valued_subs, self.merge_state.needs_index_sort())?;
      return Ok(SingletonOrMultiSortedNumericDocValuesEnum::Singleton(
        DocValues::singleton_numeric(dv)?,
      ));
    }
    let doc_id_merger = of(subs, self.merge_state.needs_index_sort())?;
    Ok(SingletonOrMultiSortedNumericDocValuesEnum::Multi(
      SortedNumericDocValuesMerge {
        doc_id: -1,
        current_sub: None,
        doc_id_merger,
        final_cost: cost,
      },
    ))
  }

  type SortedSetDocValues = DummySortedSetDocValues;
  type DocValuesSkipper = DummyDocValuesSkipper;
}

pub(crate) fn merge_numeric_values<N, DM>(
  mut subs: Vec<Sub<NumericDocValuesSub<N, DM>>>,
  index_is_sorted: bool,
) -> Result<NumericDocValuesMerge<N, DM>>
where
  N: NumericDocValues,
  DM: DocMap,
{
  let mut cost = 0;
  for sub in &mut subs {
    cost = sub.sub.values.cost()?;
  }
  let doc_id_merger = of(subs, index_is_sorted)?;
  Ok(NumericDocValuesMerge {
    doc_id: -1,
    current: None,
    doc_id_merger,
    final_cost: cost,
  })
}
// 4. SortedDocValues
struct SortedDocValuesSub<S, DM> {
  values: S,
  map: Rc<SegmentToGlobalOrds>,
  doc_map: Rc<DM>,
}
impl<S, DM> SortedDocValuesSub<S, DM>
where
  S: DocIdSetIterator,
{
  fn new(doc_map: Rc<DM>, values: S, map: Rc<SegmentToGlobalOrds>) -> Self {
    debug_assert!(values.doc_id() == -1);
    SortedDocValuesSub {
      values,
      map,
      doc_map,
    }
  }
}

impl<S, DM> SubBase for SortedDocValuesSub<S, DM>
where
  S: SortedDocValues,
  DM: DocMap,
{
  fn next_doc(&mut self) -> Result<i32> {
    self.values.next_doc()
  }

  type DocMap = DM;

  fn get_doc_map(&self) -> Result<&Self::DocMap> {
    Ok(&self.doc_map)
  }
}

pub(crate) struct EmptyDocValuesProducerMerge4<'a, MS> {
  field_info: Arc<FieldInfo>,
  merge_state: &'a MS,
  map: Rc<OrdinalMap>,
}

impl<MS> CloseableRef for EmptyDocValuesProducerMerge4<'_, MS> {}

impl<MS> DocValuesProducer for EmptyDocValuesProducerMerge4<'_, MS>
where
  MS: MergeStateAccess,
{
  type NumericDocValues = DummyNumericDocValues;
  type BinaryDocValues = DummyBinaryDocValues;
  type SortedDocValues = SortedDocValuesMerge<
    SortedDocValuesWithEmpty<<MS::DocValuesProducer as DocValuesProducer>::SortedDocValues>,
    MS::DocMap,
  >;

  fn get_sorted(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedDocValues> {
    if !Arc::ptr_eq(field, &self.field_info) {
      return Err(LuceneError::illegal_argument("wrong FieldInfo"));
    }

    // We must make new iterators + DocIDMerger for each iterator:
    let mut subs = Vec::with_capacity(self.merge_state.doc_values_producers().len());

    for i in 0..self.merge_state.doc_values_producers().len() {
      let mut values = None;

      if let Some(doc_values_producer) = &self.merge_state.doc_values_producers()[i]
        && let Some(reader_field_info) =
          self.merge_state.field_infos()[i].field_info_by_name(&self.field_info.name)?
        && *reader_field_info.get_doc_values_type() == DocValuesType::Sorted
      {
        values = Some(SortedDocValuesWithEmpty::A(
          doc_values_producer.get_sorted(&reader_field_info)?,
        ));
      }
      let values = match values {
        Some(values) => values,
        None => SortedDocValuesWithEmpty::B(DocValues::empty_sorted()),
      };

      let doc_map = self.merge_state.doc_maps()[i].clone();
      let map = self.map.get_global_ords(i).clone();

      subs.push(Sub::new(SortedDocValuesSub::new(doc_map, values, map)));
    }

    merge_sorted_values(subs, self.merge_state.needs_index_sort(), self.map.clone())
  }

  type SortedNumericDocValues = DummySortedNumericDocValues;
  type SortedSetDocValues = DummySortedSetDocValues;
  type DocValuesSkipper = DummyDocValuesSkipper;
}

pub struct SortedDocValuesMerge<S, DM> {
  doc_id: i32,
  current: Option<usize>,
  doc_id_merger: DocIDMergerEnum<SortedDocValuesSub<S, DM>>,
  final_cost: i64,
  map: Rc<OrdinalMap>,
}

impl<S, DM> DocValuesIterator for SortedDocValuesMerge<S, DM>
where
  S: SortedDocValues,
  DM: DocMap,
{
  fn advance_exact(&mut self, _target: i32) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl<S, DM> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for SortedDocValuesMerge<S, DM>
where
  S: SortedDocValues,
  DM: DocMap,
{
}
impl<S, DM> DocIdSetIterator for SortedDocValuesMerge<S, DM>
where
  S: SortedDocValues,
  DM: DocMap,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.current = self.doc_id_merger.next()?;
    match self.current {
      Some(ref current) => {
        let v = self.doc_id_merger.get_subs()[*current].mapped_doc_id;
        self.doc_id = v;
        Ok(self.doc_id)
      },
      None => {
        self.doc_id = NO_MORE_DOCS;
        Ok(NO_MORE_DOCS)
      },
    }
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.final_cost)
  }
}

impl<S, DM> SortedDocValues for SortedDocValuesMerge<S, DM>
where
  S: SortedDocValues,
  DM: DocMap,
{
  fn ord_value(&mut self) -> Result<i32> {
    let current = *self
      .current
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("current sorted-doc-values sub is missing"))?;
    let current_sub = &mut self.doc_id_merger.get_subs_mut()[current];
    let sub_ord = current_sub.sub.values.ord_value()?;
    debug_assert!(sub_ord != -1);
    Ok(current_sub.sub.map.get(sub_ord as usize)? as i32)
  }

  fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    let segment_number = self.map.get_first_segment_number(ord as usize)?;
    let segment_ord = self.map.get_first_segment_ord(ord as usize)? as i32;
    self.doc_id_merger.get_subs_mut()[segment_number as usize]
      .sub
      .values
      .lookup_ord(segment_ord)
  }

  fn get_value_count(&self) -> Result<i32> {
    Ok(self.map.get_value_count() as i32)
  }

  type TermsEnum<'a>
    = MergedTermsEnum<<S as SortedDocValues>::TermsEnum<'a>>
  where
    Self: 'a;

  fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
    let subs = self.doc_id_merger.get_subs_mut();
    let mut terms_enum_subs = Vec::with_capacity(subs.len());
    for sub in subs {
      terms_enum_subs.push(sub.sub.values.terms_enum()?);
    }
    Ok(MergedTermsEnum::new(self.map.clone(), terms_enum_subs))
  }
}
/// A merged [`TermsEnum`]. This helps avoid relying on the default terms enum, which calls
/// [`SortedDocValues::lookup_ord`] or [`SortedSetDocValues::lookup_ord`] on every call to
/// [`BytesRefIterator::next`].
pub struct MergedTermsEnum<TE> {
  subs: Vec<TE>,
  ordinal_map: Rc<OrdinalMap>,
  value_count: i64,
  ord: i64,
  term: BytesRef<Vec<u8>>,
}
impl<TE> MergedTermsEnum<TE> {
  fn new(ordinal_map: Rc<OrdinalMap>, subs: Vec<TE>) -> Self {
    let value_count = ordinal_map.get_value_count();
    Self {
      subs,
      ordinal_map,
      value_count,
      ord: -1,
      term: BytesRef::new(),
    }
  }
}

impl<TE> BytesRefIterator for MergedTermsEnum<TE>
where
  TE: TermsEnum,
{
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    self.ord += 1;
    if self.ord >= self.value_count {
      return Ok(None);
    }
    let ord = self.ord as usize;
    let sub_num = self.ordinal_map.get_first_segment_number(ord)?;
    let sub_ord = self.ordinal_map.get_first_segment_ord(ord)?;

    let sub = &mut self.subs[sub_num as usize];
    let mut end;
    loop {
      end = sub.next()?.is_none();
      if sub.ord()? >= sub_ord {
        debug_assert!(sub.ord()? == sub_ord);
        return if end {
          Ok(None)
        } else {
          self.term = sub.term()?.into_owned();
          Ok(Some(Cow::Borrowed(&self.term)))
        };
      }
    }
  }
}

impl<TE> TermsEnum for MergedTermsEnum<TE>
where
  TE: TermsEnum,
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
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_exact_with_state(
    &mut self,
    _term: &BytesRef<Vec<u8>>,
    _state: &TermStateEnum,
  ) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    Ok(Cow::Borrowed(&self.term))
  }

  fn ord(&self) -> Result<i64> {
    Ok(self.ord)
  }

  fn doc_freq(&mut self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }

  type PostingsEnum = DummyPostingsEnum;

  fn postings_with_flags(
    &mut self,
    _reuse: Option<Self::PostingsEnum>,
    _flags: i32,
  ) -> Result<Self::PostingsEnum> {
    Err(LuceneError::unsupported_operation(""))
  }

  type ImpactsEnum = DummyImpactsEnum;

  fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    Err(LuceneError::unsupported_operation(""))
  }
}
fn merge_sorted_values<S, DM>(
  subs: Vec<Sub<SortedDocValuesSub<S, DM>>>,
  index_is_sorted: bool,
  map: Rc<OrdinalMap>,
) -> Result<SortedDocValuesMerge<S, DM>>
where
  S: SortedDocValues,
  DM: DocMap,
{
  let mut cost = 0;
  for sub in &subs {
    cost += sub.sub.values.cost()?;
  }
  let final_cost = cost;

  let doc_id_merger = of(subs, index_is_sorted)?;
  Ok(SortedDocValuesMerge {
    doc_id: -1,
    current: None,
    doc_id_merger,
    final_cost,
    map,
  })
}
// 4. SortedSetDocValues
struct SortedSetDocValuesSub<S, DM> {
  values: S,
  map: Rc<SegmentToGlobalOrds>,
  doc_map: Rc<DM>,
}

impl<S, DM> SortedSetDocValuesSub<S, DM>
where
  S: DocIdSetIterator,
{
  fn new(doc_map: Rc<DM>, values: S, map: Rc<SegmentToGlobalOrds>) -> Self {
    debug_assert!(values.doc_id() == -1);
    Self {
      values,
      map,
      doc_map,
    }
  }
}

impl<S, DM> SubBase for SortedSetDocValuesSub<S, DM>
where
  S: SortedSetDocValues,
  DM: DocMap,
{
  fn next_doc(&mut self) -> Result<i32> {
    self.values.next_doc()
  }

  type DocMap = DM;

  fn get_doc_map(&self) -> Result<&Self::DocMap> {
    Ok(self.doc_map.as_ref())
  }
}
pub struct SortedSetDocValuesMerge<S, DM> {
  doc_id: i32,
  current_sub: Option<usize>,
  doc_id_merger: DocIDMergerEnum<SortedSetDocValuesSub<S, DM>>,
  final_cost: i64,
  map: Rc<OrdinalMap>,
  to_merge: Vec<S>,
}

impl<S, DM> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for SortedSetDocValuesMerge<S, DM>
where
  S: SortedSetDocValues,
  DM: DocMap,
{
}
impl<S, DM> DocIdSetIterator for SortedSetDocValuesMerge<S, DM>
where
  S: SortedSetDocValues,
  DM: DocMap,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.current_sub = self.doc_id_merger.next()?;
    match self.current_sub {
      Some(idx) => {
        let v = self.doc_id_merger.get_subs()[idx].mapped_doc_id;
        self.doc_id = v;
        Ok(v)
      },
      None => {
        self.doc_id = NO_MORE_DOCS;
        Ok(NO_MORE_DOCS)
      },
    }
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.final_cost)
  }
}

impl<S, DM> DocValuesIterator for SortedSetDocValuesMerge<S, DM>
where
  S: SortedSetDocValues,
  DM: DocMap,
{
  fn advance_exact(&mut self, _target: i32) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl<S, DM> SortedSetDocValues for SortedSetDocValuesMerge<S, DM>
where
  S: SortedSetDocValues,
  DM: DocMap,
{
  fn next_ord(&mut self) -> Result<i64> {
    let current = *self
      .current_sub
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("current sorted-set sub is missing"))?;
    let current_sub = &mut self.doc_id_merger.get_subs_mut()[current];
    let sub_ord = current_sub.sub.values.next_ord()?;
    current_sub.sub.map.get(sub_ord as usize)
  }

  fn lookup_ord(&mut self, ord: i64) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    let segment_number = self.map.get_first_segment_number(ord as usize)?;
    let segment_ord = self.map.get_first_segment_ord(ord as usize)?;
    self.to_merge[segment_number as usize].lookup_ord(segment_ord)
  }

  fn get_value_count(&self) -> Result<i64> {
    Ok(self.map.get_value_count())
  }

  fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
    let mut subs = Vec::with_capacity(self.to_merge.len());
    for dv in self.to_merge.iter_mut() {
      subs.push(dv.terms_enum()?);
    }
    Ok(MergedTermsEnum::new(self.map.clone(), subs))
  }
  type SortedDocValues = DummySortedDocValues;

  fn doc_value_count(&mut self) -> Result<i32> {
    let current = *self
      .current_sub
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("current sorted-set sub is missing"))?;
    self.doc_id_merger.get_subs_mut()[current]
      .sub
      .values
      .doc_value_count()
  }

  type TermsEnum<'a>
    = MergedTermsEnum<<S as SortedSetDocValues>::TermsEnum<'a>>
  where
    Self: 'a;
}
pub(crate) struct EmptyDocValuesProducerMerge5<'a, MS> {
  merge_field_info: Arc<FieldInfo>,
  merge_state: &'a MS,
  map: Rc<OrdinalMap>,
}

impl<MS> CloseableRef for EmptyDocValuesProducerMerge5<'_, MS> {}

pub type MergeSortedSetDocValues<P> = <P as DocValuesProducer>::SortedSetDocValues;
pub type MergeSortedSetSortedDocValues<P> =
  <MergeSortedSetDocValues<P> as SortedSetDocValues>::SortedDocValues;
impl<MS> DocValuesProducer for EmptyDocValuesProducerMerge5<'_, MS>
where
  MS: MergeStateAccess,
{
  type NumericDocValues = DummyNumericDocValues;
  type BinaryDocValues = DummyBinaryDocValues;
  type SortedDocValues = DummySortedDocValues;
  type SortedNumericDocValues = DummySortedNumericDocValues;

  type SortedSetDocValues = SingletonOrMultiSortedSetDocValuesEnum<
    SingletonSortedSetDocValues<
      SortedDocValuesMerge<
        SortedDocValuesWithEmpty<MergeSortedSetSortedDocValues<MS::DocValuesProducer>>,
        MS::DocMap,
      >,
    >,
    SortedSetDocValuesMerge<
      SortedSetDocValuesWithEmpty<MergeSortedSetDocValues<MS::DocValuesProducer>>,
      MS::DocMap,
    >,
  >;

  fn get_sorted_set(&self, field_info: &Arc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
    if !Arc::ptr_eq(field_info, &self.merge_field_info) {
      return Err(LuceneError::illegal_argument("wrong FieldInfo"));
    }

    // We must make new iterators + DocIDMerger for each iterator:
    let mut subs = Vec::new();
    let mut to_merge = Vec::with_capacity(self.merge_state.doc_values_producers().len());
    let mut cost = 0;
    let mut all_singletons = true;

    for i in 0..self.merge_state.doc_values_producers().len() {
      let mut values = None;
      let mut values_for_merge = None;

      if let Some(doc_values_producer) = &self.merge_state.doc_values_producers()[i]
        && let Some(reader_field_info) =
          self.merge_state.field_infos()[i].field_info_by_name(&self.merge_field_info.name)?
        && *reader_field_info.get_doc_values_type() == DocValuesType::SortedSet
      {
        values = Some(SortedSetDocValuesWithEmpty::A(
          doc_values_producer.get_sorted_set(&reader_field_info)?,
        ));
        values_for_merge = Some(SortedSetDocValuesWithEmpty::A(
          doc_values_producer.get_sorted_set(&reader_field_info)?,
        ));
      }

      let values = match values {
        Some(values) => values,
        None => SortedSetDocValuesWithEmpty::B(DocValues::empty_sorted_set()?),
      };
      let values_for_merge = match values_for_merge {
        Some(values) => values,
        None => SortedSetDocValuesWithEmpty::B(DocValues::empty_sorted_set()?),
      };

      cost += values.cost()?;

      if all_singletons && !values.is_single_valued() {
        all_singletons = false;
      }

      let doc_map = self.merge_state.doc_maps()[i].clone();
      let seg_map = self.map.get_global_ords(i).clone();

      subs.push(Sub::new(SortedSetDocValuesSub::new(
        doc_map, values, seg_map,
      )));
      to_merge.push(values_for_merge);
    }

    if all_singletons {
      // All subs are single-valued.
      // We specialize for that case since it makes it easier for codecs to optimize
      // for single-valued fields.
      let mut single_valued_subs = Vec::new();
      for mut sub in subs {
        let single = sub.sub.values.get_sorted_doc_values()?;
        single_valued_subs.push(Sub::new(SortedDocValuesSub::new(
          sub.sub.doc_map.clone(),
          single,
          sub.sub.map.clone(),
        )));
      }

      let dv = merge_sorted_values(
        single_valued_subs,
        self.merge_state.needs_index_sort(),
        self.map.clone(),
      )?;
      let v = DocValues::singleton_sorted(dv)?;
      return Ok(SingletonOrMultiSortedSetDocValuesEnum::Singleton(v));
    }

    let doc_id_merger = of(subs, self.merge_state.needs_index_sort())?;
    let v = SortedSetDocValuesMerge {
      doc_id: -1,
      current_sub: None,
      doc_id_merger,
      final_cost: cost,
      map: self.map.clone(),
      to_merge,
    };
    Ok(SingletonOrMultiSortedSetDocValuesEnum::Multi(v))
  }

  type DocValuesSkipper = DummyDocValuesSkipper;
}

pub enum DocValuesConsumerEnum2<A, B> {
  A(A),
  B(B),
}

impl<A, B> Closeable for DocValuesConsumerEnum2<A, B>
where
  A: Closeable,
  B: Closeable,
{
  fn close(&mut self) -> Result<()> {
    match self {
      Self::A(inner) => inner.close(),
      Self::B(inner) => inner.close(),
    }
  }
}

impl<A, B> DocValuesConsumer for DocValuesConsumerEnum2<A, B>
where
  A: DocValuesConsumer,
  B: DocValuesConsumer<IndexOutput = A::IndexOutput>,
{
  type IndexOutput = A::IndexOutput;

  fn add_numeric_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer,
  {
    match self {
      Self::A(inner) => inner.add_numeric_field(write_state, segment_info, field, values_producer),
      Self::B(inner) => inner.add_numeric_field(write_state, segment_info, field, values_producer),
    }
  }

  fn add_binary_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer,
  {
    match self {
      Self::A(inner) => inner.add_binary_field(write_state, segment_info, field, values_producer),
      Self::B(inner) => inner.add_binary_field(write_state, segment_info, field, values_producer),
    }
  }

  fn add_sorted_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer,
  {
    match self {
      Self::A(inner) => inner.add_sorted_field(write_state, segment_info, field, values_producer),
      Self::B(inner) => inner.add_sorted_field(write_state, segment_info, field, values_producer),
    }
  }

  fn add_sorted_numeric_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer,
  {
    match self {
      Self::A(inner) => {
        inner.add_sorted_numeric_field(write_state, segment_info, field, values_producer)
      },
      Self::B(inner) => {
        inner.add_sorted_numeric_field(write_state, segment_info, field, values_producer)
      },
    }
  }

  fn add_sorted_set_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer,
  {
    match self {
      Self::A(inner) => {
        inner.add_sorted_set_field(write_state, segment_info, field, values_producer)
      },
      Self::B(inner) => {
        inner.add_sorted_set_field(write_state, segment_info, field, values_producer)
      },
    }
  }

  fn merge<D1, D2, MS>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    merge_state: &MS,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    MS: MergeStateAccess,
  {
    match self {
      Self::A(inner) => inner.merge(write_state, segment_info, merge_state),
      Self::B(inner) => inner.merge(write_state, segment_info, merge_state),
    }
  }
}
