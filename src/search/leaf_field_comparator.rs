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
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::search::dummy::dummy_disi::DummyDISI;
use crate::search::scorable::Scorable;
use crate::util::error::lucene_error::Result;

/// Expert: comparator that gets instantiated on each leaf from a top-level
/// [`FieldComparator`](crate::search::field_comparator::FieldComparator)
/// instance.
///
/// A leaf comparator must define these functions:
///
/// - [`set_bottom`](LeafFieldComparator::set_bottom) This method is called by
///   [`FieldValueHitQueue`](crate::search::field_value_hit_queue::FieldValueHitQueue)
///   to notify the `FieldComparator` of the current weakest ("bottom") slot.
///   Note that this slot may not hold the weakest value according to your
///   comparator, in cases where your comparator is not the primary one (i.e.,
///   is only used to break ties from the comparators before it).
/// - [`compare_bottom`](LeafFieldComparator::compare_bottom) Compare a new hit
///   (docID) against the "weakest" (bottom) entry in the queue.
/// - [`compare_top`](LeafFieldComparator::compare_top) Compares a new hit
///   (docID) against the top value previously set by a call to
///   [`FieldComparator::set_top_value`](crate::search::field_comparator::FieldComparator::set_top_value).
/// - [`copy`](LeafFieldComparator::copy) Installs a new hit into the priority
///   queue. The
///   [`FieldValueHitQueue`](crate::search::field_value_hit_queue::FieldValueHitQueue)
///   calls this method when a new hit is competitive.
///
/// # See Also
/// - [`FieldComparator`](crate::search::field_comparator::FieldComparator)
///
/// # Lucene Experimental
/// This API is experimental and may change in future versions.
pub trait LeafFieldComparator {
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
    fn set_bottom(&mut self, slot: usize) -> Result<()>;

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
    ///
    /// # Returns
    /// - `N < 0` if the doc's value is sorted after the bottom entry (not
    ///   competitive).
    /// - `N > 0` if the doc's value is sorted before the bottom entry.
    /// - `0` if they are equal.
    ///
    /// # Errors
    /// Returns an error if an I/O error occurs.
    fn compare_bottom(&self, doc: i32) -> Result<i32>;

    /// Compare the top value with this document.
    ///
    /// This will only be invoked after `set_top_value` has been called. This
    /// should return the same result as if `top_value` were slot1 and the
    /// new document were slot2.
    ///
    /// This is only called for searches that use searchAfter (deep paging).
    ///
    /// # Arguments
    /// - `doc`: The docID that was hit.
    ///
    /// # Returns
    /// - `N < 0` if the doc's value is sorted after the top entry (not
    ///   competitive).
    /// - `N > 0` if the doc's value is sorted before the top entry.
    /// - `0` if they are equal.
    ///
    /// # Errors
    /// Returns an error if an I/O error occurs.
    fn compare_top(&self, doc: i32) -> Result<i32>;

    /// Called when a new hit is competitive.
    ///
    /// You should copy any state associated with this document that will be
    /// required for future comparisons into the specified slot.
    ///
    /// # Arguments
    /// - `slot`: The slot to copy the hit to.
    /// - `doc`: The docID relative to the current reader.
    ///
    /// # Errors
    /// Returns an error if an I/O error occurs.
    fn copy(&mut self, slot: usize, doc: i32) -> Result<()>;

    /// Sets the scorer to use in case a document's score is needed.
    ///
    /// # Arguments
    /// - `scorer`: Scorer instance to get the current hit's score, if
    ///   necessary.
    ///
    /// # Errors
    /// Returns an error if an I/O error occurs.
    fn set_scorer<S: Scorable>(&mut self, scorer: S) -> Result<()>;

    /// Returns a competitive iterator over documents stronger than already
    /// collected docs, or `None` if such an iterator is not available for
    /// the current comparator or segment.
    ///
    /// # Returns
    /// An iterator over competitive docs.
    fn competitive_iterator(&self) -> Option<impl DocIdSetIterator> {
        None::<DummyDISI>
    }

    /// Informs this leaf comparator that the hit's threshold is reached.
    ///
    /// This method is called from a collector when the hit's threshold is
    /// reached.
    fn set_hits_threshold_reached(&mut self) -> Result<()> {
        Ok(())
    }
}
