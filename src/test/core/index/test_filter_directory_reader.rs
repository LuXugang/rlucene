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
use crate::core::document::document::Document;
use crate::core::index::base_composite_reader::{
  BCRStoredFieldsImpl, BCRTermVectorsImpl, BaseCompositeReader, BaseCompositeReaderBase,
};
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::directory_reader::{self, DirectoryReader, DirectoryReaderBase};
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::filter_directory_reader::{
  DelegatingCacheHelper, FilterDirectoryReader, SubReaderWrapper,
};
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_reader::{
  CacheHelper, CacheKey, CompositeReaderContextKind, IndexReader, IndexReaderBase,
  LeafReaderContextKind,
};
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::term::Term;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::store::directory::Directory;
use crate::core::util::IOUtils;
use crate::core::util::bits::Bits;
use crate::core::util::close::CloseableRef;
use crate::core::util::dummy::dummy_comparator::DummyComparator;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, random,
};
use rand::RngExt;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

#[allow(dead_code)] // for quick search
struct TestFilterDirectoryReader;

struct DummySubReaderWrapper;

impl<LR> SubReaderWrapper<LR> for DummySubReaderWrapper
where
  LR: LeafReader,
{
  type LeafReader1 = LR;

  fn wrap_readers(&self, readers: Vec<LR>) -> Result<Vec<Self::LeafReader1>> {
    self.default_wrap_readers(readers)
  }

  type LeafReader2 = LR;

  fn wrap(&self, reader: LR) -> Result<Self::LeafReader2> {
    Ok(reader)
  }
}

struct DummyFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  in_: DR,
  base: BaseCompositeReaderBase<DR::LeafReader>,
  index_base: IndexReaderBase,
}

impl<DR> DummyFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  fn new(in_: DR) -> Result<Self> {
    let wrapper = DummySubReaderWrapper;
    let readers = wrapper.wrap_readers(in_.get_sequential_sub_readers().to_vec())?;
    let index_base = IndexReaderBase::new();
    let base = BaseCompositeReaderBase::new::<DummyComparator>(readers, None, &index_base)?;
    Ok(Self {
      in_,
      base,
      index_base,
    })
  }
}

impl<DR> BaseCompositeReader for DummyFilterDirectoryReader<DR> where DR: DirectoryReader {}

impl<DR> CompositeReader for DummyFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  type LeafReader = DR::LeafReader;
  type SubReader = Self::LeafReader;

  fn get_sequential_sub_readers(&self) -> &[Self::SubReader] {
    self.base.get_sequential_sub_readers()
  }

  fn visit_leaves<F>(&self, visitor: &mut F) -> Result<()>
  where
    F: FnMut(&Self::LeafReader) -> Result<()>,
  {
    for reader in self.get_sequential_sub_readers() {
      visitor(reader)?;
    }
    Ok(())
  }

  fn to_string(&self) -> String {
    format!("DummyFilterDirectoryReader({})", self.in_.to_string())
  }
}

impl<DR> IndexReader for DummyFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  type ContextKind = CompositeReaderContextKind;

  type TermVectors = BCRTermVectorsImpl<<Self as CompositeReader>::LeafReader>;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    self.base.term_vector(self)
  }

  fn max_doc(&self) -> Result<i32> {
    Ok(self.base.max_doc())
  }

  fn num_docs(&self) -> Result<i32> {
    self.base.num_docs()
  }

  type StoredFields = BCRStoredFieldsImpl<<Self as CompositeReader>::LeafReader>;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    self.base.stored_fields(self)
  }

  fn do_close(&self) -> Result<()> {
    self.in_.close()
  }

  type ReaderCacheHelper = DR::ReaderCacheHelper;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    self.in_.get_reader_cache_helper()
  }

  fn doc_freq(&self, term: &Term) -> Result<i32> {
    self.base.doc_freq(term, self)
  }

  fn total_term_freq(&self, term: &Term) -> Result<i64> {
    self.base.total_term_freq(term, self)
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
    self.base.get_sum_doc_freq(field, self)
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    self.base.get_doc_count(field, self)
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    self.base.get_sum_total_term_freq(field, self)
  }

  fn index_base(&self) -> &IndexReaderBase {
    &self.index_base
  }
}

impl<DR> Display for DummyFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", CompositeReader::to_string(self))
  }
}

