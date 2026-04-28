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
use crate::core::codecs::block_term_state::TermStateEnum;
use crate::core::index::BytesRef;
use crate::core::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::core::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::core::index::terms::{Terms, TermsPosting};
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::dummy::dummy_attribute_source::DummyAttributeSource;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;

pub struct DummyTermsEnum;
impl BytesRefIterator for DummyTermsEnum {
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    dummy_unreachable!()
  }
}

impl TermsEnum for DummyTermsEnum {
  type AttributeSource<'a>
    = &'a DummyAttributeSource
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = &'a mut DummyAttributeSource
  where
    Self: 'a;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    dummy_unreachable!()
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    dummy_unreachable!()
  }

  fn seek_exact(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<bool> {
    dummy_unreachable!()
  }

  fn prepare_seek_exact(&mut self, _text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    dummy_unreachable!()
  }

  fn get_prepare_seek_exact_status(&mut self, _target: &BytesRef<Vec<u8>>) -> Result<bool> {
    dummy_unreachable!()
  }

  fn seek_ceil(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    dummy_unreachable!()
  }

  fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
    dummy_unreachable!()
  }

  fn seek_exact_with_state(
    &mut self,
    _term: &BytesRef<Vec<u8>>,
    _state: &TermStateEnum,
  ) -> Result<()> {
    dummy_unreachable!()
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    dummy_unreachable!()
  }

  fn ord(&self) -> Result<i64> {
    dummy_unreachable!()
  }

  fn doc_freq(&mut self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    dummy_unreachable!()
  }

  type PostingsEnum = DummyPostingsEnum;

  fn postings_with_flags(
    &mut self,
    _reuse: Option<Self::PostingsEnum>,
    _flags: i32,
  ) -> Result<Self::PostingsEnum> {
    dummy_unreachable!()
  }

  type ImpactsEnum = DummyImpactsEnum;

  fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
    dummy_unreachable!()
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    dummy_unreachable!()
  }
}

pub struct DummyTermsEnum2<T>
where
  T: Terms,
{
  terms: T,
}
impl<T> BytesRefIterator for DummyTermsEnum2<T>
where
  T: Terms,
{
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    dummy_unreachable!()
  }
}

impl<T> TermsEnum for DummyTermsEnum2<T>
where
  T: Terms,
{
  type AttributeSource<'a>
    = &'a DummyAttributeSource
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = &'a mut DummyAttributeSource
  where
    Self: 'a;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    dummy_unreachable!()
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    dummy_unreachable!()
  }

  fn seek_exact(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<bool> {
    dummy_unreachable!()
  }

  fn prepare_seek_exact(&mut self, _text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    dummy_unreachable!()
  }

  fn get_prepare_seek_exact_status(&mut self, _target: &BytesRef<Vec<u8>>) -> Result<bool> {
    dummy_unreachable!()
  }

  fn seek_ceil(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    dummy_unreachable!()
  }

  fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
    dummy_unreachable!()
  }

  fn seek_exact_with_state(
    &mut self,
    _term: &BytesRef<Vec<u8>>,
    _state: &TermStateEnum,
  ) -> Result<()> {
    dummy_unreachable!()
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    dummy_unreachable!()
  }

  fn ord(&self) -> Result<i64> {
    dummy_unreachable!()
  }

  fn doc_freq(&mut self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    dummy_unreachable!()
  }

  type PostingsEnum = TermsPosting<T>;

  fn postings_with_flags(
    &mut self,
    _reuse: Option<Self::PostingsEnum>,
    _flags: i32,
  ) -> Result<Self::PostingsEnum> {
    dummy_unreachable!()
  }

  type ImpactsEnum = DummyImpactsEnum;

  fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
    dummy_unreachable!()
  }

  fn term_state(&mut self) -> Result<TermStateEnum> {
    dummy_unreachable!()
  }
}
