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
use crate::core::codecs::doc_values_consumer::DocValuesConsumer;
use crate::core::codecs::doc_values_producer::DocValuesProducer;
use crate::core::codecs::dummy::dummy_binary_doc_values::DummyBinaryDocValues;
use crate::core::codecs::dummy::dummy_doc_values_skipper::DummyDocValuesSkipper;
use crate::core::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_numeric_doc_values::DummySortedNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_set_doc_values::DummySortedSetDocValues;
use crate::core::index::BytesRef;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::doc_values_writer::DocValuesWriter;
use crate::core::index::docs_with_field_set::{DocsWithFieldSet, DocsWithFieldSetDISI};
use crate::core::index::field_info::FieldInfo;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::sorted_doc_values_terms_enum::SortedDocValuesTermsEnum;
use crate::core::index::sorter::DocMap;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::store::directory::Directory;
use crate::core::util::accountable::Accountable;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::bytes_ref_hash::{
  BytesRefHash, DEFAULT_CAPACITY, DirectBytesRefHash, DirectBytesStartArray,
};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::packed::PackedInts;
use crate::core::util::packed::packed_long_values::{
  Builder, PackedLongValues, PackedLongValuesIterator,
};
use crate::core::util::{BYTE_BLOCK_SIZE, ByteBlockPool, Counter, SharedCounter, TryIntoInt};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

type BufferedWriterSortedDocValues = BufferedSortedDocValues<DocsWithFieldSetDISI>;

pub(crate) enum SortedDocValuesWriterTermsEnum<'a> {
  Buffered(<BufferedWriterSortedDocValues as SortedDocValues>::TermsEnum<'a>),
  Sorting(
    <SortingSortedDocValues<BufferedWriterSortedDocValues> as SortedDocValues>::TermsEnum<'a>,
  ),
}

impl<'a> BytesRefIterator for SortedDocValuesWriterTermsEnum<'a> {
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match self {
      Self::Buffered(terms) => terms.next(),
      Self::Sorting(terms) => terms.next(),
    }
  }

  fn set_next(&mut self) -> Result<bool> {
    match self {
      Self::Buffered(terms) => terms.set_next(),
      Self::Sorting(terms) => terms.set_next(),
    }
  }
}

