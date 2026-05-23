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
use crate::core::index::base_composite_reader::BaseCompositeReader;
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::directory_reader::{DirectoryReader, DirectoryReaderBase};
use crate::core::index::dummy::dummy_cache_helper::DummyCacheHelper;
use crate::core::index::dummy::dummy_composite_reader::DummyCompositeReader;
use crate::core::index::dummy::dummy_index_commit::DummyIndexCommit;
use crate::core::index::dummy::dummy_leaf_reader::DummyLeafReader;
use crate::core::index::dummy::dummy_stored_fields::DummyStoredFields;
use crate::core::index::dummy::dummy_term_vectors::DummyTermVectors;
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_reader::{IndexReader, IndexReaderBase, IndexReaderEnum};
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
use crate::core::index::term::Term;
use crate::core::store::directory::Directory;
use crate::core::store::dummy::dummy_directory::DummyDirectory;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

pub struct DummyDirectoryReader<D>
where
  D: Directory,
{
  _marker: std::marker::PhantomData<D>,
}

impl<D> BaseCompositeReader for DummyDirectoryReader<D> where D: Directory {}

impl<D> CompositeReader for DummyDirectoryReader<D>
where
  D: Directory,
{
  type LeafReader = DummyLeafReader;
  type SubCompositeReader = DummyCompositeReader<DummyLeafReader>;

  fn get_sequential_sub_readers(
    &self,
  ) -> &[IndexReaderEnum<Self::LeafReader, Self::SubCompositeReader>] {
    dummy_unreachable!()
  }

  fn to_string(&self) -> String {
    dummy_unreachable!()
  }
}

impl<D> IndexReader for DummyDirectoryReader<D>
where
  D: Directory,
{
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

impl<D> Display for DummyDirectoryReader<D>
where
  D: Directory,
{
  fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
    dummy_unreachable!()
  }
}

impl<D> DirectoryReader for DummyDirectoryReader<D>
where
  D: Directory,
{
  type DirectoryReader = DummyDirectoryReader<D>;

  fn do_open_if_changed(&self) -> Result<Option<Self::DirectoryReader>> {
    dummy_unreachable!()
  }

  fn do_open_if_changed_with_commit<IC>(
    &self,
    _commit: Option<&IC>,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    IC: IndexCommit,
  {
    dummy_unreachable!()
  }

  fn do_open_if_changed_with_index_writer<B>(
    &self,
    _writer: IndexWriter<Self::Directory, B>,
    _apply_deletes: bool,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    B: IndexWriterBase,
  {
    dummy_unreachable!()
  }

  fn get_version(&self) -> i64 {
    dummy_unreachable!()
  }

  fn is_current<D1, B>(&self, _index_writer: &IndexWriter<D1, B>) -> Result<bool>
  where
    D1: Directory,
    B: IndexWriterBase,
  {
    dummy_unreachable!()
  }

  type IndexCommit = DummyIndexCommit<D>;

  fn get_index_commit(&self) -> Result<Self::IndexCommit> {
    dummy_unreachable!()
  }

  type Directory = DummyDirectory;

  fn directory(&self) -> &DirectoryReaderBase<Self::Directory> {
    dummy_unreachable!()
  }
}
