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
use crate::core::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::core::index::reader_slice::ReaderSlice;
use crate::core::index::terms::Terms;
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;

pub struct MultiTerms<T>
where
    T: Terms,
{
    subs: Vec<T>,
    sub_slices: Vec<ReaderSlice>,
    has_freqs: bool,
    has_offsets: bool,
    has_positions: bool,
    has_payloads: bool,
}
impl<T> MultiTerms<T>
where
    T: Terms,
{
    pub fn new(subs: Vec<T>, sub_slices: Vec<ReaderSlice>) -> Result<Self> {
        debug_assert!(
            !subs.is_empty(),
            "inefficient: don't use MultiTerms over one sub"
        );

        let mut has_freqs = true;
        let mut has_offsets = true;
        let mut has_positions = true;
        let mut has_payloads_any = false;

        for t in &subs {
            has_freqs &= t.has_freqs();
            has_offsets &= t.has_offsets();
            has_positions &= t.has_positions();
            has_payloads_any |= t.has_payloads();
        }

        // if all subs have pos, and at least one has payloads
        let has_payloads = has_positions && has_payloads_any;

        Ok(Self {
            subs,
            sub_slices,
            has_freqs,
            has_offsets,
            has_positions,
            has_payloads,
        })
    }
}
impl<T> Terms for MultiTerms<T>
where
    T: Terms,
{
    type TermsEnum = DummyTermsEnum;

    fn iterator(&self) -> Result<Self::TermsEnum> {
        todo!()
    }

    type IntersectIter = DummyTermsEnum;

    fn intersect(
        &self,
        _compiled: &mut CompiledAutomaton,
        _start_term: Option<&BytesRef<Vec<u8>>>,
    ) -> Result<Self::IntersectIter> {
        todo!()
    }

    fn size(&self) -> Result<i64> {
        Ok(-1)
    }

    fn get_sum_total_term_freq(&self) -> Result<i64> {
        let mut sum = 0i64;
        for terms in &self.subs {
            let v = terms.get_sum_total_term_freq()?;
            debug_assert!(v != -1);
            sum += v;
        }
        Ok(sum)
    }

    fn get_sum_doc_freq(&self) -> Result<i64> {
        let mut sum = 0i64;
        for terms in &self.subs {
            let v = terms.get_sum_doc_freq()?;
            debug_assert!(v != -1);
            sum += v;
        }
        Ok(sum)
    }

    fn get_doc_count(&self) -> Result<i32> {
        let mut sum = 0;
        for terms in &self.subs {
            let v = terms.get_doc_count()?;
            debug_assert!(v != -1);
            sum += v;
        }
        Ok(sum)
    }

    fn has_freqs(&self) -> bool {
        self.has_freqs
    }

    fn has_offsets(&self) -> bool {
        self.has_offsets
    }

    fn has_positions(&self) -> bool {
        self.has_positions
    }

    fn has_payloads(&self) -> bool {
        self.has_payloads
    }

    fn get_min(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        todo!()
    }

    fn get_max(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        todo!()
    }
}
