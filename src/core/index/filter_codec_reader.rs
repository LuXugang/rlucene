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
use crate::core::codecs::doc_values_producer::DocValuesProducer;
use crate::core::codecs::norms_producer::NormsProducer;
use crate::core::codecs::points_reader::PointsReader;
use crate::core::index::codec_reader::{CodecReader, StoredFieldsType, TermVectorsType};
use crate::core::index::dummy::dummy_cache_helper::DummyCacheHelper;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::fields::Fields;
use crate::core::index::index_reader::{IndexReader, IndexReaderBase};
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::term::Term;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};
use std::sync::Arc;
/// A `FilterCodecReader` contains another `CodecReader`, which it uses as its basic
/// source of data, possibly transforming the data along the way or providing additional
/// functionality.
///
///
/// **NOTE**: If this [`FilterCodecReader`] does not change the content of the
/// contained reader, you could consider delegating calls to
/// [`get_core_cache_helper`](Self::get_core_cache_helper) and
/// [`get_reader_cache_helper`](Self::get_reader_cache_helper).
pub struct FilterCodecReader<CR>
where
    CR: CodecReader,
{
    pub(crate) in_: CR,
}
impl<CR> FilterCodecReader<CR>
where
    CR: CodecReader,
{
    pub fn new(in_: CR) -> Self {
        Self { in_ }
    }
}

impl<CR> LeafReader for FilterCodecReader<CR>
where
    CR: CodecReader,
{
    type CacheHelper = DummyCacheHelper;

    fn get_core_cache_helper_ref(&self) -> Result<Option<&Self::CacheHelper>> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
        Err(LuceneError::unsupported_operation(""))
    }

    type Terms = <<Self as CodecReader>::FieldsProducer as Fields>::Terms;

    fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
        CodecReader::terms(self, field)
    }

    type NumericDocValues =
        <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::NumericDocValues;

    fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>> {
        CodecReader::get_numeric_doc_values(self, field)
    }

    type BinaryDocValues =
        <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::BinaryDocValues;

    fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>> {
        CodecReader::get_binary_doc_values(self, field)
    }

    type SortedDocValues =
        <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::SortedDocValues;

    fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>> {
        CodecReader::get_sorted_doc_values(self, field)
    }

    type SortedNumericDocValues =
        <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::SortedNumericDocValues;

    fn get_sorted_numeric_doc_values(
        &self,
        field: &str,
    ) -> Result<Option<Self::SortedNumericDocValues>> {
        CodecReader::get_sorted_numeric_doc_values(self, field)
    }

    type SortedSetDocValues =
        <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::SortedSetDocValues;

    fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>> {
        CodecReader::get_sorted_set_doc_values(self, field)
    }

    type NormNumericDocValues =
        <<Self as CodecReader>::NormsProducer as NormsProducer>::NumericDocValues;

    fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>> {
        CodecReader::get_norm_values(self, field)
    }

    type DocValuesSkipper =
        <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::DocValuesSkipper;

    fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>> {
        CodecReader::get_doc_values_skipper(self, field)
    }

    fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
        self.in_.get_field_infos()
    }

    type Bits = CR::Bits;

    fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
        self.in_.get_live_docs()
    }

    type PointValues = <<Self as CodecReader>::PointsReader as PointsReader>::PointValuesType;

    fn get_point_values(&self, field: &str) -> Result<Option<Self::PointValues>> {
        CodecReader::get_point_values(self, field)
    }

    fn check_integrity(&self) -> Result<()> {
        self.in_.check_integrity()
    }

    fn get_metadata(&self) -> Result<&LeafMetaData> {
        self.in_.get_metadata()
    }
}

impl<CR> IndexReader for FilterCodecReader<CR>
where
    CR: CodecReader,
{
    type TermVectors = TermVectorsType<<Self as CodecReader>::TermVectorsReader>;

    fn term_vectors(&self) -> Result<Self::TermVectors> {
        CodecReader::term_vectors(self)
    }

    fn max_doc(&self) -> Result<i32> {
        self.in_.max_doc()
    }

    fn num_docs(&self) -> Result<i32> {
        self.in_.num_docs()
    }

    type StoredFields = StoredFieldsType<<Self as CodecReader>::StoredFieldsReader>;

    fn stored_fields(&self) -> Result<Self::StoredFields> {
        CodecReader::stored_fields(self)
    }

    fn do_close(&self) -> Result<()> {
        self.in_.do_close()
    }

    type ReaderCacheHelper = DummyCacheHelper;

    fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn doc_freq(&self, term: &Term) -> Result<i32> {
        LeafReader::doc_freq(self, term)
    }

    fn total_term_freq(&self, term: &Term) -> Result<i64> {
        LeafReader::total_term_freq(self, term)
    }

    fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
        LeafReader::sum_doc_freq(self, field)
    }

    fn get_doc_count(&self, field: &str) -> Result<i32> {
        LeafReader::doc_count(self, field)
    }

    fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
        LeafReader::sum_total_term_freq(self, field)
    }

    fn base(&self) -> &IndexReaderBase {
        self.in_.base()
    }
}

