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
use crate::index::automaton_terms_enum::AutomatonTermsEnum;
use crate::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::index::filtered_terms_enum::FilteredTermsEnum;
use crate::index::terms::Terms;
use crate::index::terms_enum::TermsEnum;
use crate::index::BytesRef;
use crate::util::automation::compiled_automaton::CompiledAutomaton;
use crate::util::error::lucene_error::Result;
use std::borrow::Cow;

pub struct DummyTerms;
impl Terms for DummyTerms {
    fn get_terms() -> Result<()> {
        unreachable!()
    }

    type TermsEnum = DummyTermsEnum;

    fn iterator(&self) -> Result<Self::TermsEnum> {
        unreachable!()
    }

    type IntersectIter = DummyTermsEnum;

    fn intersect(
        &self,
        _compiled: &mut CompiledAutomaton,
        _start_term: Option<BytesRef<Vec<u8>>>,
    ) -> Result<Self::IntersectIter> {
        unreachable!()
    }

    fn default_intersect(
        &self,
        _compiled: &mut CompiledAutomaton,
        _start_term: Option<BytesRef<Vec<u8>>>,
    ) -> Result<FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>>
    where
        Self: Sized,
    {
        unreachable!()
    }

    fn size(&self) -> Result<i64> {
        unreachable!()
    }

    fn get_sum_total_term_freq(&self) -> Result<i64> {
        unreachable!()
    }

    fn get_sum_doc_freq(&self) -> Result<i64> {
        unreachable!()
    }

    fn get_doc_count(&self) -> Result<i32> {
        unreachable!()
    }

    fn has_freqs(&self) -> bool {
        unreachable!()
    }

    fn has_offsets(&self) -> bool {
        unreachable!()
    }

    fn has_positions(&self) -> bool {
        unreachable!()
    }

    fn has_payloads(&self) -> bool {
        unreachable!()
    }

    fn get_min<'a, T>(&'a self, _iterator: &'a mut T) -> Result<Option<Cow<'a, BytesRef<Vec<u8>>>>>
    where
        T: TermsEnum,
    {
        unreachable!()
    }

    fn get_max<'a, T>(&'a self, _iterator: &'a mut T) -> Result<Option<Cow<'a, BytesRef<Vec<u8>>>>>
    where
        T: TermsEnum,
    {
        unreachable!()
    }

    fn get_stats(&self) -> Result<String> {
        unreachable!()
    }

    fn type_name(&self) -> &'static str {
        unreachable!()
    }
}
