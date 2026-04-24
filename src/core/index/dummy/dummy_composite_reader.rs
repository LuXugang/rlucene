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
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::dummy::dummy_cache_helper::DummyCacheHelper;
use crate::core::index::dummy::dummy_leaf_reader::DummyLeafReader;
use crate::core::index::dummy::dummy_stored_fields::DummyStoredFields;
use crate::core::index::dummy::dummy_term_vectors::DummyTermVectors;
use crate::core::index::index_reader::{IndexReader, IndexReaderBase, IndexReaderEnum};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::term::Term;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

pub struct DummyCompositeReader<LR>
where
  LR: LeafReader + Clone,
{
  lr: Vec<IndexReaderEnum<LR, DummyCompositeReader<LR>>>,
}

impl DummyCompositeReader<DummyLeafReader> {
  pub fn new(lr: DummyLeafReader) -> Self {
    let v = IndexReaderEnum::Leaf(lr);
    Self { lr: vec![v] }
  }
}

impl<LR> IndexReader for DummyCompositeReader<LR>
where
  LR: LeafReader + Clone,
{
  type TermVectors = DummyTermVectors;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    dummy_unreachable!()
  }

  fn max_doc(&self) -> Result<i32> {
    Ok(1)
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

impl<LR> Display for DummyCompositeReader<LR>
where
  LR: LeafReader + Clone,
{
  fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
    dummy_unreachable!()
  }
}

impl<LR> CompositeReader for DummyCompositeReader<LR>
where
  LR: LeafReader + Clone,
{
  type LeafReader = LR;

  type SubCompositeReader = DummyCompositeReader<LR>;

  fn get_sequential_sub_readers(
    &self,
  ) -> &[IndexReaderEnum<Self::LeafReader, Self::SubCompositeReader>] {
    self.lr.as_slice()
  }

  fn to_string(&self) -> String {
    dummy_unreachable!()
  }
}