impl<CR> Display for FilterCodecReader<CR>
where
    CR: CodecReader,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FilterCodecReader({})", self.in_)
    }
}

impl<CR> CodecReader for FilterCodecReader<CR>
where
    CR: CodecReader,
{
    type StoredFieldsReader = CR::StoredFieldsReader;
    type TermVectorsReader = CR::TermVectorsReader;
    type NormsProducer = CR::NormsProducer;
    type DocValuesProducer = CR::DocValuesProducer;
    type FieldsProducer = CR::FieldsProducer;
    type PointsReader = CR::PointsReader;

    fn get_fields_reader(&self) -> Result<Option<Self::StoredFieldsReader>> {
        self.in_.get_fields_reader()
    }

    fn get_term_vectors_reader(&self) -> Result<Option<Self::TermVectorsReader>> {
        self.in_.get_term_vectors_reader()
    }

    fn get_norms_reader(&self) -> Result<Option<Self::NormsProducer>> {
        self.in_.get_norms_reader()
    }

    fn get_doc_values_reader(&self) -> Result<Option<Self::DocValuesProducer>> {
        self.in_.get_doc_values_reader()
    }

    fn get_postings_reader(&self) -> Result<Option<Self::FieldsProducer>> {
        self.in_.get_postings_reader()
    }

    fn get_points_reader(&self) -> Result<Option<Self::PointsReader>> {
        self.in_.get_points_reader()
    }
}
/// Returns a filtered codec reader with the given live docs and numDocs.
pub(crate) fn wrap_live_docs<CR, B>(
    reader: CR,
    live_docs: Option<B>,
    num_docs: i32,
) -> FilterCodecReaderImpl<CR, B>
where
    CR: CodecReader,
    B: Bits + Clone,
{
    FilterCodecReaderImpl::new(reader, live_docs, num_docs)
}

pub(crate) struct FilterCodecReaderImpl<CR, B>
where
    CR: CodecReader,
    B: Bits + Clone,
{
    base: FilterCodecReader<CR>,
    live_docs: Option<B>,
    num_docs: i32,
}
impl<CR, B> FilterCodecReaderImpl<CR, B>
where
    CR: CodecReader,
    B: Bits + Clone,
{
    pub fn new(reader: CR, live_docs: Option<B>, num_docs: i32) -> Self {
        let base = FilterCodecReader::new(reader);
        Self {
            base,
            live_docs,
            num_docs,
        }
    }
}

impl<CR, B> LeafReader for FilterCodecReaderImpl<CR, B>
where
    B: Bits + Clone,
    CR: CodecReader,
{
    type CacheHelper = CR::CacheHelper;

    fn get_core_cache_helper_ref(&self) -> Result<Option<&Self::CacheHelper>> {
        self.base.in_.get_core_cache_helper_ref()
    }

    fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
        self.base.in_.get_core_cache_helper()
    }

    type Terms = <FilterCodecReader<CR> as LeafReader>::Terms;

    fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
        LeafReader::terms(&self.base, field)
    }

    type NumericDocValues = <FilterCodecReader<CR> as LeafReader>::NumericDocValues;

    fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>> {
        LeafReader::get_numeric_doc_values(&self.base, field)
    }

    type BinaryDocValues = <FilterCodecReader<CR> as LeafReader>::BinaryDocValues;

    fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>> {
        LeafReader::get_binary_doc_values(&self.base, field)
    }

    type SortedDocValues = <FilterCodecReader<CR> as LeafReader>::SortedDocValues;

    fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>> {
        LeafReader::get_sorted_doc_values(&self.base, field)
    }

    type SortedNumericDocValues = <FilterCodecReader<CR> as LeafReader>::SortedNumericDocValues;

    fn get_sorted_numeric_doc_values(
        &self,
        field: &str,
    ) -> Result<Option<Self::SortedNumericDocValues>> {
        LeafReader::get_sorted_numeric_doc_values(&self.base, field)
    }

    type SortedSetDocValues = <FilterCodecReader<CR> as LeafReader>::SortedSetDocValues;

    fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>> {
        LeafReader::get_sorted_set_doc_values(&self.base, field)
    }

    type NormNumericDocValues = <FilterCodecReader<CR> as LeafReader>::NormNumericDocValues;

    fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>> {
        LeafReader::get_norm_values(&self.base, field)
    }

    type DocValuesSkipper = <FilterCodecReader<CR> as LeafReader>::DocValuesSkipper;

    fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>> {
        LeafReader::get_doc_values_skipper(&self.base, field)
    }

    fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
        self.base.get_field_infos()
    }

    type Bits = B;

    fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
        Ok(self.live_docs.clone())
    }

    type PointValues = <FilterCodecReader<CR> as LeafReader>::PointValues;

    fn get_point_values(&self, field: &str) -> Result<Option<Self::PointValues>> {
        LeafReader::get_point_values(&self.base, field)
    }

    fn get_metadata(&self) -> Result<&LeafMetaData> {
        self.base.get_metadata()
    }
}

