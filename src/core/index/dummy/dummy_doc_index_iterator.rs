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
use crate::core::index::knn_vector_values::DocIndexIterator;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;

pub struct DummyDocIndexIterator;

impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for DummyDocIndexIterator
{
}
impl crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for DummyDocIndexIterator {}

impl DocIdSetIterator for DummyDocIndexIterator {
  fn doc_id(&self) -> i32 {
    dummy_unreachable!()
  }

  fn next_doc(&mut self) -> crate::core::util::error::lucene_error::Result<i32> {
    dummy_unreachable!()
  }

  fn advance(&mut self, _target: i32) -> crate::core::util::error::lucene_error::Result<i32> {
    dummy_unreachable!()
  }

  fn slow_advance(&mut self, _target: i32) -> crate::core::util::error::lucene_error::Result<i32> {
    dummy_unreachable!()
  }

  fn cost(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    dummy_unreachable!()
  }
}

impl DocIndexIterator for DummyDocIndexIterator {
  fn index(&self) -> crate::core::util::error::lucene_error::Result<i32> {
    dummy_unreachable!()
  }
}