impl<DR> DirectoryReader for DummyFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  type DirectoryReader = DummyFilterDirectoryReader<DR::DirectoryReader>;

  fn do_open_if_changed(&self) -> Result<Option<Self::DirectoryReader>> {
    self.wrap_directory_reader(self.in_.do_open_if_changed()?)
  }

  fn do_open_if_changed_with_commit<IC>(
    &self,
    commit: Option<&IC>,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    IC: IndexCommit<Directory = Arc<Self::Directory>>,
  {
    self.wrap_directory_reader(self.in_.do_open_if_changed_with_commit(commit)?)
  }

  fn do_open_if_changed_with_deletes(
    &self,
    writer: &Arc<IndexWriter<Self::Directory>>,
    apply_deletes: bool,
  ) -> Result<Option<Self::DirectoryReader>> {
    self.wrap_directory_reader(
      self
        .in_
        .do_open_if_changed_with_deletes(writer, apply_deletes)?,
    )
  }

  fn get_version(&self) -> Result<i64> {
    self.in_.get_version()
  }

  fn is_current(&self) -> Result<bool> {
    self.in_.is_current()
  }

  type IndexCommit = DR::IndexCommit;

  fn get_index_commit(&self) -> Result<Self::IndexCommit> {
    self.in_.get_index_commit()
  }

  type Directory = DR::Directory;

  fn directory(&self) -> &DirectoryReaderBase<Self::Directory> {
    self.in_.directory()
  }
}

impl<DR> FilterDirectoryReader for DummyFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  type Delegate = DR;

  fn get_delegate(&self) -> &Self::Delegate {
    &self.in_
  }

  type WrapDirectoryReader = DummyFilterDirectoryReader<DR::DirectoryReader>;

  fn do_wrap_directory_reader(
    &self,
    in_: Option<<Self::Delegate as DirectoryReader>::DirectoryReader>,
  ) -> Result<Option<Self::WrapDirectoryReader>> {
    in_.map(DummyFilterDirectoryReader::new).transpose()
  }
}

#[test]
fn test_double_close() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;
  w.add_document(Document::new())?;

  let reader = Arc::new(directory_reader::open_from_writer(&w)?);
  let wrapped = DummyFilterDirectoryReader::new(reader.clone())?;

  // Calling close() on the original reader and wrapped reader should only close
  // the original reader once (as per Closeable.close() contract that close() is
  // idempotent).
  let close_wrapped_first = random.random_bool(0.5);
  IOUtils::close(0..2, |index| {
    if (index == 0) == close_wrapped_first {
      wrapped.close()
    } else {
      reader.close()
    }
  })?;

  w.close()?;
  dir.close()
}

struct NumDocsCountingSubReaderWrapper {
  num_docs_call_count: Arc<AtomicI64>,
}

impl NumDocsCountingSubReaderWrapper {
  fn new(num_docs_call_count: Arc<AtomicI64>) -> Self {
    Self {
      num_docs_call_count,
    }
  }
}

impl<LR> SubReaderWrapper<LR> for NumDocsCountingSubReaderWrapper
where
  LR: LeafReader,
{
  type LeafReader1 = Self::LeafReader2;

  fn wrap_readers(&self, readers: Vec<LR>) -> Result<Vec<Self::LeafReader1>> {
    self.default_wrap_readers(readers)
  }

  type LeafReader2 = NumDocsCountingLeafReader<LR>;

  fn wrap(&self, reader: LR) -> Result<Self::LeafReader2> {
    NumDocsCountingLeafReader::new(reader, self.num_docs_call_count.clone())
  }
}

struct NumDocsCountingLeafReader<LR>
where
  LR: LeafReader,
{
  in_: LR,
  num_docs_call_count: Arc<AtomicI64>,
  index_base: IndexReaderBase,
}

impl<LR> NumDocsCountingLeafReader<LR>
where
  LR: LeafReader,
{
  fn new(in_: LR, num_docs_call_count: Arc<AtomicI64>) -> Result<Self> {
    let index_base = IndexReaderBase::new();
    in_.register_parent_reader(&index_base)?;
    Ok(Self {
      in_,
      num_docs_call_count,
      index_base,
    })
  }
}

impl<LR> Clone for NumDocsCountingLeafReader<LR>
where
  LR: LeafReader + Clone,
{
  fn clone(&self) -> Self {
    Self {
      in_: self.in_.clone(),
      num_docs_call_count: self.num_docs_call_count.clone(),
      index_base: self.index_base.clone(),
    }
  }
}

