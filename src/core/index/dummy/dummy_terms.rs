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
use crate::core::index::automaton_terms_enum::AutomatonTermsEnum;
use crate::core::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::core::index::filtered_terms_enum::FilteredTermsEnum;
use crate::core::index::terms::Terms;
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;

pub struct DummyTerms;
impl Terms for DummyTerms {
  type TermsEnum = DummyTermsEnum;

  fn iterator(&self) -> Result<Self::TermsEnum> {
    dummy_unreachable!()
  }

  type IntersectIter = DummyTermsEnum;

  fn intersect(
    &self,
    _compiled: &CompiledAutomaton,
    _start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Self::IntersectIter> {
    dummy_unreachable!()
  }

  fn default_intersect(
    &self,
    _compiled: &CompiledAutomaton,
    _start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>>
  where
    Self: Sized,
  {
    dummy_unreachable!()
  }

  fn size(&self) -> Result<i64> {
    dummy_unreachable!()
  }

  fn get_sum_total_term_freq(&self) -> Result<i64> {
    dummy_unreachable!()
  }

  fn get_sum_doc_freq(&self) -> Result<i64> {
    dummy_unreachable!()
  }

  fn get_doc_count(&self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn has_freqs(&self) -> bool {
    dummy_unreachable!()
  }

  fn has_offsets(&self) -> bool {
    dummy_unreachable!()
  }

  fn has_positions(&self) -> bool {
    dummy_unreachable!()
  }

  fn has_payloads(&self) -> bool {
    dummy_unreachable!()
  }

  fn get_min(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    dummy_unreachable!()
  }

  fn get_max(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    dummy_unreachable!()
  }

  fn get_stats(&self) -> Result<String> {
    dummy_unreachable!()
  }
}
