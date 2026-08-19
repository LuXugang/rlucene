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
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::doc_values::SortedDocValuesWithEmpty;
use crate::core::index::doc_values_skipper::DocValuesSkipper;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::index_reader::{CacheHelper, IndexReader, LeafReaderContextKind};
use crate::core::index::knn_vector_values::KnnVectorValues;
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::point_values::PointValues;
use crate::core::index::postings_enum::FREQS;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::term::Term;
use crate::core::index::terms::{Terms, TermsPosting, get_terms};
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIteratorEnum5;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::top_docs::TopDocs;
use crate::core::search::top_docs_collector::EMPTY_TOP_DOCS;
use crate::core::search::top_knn_collector::TopKnnCollector;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::sync::Arc;

/// Provides an interface for accessing an index leaf.
///
/// Search of an index is done entirely through this abstract interface, so that
/// any implementation is searchable. Index readers implemented by this trait do
/// not consist of several sub-readers; they are atomic. They support retrieval
/// of stored fields, doc values, terms, and postings.
///
/// For efficiency, this API often refers to documents via document numbers:
/// non-negative integers that each name a unique document in the index. These
/// document numbers are ephemeral and may change as documents are added to and
/// deleted from an index. Clients should not rely on a document having the same
/// number between sessions.
///
/// NOTE: [`IndexReader`] instances are completely thread safe, meaning multiple
/// threads can call any of their methods concurrently. If your application
/// requires external synchronization, do not synchronize on the reader instance;
/// use your own non-Lucene objects instead.
pub trait LeafReader: IndexReader<ContextKind = LeafReaderContextKind> + Sized {
  type CacheHelper: CacheHelper;

  /// Optional method: return a [`CacheHelper`] that can be used to cache based
  /// on the content of this leaf regardless of deletions.
  ///
  /// Two readers that have the same data but different sets of deleted
  /// documents or doc values updates may be considered equal. Consider using
  /// [`IndexReader::get_reader_cache_helper`] if deletions or doc values updates
  /// need to be taken into account.
  ///
  /// A return value of `None` indicates that this reader is not suited for
  /// caching, which is typically the case for short-lived wrappers that alter
  /// the content of the wrapped leaf reader.
  ///
  /// Experimental: this API follows the original Lucene experimental status.
  fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>>;

