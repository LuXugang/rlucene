/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
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
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type TermsEnum = DummyTermsEnum;

    fn iterator(&self) -> Result<Self::TermsEnum> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type IntersectIter = DummyTermsEnum;

    fn intersect(
        &self,
        _compiled: &mut CompiledAutomaton,
        _start_term: Option<BytesRef<Vec<u8>>>,
    ) -> Result<Self::IntersectIter> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn default_intersect(
        &self,
        _compiled: &mut CompiledAutomaton,
        _start_term: Option<BytesRef<Vec<u8>>>,
    ) -> Result<FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>>
    where
        Self: Sized,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn size(&self) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_sum_total_term_freq(&self) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_sum_doc_freq(&self) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_doc_count(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn has_freqs(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn has_offsets(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn has_positions(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn has_payloads(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_min<'a, T>(&'a self, _iterator: &'a mut T) -> Result<Option<Cow<'a, BytesRef<Vec<u8>>>>>
    where
        T: TermsEnum,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_max<'a, T>(&'a self, _iterator: &'a mut T) -> Result<Option<Cow<'a, BytesRef<Vec<u8>>>>>
    where
        T: TermsEnum,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_stats(&self) -> Result<String> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn type_name(&self) -> &'static str {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