impl<'a> TermsEnum for SortedDocValuesWriterTermsEnum<'a> {
  type AttributeSource<'b>
    = <<BufferedWriterSortedDocValues as SortedDocValues>::TermsEnum<'a> as TermsEnum>::AttributeSource<'b>
  where
    Self: 'b;
  type AttributeSourceMut<'b>
    = <<BufferedWriterSortedDocValues as SortedDocValues>::TermsEnum<'a> as TermsEnum>::AttributeSourceMut<'b>
  where
    Self: 'b;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    match self {
      Self::Buffered(terms) => terms.attributes(),
      Self::Sorting(terms) => terms.attributes(),
    }
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    match self {
      Self::Buffered(terms) => terms.attributes_mut(),
      Self::Sorting(terms) => terms.attributes_mut(),
    }
  }

  fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
    match self {
      Self::Buffered(terms) => terms.seek_exact(term),
      Self::Sorting(terms) => terms.seek_exact(term),
    }
  }

  fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    match self {
      Self::Buffered(terms) => terms.prepare_seek_exact(text),
      Self::Sorting(terms) => terms.prepare_seek_exact(text),
    }
  }

  fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
    match self {
      Self::Buffered(terms) => terms.get_prepare_seek_exact_status(target),
      Self::Sorting(terms) => terms.get_prepare_seek_exact_status(target),
    }
  }

  fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    match self {
      Self::Buffered(terms) => terms.seek_ceil(term),
      Self::Sorting(terms) => terms.seek_ceil(term),
    }
  }

  fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
    match self {
      Self::Buffered(terms) => terms.seek_exact_with_ord(ord),
      Self::Sorting(terms) => terms.seek_exact_with_ord(ord),
    }
  }

  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    state: &TermStateEnum,
  ) -> Result<()> {
    match self {
      Self::Buffered(terms) => terms.seek_exact_with_state(term, state),
      Self::Sorting(terms) => terms.seek_exact_with_state(term, state),
    }
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    match self {
      Self::Buffered(terms) => terms.term(),
      Self::Sorting(terms) => terms.term(),
    }
  }

  fn ord(&self) -> Result<i64> {
    match self {
      Self::Buffered(terms) => terms.ord(),
      Self::Sorting(terms) => terms.ord(),
    }
  }

  fn doc_freq(&mut self) -> Result<i32> {
    match self {
      Self::Buffered(terms) => terms.doc_freq(),
      Self::Sorting(terms) => terms.doc_freq(),
    }
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    match self {
      Self::Buffered(terms) => terms.total_term_freq(),
      Self::Sorting(terms) => terms.total_term_freq(),
    }
  }

  type PostingsEnum =
    <<BufferedWriterSortedDocValues as SortedDocValues>::TermsEnum<'a> as TermsEnum>::PostingsEnum;

  fn postings(&mut self, reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
    match self {
      Self::Buffered(terms) => terms.postings(reuse),
      Self::Sorting(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn postings_with_flags(
    &mut self,
    reuse: Option<Self::PostingsEnum>,
    flags: i32,
  ) -> Result<Self::PostingsEnum> {
    match self {
      Self::Buffered(terms) => terms.postings_with_flags(reuse, flags),
      Self::Sorting(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  type ImpactsEnum =
    <<BufferedWriterSortedDocValues as SortedDocValues>::TermsEnum<'a> as TermsEnum>::ImpactsEnum;

  fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
    match self {
      Self::Buffered(terms) => terms.impacts(flags),
      Self::Sorting(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    match self {
      Self::Buffered(terms) => terms.term_state(),
      Self::Sorting(terms) => terms.term_state(),
    }
  }
}

pub(crate) enum SortedDocValuesWriterValues {
  Buffered(BufferedWriterSortedDocValues),
  Sorting(SortingSortedDocValues<BufferedWriterSortedDocValues>),
}

impl DocValuesIterator for SortedDocValuesWriterValues {
  fn advance_exact(&mut self, target: i32) -> Result<bool> {
    match self {
      Self::Buffered(values) => values.advance_exact(target),
      Self::Sorting(values) => values.advance_exact(target),
    }
  }
}

impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for SortedDocValuesWriterValues
{
}
impl DocIdSetIterator for SortedDocValuesWriterValues {
  fn doc_id(&self) -> i32 {
    match self {
      Self::Buffered(values) => values.doc_id(),
      Self::Sorting(values) => values.doc_id(),
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    match self {
      Self::Buffered(values) => values.next_doc(),
      Self::Sorting(values) => values.next_doc(),
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    match self {
      Self::Buffered(values) => values.advance(target),
      Self::Sorting(values) => values.advance(target),
    }
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    match self {
      Self::Buffered(values) => values.slow_advance(target),
      Self::Sorting(values) => values.slow_advance(target),
    }
  }

  fn cost(&self) -> Result<i64> {
    match self {
      Self::Buffered(values) => values.cost(),
      Self::Sorting(values) => values.cost(),
    }
  }
}

impl SortedDocValues for SortedDocValuesWriterValues {
  fn ord_value(&mut self) -> Result<i32> {
    match self {
      Self::Buffered(values) => values.ord_value(),
      Self::Sorting(values) => values.ord_value(),
    }
  }

  fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    match self {
      Self::Buffered(values) => values.lookup_ord(ord),
      Self::Sorting(values) => values.lookup_ord(ord),
    }
  }

  fn get_value_count(&self) -> Result<i32> {
    match self {
      Self::Buffered(values) => values.get_value_count(),
      Self::Sorting(values) => values.get_value_count(),
    }
  }

  fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
    match self {
      Self::Buffered(values) => values.lookup_term(key),
      Self::Sorting(values) => values.lookup_term(key),
    }
  }

  type TermsEnum<'a> = SortedDocValuesWriterTermsEnum<'a>;

  fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
    match self {
      Self::Buffered(values) => values
        .terms_enum()
        .map(SortedDocValuesWriterTermsEnum::Buffered),
      Self::Sorting(values) => values
        .terms_enum()
        .map(SortedDocValuesWriterTermsEnum::Sorting),
    }
  }
}

/// Buffers pending byte slices per document, sorts them by `i32` ordinal, then writes them during
/// segment flush.
pub(crate) struct SortedDocValuesWriter {
  hash: DirectBytesRefHash,
  frozen_hash: Option<Arc<DirectBytesRefHash>>,
  pending: Builder,
  docs_with_field: DocsWithFieldSet,
  iw_bytes_used: SharedCounter,
  bytes_used: i64, // this currently only tracks differences in 'pending'
  field_info: Arc<FieldInfo>,
  last_doc_id: i32,

  final_ords: Option<PackedLongValues>,
  // In Java Lucene, `finalSortedValues` corresponds to the `ids` array inside BytesRefHash.
  // Due to language limitations, we do not need to explicitly define finalSortedValues in Rust.
  // Instead of storing the sorted array,
  // we can simply define an `is_sorted` field to indicate whether the BytesRefHash::sort method has been called.
  is_sorted: bool,
  final_ord_map: Option<Arc<Vec<i32>>>,
  pool: Arc<ByteBlockPool>,
}

impl SortedDocValuesWriter {
  pub(crate) fn new(field_info: Arc<FieldInfo>, iw_bytes_used: SharedCounter) -> Result<Self> {
    let bytes_start_array =
      DirectBytesStartArray::with_counter(DEFAULT_CAPACITY as usize, iw_bytes_used.clone());
    let hash = BytesRefHash::from_bytes_start_array(DEFAULT_CAPACITY, bytes_start_array)?;
    let pending = PackedLongValues::delta_packed_long_values_builder_default(PackedInts::COMPACT)?;
    let docs_with_field = DocsWithFieldSet::new();
    let bytes_used = pending.ram_bytes_used()? + docs_with_field.ram_bytes_used()?;
    iw_bytes_used.add_and_get(bytes_used);

    Ok(Self {
      hash,
      frozen_hash: None,
      pending,
      docs_with_field,
      iw_bytes_used,
      bytes_used,
      field_info,
      last_doc_id: -1,
      final_ords: None,
      is_sorted: false,
      final_ord_map: None,
      pool: Arc::new(ByteBlockPool::default()),
    })
  }

  pub(crate) fn add_value(
    &mut self,
    doc_id: i32,
    value: &BytesRef<Vec<u8>>,
    pool: &mut ByteBlockPool,
  ) -> Result<()> {
    if doc_id <= self.last_doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "DocValuesField \"{}\" appears more than once in this document (only one value is allowed per field)",
        self.field_info.name
      )));
    }

    if value.length > (BYTE_BLOCK_SIZE as usize - 2) {
      return Err(LuceneError::illegal_argument(format!(
        "DocValuesField \"{}\" is too large, must be <= {}",
        self.field_info.name,
        BYTE_BLOCK_SIZE - 2
      )));
    }

    self.add_one_value(value, pool)?;
    self.docs_with_field.add(doc_id)?;
    self.last_doc_id = doc_id;
    Ok(())
  }

  fn add_one_value(&mut self, value: &BytesRef<Vec<u8>>, pool: &mut ByteBlockPool) -> Result<()> {
    let mut term_id = self.hash.add(value, pool)?;
    if term_id < 0 {
      term_id = -term_id - 1;
    } else {
      // reserve additional space for each unique value:
      // 1. when indexing, when hash is 50% full, rehash() suddenly needs 2*size ints.
      // 2. when flushing, we need 1 int per value (slot in the ordMap).
      self
        .iw_bytes_used
        .add_and_get((2 * BitUtil::INT_BYTES) as i64);
    }

    self.pending.add(term_id as i64)?;
    self.update_bytes_used()
  }

  fn update_bytes_used(&mut self) -> Result<()> {
    let new_bytes_used = self.pending.ram_bytes_used()? + self.docs_with_field.ram_bytes_used()?;
    let delta = new_bytes_used - self.bytes_used;
    self.iw_bytes_used.add_and_get(delta);
    self.bytes_used = new_bytes_used;
    Ok(())
  }

  fn sort_doc_values<SDV, DM>(
    max_doc: usize,
    sort_map: &DM,
    old_values: &mut SDV,
  ) -> Result<Vec<i32>>
  where
    SDV: SortedDocValues,
    DM: DocMap,
  {
    let mut ords = vec![-1; max_doc];
    let mut doc_id;
    loop {
      doc_id = old_values.next_doc()?;
      if doc_id == NO_MORE_DOCS {
        break;
      }
      let new_doc_id = sort_map.old_to_new(doc_id)?;
      ords[new_doc_id as usize] = old_values.ord_value()?;
    }
    Ok(ords)
  }
}

impl Display for SortedDocValuesWriter {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl DocValuesWriter for SortedDocValuesWriter {
  fn flush<D1, D2, DM, DC>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    sort_map: Option<&DM>,
    dv_consumer: &mut DC,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = DC::IndexOutput>,
    DM: DocMap,
    DC: DocValuesConsumer,
  {
    if !self.is_sorted {
      return Err(LuceneError::illegal_state(
        "must be finished before getting doc values",
      ));
    }
    let Some(frozen_hash) = self.frozen_hash.clone() else {
      return Err(LuceneError::illegal_state(
        "must be finished before getting doc values",
      ));
    };
    let Some(final_ords) = self.final_ords.as_ref() else {
      return Err(LuceneError::illegal_state(
        "must be finished before getting doc values",
      ));
    };
    let Some(final_ord_map) = self.final_ord_map.as_ref() else {
      return Err(LuceneError::illegal_state(
        "must be finished before getting doc values",
      ));
    };
    dv_consumer.add_sorted_field(
      write_state,
      segment_info,
      &self.field_info,
      &get_doc_values_producer(
        self.field_info.clone(),
        frozen_hash,
        self.pool.clone(),
        final_ords,
        final_ord_map.clone(),
        &self.docs_with_field,
        sort_map,
      )?,
    )?;
    Ok(())
  }

  type DocIdSetIterator = BufferedSortedDocValues<DocsWithFieldSetDISI>;

  fn get_doc_values(&self) -> Result<Self::DocIdSetIterator> {
    if !self.is_sorted {
      return Err(LuceneError::illegal_state(
        "must be finished before getting doc values",
      ));
    }
    let Some(frozen_hash) = self.frozen_hash.as_ref() else {
      return Err(LuceneError::illegal_state(
        "must be finished before getting doc values",
      ));
    };
    let Some(final_ords) = self.final_ords.as_ref() else {
      return Err(LuceneError::illegal_state(
        "must be finished before getting doc values",
      ));
    };
    let Some(final_ord_map) = self.final_ord_map.as_ref() else {
      return Err(LuceneError::illegal_state(
        "must be finished before getting doc values",
      ));
    };
    BufferedSortedDocValues::new(
      frozen_hash.clone(),
      self.pool.clone(),
      final_ords,
      final_ord_map.clone(),
      self.docs_with_field.iterator()?,
    )
  }

  fn finish(&mut self, pool: Arc<ByteBlockPool>) -> Result<()> {
    self.pool = pool;
    self.docs_with_field.finish();
    if !self.is_sorted {
      let value_count = self.hash.size();
      self.update_bytes_used()?;
      debug_assert!(self.final_ord_map.is_none() && self.final_ords.is_none());

      self.hash.sort(self.pool.as_ref())?;
      self.is_sorted = true;
      let ords = self.pending.build()?;

      let mut ord_map = vec![0i32; value_count as usize];
      for ord in 0..value_count as usize {
        let index = self.hash.ids[ord] as usize;
        ord_map[index] = ord as i32;
      }
      let replacement = BytesRefHash::new()?;
      self.frozen_hash = Some(Arc::new(std::mem::replace(&mut self.hash, replacement)));
      self.final_ords = Some(ords);
      self.final_ord_map = Some(Arc::new(ord_map));
    }
    Ok(())
  }
}

pub(crate) struct DocValuesProducerImpl<'a> {
  hash: Arc<DirectBytesRefHash>,
  pool: Arc<ByteBlockPool>,
  ords: &'a PackedLongValues,
  ord_map: Arc<Vec<i32>>,
  docs_with_field: &'a DocsWithFieldSet,
  writer_field_info: Arc<FieldInfo>,
  sorted: Option<Arc<Vec<i32>>>,
}

impl CloseableRef for DocValuesProducerImpl<'_> {
  fn close(&self) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl<'a> DocValuesProducerImpl<'a> {
  pub(crate) fn new(
    hash: Arc<DirectBytesRefHash>,
    pool: Arc<ByteBlockPool>,
    ords: &'a PackedLongValues,
    ord_map: Arc<Vec<i32>>,
    docs_with_field: &'a DocsWithFieldSet,
    writer_field_info: Arc<FieldInfo>,
    sorted: Option<Arc<Vec<i32>>>,
  ) -> Result<Self> {
    Ok(Self {
      hash,
      pool,
      ords,
      ord_map,
      docs_with_field,
      writer_field_info,
      sorted,
    })
  }
}

impl DocValuesProducer for DocValuesProducerImpl<'_> {
  type NumericDocValues = DummyNumericDocValues;
  type BinaryDocValues = DummyBinaryDocValues;
  type SortedDocValues = SortedDocValuesWriterValues;

  fn get_sorted(&self, field_info_in: &Arc<FieldInfo>) -> Result<Self::SortedDocValues> {
    if !Arc::ptr_eq(&self.writer_field_info, field_info_in) {
      return Err(LuceneError::illegal_argument("wrong fieldInfo"));
    }
    let buf = BufferedSortedDocValues::new(
      self.hash.clone(),
      self.pool.clone(),
      self.ords,
      self.ord_map.clone(),
      self.docs_with_field.iterator()?,
    )?;
    match self.sorted.as_ref() {
      Some(sorted) => Ok(SortedDocValuesWriterValues::Sorting(
        SortingSortedDocValues::new(buf, sorted.clone()),
      )),
      None => Ok(SortedDocValuesWriterValues::Buffered(buf)),
    }
  }

  type SortedNumericDocValues = DummySortedNumericDocValues;
  type SortedSetDocValues = DummySortedSetDocValues;
  type DocValuesSkipper = DummyDocValuesSkipper;
}

