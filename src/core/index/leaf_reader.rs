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
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::core::index::doc_values::EmptySorted;
use crate::core::index::doc_values_skipper::DocValuesSkipper;
use crate::core::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::index_reader::{CacheHelper, IndexReader};
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::point_values::PointValues;
use crate::core::index::postings_enum::{FREQS, PostingsEnumEnum2};
use crate::core::index::sorted_doc_values::{SortedDocValues, SortedDocValuesEnum2};
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::term::Term;
use crate::core::index::terms::{Terms, TermsPosting, terms_util};
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIteratorEnum5;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::sync::Arc;

pub trait LeafReader: IndexReader {
    type CacheHelper: CacheHelper;
    fn get_core_cache_helper_ref(&self) -> Result<Option<&Self::CacheHelper>>;
    fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>>;

    fn doc_freq(&self, term: &Term) -> Result<i32>
    where
        Self: Sized,
    {
        let terms = terms_util::get_terms(self, term.field())?;
        let mut terms_enum = terms.iterator()?;

        if terms_enum.seek_exact(term.bytes())? {
            terms_enum.doc_freq()
        } else {
            Ok(0)
        }
    }
    /// Returns the number of documents containing the term `t`.
    /// This method returns `0` if the term or field does not exist.
    /// This method does not take into account deleted documents
    /// that have not yet been merged away.
    fn get_total_term_freq(&self, term: &Term) -> Result<i64>
    where
        Self: Sized,
    {
        let terms = terms_util::get_terms(self, term.field())?;
        let mut terms_enum = terms.iterator()?;

        if terms_enum.seek_exact(term.bytes())? {
            terms_enum.total_term_freq()
        } else {
            Ok(0)
        }
    }
    fn get_sum_doc_freq(&self, field: &str) -> Result<i64>
    where
        Self: Sized,
    {
        if let Some(terms) = self.terms(field)? {
            terms.get_sum_doc_freq()
        } else {
            Ok(0)
        }
    }

    fn get_doc_count(&self, field: &str) -> Result<i32>
    where
        Self: Sized,
    {
        if let Some(terms) = self.terms(field)? {
            terms.get_doc_count()
        } else {
            Ok(0)
        }
    }

    fn get_sum_total_term_freq(&self, field: &str) -> Result<i64>
    where
        Self: Sized,
    {
        if let Some(terms) = self.terms(field)? {
            terms.get_sum_total_term_freq()
        } else {
            Ok(0)
        }
    }

    type Terms: Terms;
    fn terms(&self, field: &str) -> Result<Option<Self::Terms>>;
    /// Returns [`PostingsEnum`](crate::core::index::postings_enum::PostingsEnum) for the specified term.
    /// This will return `None` if either the field or term does not exist.
    ///
    /// **NOTE:** The returned [`PostingsEnum`](crate::core::index::postings_enum::PostingsEnum) may contain deleted docs.
    ///
    /// See [`TermsEnum::postings`].
    fn postings_with_flag(
        &self,
        term: &Term,
        flags: i32,
    ) -> Result<Option<LeafPostingsEnum<Self::Terms>>>
    where
        Self: Sized,
    {
        let terms = terms_util::get_terms(self, term.field())?;
        let mut terms_enum = terms.iterator()?;
        if terms_enum.seek_exact(term.bytes())? {
            Ok(Some(terms_enum.postings_with_flags(None, flags)?))
        } else {
            Ok(None)
        }
    }
    /// Returns [`PostingsEnum`](crate::core::index::postings_enum::PostingsEnum) for the specified term with [`FREQS`].
    ///
    /// Use this method if you only require documents and frequencies,
    /// and do not need any proximity data.
    /// This method is equivalent to [`Self::postings_with_flag`].
    ///
    /// **NOTE:** The returned [`PostingsEnum`](crate::core::index::postings_enum::PostingsEnum) may contain deleted docs.
    ///
    /// See [`Self::postings_with_flag`].
    fn postings(&self, term: &Term) -> Result<Option<LeafPostingsEnum<Self::Terms>>>
    where
        Self: Sized,
    {
        self.postings_with_flag(term, FREQS as i32)
    }

    type NumericDocValues: NumericDocValues;
    fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>>;

    type BinaryDocValues: BinaryDocValues;
    fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>>;

    type SortedDocValues: SortedDocValues;
    fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>>;

    type SortedNumericDocValues: SortedNumericDocValues;
    fn get_sorted_numeric_doc_values(
        &self,
        field: &str,
    ) -> Result<Option<Self::SortedNumericDocValues>>;

    type SortedSetDocValues: SortedSetDocValues;
    fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>>;

    type NormNumericDocValues: NumericDocValues;
    fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>>;

    type DocValuesSkipper: DocValuesSkipper;
    fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>>;

    fn get_field_infos(&self) -> Result<Arc<FieldInfos>>;

    type Bits: Bits;
    fn get_live_docs(&self) -> Result<Option<Self::Bits>>;

    type PointValues: PointValues;
    fn get_point_values(&self, field: &str) -> Result<Option<Self::PointValues>>;

    fn check_integrity(&self) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn get_metadata(&self) -> Result<&LeafMetaData>;
}
pub(crate) fn get_context<LR>(leaf_reader: LR) -> Result<LeafReaderContext<LR>>
where
    LR: LeafReader,
{
    Ok(LeafReaderContext::from_top_lr(leaf_reader))
}

// DummyPostingsEnum from  EmptyTerms's EmptyTermsEnum's PostingsEnum
pub type LeafPostingsEnum<T> = PostingsEnumEnum2<TermsPosting<T>, DummyPostingsEnum>;

