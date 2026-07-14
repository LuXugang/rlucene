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
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::dummy::dummy_cache_helper::DummyCacheHelper;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::index_reader::{IndexReader, IndexReaderBase, LeafReaderContextKind};
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::term::Term;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
/// # Note
/// See [`JavaIntermediateBaseClass`](crate::migration_notes::JavaIntermediateBaseClass)
#[allow(dead_code)]
pub struct FilterCodecReader;

/// Returns a filtered codec reader with the given live docs and numDocs.
pub(crate) fn wrap_live_docs<CR, B>(
  reader: CR,
  live_docs: Option<B>,
  num_docs: i32,
) -> CodecReaderImpl<CR, B>
where
  CR: CodecReader,
  B: Bits + Clone,
{
  CodecReaderImpl::new(reader, live_docs, num_docs)
}

pub struct CodecReaderImpl<CR, B>
where
  CR: CodecReader,
  B: Bits + Clone,
{
  reader: CR,
  live_docs: Option<B>,
  num_docs: i32,
  index_base: IndexReaderBase,
}

impl<CR, B> CodecReaderImpl<CR, B>
where
  CR: CodecReader,
  B: Bits + Clone,
{
  fn new(reader: CR, live_docs: Option<B>, num_docs: i32) -> Self {
    Self {
      reader,
      live_docs,
      num_docs,
      index_base: IndexReaderBase::new(),
    }
  }

  pub fn get_delegate(&self) -> &CR {
    &self.reader
  }
}

impl<CR, B> Clone for CodecReaderImpl<CR, B>
where
  CR: CodecReader + Clone,
  B: Bits + Clone,
{
  fn clone(&self) -> Self {
    Self {
      reader: self.reader.clone(),
      live_docs: self.live_docs.clone(),
      num_docs: self.num_docs,
      index_base: IndexReaderBase::new(),
    }
  }
}

impl<CR, B> Display for CodecReaderImpl<CR, B>
where
  CR: CodecReader,
  B: Bits + Clone,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "LiveDocsCodecReader({})", self.reader)
  }
}

impl<CR, B> IndexReader for CodecReaderImpl<CR, B>
where
  CR: CodecReader,
  B: Bits + Clone,
{
  type ContextKind = LeafReaderContextKind;

  type TermVectors = CR::TermVectors;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    IndexReader::term_vectors(&self.reader)
  }

  fn max_doc(&self) -> Result<i32> {
    self.reader.max_doc()
  }

  fn num_docs(&self) -> Result<i32> {
    Ok(self.num_docs)
  }

  type StoredFields = CR::StoredFields;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    IndexReader::stored_fields(&self.reader)
  }

  fn do_close(&self) -> Result<()> {
    self.reader.do_close()
  }

  type ReaderCacheHelper = DummyCacheHelper;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    Ok(None)
  }

  fn doc_freq(&self, term: &Term) -> Result<i32> {
    IndexReader::doc_freq(&self.reader, term)
  }

  fn total_term_freq(&self, term: &Term) -> Result<i64> {
    IndexReader::total_term_freq(&self.reader, term)
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
    IndexReader::get_sum_doc_freq(&self.reader, field)
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    IndexReader::get_doc_count(&self.reader, field)
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    IndexReader::get_sum_total_term_freq(&self.reader, field)
  }

  fn index_base(&self) -> &IndexReaderBase {
    &self.index_base
  }
}

