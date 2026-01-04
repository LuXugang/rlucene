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
use crate::core::index::composite_reader::{CompositeReader, CompositeReaderTerms, get_context};
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::multi_terms_enum::{MultiTermsEnum, MultiTermsEnumType};
use crate::core::index::postings_enum::ALL;
use crate::core::index::reader_slice::ReaderSlice;
use crate::core::index::terms::{Terms, TermsEnum2};
use crate::core::index::terms_enum::{EmptyTermsEnum, TermsEnum, TermsEnumEnum2};
use crate::core::index::terms_enum_index::TermsEnumIndex;
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{ToInt, TryIntoInt};
use std::borrow::Cow;
use std::rc::Rc;

/// Exposes flex API, merged from flex API of sub-segments.
pub struct MultiTerms<T>
where
    T: Terms,
{
    subs: Vec<T>,
    sub_slices: Vec<Rc<ReaderSlice>>,
    has_freqs: bool,
    has_offsets: bool,
    has_positions: bool,
    has_payloads: bool,
}
impl<T> MultiTerms<T>
where
    T: Terms,
{
    /// Sole constructor. Use [`Self::get_terms`] instead if possible.
    ///
    /// # Parameters
    /// * `subs` – The [`Terms`] instances of all sub-readers.
    /// * `sub_slices` – A parallel array (matching `subs`) describing the sub-reader slices.
    pub fn new(subs: Vec<T>, sub_slices: Vec<Rc<ReaderSlice>>) -> Result<Self> {
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
pub type IntersectIterType<T> =
    TermsEnumEnum2<MultiTermsEnumType<<T as Terms>::IntersectIter>, EmptyTermsEnum>;
pub type IteratorType<T> =
    TermsEnumEnum2<MultiTermsEnumType<<T as Terms>::TermsEnum>, EmptyTermsEnum>;
impl<T> Terms for MultiTerms<T>
where
    T: Terms,
{
    type TermsEnum = IteratorType<T>;

    fn iterator(&self) -> Result<Self::TermsEnum> {
        let mut terms_enums = Vec::new();

        for (i, sub) in self.subs.iter().enumerate() {
            let terms_enum = sub.iterator()?;
            terms_enums.push(TermsEnumIndex::new(Some(terms_enum), i.try_convert()?));
        }

        if !terms_enums.is_empty() {
            let v = MultiTermsEnum::new(self.sub_slices.clone())?;
            Ok(TermsEnumEnum2::A(v.reset(terms_enums)?))
        } else {
            Ok(TermsEnumEnum2::B(EmptyTermsEnum))
        }
    }

    type IntersectIter = IntersectIterType<T>;

    fn intersect(
        &self,
        compiled: &mut CompiledAutomaton,
        start_term: Option<&BytesRef<Vec<u8>>>,
    ) -> Result<Self::IntersectIter> {
        let mut terms_enums = Vec::new();

        for (i, sub) in self.subs.iter().enumerate() {
            let terms_enum = sub.intersect(compiled, start_term)?;
            terms_enums.push(TermsEnumIndex::new(Some(terms_enum), i.try_convert()?));
        }
        if !terms_enums.is_empty() {
            let v = MultiTermsEnum::new(self.sub_slices.clone())?;
            Ok(TermsEnumEnum2::A(v.reset(terms_enums)?))
        } else {
            Ok(TermsEnumEnum2::B(EmptyTermsEnum))
        }
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
        let mut min_term = None;

        for terms in &self.subs {
            if let Some(term) = terms.get_min()? {
                match &min_term {
                    None => min_term = Some(term),
                    Some(cur) => {
                        if term.as_ref().cmp(cur.as_ref()).to_int() < 0 {
                            min_term = Some(term);
                        }
                    },
                }
            }
        }

        Ok(min_term)
    }

    fn get_max(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        let mut max_term = None;

        for terms in &self.subs {
            if let Some(term) = terms.get_max()? {
                match &max_term {
                    None => max_term = Some(term),
                    Some(cur) => {
                        if term.as_ref().cmp(cur.as_ref()).to_int() > 0 {
                            max_term = Some(term);
                        }
                    },
                }
            }
        }

        Ok(max_term)
    }
}

pub type TermsType<CR> = TermsEnum2<CompositeReaderTerms<CR>, MultiTerms<CompositeReaderTerms<CR>>>;
pub type TermsPostingType<CR> = <<TermsType<CR> as Terms>::TermsEnum as TermsEnum>::PostingsEnum;
/// This method may return `None` if the field does not exist or if it has no terms.
pub fn get_terms<CR>(reader: CR, field: &str) -> Result<Option<TermsType<CR>>>
where
    CR: CompositeReader,
{
    let max_doc = reader.max_doc()?;
    let reader = get_context(reader)?;
    let leaves = reader.leaves()?;

    if leaves.len() == 1 {
        return match leaves[0].reader().terms(field)? {
            Some(terms) => Ok(Some(TermsEnum2::A(terms))),
            None => return Ok(None),
        };
    }

    let mut terms_per_leaf = Vec::with_capacity(leaves.len());
    let mut slice_per_leaf = Vec::with_capacity(leaves.len());

    for (leaf_idx, ctx) in leaves.iter().enumerate() {
        if let Some(sub_terms) = ctx.reader().terms(field)? {
            terms_per_leaf.push(sub_terms);
            slice_per_leaf.push(Rc::new(ReaderSlice::new(
                ctx.doc_base,
                max_doc,
                leaf_idx.try_convert()?,
            )));
        }
    }

    if terms_per_leaf.is_empty() {
        Ok(None)
    } else {
        Ok(Some(TermsEnum2::B(MultiTerms::new(
            terms_per_leaf,
            slice_per_leaf,
        )?)))
    }
}
/// Returns `PostingsEnum` for the specified field and term.
///
/// This returns `None` if the field or term does not exist, or if positions were not indexed.
///
/// See `get_term_postings_enum` with flags.
pub fn get_term_postings_enum_default<CR>(
    reader: CR,
    field: &str,
    term: &BytesRef<Vec<u8>>,
) -> Result<Option<TermsPostingType<CR>>>
where
    CR: CompositeReader,
{
    get_term_postings_enum(reader, field, term, ALL as i32)
}

/// Returns `PostingsEnum` for the specified field and term, with control over whether freqs,
/// positions, offsets or payloads are required.
///
/// This returns `None` if the field or term does not exist.
/// See `TermsEnum::postings`.
pub fn get_term_postings_enum<CR>(
    reader: CR,
    field: &str,
    term: &BytesRef<Vec<u8>>,
    flags: i32,
) -> Result<Option<TermsPostingType<CR>>>
where
    CR: CompositeReader,
{
    if let Some(terms) = get_terms(reader, field)? {
        let mut terms_enum = terms.iterator()?;
        if terms_enum.seek_exact(term)? {
            return Ok(Some(terms_enum.postings_with_flags(None, flags)?));
        }
    }
    Ok(None)
}
