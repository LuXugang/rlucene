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
use crate::core::codecs::dummy::dummy_norms_producer::DummyNormsProducer;
use crate::core::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::core::codecs::dummy::dummy_sorted_numeric_doc_values::DummySortedNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_set_doc_values::DummySortedSetDocValues;
use crate::core::codecs::lucene90_doc_values_producer::Lucene90DocValuesProducer;
use crate::core::codecs::lucene90_points_reader::Lucene90PointsReader;
use crate::core::codecs::lucene101::lucene101_postings_reader::Lucene101PostingsReader;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::dummy::dummy_cache_helper::DummyCacheHelper;
use crate::core::index::dummy::dummy_point_value_base::DummyPointValues;
use crate::core::index::dummy::dummy_stored_fields::DummyStoredFields;
use crate::core::index::dummy::dummy_term_vectors::DummyTermVectors;
use crate::core::index::dummy::dummy_terms::DummyTerms;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::index_reader::{IndexReader, IndexReaderBase};
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::term::Term;
use crate::core::store::dummy::dummy_index_input::DummyIndexInput;
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub struct DummyCodecReader;

impl LeafReader for DummyCodecReader {
    type CacheHelper = DummyCacheHelper;

    fn get_core_cache_helper_ref(&self) -> Result<Option<&Self::CacheHelper>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type Terms = DummyTerms;

    fn terms(&self, _field: &str) -> Result<Option<Self::Terms>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type NumericDocValues = DummyNumericDocValues;

    fn get_numeric_doc_values(&self, _field: &str) -> Result<Option<Self::NumericDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type BinaryDocValues = DummyBinaryDocValues;

    fn get_binary_doc_values(&self, _field: &str) -> Result<Option<Self::BinaryDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type SortedDocValues = DummySortedDocValues;

    fn get_sorted_doc_values(&self, _field: &str) -> Result<Option<Self::SortedDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type SortedNumericDocValues = DummySortedNumericDocValues;

    fn get_sorted_numeric_doc_values(
        &self,
        _field: &str,
    ) -> Result<Option<Self::SortedNumericDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type SortedSetDocValues = DummySortedSetDocValues;

    fn get_sorted_set_doc_values(&self, _field: &str) -> Result<Option<Self::SortedSetDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type NormNumericDocValues = DummyNumericDocValues;

    fn get_norm_values(&self, _field: &str) -> Result<Option<Self::NormNumericDocValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type DocValuesSkipper = DummyDocValuesSkipper;

    fn get_doc_values_skipper(&self, _field: &str) -> Result<Option<Self::DocValuesSkipper>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type Bits = DummyBits;

    fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type PointValues = DummyPointValues;

    fn get_point_values(&self, _field: &str) -> Result<Option<Self::PointValues>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_metadata(&self) -> Result<&LeafMetaData> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl IndexReader for DummyCodecReader {
    type TermVectors<'a> = DummyTermVectors;

    fn term_vectors(&self) -> Result<Self::TermVectors<'_>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn max_doc(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn num_docs(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type StoredFields<'a> = DummyStoredFields;

    fn stored_fields(&self) -> Result<Self::StoredFields<'_>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn do_close(&self) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type ReaderCacheHelper = DummyCacheHelper;

    fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn doc_freq(&self, _term: &Term) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn total_term_freq(&self, _term: &Term) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_sum_doc_freq(&self, _field: &str) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_doc_count(&self, _field: &str) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_sum_total_term_freq(&self, _field: &str) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn base(&self) -> &IndexReaderBase {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl Display for DummyCodecReader {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("Dummy implementation: this method should never be called in real usage")
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

    fn get_fields_reader(&self) -> Result<Cow<'_, Self::StoredFieldsReader>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_term_vectors_reader(&self) -> Result<Option<Cow<'_, Self::TermVectorsReader>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_norms_reader(&self) -> Result<Option<Cow<'_, Self::NormsProducer>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_doc_values_reader(&self) -> Result<Option<Cow<'_, Self::DocValuesProducer>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_postings_reader(&self) -> Result<Option<Cow<'_, Self::FieldsProducer>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_points_reader(&self) -> Result<Option<Cow<'_, Self::PointsReader>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
