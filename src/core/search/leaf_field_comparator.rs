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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::search::comparators::doc_comparator::{DocComparatorIterator, DocLeafComparator};
use crate::core::search::comparators::double_comparator::DoubleLeafComparator;
use crate::core::search::comparators::float_comparator::FloatLeafComparator;
use crate::core::search::comparators::int_comparator::IntLeafComparator;
use crate::core::search::comparators::long_comparator::LongLeafComparator;
use crate::core::search::comparators::numeric_comparator::NumericCompetitiveIterator;
use crate::core::search::comparators::term_ord_val_comparator::{
    TermOrdValCompetitiveIterator, TermOrdValLeafComparator,
};
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum4};
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::field_comparator::{
    FieldComparator, FieldComparatorEnum, RelevanceLeafComparator, TermValLeafComparator,
};
use crate::core::search::scorable::Scorable;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Expert: comparator that gets instantiated on each leaf from a top-level
/// [`FieldComparator`]
/// instance.
///
/// A leaf comparator must define these functions:
///
/// - [`set_bottom`](LeafFieldComparator::set_bottom) This method is called by
///   [`FieldValueHitQueue`](crate::core::search::field_value_hit_queue::FieldValueHitQueue)
///   to notify the `FieldComparator` of the current weakest ("bottom") slot.
///   Note that this slot may not hold the weakest value according to your
///   comparator, in cases where your comparator is not the primary one (i.e.,
///   is only used to break ties from the comparators before it).
/// - [`compare_bottom`](LeafFieldComparator::compare_bottom) Compare a new hit
///   (docID) against the "weakest" (bottom) entry in the queue.
/// - [`compare_top`](LeafFieldComparator::compare_top) Compares a new hit
///   (docID) against the top value previously set by a call to
///   [`FieldComparator::set_top_value`].
/// - [`copy`](LeafFieldComparator::copy) Installs a new hit into the priority
///   queue. The
///   [`FieldValueHitQueue`](crate::core::search::field_value_hit_queue::FieldValueHitQueue)
///   calls this method when a new hit is competitive.
///
/// # See Also
/// - [`FieldComparator`]
///
/// # Lucene Experimental
/// This API is experimental and may change in future versions.
pub trait LeafFieldComparator {
    type FieldComparator: FieldComparator;
    /// Set the bottom slot, i.e., the "weakest" (sorted last) entry in the
    /// queue. When `compare_bottom` is called, you should compare against
    /// this slot.
    ///
    /// This will always be called before `compare_bottom`.
    ///
    /// # Arguments
    /// - `slot`: The currently weakest (sorted last) slot in the queue.
    ///
    /// # Errors
    /// Returns an error if an I/O error occurs.
    fn set_bottom(&mut self, slot: usize, comparator: &mut Self::FieldComparator) -> Result<()>;

    /// Compare the bottom of the queue with this document.
    ///
    /// This will only be invoked after `set_bottom` has been called. This
    /// should return the same result as if `bottom` were slot1 and the new
    /// document were slot2.
    ///
    /// For a search that hits many results, this method will be the hotspot
    /// (invoked the most frequently).
    ///
    /// # Arguments
    /// - `doc`: The docID that was hit.
    /// - `scorer`: The scorer instance currently used to evaluate the hit.
    /// - `comparator`: The parent field comparator associated with this leaf comparator.
    ///
    /// # Returns
    /// - `N < 0` if the doc's value is sorted after the bottom entry (not
    ///   competitive).
    /// - `N > 0` if the doc's value is sorted before the bottom entry.
    /// - `0` if they are equal.
    ///
    /// # Errors
    /// Returns an error if an I/O error occurs.
    fn compare_bottom<S>(
        &mut self,
        doc: i32,
        scorer: &mut S,
        comparator: &mut Self::FieldComparator,
    ) -> Result<i32>
    where
        S: Scorable + ?Sized;