impl<LR> Display for NumDocsCountingLeafReader<LR>
where
  LR: LeafReader,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "NumDocsCountingLeafReader({})", self.in_)
  }
}

impl<LR> IndexReader for NumDocsCountingLeafReader<LR>
where
  LR: LeafReader,
{
  type ContextKind = LeafReaderContextKind;

  type TermVectors = LR::TermVectors;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    self.in_.term_vectors()
  }

  fn max_doc(&self) -> Result<i32> {
    self.in_.max_doc()
  }

  fn num_docs(&self) -> Result<i32> {
    self.num_docs_call_count.fetch_add(1, Ordering::SeqCst);
    self.in_.num_docs()
  }

  type StoredFields = LR::StoredFields;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    self.in_.stored_fields()
  }

  fn do_close(&self) -> Result<()> {
    self.in_.close()
  }

  type ReaderCacheHelper = LR::ReaderCacheHelper;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    self.in_.get_reader_cache_helper()
  }

  fn doc_freq(&self, term: &Term) -> Result<i32> {
    IndexReader::doc_freq(&self.in_, term)
  }

  fn total_term_freq(&self, term: &Term) -> Result<i64> {
    self.in_.total_term_freq(term)
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
    IndexReader::get_sum_doc_freq(&self.in_, field)
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    IndexReader::get_doc_count(&self.in_, field)
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    IndexReader::get_sum_total_term_freq(&self.in_, field)
  }

  fn index_base(&self) -> &IndexReaderBase {
    &self.index_base
  }
}

impl<LR> LeafReader for NumDocsCountingLeafReader<LR>
where
  LR: LeafReader,
{
  type CacheHelper = LR::CacheHelper;

  fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
    self.in_.get_core_cache_helper()
  }

  type Terms = LR::Terms;

  fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
    self.in_.terms(field)
  }

  type NumericDocValues = LR::NumericDocValues;

  fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>> {
    self.in_.get_numeric_doc_values(field)
  }

  type BinaryDocValues = LR::BinaryDocValues;

  fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>> {
    self.in_.get_binary_doc_values(field)
  }

  type SortedDocValues = LR::SortedDocValues;

  fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>> {
    self.in_.get_sorted_doc_values(field)
  }

  type SortedNumericDocValues = LR::SortedNumericDocValues;

  fn get_sorted_numeric_doc_values(
    &self,
    field: &str,
  ) -> Result<Option<Self::SortedNumericDocValues>> {
    self.in_.get_sorted_numeric_doc_values(field)
  }

  type SortedSetDocValues = LR::SortedSetDocValues;

  fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>> {
    self.in_.get_sorted_set_doc_values(field)
  }

  type NormNumericDocValues = LR::NormNumericDocValues;

  fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>> {
    self.in_.get_norm_values(field)
  }

  type DocValuesSkipper = LR::DocValuesSkipper;

  fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>> {
    self.in_.get_doc_values_skipper(field)
  }

  type FloatVectorValues = LR::FloatVectorValues;

  fn get_float_vector_values(&self, field: &str) -> Result<Option<Self::FloatVectorValues>> {
    self.in_.get_float_vector_values(field)
  }

  type ByteVectorValues = LR::ByteVectorValues;

  fn get_byte_vector_values(&self, field: &str) -> Result<Option<Self::ByteVectorValues>> {
    self.in_.get_byte_vector_values(field)
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
    self
      .in_
      .search_nearest_vectors_f32(field, target, knn_collector, accept_docs)
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
    self
      .in_
      .search_nearest_vectors_u8(field, target, knn_collector, accept_docs)
  }

  fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
    self.in_.get_field_infos()
  }

  type Bits = LR::Bits;

  fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
    self.in_.get_live_docs()
  }

  type PointValues = LR::PointValues;

  fn get_point_values(&self, field: &str) -> Result<Option<Self::PointValues>> {
    self.in_.get_point_values(field)
  }

  fn check_integrity(&self) -> Result<()> {
    self.in_.check_integrity()
  }

  fn get_metadata(&self) -> Result<&LeafMetaData> {
    self.in_.get_metadata()
  }
}

