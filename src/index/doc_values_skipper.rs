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
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::util::error::lucene_error::Result;

/// Skipper for DocValues.
///
/// A skipper has a position that can only be advanced via
/// [`advance(target)`](DocValuesSkipper::advance). The next advance position
/// must be greater than [`max_doc_id(0)`](DocValuesSkipper::max_doc_id).
/// A skipper's position, along with a `level`, determines the interval at which
/// the skipper is currently situated.
pub trait DocValuesSkipper {
    /// Advance this skipper so that all levels contain the next document on or
    /// after `target`.
    ///
    /// NOTE: The behavior is undefined if `target` is less than or equal to
    /// `max_doc_id(0)`.
    ///
    /// NOTE: `min_doc_id(0)` may return a doc ID that is greater than `target`
    /// if the target document doesn't have a value.
    fn advance(&mut self, target: i32) -> Result<()>;

    /// Return the number of levels. This number may change when moving to a
    /// different interval.
    fn num_levels(&self) -> i32;

    /// Return the minimum doc ID of the interval on the given level, inclusive.
    ///
    /// This returns `-1` if [`advance(target)`](DocValuesSkipper::advance) has
    /// not been called yet and `NO_MORE_DOCS` if the iterator is exhausted.
    /// This method is non-increasing when `level` increases.
    /// In other words: `min_doc_id(level+1) <= min_doc_id(level)`.
    fn min_doc_id(&self, level: i32) -> i32;

    /// Return the maximum doc ID of the interval on the given level, inclusive.
    ///
    /// This returns `-1` if [`advance(target)`](DocValuesSkipper::advance) has
    /// not been called yet and [`NO_MORE_DOCS`] if the iterator is
    /// exhausted. This method is non-decreasing when `level` decreases.
    /// In other words: `max_doc_id(level+1) >= max_doc_id(level)`.
    fn max_doc_id(&self, level: i32) -> i32;

    /// Return the minimum value of the interval at the given level, inclusive.
    ///
    /// NOTE: It is only guaranteed that values in this interval are greater
    /// than or equal to the returned value. There is no guarantee that one
    /// document actually has this value.
    fn min_value(&self, level: i32) -> i64;

    /// Return the maximum value of the interval at the given level, inclusive.
    ///
    /// NOTE: It is only guaranteed that values in this interval are less than
    /// or equal to the returned value. There is no guarantee that one
    /// document actually has this value.
    fn max_value(&self, level: i32) -> i64;

    /// Return the number of documents that have a value in the interval
    /// associated with the given level.
    fn doc_count_level(&self, level: i32) -> i32;

    /// Return the global minimum value.
    ///
    /// NOTE: It is only guaranteed that values are greater than or equal to the
    /// returned value. There is no guarantee that one document actually has
    /// this value.
    fn global_min_value(&self) -> i64;

    /// Return the global maximum value.
    ///
    /// NOTE: It is only guaranteed that values are less than or equal to the
    /// returned value. There is no guarantee that one document actually has
    /// this value.
    fn global_max_value(&self) -> i64;

    /// Return the global number of documents with a value for the field.
    fn global_doc_count(&self) -> i32;

    /// Advance this skipper so that all levels intersect the range given by
    /// `min_value` and `max_value`. If there are no intersecting levels,
    /// the skipper is exhausted.
    fn advance_by_range(&mut self, min_value: i64, max_value: i64) -> Result<()> {
        if self.min_doc_id(0) == -1 {
            // `advance` has not been called yet
            self.advance(0)?;
        }
        // check if the current interval intersects the provided range
        while self.min_doc_id(0) != NO_MORE_DOCS
            && (self.min_value(0) > max_value || self.max_value(0) < min_value)
        {
            let mut max_doc_id = self.max_doc_id(0);
            let mut next_level = 1;
            // check if the next levels intersect to skip as many docs as
            // possible
            while next_level < self.num_levels()
                && (self.min_value(next_level) > max_value
                    || self.max_value(next_level) < min_value)
            {
                max_doc_id = self.max_doc_id(next_level);
                next_level += 1;
            }
            self.advance(max_doc_id + 1)?;
        }
        Ok(())
    }
}
