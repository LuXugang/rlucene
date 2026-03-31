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
use crate::core::codecs::dummy::dummy_binary_doc_values::DummyBinaryDocValues;
use crate::core::codecs::dummy::dummy_doc_values_skipper::DummyDocValuesSkipper;
use crate::core::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::core::codecs::dummy::dummy_sorted_numeric_doc_values::DummySortedNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_set_doc_values::DummySortedSetDocValues;
use crate::core::index::dummy::dummy_byte_vector_values::DummyByteVectorValues;
use crate::core::index::dummy::dummy_cache_helper::DummyCacheHelper;
use crate::core::index::dummy::dummy_float_vector_values::DummyFloatVectorValues;
use crate::core::index::dummy::dummy_point_value_base::DummyPointValues;
use crate::core::index::dummy::dummy_stored_fields::DummyStoredFields;
use crate::core::index::dummy::dummy_term_vectors::DummyTermVectors;
use crate::core::index::dummy::dummy_terms::DummyTerms;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::index_reader::{IndexReader, IndexReaderBase};
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::term::Term;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::util::bits::Bits;
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[derive(Clone, Default)]
pub(crate) struct DocValuesLeafReader;

impl Display for DocValuesLeafReader {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl IndexReader for DocValuesLeafReader {
  type TermVectors = DummyTermVectors;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn max_doc(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn num_docs(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  type StoredFields = DummyStoredFields;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn do_close(&self) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  type ReaderCacheHelper = DummyCacheHelper;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn doc_freq(&self, term: &Term) -> Result<i32> {
    LeafReader::doc_freq(self, term)
  }

  fn total_term_freq(&self, term: &Term) -> Result<i64> {
    LeafReader::get_total_term_freq(self, term)
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
    LeafReader::get_sum_doc_freq(self, field)
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    LeafReader::get_doc_count(self, field)
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    LeafReader::get_sum_total_term_freq(self, field)
  }
  fn index_base(&self) -> &IndexReaderBase {
    unreachable!("Dummy implementation: this method should never be called in real usage")
  }
}

impl LeafReader for DocValuesLeafReader {
  type CacheHelper = DummyCacheHelper;

  fn get_core_cache_helper_ref(&self) -> Result<Option<&Self::CacheHelper>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type Terms = DummyTerms;

  fn terms(&self, _field: &str) -> Result<Option<Self::Terms>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type NumericDocValues = DummyNumericDocValues;

  fn get_numeric_doc_values(&self, _field: &str) -> Result<Option<Self::NumericDocValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type BinaryDocValues = DummyBinaryDocValues;

  fn get_binary_doc_values(&self, _field: &str) -> Result<Option<Self::BinaryDocValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type SortedDocValues = DummySortedDocValues;

  fn get_sorted_doc_values(&self, _field: &str) -> Result<Option<Self::SortedDocValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type SortedNumericDocValues = DummySortedNumericDocValues;

  fn get_sorted_numeric_doc_values(
    &self,
    _field: &str,
  ) -> Result<Option<Self::SortedNumericDocValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type SortedSetDocValues = DummySortedSetDocValues;

  fn get_sorted_set_doc_values(&self, _field: &str) -> Result<Option<Self::SortedSetDocValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type NormNumericDocValues = DummyNumericDocValues;

  fn get_norm_values(&self, _field: &str) -> Result<Option<Self::NormNumericDocValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type DocValuesSkipper = DummyDocValuesSkipper;

  fn get_doc_values_skipper(&self, _field: &str) -> Result<Option<Self::DocValuesSkipper>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type FloatVectorValues = DummyFloatVectorValues;

  fn get_float_vector_values(&self, _field: &str) -> Result<Option<Self::FloatVectorValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type ByteVectorValues = DummyByteVectorValues;

  fn get_byte_vector_values(&self, _field: &str) -> Result<Option<Self::ByteVectorValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn search_nearest_vectors_f32<B, K>(
    &self,
    _field: &str,
    _target: Vec<f32>,
    _knn_collector: &mut K,
    _accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    Err(LuceneError::unsupported_operation(""))
  }

  fn search_nearest_vectors_u8<B, K>(
    &self,
    _field: &str,
    _target: Vec<u8>,
    _knn_collector: &mut K,
    _accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type Bits = DummyBits;

  fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type PointValues = DummyPointValues;

  fn get_point_values(&self, _field: &str) -> Result<Option<Self::PointValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn check_integrity(&self) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_metadata(&self) -> Result<&LeafMetaData> {
    Err(LuceneError::unsupported_operation(""))
  }
}
