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
use crate::core::index::base_composite_reader::{BaseCompositeReader, BaseCompositeReaderBase};
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::directory_reader::{DirectoryReader, DirectoryReaderBase};
use crate::core::index::dummy::dummy_index_commit::DummyIndexCommit;
use crate::core::index::dummy::dummy_index_reader::DummyIndexReader;
use crate::core::index::dummy::dummy_stored_fields::DummyStoredFields;
use crate::core::index::dummy::dummy_term_vectors::DummyTermVectors;
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::store::dummy::dummy_directory::DummyDirectory;
use crate::core::util::dummy::dummy_comparator::DummyComparator;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

pub struct DummyDirectoryReader;

impl BaseCompositeReader for DummyDirectoryReader {
    type Comparator = DummyComparator<Self::IndexReader>;

    fn base_composite_reader_base(
        &self,
    ) -> &BaseCompositeReaderBase<Self::IndexReader, Self::Comparator> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl CompositeReader for DummyDirectoryReader {
    type IndexReader = DummyIndexReader;

    fn get_sequential_sub_readers(&self) -> &[Self::IndexReader] {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl IndexReader for DummyDirectoryReader {
    type TermVectors = DummyTermVectors;

    fn term_vectors(&self) -> Result<Self::TermVectors> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn max_doc(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn num_docs(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type StoredFields = DummyStoredFields;

    fn stored_fields(&self) -> Result<Self::StoredFields> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn do_close(&mut self) -> Result<()> {
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
}

impl Display for DummyDirectoryReader {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl DirectoryReader for DummyDirectoryReader {
    type DirectoryReader = DummyDirectoryReader;

    fn do_open_if_changed(&self) -> Result<Option<Self::DirectoryReader>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn do_open_if_changed_with_commit<IC>(
        &self,
        _commit: IC,
    ) -> Result<Option<Self::DirectoryReader>>
    where
        IC: IndexCommit,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn do_open_if_changed_with_index_writer<L, B>(
        &self,
        _writer: IndexWriter<Self::Directory, L, B>,
        _apply_deletes: bool,
    ) -> Result<Self::DirectoryReader>
    where
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_version(&self) -> i64 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn is_current(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type IndexCommit = DummyIndexCommit;

    fn get_index_commit(&self) -> Result<Self::IndexCommit> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type Directory = DummyDirectory;

    fn directory(&self) -> &DirectoryReaderBase<Self::Directory> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
