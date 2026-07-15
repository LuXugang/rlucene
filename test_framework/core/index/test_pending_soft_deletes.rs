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
use crate::core::index::BytesRef;
use crate::core::index::doc_values_field_updates::{
  DocValuesFieldInnerIter, DocValuesFieldIterator, DocValuesFieldIteratorEnum,
  DocValuesFieldUpdatesBase,
};
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::LuceneError;
use std::borrow::Cow;

#[allow(dead_code)] // for quick search
struct TestPendingSoftDeletes;
pub(crate) struct TestSingleUpdateDocValuesFieldUpdates {
  docs_changed: Vec<i32>,
  has_value: bool,
}

impl TestSingleUpdateDocValuesFieldUpdates {
  pub(crate) fn new(docs_changed: Vec<i32>, has_value: bool) -> Self {
    Self {
      docs_changed,
      has_value,
    }
  }
}

impl Accountable for TestSingleUpdateDocValuesFieldUpdates {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(0)
  }
}

impl DocValuesFieldUpdatesBase for TestSingleUpdateDocValuesFieldUpdates {
  fn finish(&mut self) {}

  fn add_value(
    &mut self,
    _doc: i32,
    _value: i64,
    _index: usize,
  ) -> crate::core::util::error::lucene_error::Result<()> {
    Err(LuceneError::unsupported_operation("add_value"))
  }

  fn add_byte_ref(
    &mut self,
    _doc: i32,
    _value: &BytesRef<Vec<u8>>,
    _index: usize,
  ) -> crate::core::util::error::lucene_error::Result<()> {
    Err(LuceneError::unsupported_operation("add_byte_ref"))
  }

  fn add_iterator<T>(
    &mut self,
    _doc_id: i32,
    _iterator: &mut T,
    _index: usize,
  ) -> crate::core::util::error::lucene_error::Result<()>
  where
    T: DocValuesFieldIterator,
  {
    Err(LuceneError::unsupported_operation("add_iterator"))
  }

  fn iterator(
    &self,
    _inner: DocValuesFieldInnerIter,
    del_gen: i64,
  ) -> crate::core::util::error::lucene_error::Result<DocValuesFieldIteratorEnum> {
    Ok(DocValuesFieldIteratorEnum::SingleUpdate(
      TestSingleUpdateDocValuesFieldIterator::new(
        self.docs_changed.clone(),
        del_gen,
        self.has_value,
      ),
    ))
  }

  fn swap(&mut self, _i: usize, _j: usize) -> crate::core::util::error::lucene_error::Result<()> {
    Ok(())
  }

  fn grow(&mut self, _size: i32) -> crate::core::util::error::lucene_error::Result<()> {
    Ok(())
  }

  fn resize(&mut self, _size: i32) -> crate::core::util::error::lucene_error::Result<()> {
    Ok(())
  }

  fn sub_type(&self) -> DocValuesType {
    DocValuesType::Numeric
  }
}
pub(crate) struct TestSingleUpdateDocValuesFieldIterator {
  docs_changed: Vec<i32>,
  idx: usize,
  doc: i32,
  del_gen: i64,
  has_value: bool,
}

impl TestSingleUpdateDocValuesFieldIterator {
  fn new(docs_changed: Vec<i32>, del_gen: i64, has_value: bool) -> Self {
    Self {
      docs_changed,
      idx: 0,
      doc: -1,
      del_gen,
      has_value,
    }
  }
}

impl DocValuesIterator for TestSingleUpdateDocValuesFieldIterator {}

impl DocIdSetIterator for TestSingleUpdateDocValuesFieldIterator {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> crate::core::util::error::lucene_error::Result<i32> {
    if self.idx >= self.docs_changed.len() {
      self.doc = NO_MORE_DOCS;
      return Ok(self.doc);
    }
    self.doc = self.docs_changed[self.idx];
    self.idx += 1;
    Ok(self.doc)
  }
}

impl DocValuesFieldIterator for TestSingleUpdateDocValuesFieldIterator {
  fn long_value(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(1)
  }

  fn binary_value(
    &mut self,
  ) -> crate::core::util::error::lucene_error::Result<Cow<'_, BytesRef<Vec<u8>>>> {
    Err(LuceneError::unsupported_operation("binary_value"))
  }

  fn del_gen(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(self.del_gen)
  }

  fn has_value(&self) -> crate::core::util::error::lucene_error::Result<bool> {
    Ok(self.has_value)
  }
}
