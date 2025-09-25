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
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::leaf_field_comparator::LeafFieldComparator;
use crate::core::util::ToInt;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Expert: a `FieldComparator` compares hits so as to determine their sort order when collecting the
/// top results with [`TopFieldCollector`](crate::core::search::top_field_collector::TopFieldCollector).
/// The concrete public `FieldComparator` implementations
/// correspond to the `SortField` types.
///
/// The document IDs passed to these methods must only move forwards, since they are using doc
/// values iterators to retrieve sort values.
///
/// This API is designed to achieve high performance sorting, by exposing a tight interaction with
/// [`FieldValueHitQueue`](crate::core::search::field_value_hit_queue::FieldValueHitQueue) as it visits hits. Whenever a hit is competitive, it's enrolled into a
/// virtual slot, which is an int ranging from 0 to numHits-1. Segment transitions are handled by
/// creating a dedicated per-segment [`LeafFieldComparator`] which also needs to interact with the
/// [`FieldValueHitQueue`](crate::core::search::field_value_hit_queue::FieldValueHitQueue) but can optimize based on the segment to collect.
///
/// The following functions need to be implemented:
/// - `compare` Compare a hit at 'slot a' with hit 'slot b'.
/// - [`Self::set_top_value`] Called by [`TopFieldCollector`](crate::core::search::top_field_collector::TopFieldCollector) to notify the comparator of the top most
///   value, which is used by future calls to [`LeafFieldComparator::compare_top`].
/// - [`get_leaf_comparator`] Invoked when the search is switching to the next segment. You may need
///   to update internal state of the comparator, e.g. retrieving new values from DocValues.
/// - `value` Return the sort value stored in the specified slot. This is only called at the end of
///   the search, in order to populate [`FieldDoc::fields`](crate::core::search::field_doc::FieldDoc) when returning the top results.
///
/// See also:
/// - [`LeafFieldComparator`]
/// - `lucene.experimental`
pub trait FieldComparator {
    // f64 f32 not implement Ord
    type V: PartialOrd;
    /// Compare hit at slot1 with hit at slot2.
    ///
    /// Returns:
    /// - `N < 0` if slot2's value is sorted after slot1
    /// - `N > 0` if slot2's value is sorted before slot1
    /// - `0` if they are equal
    fn compare(&self, slot1: i32, slot2: i32) -> i32;

    /// Record the top value, for future calls to [`LeafFieldComparator::compare_top`].
    /// This is only called for searches that use `search_after` (deep paging),
    /// and is invoked before any calls to [`Self::get_leaf_comparator`].
    fn set_top_value(&mut self, value: Self::V);

    /// Return the actual value in the slot.
    ///
    /// # Parameters
    /// - `slot`: the slot index
    ///
    /// # Returns
    /// The value stored in this slot.
    fn value(&self, slot: i32) -> &Self::V;

    type LeafFieldComparator: LeafFieldComparator;
    /// Get a per-segment [`LeafFieldComparator`] to collect the given
    /// [`LeafReaderContext`].
    ///
    /// All docIDs supplied to this [`LeafFieldComparator`] are relative to the current reader
    /// (you must add `docBase` if you need to map it to a top-level docID).
    ///
    /// # Parameters
    /// - `context`: current reader context
    ///
    /// # Returns
    /// The comparator to use for this segment.
    ///
    /// # Errors
    /// Returns an error if there is a low-level I/O problem.
    fn get_leaf_comparator<LR>(self, context: &LeafReaderContext<LR>) -> Self::LeafFieldComparator
    where
        LR: LeafReader;

    /// Returns a negative integer if `first` is less than `second`, `0` if they are equal,
    /// and a positive integer otherwise.
    ///
    /// Default implementation assumes the type implements [`Ord`] (like Java's `Comparable`)
    /// and invokes `.cmp`.
    ///
    /// Be sure to override this method if your `FieldComparator`'s type isn't comparable
    /// or if your values may sometimes be `null` (represented as [`Option::None`] in Rust).
    fn compare_values(&self, first: Option<&Self::V>, second: Option<&Self::V>) -> Result<i32> {
        match (first, second) {
            (None, None) => Ok(0),
            (None, Some(_)) => Ok(-1),
            (Some(_), None) => Ok(1),
            (Some(f), Some(s)) => {
                match f.partial_cmp(s) {
                    Some(ord) => Ok(ord.to_int()),
                    // In case of NaN for f64 or other non-comparable values
                    None => self.fallback_compare(f, s),
                }
            },
        }
    }
    fn fallback_compare(&self, _first: &Self::V, _second: &Self::V) -> Result<i32> {
        Err(LuceneError::illegal_state(
            "compare_values cannot compare value ,you should Implement this method",
        ))
    }
    /// Informs the comparator that sort is done on this single field.
    /// This is useful to enable some optimizations for skipping non-competitive documents.
    fn set_single_sort(&mut self) {}

    /// Informs the comparator that the skipping of documents should be disabled.
    /// This function is called by [`TopFieldCollector`](crate::core::search::top_field_collector::TopFieldCollector) in cases when the skipping functionality
    /// should not be applied or not necessary.
    ///
    /// An example could be when search sort is a part of the index sort, and can be already efficiently
    /// handled by [`TopFieldCollector`](crate::core::search::top_field_collector::TopFieldCollector), and doing extra work for skipping in the comparator is redundant.
    fn disable_skipping(&mut self) {}
}
