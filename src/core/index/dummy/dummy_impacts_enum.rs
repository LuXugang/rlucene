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
use crate::core::index::dummy::dummy_impacts::DummyImpacts;
use crate::core::index::impacts_enum::ImpactsEnum;
use crate::core::index::impacts_source::ImpactsSource;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;

pub struct DummyImpactsEnum;

impl PostingsEnum for DummyImpactsEnum {
  fn freq(&mut self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn next_position(&mut self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn start_offset(&self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn end_offset(&self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    dummy_unreachable!()
  }
}

impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for DummyImpactsEnum {}
impl DocIdSetIterator for DummyImpactsEnum {
  fn doc_id(&self) -> i32 {
    dummy_unreachable!()
  }

  fn next_doc(&mut self) -> Result<i32> {
    dummy_unreachable!()
  }
}

impl ImpactsSource for DummyImpactsEnum {
  fn advance_shallow(&mut self, _target: i32) -> Result<()> {
    dummy_unreachable!()
  }

  type Impacts<'a>
    = DummyImpacts
  where
    Self: 'a;

  fn get_impacts(&self) -> Result<Self::Impacts<'_>> {
    dummy_unreachable!()
  }
}

impl ImpactsEnum for DummyImpactsEnum {}