impl<CR, B> LeafReader for CodecReaderImpl<CR, B>
where
  CR: CodecReader,
  B: Bits + Clone,
{
  type CacheHelper = CR::CacheHelper;

  fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
    self.reader.get_core_cache_helper()
  }

  type Terms = CR::Terms;

  fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
    LeafReader::terms(&self.reader, field)
  }

  type NumericDocValues = CR::NumericDocValues;

  fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>> {
    LeafReader::get_numeric_doc_values(&self.reader, field)
  }

  type BinaryDocValues = CR::BinaryDocValues;

  fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>> {
    LeafReader::get_binary_doc_values(&self.reader, field)
  }

  type SortedDocValues = CR::SortedDocValues;

  fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>> {
    LeafReader::get_sorted_doc_values(&self.reader, field)
  }

  type SortedNumericDocValues = CR::SortedNumericDocValues;

  fn get_sorted_numeric_doc_values(
    &self,
    field: &str,
  ) -> Result<Option<Self::SortedNumericDocValues>> {
    LeafReader::get_sorted_numeric_doc_values(&self.reader, field)
  }

  type SortedSetDocValues = CR::SortedSetDocValues;

  fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>> {
    LeafReader::get_sorted_set_doc_values(&self.reader, field)
  }

  type NormNumericDocValues = CR::NormNumericDocValues;

  fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>> {
    LeafReader::get_norm_values(&self.reader, field)
  }

  type DocValuesSkipper = CR::DocValuesSkipper;

  fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>> {
    LeafReader::get_doc_values_skipper(&self.reader, field)
  }

  type FloatVectorValues = CR::FloatVectorValues;

  fn get_float_vector_values(&self, field: &str) -> Result<Option<Self::FloatVectorValues>> {
    LeafReader::get_float_vector_values(&self.reader, field)
  }

  type ByteVectorValues = CR::ByteVectorValues;

  fn get_byte_vector_values(&self, field: &str) -> Result<Option<Self::ByteVectorValues>> {
    LeafReader::get_byte_vector_values(&self.reader, field)
  }

  fn search_nearest_vectors_f32<BitsT, K>(
    &self,
    field: &str,
    target: Vec<f32>,
    knn_collector: &mut K,
    accept_docs: Option<BitsT>,
  ) -> Result<()>
  where
    BitsT: Bits,
    K: KnnCollector,
  {
    LeafReader::search_nearest_vectors_f32(&self.reader, field, target, knn_collector, accept_docs)
  }

  fn search_nearest_vectors_u8<BitsT, K>(
    &self,
    field: &str,
    target: Vec<u8>,
    knn_collector: &mut K,
    accept_docs: Option<BitsT>,
  ) -> Result<()>
  where
    BitsT: Bits,
    K: KnnCollector,
  {
    LeafReader::search_nearest_vectors_u8(&self.reader, field, target, knn_collector, accept_docs)
  }

  fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
    self.reader.get_field_infos()
  }

  type Bits = B;

  fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
    Ok(self.live_docs.clone())
  }

  type PointValues = CR::PointValues;

  fn get_point_values(&self, field: &str) -> Result<Option<Self::PointValues>> {
    LeafReader::get_point_values(&self.reader, field)
  }

  fn get_metadata(&self) -> Result<&LeafMetaData> {
    self.reader.get_metadata()
  }
}

impl<CR, B> CodecReader for CodecReaderImpl<CR, B>
where
  CR: CodecReader,
  B: Bits + Clone,
{
  type StoredFieldsReader = CR::StoredFieldsReader;
  type TermVectorsReader = CR::TermVectorsReader;
  type NormsProducer = CR::NormsProducer;
  type DocValuesProducer = CR::DocValuesProducer;
  type FieldsProducer = CR::FieldsProducer;
  type PointsReader = CR::PointsReader;
  type KnnVectorsReader = CR::KnnVectorsReader;

  fn get_fields_reader(&self) -> Result<Option<Self::StoredFieldsReader>> {
    self.reader.get_fields_reader()
  }

  fn get_term_vectors_reader(&self) -> Result<Option<Self::TermVectorsReader>> {
    self.reader.get_term_vectors_reader()
  }

  fn get_norms_reader(&self) -> Result<Option<Self::NormsProducer>> {
    self.reader.get_norms_reader()
  }

  fn get_doc_values_reader(&self) -> Result<Option<Self::DocValuesProducer>> {
    self.reader.get_doc_values_reader()
  }

  fn get_postings_reader(&self) -> Result<Option<Self::FieldsProducer>> {
    self.reader.get_postings_reader()
  }

  fn get_points_reader(&self) -> Result<Option<Self::PointsReader>> {
    self.reader.get_points_reader()
  }

  fn get_vector_reader(&self) -> Result<Option<Self::KnnVectorsReader>> {
    self.reader.get_vector_reader()
  }
}