    /// Compare the top value with this document.
    ///
    /// This will only be invoked after `set_top_value` has been called. This
    /// should return the same result as if `top_value` were slot1 and the
    /// new document were slot2.
    ///
    /// This is only called for searches that use searchAfter (deep paging).
    /// # Arguments
    /// - `doc`: The docID that was hit.
    /// - `scorer`: The scorer instance currently used to evaluate the hit.
    /// - `comparator`: The parent field comparator associated with this leaf comparator.
    ///
    /// # Returns
    /// - `N < 0` if the doc's value is sorted after the top entry (not
    ///   competitive).
    /// - `N > 0` if the doc's value is sorted before the top entry.
    /// - `0` if they are equal.
    ///
    /// # Errors
    /// Returns an error if an I/O error occurs.
    fn compare_top<S>(
        &mut self,
        doc: i32,
        scorer: &mut S,
        comparator: &mut Self::FieldComparator,
    ) -> Result<i32>
    where
        S: Scorable + ?Sized;

    /// Called when a new hit is competitive.
    ///
    /// You should copy any state associated with this document that will be
    /// required for future comparisons into the specified slot.
    ///
    /// # Arguments
    /// - `slot`: The slot to copy the hit to.
    /// - `doc`: The docID relative to the current reader.
    /// - `scorer`: The scorer instance currently used to evaluate the hit.
    /// - `comparator`: The parent field comparator associated with this leaf comparator.
    ///
    /// # Errors
    /// Returns an error if an I/O error occurs.
    fn copy<S>(
        &mut self,
        slot: usize,
        doc: i32,
        scorer: &mut S,
        comparator: &mut Self::FieldComparator,
    ) -> Result<()>
    where
        S: Scorable + ?Sized;

    /// Sets the scorer to use in case a document's score is needed.
    ///
    /// # Arguments
    /// - `scorer`: Scorer instance to get the current hit's score, if
    ///   necessary.
    /// - `comparator`: The parent field comparator associated with this leaf comparator.
    ///
    /// # Errors
    /// Returns an error if an I/O error occurs.
    fn set_scorer<S>(
        &mut self,
        scorer: &mut S,
        comparator: &mut Self::FieldComparator,
    ) -> Result<()>
    where
        S: Scorable + ?Sized;

    type DocIdSetIteratorRef<'a>: DocIdSetIterator
    where
        Self: 'a;
    /// Returns a competitive iterator over documents stronger than already
    /// collected docs, or `None` if such an iterator is not available for
    /// the current comparator or segment.
    ///
    /// # Returns
    /// An iterator over competitive docs.
    ///
    /// # Arguments
    /// - `comparator`: The parent field comparator associated with this leaf comparator.
    fn competitive_iterator(
        &mut self,
        _comparator: &mut Self::FieldComparator,
    ) -> Result<Option<Self::DocIdSetIteratorRef<'_>>> {
        Ok(None)
    }

    /// Informs this leaf comparator that the hit's threshold is reached.
    ///
    /// This method is called from a collector when the hit's threshold is
    /// reached.
    ///
    /// # Arguments
    /// - `comparator`: The parent field comparator associated with this leaf comparator.
    fn set_hits_threshold_reached(
        &mut self,
        _comparator: &mut Self::FieldComparator,
    ) -> Result<()> {
        Ok(())
    }
}

pub type LeafFieldComparatorDocIdSetIterator<LR> = DocIdSetIteratorEnum4<
    DocComparatorIterator,
    NumericCompetitiveIterator<LR>,
    DummyDISI,
    TermOrdValCompetitiveIterator<LR>,
>;
pub type LeafFieldComparatorDocIdSetIteratorRef<'a, LR> = DocIdSetIteratorEnum4<
    <DocLeafComparator as LeafFieldComparator>::DocIdSetIteratorRef<'a>,
    <DoubleLeafComparator<LR> as LeafFieldComparator>::DocIdSetIteratorRef<'a>,
    &'a mut DummyDISI,
    <TermOrdValLeafComparator<LR> as LeafFieldComparator>::DocIdSetIteratorRef<'a>,
>;

pub enum LeafFieldComparatorEnum<LR>
where
    LR: LeafReader,
{
    Relevance(RelevanceLeafComparator),
    Doc(DocLeafComparator),
    Double(DoubleLeafComparator<LR>),
    Float(FloatLeafComparator<LR>),
    Int(IntLeafComparator<LR>),
    Long(LongLeafComparator<LR>),
    TermVal(TermValLeafComparator<LR>),
    TermOrdVal(TermOrdValLeafComparator<LR>),
}

