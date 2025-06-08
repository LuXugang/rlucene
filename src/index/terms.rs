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
use std::borrow::Cow;

use crate::index::automaton_terms_enum::AutomatonTermsEnum;
use crate::index::filtered_terms_enum::FilteredTermsEnum;
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::{BytesRef, BytesRefBuilder};
use crate::util::access::AccessVec;
use crate::util::automation::compiled_automaton::CompiledAutomaton;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::Result;

/// Trait representing base term statistics and access.
pub trait Terms {
    /// Returns the [`Terms`] index for this field, or [`Terms::EMPTY`] if it
    /// has none.
    ///
    /// Returns:
    /// - A `Terms` instance, or an empty instance if the field does not exist
    ///   in this reader.
    ///
    /// Errors:
    /// - Returns an error if an I/O error occurs.
    fn get_terms() -> Result<()> {
        unimplemented!()
    }

    type TermsEnum: TermsEnum;
    /// Returns an iterator that will step through all terms. This method will
    /// not return None.
    fn iterator(&self) -> Result<Self::TermsEnum>;

    type IntersectIter: TermsEnum;
    /// Returns a [`TermsEnum`] that iterates over all terms and documents
    /// accepted by the given [`CompiledAutomaton`].
    ///
    /// If `start_term` is provided, the returned enum will only return terms
    /// strictly greater than `start_term`, but you must still call `next()`
    /// first to advance to the first term. The provided `start_term` must
    /// be accepted by the automaton.
    ///
    /// This is an expert-level, low-level API that only works for
    /// [`AutomatonType::NORMAL`] compiled automata. To handle any type of
    /// compiled automaton, use
    /// [`CompiledAutomaton::get_terms_enum`](crate::util::automation::compiled_automaton::CompiledAutomaton::get_terms_enum)
    /// instead.
    ///
    /// **Note**: The returned `TermsEnum` does **not** support seeking.
    fn intersect(
        &self,
        compiled: &mut CompiledAutomaton,
        start_term: Option<BytesRef<Vec<u8>>>,
    ) -> Result<Self::IntersectIter>;

    fn default_intersect(
        &self,
        compiled: &mut CompiledAutomaton,
        start_term: Option<BytesRef<Vec<u8>>>,
    ) -> Result<FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>>
    where
        Self: Sized,
    {
        let terms_enum = self.iterator()?;
        let automaton_terms_enum = if start_term.is_some() {
            AutomatonTermsEnum::new_with_start_term(compiled, start_term)?
        } else {
            AutomatonTermsEnum::new(compiled)?
        };
        Ok(FilteredTermsEnum::new(terms_enum, automaton_terms_enum))
    }
    /// Returns the number of terms for this field, or `-1` if this measure
    /// isn't stored by the codec.
    ///
    /// Note that, like other term measures, this value does **not** take
    /// deleted documents into account.
    fn size(&self) -> Result<i64>;

    /// Returns the sum of
    /// [`TermsEnum::total_term_freq`]
    /// for all terms in this field. Note that, like other term measures,
    /// this value does **not** take deleted documents into account.
    fn get_sum_total_term_freq(&self) -> Result<i64>;

    /// Returns the sum of
    /// [`TermsEnum::doc_freq`]
    /// for all terms in this field. Note that, like other term measures,
    /// this value does **not** take deleted documents into account.
    fn get_sum_doc_freq(&self) -> Result<i64>;

    /// Returns the number of documents that have at least one term for this
    /// field. Note that, like other term measures, this value does **not**
    /// take deleted documents into account.
    fn get_doc_count(&self) -> Result<i32>;

    /// Returns `true` if documents in this field store per-document term
    /// frequency
    /// (see [`PostingsEnum::freq`](crate::index::postings_enum::PostingsEnum::freq)).
    fn has_freqs(&self) -> bool;

    /// Returns true if documents in this field store offsets.
    fn has_offsets(&self) -> bool;

    /// Returns true if documents in this field store positions.
    fn has_positions(&self) -> bool;

    /// Returns true if documents in this field store payloads.
    fn has_payloads(&self) -> bool;

    /// Returns the smallest term (in lexicographic order) in the field.  
    /// Note that, like other term measures, this does **not** take deleted
    /// documents into account. Returns `None` when there are no terms.
    fn get_min<'a, T>(&'a self, iterator: &'a mut T) -> Result<Option<Cow<'a, BytesRef<Vec<u8>>>>>
    where
        T: TermsEnum,
    {
        iterator.next()
    }

    /// Returns the largest term (in lexicographic order) in the field.  
    /// Note that, like other term measures, this does **not** take deleted
    /// documents into account. Returns `None` when there are no terms.
    fn get_max<'a, T>(&'a self, iterator: &'a mut T) -> Result<Option<Cow<'a, BytesRef<Vec<u8>>>>>
    where
        T: TermsEnum,
    {
        let size = self.size()?;
        match size.cmp(&0) {
            std::cmp::Ordering::Equal => return Ok(None),
            std::cmp::Ordering::Greater => {
                iterator.seek_exact_with_ord(size - 1)?;
                return Ok(Some(iterator.term()?));
            },
            std::cmp::Ordering::Less => {},
        }
        // otherwise: binary search
        let mut iterator = self.iterator()?;
        let v = iterator.next()?;
        if v.is_none() {
            return Ok(None);
        }

        let mut scratch = BytesRefBuilder::new();
        scratch.append_byte(0);
        // Iterates over digits:
        loop {
            let mut low = 0;
            let mut high = 256;
            // Binary search current digit to find the highest
            // digit before END:
            while low != high {
                let mid = (((low + high) as u32) >> 1) as i32;
                scratch.set_byte_at(scratch.length() - 1, mid as u8);
                match iterator.seek_ceil(scratch.get_bytes_mut_ref())? {
                    SeekStatus::End => {
                        if mid == 0 {
                            scratch.set_length(scratch.length() - 1);
                            return Ok(Some(Cow::Owned(scratch.get_bytes_owner())));
                        }
                        high = mid;
                    },
                    _ => {
                        if low == mid {
                            break;
                        }
                        low = mid;
                    },
                }
            }

            scratch.set_length(scratch.length() + 1);
            scratch.grow(scratch.length());
        }
    }

    /// Returns debugging statistics string.
    fn get_stats(&self) -> Result<String> {
        Ok(format!(
            "impl={},size={},docCount={},sumTotalTermFreq={},sumDocFreq={}",
            self.type_name(),
            self.size()?,
            self.get_doc_count()?,
            self.get_sum_total_term_freq()?,
            self.get_sum_doc_freq()?
        ))
    }

    /// Helper to get the type name of the implementation.
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