impl<CR, B> IndexReader for FilterCodecReaderImpl<CR, B>
where
    B: Bits + Clone,
    CR: CodecReader,
{
    type TermVectors = <FilterCodecReader<CR> as IndexReader>::TermVectors;

    fn term_vectors(&self) -> Result<Self::TermVectors> {
        IndexReader::term_vectors(&self.base)
    }

    fn max_doc(&self) -> Result<i32> {
        self.base.max_doc()
    }

    fn num_docs(&self) -> Result<i32> {
        Ok(self.num_docs)
    }

    type StoredFields = <FilterCodecReader<CR> as IndexReader>::StoredFields;

    fn stored_fields(&self) -> Result<Self::StoredFields> {
        IndexReader::stored_fields(&self.base)
    }

    fn do_close(&self) -> Result<()> {
        self.base.do_close()
    }

    type ReaderCacheHelper = DummyCacheHelper;

    fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
        Ok(None)
    }

    fn doc_freq(&self, term: &Term) -> Result<i32> {
        IndexReader::doc_freq(&self.base, term)
    }

    fn total_term_freq(&self, term: &Term) -> Result<i64> {
        IndexReader::total_term_freq(&self.base, term)
    }

    fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
        IndexReader::get_sum_doc_freq(&self.base, field)
    }

    fn get_doc_count(&self, field: &str) -> Result<i32> {
        IndexReader::get_doc_count(&self.base, field)
    }

    fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
        IndexReader::get_sum_total_term_freq(&self.base, field)
    }

    fn base(&self) -> &IndexReaderBase {
        self.base.base()
    }
}

impl<CR, B> Display for FilterCodecReaderImpl<CR, B>
where
    B: Bits + Clone,
    CR: CodecReader,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FilterCodecReaderImpl({})", self.base)
    }
}

impl<CR, B> CodecReader for FilterCodecReaderImpl<CR, B>
where
    CR: CodecReader,
    B: Bits + Clone,
{
    type StoredFieldsReader = <FilterCodecReader<CR> as CodecReader>::StoredFieldsReader;
    type TermVectorsReader = <FilterCodecReader<CR> as CodecReader>::TermVectorsReader;
    type NormsProducer = <FilterCodecReader<CR> as CodecReader>::NormsProducer;
    type DocValuesProducer = <FilterCodecReader<CR> as CodecReader>::DocValuesProducer;
    type FieldsProducer = <FilterCodecReader<CR> as CodecReader>::FieldsProducer;
    type PointsReader = <FilterCodecReader<CR> as CodecReader>::PointsReader;

    fn get_fields_reader(&self) -> Result<Option<Self::StoredFieldsReader>> {
        self.base.get_fields_reader()
    }

    fn get_term_vectors_reader(&self) -> Result<Option<Self::TermVectorsReader>> {
        self.base.get_term_vectors_reader()
    }

    fn get_norms_reader(&self) -> Result<Option<Self::NormsProducer>> {
        self.base.get_norms_reader()
    }

    fn get_doc_values_reader(&self) -> Result<Option<Self::DocValuesProducer>> {
        self.base.get_doc_values_reader()
    }

    fn get_postings_reader(&self) -> Result<Option<Self::FieldsProducer>> {
        self.base.get_postings_reader()
    }

    fn get_points_reader(&self) -> Result<Option<Self::PointsReader>> {
        self.base.get_points_reader()
    }
}