pub(crate) struct BufferedSortedDocValues<D> {
  hash: Arc<DirectBytesRefHash>,
  pool: Arc<ByteBlockPool>,
  scratch: BytesRef<Vec<u8>>,
  ord_map: Arc<Vec<i32>>,
  ord: i32,
  iter: PackedLongValuesIterator,
  docs_with_field: D,
}

impl<D> BufferedSortedDocValues<D> {
  pub(crate) fn new(
    hash: Arc<DirectBytesRefHash>,
    pool: Arc<ByteBlockPool>,
    doc_to_ord: &PackedLongValues,
    ord_map: Arc<Vec<i32>>,
    docs_with_field: D,
  ) -> Result<Self> {
    Ok(Self {
      hash,
      pool,
      scratch: BytesRef::new(),
      ord_map,
      ord: -1,
      iter: doc_to_ord.iterator()?,
      docs_with_field,
    })
  }
}

impl<D> DocValuesIterator for BufferedSortedDocValues<D>
where
  D: DocIdSetIterator,
{
  fn advance_exact(&mut self, _target: i32) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl<D> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for BufferedSortedDocValues<D>
where
  D: DocIdSetIterator,
{
}
impl<D> DocIdSetIterator for BufferedSortedDocValues<D>
where
  D: DocIdSetIterator,
{
  fn doc_id(&self) -> i32 {
    self.docs_with_field.doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    let doc_id = self.docs_with_field.next_doc()?;
    if doc_id != NO_MORE_DOCS {
      let raw_ord: i32 = self.iter.next_value()?.try_convert()?;
      let mapped = self.ord_map[raw_ord as usize];
      self.ord = mapped;
    }
    Ok(doc_id)
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::unsupported_operation("use next_doc instead"))
  }

  fn cost(&self) -> Result<i64> {
    self.docs_with_field.cost()
  }
}