  fn doc_freq(&self, term: &Term) -> Result<i32>
  where
    Self: Sized,
  {
    let terms = get_terms(self, term.field())?;
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
    let terms = get_terms(self, term.field())?;
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
  /// Returns the [`Terms`] index for this field, or `None` if it has none.
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
    let Some(terms) = self.terms(term.field())? else {
      return Ok(None);
    };
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
  /// Returns [`NumericDocValues`] for this field, or `None` if no numeric doc
  /// values were indexed for this field.
  ///
  /// The returned instance should only be used by a single thread.
  fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>>;

  type BinaryDocValues: BinaryDocValues;
  /// Returns [`BinaryDocValues`] for this field, or `None` if no binary doc
  /// values were indexed for this field.
  ///
  /// The returned instance should only be used by a single thread.
  fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>>;

  type SortedDocValues: SortedDocValues;
  /// Returns [`SortedDocValues`] for this field, or `None` if no
  /// [`SortedDocValues`] were indexed for this field.
  ///
  /// The returned instance should only be used by a single thread.
  fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>>;

  type SortedNumericDocValues: SortedNumericDocValues;
  /// Returns [`SortedNumericDocValues`] for this field, or `None` if no
  /// [`SortedNumericDocValues`] were indexed for this field.
  ///
  /// The returned instance should only be used by a single thread.
  fn get_sorted_numeric_doc_values(
    &self,
    field: &str,
  ) -> Result<Option<Self::SortedNumericDocValues>>;

  type SortedSetDocValues: SortedSetDocValues;
  /// Returns [`SortedSetDocValues`] for this field, or `None` if no
  /// [`SortedSetDocValues`] were indexed for this field.
  ///
  /// The returned instance should only be used by a single thread.
  fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>>;

  type NormNumericDocValues: NumericDocValues;
  /// Returns [`NumericDocValues`] representing norms for this field, or `None`
  /// if no [`NumericDocValues`] were indexed.
  ///
  /// The returned instance should only be used by a single thread.
  fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>>;

  type DocValuesSkipper: DocValuesSkipper;
  /// Returns a [`DocValuesSkipper`] allowing skipping ranges of doc IDs that
  /// are not of interest, or `None` if a skip index was not indexed.
  ///
  /// The returned instance should be confined to the thread that created it.
  fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>>;

  type FloatVectorValues: FloatVectorValues;
  /// Returns [`FloatVectorValues`] for this field, or `None` if no
  /// [`FloatVectorValues`] were indexed.
  ///
  /// The returned instance should only be used by a single thread.
  ///
  /// Experimental: this API follows the original Lucene experimental status.
  fn get_float_vector_values(&self, field: &str) -> Result<Option<Self::FloatVectorValues>>;

  type ByteVectorValues: ByteVectorValues;
  /// Returns [`ByteVectorValues`] for this field, or `None` if no
  /// [`ByteVectorValues`] were indexed.
  ///
  /// The returned instance should only be used by a single thread.
  ///
  /// Experimental: this API follows the original Lucene experimental status.
  fn get_byte_vector_values(&self, field: &str) -> Result<Option<Self::ByteVectorValues>>;

  /// Returns the k nearest neighbor documents as determined by comparison of
  /// their vector values for this field to the given vector, by the field's
  /// similarity function.
  ///
  /// The score of each document is derived from the vector similarity in a way
  /// that ensures scores are positive and that a larger score corresponds to a
  /// higher ranking.
  ///
  /// The search is allowed to be approximate, meaning the results are not
  /// guaranteed to be the true k closest neighbors. For large values of k, for
  /// example when k is close to the total number of documents, the search may
  /// also retrieve fewer than k documents.
  ///
  /// The returned [`TopDocs`] will contain a [`ScoreDoc`] for each nearest
  /// neighbor, sorted in order of similarity to the query vector with decreasing
  /// scores. The total hits contain the number of documents visited during the
  /// search. If the search stopped early because it hit `visited_limit`, that is
  /// indicated through the total hits relation.
  ///
  /// `accept_docs` represents the allowed documents to match, or `None` if they
  /// are all allowed to match. `visited_limit` is the maximum number of nodes
  /// that the search is allowed to visit.
  ///
  /// Experimental: this API follows the original Lucene experimental status.
  fn search_nearest_vectors_f32_with_limit(
    &self,
    field: &str,
    target: Vec<f32>,
    mut k: usize,
    accept_docs: Option<impl Bits>,
    visited_limit: usize,
  ) -> Result<TopDocs<ScoreDoc>> {
    let fi = self.get_field_infos()?.field_info_by_name(field)?;
    let Some(fi) = fi else {
      return Ok(EMPTY_TOP_DOCS.clone());
    };

    if fi.get_vector_dimension() == 0 {
      return Ok(EMPTY_TOP_DOCS.clone());
    }

    let float_vector_values = self.get_float_vector_values(&fi.name)?;
    let Some(float_vector_values) = float_vector_values else {
      return Ok(EMPTY_TOP_DOCS.clone());
    };

    k = k.min(float_vector_values.size());
    if k == 0 {
      return Ok(EMPTY_TOP_DOCS.clone());
    }

    let mut collector = TopKnnCollector::new(k, visited_limit)?;
    self.search_nearest_vectors_f32(field, target, &mut collector, accept_docs)?;
    collector.top_docs()
  }

  /// Returns the k nearest neighbor documents as determined by comparison of
  /// their vector values for this field to the given vector, by the field's
  /// similarity function.
  ///
  /// The score of each document is derived from the vector similarity in a way
  /// that ensures scores are positive and that a larger score corresponds to a
  /// higher ranking.
  ///
  /// The search is allowed to be approximate, meaning the results are not
  /// guaranteed to be the true k closest neighbors. For large values of k, for
  /// example when k is close to the total number of documents, the search may
  /// also retrieve fewer than k documents.
  ///
  /// The returned [`TopDocs`] will contain a [`ScoreDoc`] for each nearest
  /// neighbor, sorted in order of similarity to the query vector with decreasing
  /// scores. The total hits contain the number of documents visited during the
  /// search. If the search stopped early because it hit `visited_limit`, that is
  /// indicated through the total hits relation.
  ///
  /// `accept_docs` represents the allowed documents to match, or `None` if they
  /// are all allowed to match. `visited_limit` is the maximum number of nodes
  /// that the search is allowed to visit.
  ///
  /// Experimental: this API follows the original Lucene experimental status.
  fn search_nearest_vectors_u8_with_limit(
    &self,
    field: &str,
    target: Vec<u8>,
    mut k: usize,
    accept_docs: Option<impl Bits>,
    visited_limit: usize,
  ) -> Result<TopDocs<ScoreDoc>> {
    let fi = self.get_field_infos()?.field_info_by_name(field)?;
    let Some(fi) = fi else {
      return Ok(EMPTY_TOP_DOCS.clone());
    };

    if fi.get_vector_dimension() == 0 {
      return Ok(EMPTY_TOP_DOCS.clone());
    }

    let float_vector_values = self.get_float_vector_values(&fi.name)?;
    let Some(float_vector_values) = float_vector_values else {
      return Ok(EMPTY_TOP_DOCS.clone());
    };

    k = k.min(float_vector_values.size());
    if k == 0 {
      return Ok(EMPTY_TOP_DOCS.clone());
    }

    let mut collector = TopKnnCollector::new(k, visited_limit)?;
    self.search_nearest_vectors_u8(field, target, &mut collector, accept_docs)?;
    collector.top_docs()
  }

  /// Finds nearest neighbor documents by comparing their vector values for this
  /// field to the given vector, by the field's similarity function.
  ///
  /// The score of each document is derived from the vector similarity in a way
  /// that ensures scores are positive and that a larger score corresponds to a
  /// higher ranking.
  ///
  /// The search is allowed to be approximate, meaning the results are not
  /// guaranteed to be the true k closest neighbors. For large values of k, for
  /// example when k is close to the total number of documents, the search may
  /// also retrieve fewer than k documents.
  ///
  /// Results are gathered by `knn_collector`. `accept_docs` represents the
  /// allowed documents to match, or `None` if they are all allowed to match.
  ///
  /// The behavior is undefined if the given field does not have KNN vectors
  /// enabled on its [`FieldInfo`](crate::core::index::field_info::FieldInfo).
  ///
  /// Experimental: this API follows the original Lucene experimental status.
  fn search_nearest_vectors_f32<B, K>(
    &self,
    field: &str,
    target: Vec<f32>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector;

  /// Finds nearest neighbor documents by comparing their vector values for this
  /// field to the given vector, by the field's similarity function.
  ///
  /// The score of each document is derived from the vector similarity in a way
  /// that ensures scores are positive and that a larger score corresponds to a
  /// higher ranking.
  ///
  /// The search is allowed to be approximate, meaning the results are not
  /// guaranteed to be the true k closest neighbors. For large values of k, for
  /// example when k is close to the total number of documents, the search may
  /// also retrieve fewer than k documents.
  ///
  /// Results are gathered by `knn_collector`. `accept_docs` represents the
  /// allowed documents to match, or `None` if they are all allowed to match.
  ///
  /// The behavior is undefined if the given field does not have KNN vectors
  /// enabled on its [`FieldInfo`](crate::core::index::field_info::FieldInfo).
  ///
  /// Experimental: this API follows the original Lucene experimental status.
  fn search_nearest_vectors_u8<B, K>(
    &self,
    field: &str,
    target: Vec<u8>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector;

  /// Gets the [`FieldInfos`] describing all fields in this reader.
  ///
  /// Implementations should cache the [`FieldInfos`] instance returned by this
  /// method such that subsequent calls return the same instance.
  ///
  /// Experimental: this API follows the original Lucene experimental status.
  fn get_field_infos(&self) -> Result<Arc<FieldInfos>>;

  type Bits: Bits;
  /// Returns the [`Bits`] representing live, not deleted, docs.
  ///
  /// A set bit indicates that the doc ID has not been deleted. If this method
  /// returns `None`, there are no deleted documents and all documents are live.
  ///
  /// The returned instance has been safely published for use by multiple
  /// threads without additional synchronization.
  fn get_live_docs(&self) -> Result<Option<Self::Bits>>;

  type PointValues: PointValues;
  /// Returns the [`PointValues`] used for numeric or spatial searches for the
  /// given field, or `None` if there are no point fields.
  fn get_point_values(&self, field: &str) -> Result<Option<Self::PointValues>>;

  /// Checks consistency of this reader.
  ///
  /// Note that this may be costly in terms of I/O, for example it may involve
  /// computing a checksum value against large data files.
  ///
  /// Internal: this API follows the original Lucene internal status.
  fn check_integrity(&self) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  /// Returns metadata about this leaf.
  ///
  /// Experimental: this API follows the original Lucene experimental status.
  fn get_metadata(&self) -> Result<&LeafMetaData>;
}

pub type LeafPostingsEnum<T> = TermsPosting<T>;

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
pub type LRSortedDocValuesEmpty<LR> = SortedDocValuesWithEmpty<<LR as LeafReader>::SortedDocValues>;
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
// ByteVectorValues
pub type LRByteVectorValues<LR> = <LR as LeafReader>::ByteVectorValues;
// FloatVectorValues
pub type LRFloatVectorValues<LR> = <LR as LeafReader>::FloatVectorValues;
pub type LRDisis<LR> = DocIdSetIteratorEnum5<
  LRNumericDocValues<LR>,
  LRBinaryDocValues<LR>,
  LRSortedDocValues<LR>,
  LRSortedNumericDocValues<LR>,
  LRSortedSetDocValues<LR>,
>;
pub type IRCByteVectorIter<LR> = <LRByteVectorValues<LR> as KnnVectorValues>::DocIndexIterator;
pub type IRCFloatVectorIter<LR> = <LRFloatVectorValues<LR> as KnnVectorValues>::DocIndexIterator;
// Bits
pub type LRBits<LR> = <LR as LeafReader>::Bits;

impl<LR> LeafReader for Arc<LR>
where
  LR: LeafReader,
{
  type CacheHelper = LR::CacheHelper;

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

  type FloatVectorValues = LR::FloatVectorValues;

  fn get_float_vector_values(&self, field: &str) -> Result<Option<Self::FloatVectorValues>> {
    (**self).get_float_vector_values(field)
  }

  type ByteVectorValues = LR::ByteVectorValues;

  fn get_byte_vector_values(&self, field: &str) -> Result<Option<Self::ByteVectorValues>> {
    (**self).get_byte_vector_values(field)
  }

  fn search_nearest_vectors_f32<B, K>(
    &self,
    field: &str,
    target: Vec<f32>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    (**self).search_nearest_vectors_f32(field, target, knn_collector, accept_docs)
  }

  fn search_nearest_vectors_u8<B, K>(
    &self,
    field: &str,
    target: Vec<u8>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    (**self).search_nearest_vectors_u8(field, target, knn_collector, accept_docs)
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

impl<LR> LeafReader for &LR
where
  LR: LeafReader,
{
  type CacheHelper = LR::CacheHelper;

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

  type FloatVectorValues = LR::FloatVectorValues;

  fn get_float_vector_values(&self, field: &str) -> Result<Option<Self::FloatVectorValues>> {
    (**self).get_float_vector_values(field)
  }

  type ByteVectorValues = LR::ByteVectorValues;

  fn get_byte_vector_values(&self, field: &str) -> Result<Option<Self::ByteVectorValues>> {
    (**self).get_byte_vector_values(field)
  }

  fn search_nearest_vectors_f32<B, K>(
    &self,
    field: &str,
    target: Vec<f32>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    (**self).search_nearest_vectors_f32(field, target, knn_collector, accept_docs)
  }

  fn search_nearest_vectors_u8<B, K>(
    &self,
    field: &str,
    target: Vec<u8>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    (**self).search_nearest_vectors_u8(field, target, knn_collector, accept_docs)
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
