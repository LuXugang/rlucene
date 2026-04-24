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
use crate::core::codecs::block_tree::lucene90_block_tree_terms_reader::Lucene90BlockTreeTermsReader;
use crate::core::codecs::compressing::lucene90_compressing_stored_fields_reader::Lucene90CompressingStoredFieldsReader;
use crate::core::codecs::compressing::lucene90_compressing_term_vectors_reader::Lucene90CompressingTermVectorsReader;
use crate::core::codecs::dummy::dummy_binary_doc_values::DummyBinaryDocValues;
use crate::core::codecs::dummy::dummy_doc_values_skipper::DummyDocValuesSkipper;
use crate::core::codecs::dummy::dummy_knn_vectors_reader::DummyKnnVectorsReader;
use crate::core::codecs::dummy::dummy_norms_producer::DummyNormsProducer;
use crate::core::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::core::codecs::dummy::dummy_sorted_numeric_doc_values::DummySortedNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_set_doc_values::DummySortedSetDocValues;
use crate::core::codecs::lucene90_doc_values_producer::Lucene90DocValuesProducer;
use crate::core::codecs::lucene90_points_reader::Lucene90PointsReader;
use crate::core::codecs::lucene101::lucene101_postings_reader::Lucene101PostingsReader;
use crate::core::index::codec_reader::CodecReader;
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
use crate::core::store::dummy::dummy_index_input::DummyIndexInput;
use crate::core::util::bits::Bits;
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub struct DummyCodecReader;

impl LeafReader for DummyCodecReader {
  type CacheHelper = DummyCacheHelper;

  fn get_core_cache_helper_ref(&self) -> Result<Option<&Self::CacheHelper>> {
    dummy_unreachable!()
  }

  fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
    dummy_unreachable!()
  }

  type Terms = DummyTerms;

  fn terms(&self, _field: &str) -> Result<Option<Self::Terms>> {
    dummy_unreachable!()
  }

  type NumericDocValues = DummyNumericDocValues;

  fn get_numeric_doc_values(&self, _field: &str) -> Result<Option<Self::NumericDocValues>> {
    dummy_unreachable!()
  }

  type BinaryDocValues = DummyBinaryDocValues;

  fn get_binary_doc_values(&self, _field: &str) -> Result<Option<Self::BinaryDocValues>> {
    dummy_unreachable!()
  }

  type SortedDocValues = DummySortedDocValues;

  fn get_sorted_doc_values(&self, _field: &str) -> Result<Option<Self::SortedDocValues>> {
    dummy_unreachable!()
  }

  type SortedNumericDocValues = DummySortedNumericDocValues;

  fn get_sorted_numeric_doc_values(
    &self,
    _field: &str,
  ) -> Result<Option<Self::SortedNumericDocValues>> {
    dummy_unreachable!()
  }

  type SortedSetDocValues = DummySortedSetDocValues;

  fn get_sorted_set_doc_values(&self, _field: &str) -> Result<Option<Self::SortedSetDocValues>> {
    dummy_unreachable!()
  }

  type NormNumericDocValues = DummyNumericDocValues;

  fn get_norm_values(&self, _field: &str) -> Result<Option<Self::NormNumericDocValues>> {
    dummy_unreachable!()
  }

  type DocValuesSkipper = DummyDocValuesSkipper;

  fn get_doc_values_skipper(&self, _field: &str) -> Result<Option<Self::DocValuesSkipper>> {
    dummy_unreachable!()
  }

  type FloatVectorValues = DummyFloatVectorValues;

  fn get_float_vector_values(&self, _field: &str) -> Result<Option<Self::FloatVectorValues>> {
    dummy_unreachable!()
  }

  type ByteVectorValues = DummyByteVectorValues;

  fn get_byte_vector_values(&self, _field: &str) -> Result<Option<Self::ByteVectorValues>> {
    dummy_unreachable!()
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
    dummy_unreachable!()
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
    dummy_unreachable!()
  }

  fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
    dummy_unreachable!()
  }

  type Bits = DummyBits;

  fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
    dummy_unreachable!()
  }

  type PointValues = DummyPointValues;

  fn get_point_values(&self, _field: &str) -> Result<Option<Self::PointValues>> {
    dummy_unreachable!()
  }

  fn get_metadata(&self) -> Result<&LeafMetaData> {
    dummy_unreachable!()
  }
}

impl IndexReader for DummyCodecReader {
  type TermVectors = DummyTermVectors;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    dummy_unreachable!()
  }

  fn max_doc(&self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn num_docs(&self) -> Result<i32> {
    dummy_unreachable!()
  }

  type StoredFields = DummyStoredFields;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    dummy_unreachable!()
  }

  fn do_close(&self) -> Result<()> {
    dummy_unreachable!()
  }

  type ReaderCacheHelper = DummyCacheHelper;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    dummy_unreachable!()
  }

  fn doc_freq(&self, _term: &Term) -> Result<i32> {
    dummy_unreachable!()
  }

  fn total_term_freq(&self, _term: &Term) -> Result<i64> {
    dummy_unreachable!()
  }

  fn get_sum_doc_freq(&self, _field: &str) -> Result<i64> {
    dummy_unreachable!()
  }

  fn get_doc_count(&self, _field: &str) -> Result<i32> {
    dummy_unreachable!()
  }

  fn get_sum_total_term_freq(&self, _field: &str) -> Result<i64> {
    dummy_unreachable!()
  }

  fn index_base(&self) -> &IndexReaderBase {
    dummy_unreachable!()
  }
}

impl Display for DummyCodecReader {
  fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
    dummy_unreachable!()
  }
}

impl CodecReader for DummyCodecReader {
  type StoredFieldsReader = Lucene90CompressingStoredFieldsReader<DummyIndexInput>;
  type TermVectorsReader = Lucene90CompressingTermVectorsReader<DummyIndexInput>;
  type NormsProducer = DummyNormsProducer;
  type DocValuesProducer = Lucene90DocValuesProducer<DummyIndexInput>;
  type FieldsProducer =
    Lucene90BlockTreeTermsReader<DummyIndexInput, Lucene101PostingsReader<DummyIndexInput>>;
  type PointsReader = Lucene90PointsReader<DummyIndexInput>;
  type KnnVectorsReader = DummyKnnVectorsReader;

  fn get_fields_reader(&self) -> Result<Option<Self::StoredFieldsReader>> {
    dummy_unreachable!()
  }

  fn get_term_vectors_reader(&self) -> Result<Option<Self::TermVectorsReader>> {
    dummy_unreachable!()
  }

  fn get_norms_reader(&self) -> Result<Option<Self::NormsProducer>> {
    dummy_unreachable!()
  }

  fn get_doc_values_reader(&self) -> Result<Option<Self::DocValuesProducer>> {
    dummy_unreachable!()
  }

  fn get_postings_reader(&self) -> Result<Option<Self::FieldsProducer>> {
    dummy_unreachable!()
  }

  fn get_points_reader(&self) -> Result<Option<Self::PointsReader>> {
    dummy_unreachable!()
  }

  fn get_vector_reader(&self) -> Result<Option<Self::KnnVectorsReader>> {
    todo!()
  }
}