struct NumDocsCountingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  in_: DR,
  num_docs_call_count: Arc<AtomicI64>,
  base: BaseCompositeReaderBase<NumDocsCountingLeafReader<DR::LeafReader>>,
  index_base: IndexReaderBase,
}

impl<DR> NumDocsCountingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  fn new(in_: DR, num_docs_call_count: Arc<AtomicI64>) -> Result<Self> {
    let wrapper = NumDocsCountingSubReaderWrapper::new(num_docs_call_count.clone());
    let readers = wrapper.wrap_readers(in_.get_sequential_sub_readers().to_vec())?;
    let index_base = IndexReaderBase::new();
    let base = BaseCompositeReaderBase::new::<DummyComparator>(readers, None, &index_base)?;
    Ok(Self {
      in_,
      num_docs_call_count,
      base,
      index_base,
    })
  }
}

impl<DR> BaseCompositeReader for NumDocsCountingFilterDirectoryReader<DR> where DR: DirectoryReader {}

impl<DR> CompositeReader for NumDocsCountingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  type LeafReader = NumDocsCountingLeafReader<DR::LeafReader>;
  type SubReader = Self::LeafReader;

  fn get_sequential_sub_readers(&self) -> &[Self::SubReader] {
    self.base.get_sequential_sub_readers()
  }

  fn visit_leaves<F>(&self, visitor: &mut F) -> Result<()>
  where
    F: FnMut(&Self::LeafReader) -> Result<()>,
  {
    for reader in self.get_sequential_sub_readers() {
      visitor(reader)?;
    }
    Ok(())
  }

  fn to_string(&self) -> String {
    format!(
      "NumDocsCountingFilterDirectoryReader({})",
      self.in_.to_string()
    )
  }
}

impl<DR> IndexReader for NumDocsCountingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  type ContextKind = CompositeReaderContextKind;

  type TermVectors = BCRTermVectorsImpl<<Self as CompositeReader>::LeafReader>;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    self.base.term_vector(self)
  }

  fn max_doc(&self) -> Result<i32> {
    Ok(self.base.max_doc())
  }

  fn num_docs(&self) -> Result<i32> {
    self.base.num_docs()
  }

  type StoredFields = BCRStoredFieldsImpl<<Self as CompositeReader>::LeafReader>;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    self.base.stored_fields(self)
  }

  fn do_close(&self) -> Result<()> {
    self.in_.close()
  }

  type ReaderCacheHelper = DR::ReaderCacheHelper;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    self.in_.get_reader_cache_helper()
  }

  fn doc_freq(&self, term: &Term) -> Result<i32> {
    self.base.doc_freq(term, self)
  }

  fn total_term_freq(&self, term: &Term) -> Result<i64> {
    self.base.total_term_freq(term, self)
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
    self.base.get_sum_doc_freq(field, self)
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    self.base.get_doc_count(field, self)
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    self.base.get_sum_total_term_freq(field, self)
  }

  fn index_base(&self) -> &IndexReaderBase {
    &self.index_base
  }
}

impl<DR> Display for NumDocsCountingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", CompositeReader::to_string(self))
  }
}

impl<DR> DirectoryReader for NumDocsCountingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  type DirectoryReader = NumDocsCountingFilterDirectoryReader<DR::DirectoryReader>;

  fn do_open_if_changed(&self) -> Result<Option<Self::DirectoryReader>> {
    self.wrap_directory_reader(self.in_.do_open_if_changed()?)
  }

  fn do_open_if_changed_with_commit<IC>(
    &self,
    commit: Option<&IC>,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    IC: IndexCommit<Directory = Arc<Self::Directory>>,
  {
    self.wrap_directory_reader(self.in_.do_open_if_changed_with_commit(commit)?)
  }

  fn do_open_if_changed_with_deletes(
    &self,
    writer: &Arc<IndexWriter<Self::Directory>>,
    apply_deletes: bool,
  ) -> Result<Option<Self::DirectoryReader>> {
    self.wrap_directory_reader(
      self
        .in_
        .do_open_if_changed_with_deletes(writer, apply_deletes)?,
    )
  }

  fn get_version(&self) -> Result<i64> {
    self.in_.get_version()
  }

  fn is_current(&self) -> Result<bool> {
    self.in_.is_current()
  }

  type IndexCommit = DR::IndexCommit;

  fn get_index_commit(&self) -> Result<Self::IndexCommit> {
    self.in_.get_index_commit()
  }

  type Directory = DR::Directory;

  fn directory(&self) -> &DirectoryReaderBase<Self::Directory> {
    self.in_.directory()
  }
}

