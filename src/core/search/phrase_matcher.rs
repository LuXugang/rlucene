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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::Result;
pub trait PhraseMatcher {
    type ApproximationApproximation: DocIdSetIterator;
    /// Approximation that only matches documents that have all terms.
    fn approximation(&mut self) -> &Self::ImpactsApproximation;

    type ImpactsApproximation;
    /// Approximation that is aware of impacts.
    fn impacts_approximation(&mut self) -> &mut Self::ImpactsApproximation;

    /// An upper bound on the number of possible matches on this document.
    fn max_freq(&mut self) -> Result<f32>;

    /// Called after `approximation` has been advanced.
    fn reset(&mut self) -> Result<()>;

    /// Find the next match on the current document.
    ///
    /// Returns `false` if there are no more matches.
    fn next_match(&mut self) -> Result<bool>;

    /// The slop-adjusted weight of the current match.
    ///
    /// The sum of the slop-adjusted weights is used as the freq for scoring.
    fn sloppy_weight(&self) -> f32;

    /// The start position of the current match.
    fn start_position(&self) -> i32;

    /// The end position of the current match.
    fn end_position(&self) -> i32;

    /// The start offset of the current match.
    fn start_offset(&self) -> Result<i32>;

    /// The end offset of the current match.
    fn end_offset(&self) -> Result<i32>;

    /// An estimate of the average cost of finding all matches on a document.
    ///
    /// See `TwoPhaseIterator::match_cost`.
    fn get_match_cost(&self) -> f32;
}
