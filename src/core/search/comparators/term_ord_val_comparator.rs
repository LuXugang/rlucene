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
use crate::core::index::doc_values::Sorted;
use crate::core::index::leaf_reader::{LRPosting, LRTermsEnum, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::postings_enum::NONE;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::dummy::dummy_leaf_field_comparator::DummyLeafFieldComparator;
use crate::core::search::field_comparator::FieldComparator;
use crate::core::search::index_searcher::get_max_clause_count;
use crate::core::search::pruning::Pruning;
use crate::core::util::ToInt;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};
use std::collections::VecDeque;

/// Sorts by field's natural Term sort order, using ordinals.
///
/// This is functionally equivalent to
/// [`TermValComparator`](crate::core::search::field_comparator::TermValComparator),
/// but it first resolves the string to their relative ordinal positions
/// (using the index returned by
/// [`LeafReader::getSortedDocValues`](LeafReader::get_sorted_doc_values)),
/// and does most comparisons using the ordinals.
///
/// For medium to large results, this comparator will be much faster than
/// [`TermValComparator`](crate::core::search::field_comparator::TermValComparator).
/// For very small result sets it may be slower.
pub struct TermOrdValComparator {
    /// Ords for each slot.
    pub(crate) ords: Vec<i32>,
    /// Values for each slot.
    pub(crate) values: Vec<Option<BytesRef<Vec<u8>>>>,
    /// Which reader last copied a value into the slot. When
    ///   we compare two slots, we just compare-by-ord if the
    ///  readerGen is the same; else we must compare the
    ///  values (slower).
    pub(crate) reader_gen: Vec<i32>,
    /// Gen of current reader we are on.
    pub(crate) current_reader_gen: i32,
    field: String,
    reverse: bool,
    sort_missing_last: bool,
    /// Bottom value (same as `values[bottomSlot]` once bottomSlot is set).  Cached for faster compares.
    pub(crate) bottom_value: Option<BytesRef<Vec<u8>>>,
    /* Bottom slot, or -1 if queue isn't full yet */
    pub(crate) bottom_slot: i32,
    /// Set by setTopValue.
    pub(crate) top_value: Option<BytesRef<Vec<u8>>>,
    /// -1 if missing values are sorted first, 1 if they are sorted last
    pub(crate) missing_sort_cmp: i32,
    /// Whether this is the only comparator.
    single_sort: bool,
    /// Whether this comparator is allowed to skip documents.
    can_skip_documents: bool,
    /// Whether the collector is done with counting hits so that we can start skipping documents.
    hits_threshold_reached: bool,
}
impl TermOrdValComparator {
    pub fn new(
        num_hits: i32,
        field: String,
        sort_missing_last: bool,
        reverse: bool,
        pruning: Pruning,
    ) -> Self {
        let can_skip_documents = pruning != Pruning::None;
        Self {
            ords: vec![0; num_hits as usize],
            values: vec![None; num_hits as usize],
            reader_gen: vec![0; num_hits as usize],
            current_reader_gen: -1,
            field,
            reverse,
            sort_missing_last,
            bottom_value: None,
            bottom_slot: -1,
            top_value: None,
            missing_sort_cmp: if sort_missing_last { 1 } else { -1 },
            single_sort: false,
            can_skip_documents,
            hits_threshold_reached: false,
        }
    }
}
impl FieldComparator for TermOrdValComparator {
    type V = BytesRef<Vec<u8>>;

    fn compare(&self, slot1: i32, slot2: i32) -> i32 {
        let slot1 = slot1 as usize;
        let slot2 = slot2 as usize;
        if self.reader_gen[slot1] == self.reader_gen[slot2] {
            return self.ords[slot1] - self.ords[slot2];
        }

        let val1 = self.values[slot1].as_ref();
        let val2 = self.values[slot2].as_ref();

        match (val1, val2) {
            (None, None) => 0,
            (None, Some(_)) => self.missing_sort_cmp,
            (Some(_), None) => -self.missing_sort_cmp,
            (Some(v1), Some(v2)) => v1.cmp(v2).to_int(),
        }
    }

    fn set_top_value(&mut self, value: Self::V) {
        // None is fine: it means the last doc of the prior
        // search was missing this value
        self.top_value = Some(value);
    }