impl<DR> FilterDirectoryReader for NumDocsCountingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
{
  type Delegate = DR;

  fn get_delegate(&self) -> &Self::Delegate {
    &self.in_
  }

  type WrapDirectoryReader = NumDocsCountingFilterDirectoryReader<DR::DirectoryReader>;

  fn do_wrap_directory_reader(
    &self,
    in_: Option<<Self::Delegate as DirectoryReader>::DirectoryReader>,
  ) -> Result<Option<Self::WrapDirectoryReader>> {
    in_
      .map(|reader| {
        NumDocsCountingFilterDirectoryReader::new(reader, self.num_docs_call_count.clone())
      })
      .transpose()
  }
}

#[test]
fn test_filter_directory_reader_num_docs_is_lazy() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;
  w.add_document(Document::new())?;
  let directory_reader = Arc::new(directory_reader::open_from_writer(&w)?);
  w.close()?;

  let num_docs_call_count = Arc::new(AtomicI64::new(0));
  let directory_reader_wrapper = NumDocsCountingFilterDirectoryReader::new(
    directory_reader.clone(),
    num_docs_call_count.clone(),
  )?;
  assert_eq!(0, num_docs_call_count.load(Ordering::SeqCst));
  assert_eq!(1, directory_reader_wrapper.num_docs()?);
  assert_eq!(1, num_docs_call_count.load(Ordering::SeqCst)); // one segment, so called once
  assert_eq!(1, directory_reader_wrapper.num_docs()?);
  assert_eq!(1, num_docs_call_count.load(Ordering::SeqCst));

  directory_reader.close()?;
  dir.close()
}

struct DummyLastingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
  DR::ReaderCacheHelper: Clone,
{
  in_: DR,
  base: BaseCompositeReaderBase<DR::LeafReader>,
  index_base: IndexReaderBase,
  cache_helper: Option<DelegatingCacheHelper<DR::ReaderCacheHelper>>,
}

impl<DR> DummyLastingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
  DR::ReaderCacheHelper: Clone,
{
  fn new(in_: DR) -> Result<Self> {
    let wrapper = DummySubReaderWrapper;
    let readers = wrapper.wrap_readers(in_.get_sequential_sub_readers().to_vec())?;
    let index_base = IndexReaderBase::new();
    let base = BaseCompositeReaderBase::new::<DummyComparator>(readers, None, &index_base)?;
    let cache_helper = in_
      .get_reader_cache_helper()?
      .map(DelegatingCacheHelper::new);
    Ok(Self {
      in_,
      base,
      index_base,
      cache_helper,
    })
  }
}

impl<DR> BaseCompositeReader for DummyLastingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
  DR::ReaderCacheHelper: Clone,
{
}

impl<DR> CompositeReader for DummyLastingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
  DR::ReaderCacheHelper: Clone,
{
  type LeafReader = DR::LeafReader;
  type SubReader = Self::LeafReader;

  fn get_sequential_sub_readers(&self) -> &[Self::SubReader] {
    self.base.get_sequential_sub_readers()
  }

  fn visit_leaves<F>(&self, visitor: &mut F) -> Result<()>
  where
    F: FnMut(&Self::LeafReader) -> Result<()>,
  {
    for reader in self.get_sequential_sub_readers() {
      visitor(reader)?;
    }
    Ok(())
  }

  fn to_string(&self) -> String {
    format!(
      "DummyLastingFilterDirectoryReader({})",
      self.in_.to_string()
    )
  }
}

impl<DR> IndexReader for DummyLastingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
  DR::ReaderCacheHelper: Clone,
{
  type ContextKind = CompositeReaderContextKind;

  type TermVectors = BCRTermVectorsImpl<<Self as CompositeReader>::LeafReader>;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    self.base.term_vector(self)
  }

  fn max_doc(&self) -> Result<i32> {
    Ok(self.base.max_doc())
  }

  fn num_docs(&self) -> Result<i32> {
    self.base.num_docs()
  }

  type StoredFields = BCRStoredFieldsImpl<<Self as CompositeReader>::LeafReader>;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    self.base.stored_fields(self)
  }

  fn do_close(&self) -> Result<()> {
    self.in_.close()
  }

  type ReaderCacheHelper = DelegatingCacheHelper<DR::ReaderCacheHelper>;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    Ok(self.cache_helper.clone())
  }

  fn doc_freq(&self, term: &Term) -> Result<i32> {
    self.base.doc_freq(term, self)
  }

  fn total_term_freq(&self, term: &Term) -> Result<i64> {
    self.base.total_term_freq(term, self)
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
    self.base.get_sum_doc_freq(field, self)
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    self.base.get_doc_count(field, self)
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    self.base.get_sum_total_term_freq(field, self)
  }

  fn index_base(&self) -> &IndexReaderBase {
    &self.index_base
  }
}