impl<D> SortedDocValues for BufferedSortedDocValues<D>
where
  D: DocIdSetIterator,
{
  fn ord_value(&mut self) -> Result<i32> {
    Ok(self.ord)
  }

  fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    debug_assert!(ord >= 0 && (ord as usize) < self.hash.ids.len());
    let index = self.hash.ids[ord as usize];
    debug_assert!(
      index >= 0 && (index as usize) < self.hash.ids.len(),
      "sorted_values[ord] out of range"
    );
    self
      .hash
      .get(index, &mut self.scratch, self.pool.as_ref())?;
    Ok(Cow::Borrowed(&self.scratch))
  }

  fn get_value_count(&self) -> Result<i32> {
    Ok(self.hash.size())
  }

  type TermsEnum<'a>
    = SortedDocValuesTermsEnum<&'a mut Self>
  where
    D: 'a;

  fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
    self.default_terms_enum()
  }
}

pub struct SortingSortedDocValues<S> {
  input: S,
  ords: Arc<Vec<i32>>,
  doc_id: i32,
}

impl<S> SortingSortedDocValues<S> {
  pub(crate) fn new(input: S, ords: Arc<Vec<i32>>) -> Self {
    Self {
      input,
      ords,
      doc_id: -1,
    }
  }
}

impl<S> DocValuesIterator for SortingSortedDocValues<S>
where
  S: SortedDocValues,
{
  fn advance_exact(&mut self, target: i32) -> Result<bool> {
    // Needed by `IndexSorter::StringSorter`.
    self.doc_id = target;
    Ok(self.ords[target as usize] != -1)
  }
}

