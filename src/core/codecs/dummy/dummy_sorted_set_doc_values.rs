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
use crate::core::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::core::index::BytesRef;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;

pub struct DummySortedSetDocValues;

impl DocValuesIterator for DummySortedSetDocValues {
  fn advance_exact(&mut self, _target: i32) -> Result<bool> {
    dummy_unreachable!()
  }
}

impl DocIdSetIterator for DummySortedSetDocValues {
  fn doc_id(&self) -> i32 {
    dummy_unreachable!()
  }

  fn next_doc(&mut self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    dummy_unreachable!()
  }

  fn slow_advance(&mut self, _target: i32) -> Result<i32> {
    dummy_unreachable!()
  }

  fn cost(&self) -> Result<i64> {
    dummy_unreachable!()
  }
}

impl SortedSetDocValues for DummySortedSetDocValues {
  fn next_ord(&mut self) -> Result<i64> {
    dummy_unreachable!()
  }

  fn doc_value_count(&mut self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn lookup_ord(&mut self, _ord: i64) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    dummy_unreachable!()
  }

  fn get_value_count(&self) -> Result<i64> {
    dummy_unreachable!()
  }

  fn lookup_term(&mut self, _key: &BytesRef<Vec<u8>>) -> Result<i64> {
    dummy_unreachable!()
  }

  type TermsEnum<'a> = DummyTermsEnum;

  fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
    dummy_unreachable!()
  }

  fn is_single_valued(&self) -> bool {
    dummy_unreachable!()
  }

  type SortedDocValues = DummySortedDocValues;

  fn get_sorted_doc_values(&mut self) -> Result<Self::SortedDocValues> {
    dummy_unreachable!()
  }
}