impl<DR> Display for DummyLastingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
  DR::ReaderCacheHelper: Clone,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", CompositeReader::to_string(self))
  }
}

impl<DR> DirectoryReader for DummyLastingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
  DR::ReaderCacheHelper: Clone,
{
  type DirectoryReader = DummyFilterDirectoryReader<DR::DirectoryReader>;

  fn do_open_if_changed(&self) -> Result<Option<Self::DirectoryReader>> {
    self.wrap_directory_reader(self.in_.do_open_if_changed()?)
  }

  fn do_open_if_changed_with_commit<IC>(
    &self,
    commit: Option<&IC>,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    IC: IndexCommit<Directory = Arc<Self::Directory>>,
  {
    self.wrap_directory_reader(self.in_.do_open_if_changed_with_commit(commit)?)
  }

  fn do_open_if_changed_with_deletes(
    &self,
    writer: &Arc<IndexWriter<Self::Directory>>,
    apply_deletes: bool,
  ) -> Result<Option<Self::DirectoryReader>> {
    self.wrap_directory_reader(
      self
        .in_
        .do_open_if_changed_with_deletes(writer, apply_deletes)?,
    )
  }

  fn get_version(&self) -> Result<i64> {
    self.in_.get_version()
  }

  fn is_current(&self) -> Result<bool> {
    self.in_.is_current()
  }

  type IndexCommit = DR::IndexCommit;

  fn get_index_commit(&self) -> Result<Self::IndexCommit> {
    self.in_.get_index_commit()
  }

  type Directory = DR::Directory;

  fn directory(&self) -> &DirectoryReaderBase<Self::Directory> {
    self.in_.directory()
  }
}

impl<DR> FilterDirectoryReader for DummyLastingFilterDirectoryReader<DR>
where
  DR: DirectoryReader,
  DR::ReaderCacheHelper: Clone,
{
  type Delegate = DR;

  fn get_delegate(&self) -> &Self::Delegate {
    &self.in_
  }

  type WrapDirectoryReader = DummyFilterDirectoryReader<DR::DirectoryReader>;

  fn do_wrap_directory_reader(
    &self,
    in_: Option<<Self::Delegate as DirectoryReader>::DirectoryReader>,
  ) -> Result<Option<Self::WrapDirectoryReader>> {
    in_.map(DummyFilterDirectoryReader::new).transpose()
  }
}

#[test]
fn test_delegating_cache_helper() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;
  w.add_document(Document::new())?;

  let reader = Arc::new(directory_reader::open_from_writer(&w)?);
  let wrapped = DummyLastingFilterDirectoryReader::new(reader.clone())?;

  let reader_cache_helper = reader
    .get_reader_cache_helper()?
    .expect("reader cache helper must exist");
  let wrapped_cache_helper = wrapped
    .get_reader_cache_helper()?
    .expect("wrapped reader cache helper must exist");
  assert_ne!(
    reader_cache_helper.get_key(),
    wrapped_cache_helper.get_key()
  );

  let close_called_counter = Arc::new(AtomicI64::new(0));
  let listener_counter = close_called_counter.clone();
  let wrapped_cache_key = wrapped_cache_helper.get_key();
  wrapped_cache_helper.add_closed_listener(Arc::new(move |key: &CacheKey| {
    listener_counter.fetch_add(1, Ordering::SeqCst);
    assert_eq!(key, &wrapped_cache_key);
    Ok(())
  }))?;

  reader.close()?;
  assert_eq!(1, close_called_counter.load(Ordering::SeqCst));
  wrapped.close()?;
  assert_eq!(1, close_called_counter.load(Ordering::SeqCst));

  w.close()?;
  dir.close()
}