impl<S> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for SortingSortedDocValues<S>
where
  S: SortedDocValues,
{
}
impl<S> DocIdSetIterator for SortingSortedDocValues<S>
where
  S: SortedDocValues,
{
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    loop {
      self.doc_id += 1;
      if self.doc_id as usize == self.ords.len() {
        self.doc_id = NO_MORE_DOCS;
        break;
      }
      if self.ords[self.doc_id as usize] != -1 {
        break;
      }
      // skip missing docs
    }
    Ok(self.doc_id)
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::unsupported_operation("use next_doc instead"))
  }

  fn cost(&self) -> Result<i64> {
    self.input.cost()
  }
}

impl<S> SortedDocValues for SortingSortedDocValues<S>
where
  S: SortedDocValues,
{
  fn ord_value(&mut self) -> Result<i32> {
    Ok(self.ords[self.doc_id as usize])
  }

  fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    self.input.lookup_ord(ord)
  }

  fn get_value_count(&self) -> Result<i32> {
    self.input.get_value_count()
  }

  type TermsEnum<'a>
    = SortedDocValuesTermsEnum<&'a mut Self>
  where
    S: 'a;

  fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
    self.default_terms_enum()
  }
}

pub(crate) fn get_doc_values_producer<'a, DM>(
  writer_field_info: Arc<FieldInfo>,
  hash: Arc<DirectBytesRefHash>,
  pool: Arc<ByteBlockPool>,
  ords: &'a PackedLongValues,
  ord_map: Arc<Vec<i32>>,
  docs_with_field: &'a DocsWithFieldSet,
  sort_map: Option<&DM>,
) -> Result<DocValuesProducerImpl<'a>>
where
  DM: DocMap,
{
  let sorted = if let Some(sort_map) = sort_map {
    let docs_iter = docs_with_field.iterator()?;
    let mut old_values =
      BufferedSortedDocValues::new(hash.clone(), pool.clone(), ords, ord_map.clone(), docs_iter)?;
    Some(Arc::new(SortedDocValuesWriter::sort_doc_values(
      sort_map.size() as usize,
      sort_map,
      &mut old_values,
    )?))
  } else {
    None
  };

  DocValuesProducerImpl::new(
    hash,
    pool.clone(),
    ords,
    ord_map,
    docs_with_field,
    writer_field_info,
    sorted,
  )
}