    fn value(&self, slot: i32) -> &Self::V {
        self.values[slot as usize]
            .as_ref()
            .expect("value in slot must be present")
    }

    type LeafFieldComparator<LR>
        = DummyLeafFieldComparator
    where
        LR: LeafReader;

    fn get_leaf_comparator<LR>(
        self,
        context: &LeafReaderContext<LR>,
    ) -> crate::core::util::error::lucene_error::Result<Self::LeafFieldComparator<LR>>
    where
        LR: LeafReader,
    {
        todo!()
    }

    fn compare_values(&self, val1: Option<&Self::V>, val2: Option<&Self::V>) -> i32 {
        match (val1, val2) {
            (None, None) => 0,
            (None, Some(_)) => self.missing_sort_cmp,
            (Some(_), None) => -self.missing_sort_cmp,
            (Some(v1), Some(v2)) => v1.cmp(v2).to_int(),
        }
    }

    fn set_single_sort(&mut self) {
        self.single_sort = true;
    }

    fn disable_skipping(&mut self) {
        self.can_skip_documents = false;
    }
}
const MAX_TERMS: i32 = 1024;
pub(crate) struct CompetitiveIterator<LR>
where
    LR: LeafReader,
{
    max_doc: i32,
    field: String,
    dense: bool,
    doc_values_terms: <Sorted<LR> as SortedDocValues>::TermsEnum,
    doc: i32,
    postings: VecDeque<i32>,
    postings_init: bool,
    terms_enum: Option<LRTermsEnum<LR>>,
    docs_with_field: Option<Sorted<LR>>,
    // if docs_with_field is active, dense must be false
    using_skip: bool,
    disjunction: Option<PriorityQueue<PostingsEnumAndOrd<LR>, PostingsEnumAndOrdCmp>>,
}
impl<LR> CompetitiveIterator<LR>
where
    LR: LeafReader,
{
    pub fn new(
        reader: &LR,
        field: String,
        dense: bool,
        doc_values_terms: <Sorted<LR> as SortedDocValues>::TermsEnum,
        docs_with_field: Option<Sorted<LR>>,
        terms_enum: LRTermsEnum<LR>,
    ) -> Result<Self> {
        let max_doc = reader.max_doc()?;
        debug_assert!(
            !(dense && docs_with_field.is_some()),
            "docs_with_field must be None when dense = true"
        );
        Ok(Self {
            max_doc,
            field,
            dense,
            doc_values_terms,
            doc: -1,
            postings: VecDeque::new(),
            postings_init: false,
            terms_enum: Some(terms_enum),
            docs_with_field,
            using_skip: false,
            disjunction: None,
        })
    }
    /// Update this iterator to only match postings whose term has an ordinal between `minOrd` included and `maxOrd` included.
    fn update(&mut self, min_ord: i32, max_ord: i32) -> Result<()> {
        let max_terms = std::cmp::min(MAX_TERMS, get_max_clause_count());
        let size = std::cmp::max(0, max_ord - min_ord + 1);

        if size > max_terms {
            self.using_skip = true;
        } else if !self.postings_init {
            self.init(min_ord, max_ord)?;
        } else if size < self.postings.len() as i32 {
            // One or more ords got removed
            debug_assert!(self.postings.is_empty() || *self.postings.front().unwrap() <= min_ord);
            while !self.postings.is_empty() && *self.postings.front().unwrap() < min_ord {
                self.postings.pop_front();
            }

            debug_assert!(self.postings.is_empty() || *self.postings.back().unwrap() >= max_ord);
            while !self.postings.is_empty() && *self.postings.back().unwrap() > max_ord {
                self.postings.pop_back();
            }
            let disjunction = self.disjunction.as_mut().unwrap();
            let iterms = disjunction.take_heap_array();
            debug_assert!(
                iterms.len() == self.postings.len(),
                "priority queue size must match postings size"
            );
            let (min_ord, max_ord) = if !self.postings.is_empty() {
                (
                    *self.postings.front().unwrap(),
                    *self.postings.back().unwrap(),
                )
            } else {
                (0, 0)
            };
            for v in iterms {
                if v.ord < min_ord || v.ord > max_ord {
                    // this ord was removed
                    continue;
                }
                disjunction.add(v)?;
            }
        }

        Ok(())
    }
    /// For the first time, this iterator is allowed to skip documents.
    /// It needs to pull [`PostingsEnum`](crate::core::index::postings_enum::PostingsEnum)s from the terms dictionary of the inverted index
    /// and create a priority queue out of them.
    fn init(&mut self, min_ord: i32, max_ord: i32) -> Result<()> {
        self.postings_init = true;
        let size = std::cmp::max(0, max_ord - min_ord + 1);
        self.postings = VecDeque::with_capacity(size as usize);

        debug_assert!(self.disjunction.is_none());
        let mut disjunction = PriorityQueue::new(size, PostingsEnumAndOrdCmp)?;
        if size > 0 {
            self.doc_values_terms.seek_exact_with_ord(min_ord as i64)?;
            let min_term = self.doc_values_terms.term()?;
            let terms = self
                .terms_enum
                .as_mut()
                .ok_or_else(|| LuceneError::IllegalState("terms_enum not initialized".into()))?;

            if !terms.seek_exact(&min_term)? {
                return Err(LuceneError::illegal_state(format!(
                    "Term {} exists in doc values but not in the terms index",
                    min_term
                )));
            }

            disjunction.add(PostingsEnumAndOrd::<LR>::new(
                terms.postings_with_flags(None, NONE as i32)?,
                min_ord,
            ))?;
            self.postings.push_back(min_ord);

            for ord in (min_ord + 1)..=max_ord {
                let next = terms.next()?;
                if next.is_none() {
                    return Err(LuceneError::illegal_state(format!(
                        "Terms have more than {ord} unique terms while doc values have exactly {ord} terms"
                    )));
                }

                debug_assert!(
                    self.doc_values_terms.seek_exact(next.unwrap().as_ref())?
                        && self.doc_values_terms.ord()? == ord as i64,
                    "docValuesTerms not aligned with terms index"
                );
                disjunction.add(PostingsEnumAndOrd::new(
                    terms.postings_with_flags(None, NONE as i32)?,
                    ord,
                ))?;
                self.postings.push_back(ord);
            }
        }
        self.disjunction = Some(disjunction);
        Ok(())
    }
}
impl<LR> DocIdSetIterator for CompetitiveIterator<LR>
where
    LR: LeafReader,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc_id() + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        if target >= self.max_doc {
            self.doc = NO_MORE_DOCS;
            return Ok(self.doc);
        }

        if self.disjunction.is_none() {
            if self.using_skip {
                // The field is sparse and we're only interested in documents that have a value.
                debug_assert!(!self.dense);
                self.doc = self.docs_with_field.as_mut().unwrap().advance(target)?;
                return Ok(self.doc);
            } else {
                // We haven't started skipping yet
                self.doc = target;
                return Ok(self.doc);
            }
        }

        let disjunction = self.disjunction.as_mut().unwrap();
        let top = disjunction.top_mut();

        if top.is_none() {
            // priority queue is empty, none of the remaining documents are competitive
            self.doc = NO_MORE_DOCS;
            return Ok(self.doc);
        }

        let mut top = top.unwrap();

        while top.postings.doc_id() < target {
            top.postings.advance(target)?;
            top = disjunction.update_top()?;
        }

        self.doc = top.postings.doc_id();
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.max_doc as i64)
    }
}
struct PostingsEnumAndOrd<LR>
where
    LR: LeafReader,
{
    postings: LRPosting<LR>,
    ord: i32,
}
impl<LR> PostingsEnumAndOrd<LR>
where
    LR: LeafReader,
{
    pub fn new(postings: LRPosting<LR>, ord: i32) -> Self {
        Self { postings, ord }
    }
}
struct PostingsEnumAndOrdCmp;
impl<LR> Compare<PostingsEnumAndOrd<LR>> for PostingsEnumAndOrdCmp
where
    LR: LeafReader,
{
    fn less_than(&self, a: &PostingsEnumAndOrd<LR>, b: &PostingsEnumAndOrd<LR>) -> Result<bool> {
        Ok(a.postings.doc_id() < b.postings.doc_id())
    }
}