// TermsEnum
pub type LRTermsEnum<LR> = <<LR as LeafReader>::Terms as Terms>::TermsEnum;
// NumericDocValues
pub type LRNumericDocValues<LR> = <LR as LeafReader>::NumericDocValues;
// BinaryDocValues
pub type LRBinaryDocValues<LR> = <LR as LeafReader>::BinaryDocValues;
// SortedNumericDocValues
pub type LRSortedNumericDocValues<LR> = <LR as LeafReader>::SortedNumericDocValues;
// SortedDocValues
pub type LRSortedDocValues<LR> = <LR as LeafReader>::SortedDocValues;
// SortedSetDocValues
pub type LRSortedSetDocValues<LR> = <LR as LeafReader>::SortedSetDocValues;
pub type LRSortedDocValuesEmpty<LR> =
    SortedDocValuesEnum2<<LR as LeafReader>::SortedDocValues, EmptySorted>;
// ImpactsEnum
pub type LRImpactsEnum<LR> =
    <<<LR as LeafReader>::Terms as Terms>::TermsEnum as TermsEnum>::ImpactsEnum;
// PostingsEnum
pub type LRPosting<LR> =
    <<<LR as LeafReader>::Terms as Terms>::TermsEnum as TermsEnum>::PostingsEnum;
pub type LRNormNumericDocValues<LR> = <LR as LeafReader>::NormNumericDocValues;
// DocValuesSkipper
pub type LRDocValuesSkipper<LR> = <LR as LeafReader>::DocValuesSkipper;
// PointValues
pub type LRPointValues<LR> = <LR as LeafReader>::PointValues;
// CacherHelp
pub type LRCacherHelper<LR> = <LR as LeafReader>::CacheHelper;

pub type LRDisis<LR> = DocIdSetIteratorEnum5<
    LRNumericDocValues<LR>,
    LRBinaryDocValues<LR>,
    LRSortedDocValues<LR>,
    LRSortedNumericDocValues<LR>,
    LRSortedSetDocValues<LR>,
>;
// Bits
pub type LRBits<LR> = <LR as LeafReader>::Bits;

impl<LR> LeafReader for Arc<LR>
where
    LR: LeafReader,
{
    type CacheHelper = LR::CacheHelper;

    fn get_core_cache_helper_ref(&self) -> Result<Option<&Self::CacheHelper>> {
        (**self).get_core_cache_helper_ref()
    }

    fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
        (**self).get_core_cache_helper()
    }

    fn doc_freq(&self, term: &Term) -> Result<i32>
    where
        Self: Sized,
    {
        LeafReader::doc_freq(&(**self), term)
    }

    fn get_total_term_freq(&self, term: &Term) -> Result<i64>
    where
        Self: Sized,
    {
        (**self).get_total_term_freq(term)
    }

    fn get_sum_doc_freq(&self, field: &str) -> Result<i64>
    where
        Self: Sized,
    {
        LeafReader::get_sum_doc_freq(&(**self), field)
    }

    fn get_doc_count(&self, field: &str) -> Result<i32>
    where
        Self: Sized,
    {
        LeafReader::get_doc_count(&(**self), field)
    }

    fn get_sum_total_term_freq(&self, field: &str) -> Result<i64>
    where
        Self: Sized,
    {
        LeafReader::get_sum_total_term_freq(&(**self), field)
    }

    type Terms = LR::Terms;

    fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
        (**self).terms(field)
    }

    fn postings_with_flag(
        &self,
        term: &Term,
        flags: i32,
    ) -> Result<Option<LeafPostingsEnum<Self::Terms>>>
    where
        Self: Sized,
    {
        (**self).postings_with_flag(term, flags)
    }

    fn postings(&self, term: &Term) -> Result<Option<LeafPostingsEnum<Self::Terms>>>
    where
        Self: Sized,
    {
        (**self).postings(term)
    }

    type NumericDocValues = LR::NumericDocValues;

    fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>> {
        (**self).get_numeric_doc_values(field)
    }

    type BinaryDocValues = LR::BinaryDocValues;

    fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>> {
        (**self).get_binary_doc_values(field)
    }

    type SortedDocValues = LR::SortedDocValues;

    fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>> {
        (**self).get_sorted_doc_values(field)
    }

    type SortedNumericDocValues = LR::SortedNumericDocValues;

    fn get_sorted_numeric_doc_values(
        &self,
        field: &str,
    ) -> Result<Option<Self::SortedNumericDocValues>> {
        (**self).get_sorted_numeric_doc_values(field)
    }

    type SortedSetDocValues = LR::SortedSetDocValues;

    fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>> {
        (**self).get_sorted_set_doc_values(field)
    }

    type NormNumericDocValues = LR::NormNumericDocValues;

    fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>> {
        (**self).get_norm_values(field)
    }

    type DocValuesSkipper = LR::DocValuesSkipper;

    fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>> {
        (**self).get_doc_values_skipper(field)
    }

    fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
        (**self).get_field_infos()
    }

    type Bits = LR::Bits;

    fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
        (**self).get_live_docs()
    }

    type PointValues = LR::PointValues;

    fn get_point_values(&self, field: &str) -> Result<Option<Self::PointValues>> {
        (**self).get_point_values(field)
    }

    fn check_integrity(&self) -> Result<()> {
        (**self).check_integrity()
    }

    fn get_metadata(&self) -> Result<&LeafMetaData> {
        (**self).get_metadata()
    }
}