impl<LR> LeafFieldComparator for LeafFieldComparatorEnum<LR>
where
    LR: LeafReader,
{
    type FieldComparator = FieldComparatorEnum;

    fn set_bottom(&mut self, slot: usize, comparator: &mut Self::FieldComparator) -> Result<()> {
        match (self, comparator) {
            (Self::Relevance(comparator), FieldComparatorEnum::Relevance(c)) => {
                comparator.set_bottom(slot, c)
            },
            (Self::Doc(comparator), FieldComparatorEnum::Doc(c)) => comparator.set_bottom(slot, c),
            (Self::Double(comparator), FieldComparatorEnum::Double(c)) => {
                comparator.set_bottom(slot, c)
            },
            (Self::Double(comparator), FieldComparatorEnum::SortedNumericDouble(c)) => {
                comparator.set_bottom(slot, &mut c.base)
            },
            (Self::Float(comparator), FieldComparatorEnum::Float(c)) => {
                comparator.set_bottom(slot, c)
            },
            (Self::Float(comparator), FieldComparatorEnum::SortedNumericFloat(c)) => {
                comparator.set_bottom(slot, &mut c.base)
            },
            (Self::Int(comparator), FieldComparatorEnum::Int(c)) => comparator.set_bottom(slot, c),
            (Self::Int(comparator), FieldComparatorEnum::SortedNumericInt(c)) => {
                comparator.set_bottom(slot, &mut c.base)
            },
            (Self::Long(comparator), FieldComparatorEnum::Long(c)) => {
                comparator.set_bottom(slot, c)
            },
            (Self::Long(comparator), FieldComparatorEnum::SortedNumericLong(c)) => {
                comparator.set_bottom(slot, &mut c.base)
            },
            (Self::TermVal(comparator), FieldComparatorEnum::TermVal(c)) => {
                comparator.set_bottom(slot, c)
            },
            (Self::TermOrdVal(comparator), FieldComparatorEnum::TermOrdValue(c)) => {
                comparator.set_bottom(slot, c)
            },

            (Self::TermOrdVal(comparator), FieldComparatorEnum::SortedDocValuesTermOrdVal(c)) => {
                comparator.set_bottom(slot, &mut c.base)
            },
            _ => Err(LuceneError::illegal_state("Mismatched comparator types")),
        }
    }

    fn compare_bottom<S>(
        &mut self,
        doc: i32,
        scorer: &mut S,
        comparator: &mut Self::FieldComparator,
    ) -> Result<i32>
    where
        S: Scorable + ?Sized,
    {
        match (self, comparator) {
            (Self::Relevance(comparator), FieldComparatorEnum::Relevance(c)) => {
                comparator.compare_bottom(doc, scorer, c)
            },
            (Self::Doc(comparator), FieldComparatorEnum::Doc(c)) => {
                comparator.compare_bottom(doc, scorer, c)
            },
            (Self::Double(comparator), FieldComparatorEnum::Double(c)) => {
                comparator.compare_bottom(doc, scorer, c)
            },
            (Self::Double(comparator), FieldComparatorEnum::SortedNumericDouble(c)) => {
                comparator.compare_bottom(doc, scorer, &mut c.base)
            },
            (Self::Float(comparator), FieldComparatorEnum::Float(c)) => {
                comparator.compare_bottom(doc, scorer, c)
            },
            (Self::Float(comparator), FieldComparatorEnum::SortedNumericFloat(c)) => {
                comparator.compare_bottom(doc, scorer, &mut c.base)
            },
            (Self::Int(comparator), FieldComparatorEnum::Int(c)) => {
                comparator.compare_bottom(doc, scorer, c)
            },
            (Self::Int(comparator), FieldComparatorEnum::SortedNumericInt(c)) => {
                comparator.compare_bottom(doc, scorer, &mut c.base)
            },
            (Self::Long(comparator), FieldComparatorEnum::Long(c)) => {
                comparator.compare_bottom(doc, scorer, c)
            },
            (Self::Long(comparator), FieldComparatorEnum::SortedNumericLong(c)) => {
                comparator.compare_bottom(doc, scorer, &mut c.base)
            },
            (Self::TermVal(comparator), FieldComparatorEnum::TermVal(c)) => {
                comparator.compare_bottom(doc, scorer, c)
            },
            (Self::TermOrdVal(comparator), FieldComparatorEnum::TermOrdValue(c)) => {
                comparator.compare_bottom(doc, scorer, c)
            },

            (Self::TermOrdVal(comparator), FieldComparatorEnum::SortedDocValuesTermOrdVal(c)) => {
                comparator.compare_bottom(doc, scorer, &mut c.base)
            },
            _ => Err(LuceneError::illegal_state("Mismatched comparator types")),
        }
    }

    fn compare_top<S>(
        &mut self,
        doc: i32,
        scorer: &mut S,
        comparator: &mut Self::FieldComparator,
    ) -> Result<i32>
    where
        S: Scorable + ?Sized,
    {
        match (self, comparator) {
            (Self::Relevance(comparator), FieldComparatorEnum::Relevance(c)) => {
                comparator.compare_top(doc, scorer, c)
            },
            (Self::Doc(comparator), FieldComparatorEnum::Doc(c)) => {
                comparator.compare_top(doc, scorer, c)
            },
            (Self::Double(comparator), FieldComparatorEnum::Double(c)) => {
                comparator.compare_top(doc, scorer, c)
            },
            (Self::Double(comparator), FieldComparatorEnum::SortedNumericDouble(c)) => {
                comparator.compare_top(doc, scorer, &mut c.base)
            },
            (Self::Float(comparator), FieldComparatorEnum::Float(c)) => {
                comparator.compare_top(doc, scorer, c)
            },
            (Self::Float(comparator), FieldComparatorEnum::SortedNumericFloat(c)) => {
                comparator.compare_top(doc, scorer, &mut c.base)
            },
            (Self::Int(comparator), FieldComparatorEnum::Int(c)) => {
                comparator.compare_top(doc, scorer, c)
            },
            (Self::Int(comparator), FieldComparatorEnum::SortedNumericInt(c)) => {
                comparator.compare_top(doc, scorer, &mut c.base)
            },
            (Self::Long(comparator), FieldComparatorEnum::Long(c)) => {
                comparator.compare_top(doc, scorer, c)
            },
            (Self::Long(comparator), FieldComparatorEnum::SortedNumericLong(c)) => {
                comparator.compare_top(doc, scorer, &mut c.base)
            },
            (Self::TermVal(comparator), FieldComparatorEnum::TermVal(c)) => {
                comparator.compare_top(doc, scorer, c)
            },
            (Self::TermOrdVal(comparator), FieldComparatorEnum::TermOrdValue(c)) => {
                comparator.compare_top(doc, scorer, c)
            },

            (Self::TermOrdVal(comparator), FieldComparatorEnum::SortedDocValuesTermOrdVal(c)) => {
                comparator.compare_top(doc, scorer, &mut c.base)
            },
            _ => Err(LuceneError::illegal_state("Mismatched comparator types")),
        }
    }

    fn copy<S>(
        &mut self,
        slot: usize,
        doc: i32,
        scorer: &mut S,
        comparator: &mut Self::FieldComparator,
    ) -> Result<()>
    where
        S: Scorable + ?Sized,
    {
        match (self, comparator) {
            (Self::Relevance(comparator), FieldComparatorEnum::Relevance(c)) => {
                comparator.copy(slot, doc, scorer, c)
            },
            (Self::Doc(comparator), FieldComparatorEnum::Doc(c)) => {
                comparator.copy(slot, doc, scorer, c)
            },
            (Self::Double(comparator), FieldComparatorEnum::Double(c)) => {
                comparator.copy(slot, doc, scorer, c)
            },
            (Self::Double(comparator), FieldComparatorEnum::SortedNumericDouble(c)) => {
                comparator.copy(slot, doc, scorer, &mut c.base)
            },
            (Self::Float(comparator), FieldComparatorEnum::Float(c)) => {
                comparator.copy(slot, doc, scorer, c)
            },
            (Self::Float(comparator), FieldComparatorEnum::SortedNumericFloat(c)) => {
                comparator.copy(slot, doc, scorer, &mut c.base)
            },
            (Self::Int(comparator), FieldComparatorEnum::Int(c)) => {
                comparator.copy(slot, doc, scorer, c)
            },
            (Self::Int(comparator), FieldComparatorEnum::SortedNumericInt(c)) => {
                comparator.copy(slot, doc, scorer, &mut c.base)
            },
            (Self::Long(comparator), FieldComparatorEnum::Long(c)) => {
                comparator.copy(slot, doc, scorer, c)
            },
            (Self::Long(comparator), FieldComparatorEnum::SortedNumericLong(c)) => {
                comparator.copy(slot, doc, scorer, &mut c.base)
            },
            (Self::TermVal(comparator), FieldComparatorEnum::TermVal(c)) => {
                comparator.copy(slot, doc, scorer, c)
            },
            (Self::TermOrdVal(comparator), FieldComparatorEnum::TermOrdValue(c)) => {
                comparator.copy(slot, doc, scorer, c)
            },

            (Self::TermOrdVal(comparator), FieldComparatorEnum::SortedDocValuesTermOrdVal(c)) => {
                comparator.copy(slot, doc, scorer, &mut c.base)
            },
            _ => Err(LuceneError::illegal_state("Mismatched comparator types")),
        }
    }

    fn set_scorer<S>(
        &mut self,
        scorer: &mut S,
        comparator: &mut Self::FieldComparator,
    ) -> Result<()>
    where
        S: Scorable + ?Sized,
    {
        match (self, comparator) {
            (Self::Relevance(comparator), FieldComparatorEnum::Relevance(c)) => {
                comparator.set_scorer(scorer, c)
            },
            (Self::Doc(comparator), FieldComparatorEnum::Doc(c)) => {
                comparator.set_scorer(scorer, c)
            },
            (Self::Double(comparator), FieldComparatorEnum::Double(c)) => {
                comparator.set_scorer(scorer, c)
            },
            (Self::Double(comparator), FieldComparatorEnum::SortedNumericDouble(c)) => {
                comparator.set_scorer(scorer, &mut c.base)
            },
            (Self::Float(comparator), FieldComparatorEnum::Float(c)) => {
                comparator.set_scorer(scorer, c)
            },
            (Self::Float(comparator), FieldComparatorEnum::SortedNumericFloat(c)) => {
                comparator.set_scorer(scorer, &mut c.base)
            },
            (Self::Int(comparator), FieldComparatorEnum::Int(c)) => {
                comparator.set_scorer(scorer, c)
            },
            (Self::Int(comparator), FieldComparatorEnum::SortedNumericInt(c)) => {
                comparator.set_scorer(scorer, &mut c.base)
            },
            (Self::Long(comparator), FieldComparatorEnum::Long(c)) => {
                comparator.set_scorer(scorer, c)
            },
            (Self::Long(comparator), FieldComparatorEnum::SortedNumericLong(c)) => {
                comparator.set_scorer(scorer, &mut c.base)
            },
            (Self::TermVal(comparator), FieldComparatorEnum::TermVal(c)) => {
                comparator.set_scorer(scorer, c)
            },
            (Self::TermOrdVal(comparator), FieldComparatorEnum::TermOrdValue(c)) => {
                comparator.set_scorer(scorer, c)
            },
            (Self::TermOrdVal(comparator), FieldComparatorEnum::SortedDocValuesTermOrdVal(c)) => {
                comparator.set_scorer(scorer, &mut c.base)
            },
            _ => Err(LuceneError::illegal_state("Mismatched comparator types")),
        }
    }

    type DocIdSetIteratorRef<'a>
        = LeafFieldComparatorDocIdSetIteratorRef<'a, LR>
    where
        LR: 'a;

    fn competitive_iterator(
        &mut self,
        comparator: &mut Self::FieldComparator,
    ) -> Result<Option<Self::DocIdSetIteratorRef<'_>>> {
        match (self, comparator) {
            (Self::Relevance(comparator), FieldComparatorEnum::Relevance(c)) => comparator
                .competitive_iterator(c)
                .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::C)),
            (Self::Doc(comparator), FieldComparatorEnum::Doc(c)) => comparator
                .competitive_iterator(c)
                .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::A)),
            (Self::Double(comparator), FieldComparatorEnum::Double(c)) => comparator
                .competitive_iterator(c)
                .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::B)),
            (Self::Double(comparator), FieldComparatorEnum::SortedNumericDouble(c)) => comparator
                .competitive_iterator(&mut c.base)
                .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::B)),
            (Self::Float(comparator), FieldComparatorEnum::Float(c)) => comparator
                .competitive_iterator(c)
                .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::B)),
            (Self::Float(comparator), FieldComparatorEnum::SortedNumericFloat(c)) => comparator
                .competitive_iterator(&mut c.base)
                .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::B)),
            (Self::Int(comparator), FieldComparatorEnum::Int(c)) => comparator
                .competitive_iterator(c)
                .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::B)),
            (Self::Int(comparator), FieldComparatorEnum::SortedNumericInt(c)) => comparator
                .competitive_iterator(&mut c.base)
                .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::B)),
            (Self::Long(comparator), FieldComparatorEnum::Long(c)) => comparator
                .competitive_iterator(c)
                .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::B)),
            (Self::Long(comparator), FieldComparatorEnum::SortedNumericLong(c)) => comparator
                .competitive_iterator(&mut c.base)
                .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::B)),
            (Self::TermVal(comparator), FieldComparatorEnum::TermVal(c)) => comparator
                .competitive_iterator(c)
                .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::C)),
            (Self::TermOrdVal(comparator), FieldComparatorEnum::TermOrdValue(c)) => comparator
                .competitive_iterator(c)
                .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::D)),

            (Self::TermOrdVal(comparator), FieldComparatorEnum::SortedDocValuesTermOrdVal(c)) => {
                comparator
                    .competitive_iterator(&mut c.base)
                    .map(|opt| opt.map(LeafFieldComparatorDocIdSetIteratorRef::<'_, LR>::D))
            },
            _ => Ok(None),
        }
    }

    fn set_hits_threshold_reached(&mut self, comparator: &mut Self::FieldComparator) -> Result<()> {
        match (self, comparator) {
            (Self::Relevance(comparator), FieldComparatorEnum::Relevance(c)) => {
                comparator.set_hits_threshold_reached(c)
            },
            (Self::Doc(comparator), FieldComparatorEnum::Doc(c)) => {
                comparator.set_hits_threshold_reached(c)
            },
            (Self::Double(comparator), FieldComparatorEnum::Double(c)) => {
                comparator.set_hits_threshold_reached(c)
            },
            (Self::Double(comparator), FieldComparatorEnum::SortedNumericDouble(c)) => {
                comparator.set_hits_threshold_reached(&mut c.base)
            },
            (Self::Float(comparator), FieldComparatorEnum::Float(c)) => {
                comparator.set_hits_threshold_reached(c)
            },
            (Self::Float(comparator), FieldComparatorEnum::SortedNumericFloat(c)) => {
                comparator.set_hits_threshold_reached(&mut c.base)
            },
            (Self::Int(comparator), FieldComparatorEnum::Int(c)) => {
                comparator.set_hits_threshold_reached(c)
            },
            (Self::Int(comparator), FieldComparatorEnum::SortedNumericInt(c)) => {
                comparator.set_hits_threshold_reached(&mut c.base)
            },
            (Self::Long(comparator), FieldComparatorEnum::Long(c)) => {
                comparator.set_hits_threshold_reached(c)
            },
            (Self::Long(comparator), FieldComparatorEnum::SortedNumericLong(c)) => {
                comparator.set_hits_threshold_reached(&mut c.base)
            },
            (Self::TermVal(comparator), FieldComparatorEnum::TermVal(c)) => {
                comparator.set_hits_threshold_reached(c)
            },
            (Self::TermOrdVal(comparator), FieldComparatorEnum::TermOrdValue(c)) => {
                comparator.set_hits_threshold_reached(c)
            },

            (Self::TermOrdVal(comparator), FieldComparatorEnum::SortedDocValuesTermOrdVal(c)) => {
                comparator.set_hits_threshold_reached(&mut c.base)
            },
            _ => Err(LuceneError::illegal_state("Mismatched comparator types")),
        }
    }
}
