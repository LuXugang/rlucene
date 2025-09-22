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
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::scorer::Scorer;
use crate::core::util::error::lucene_error::Result;
/// A supplier of [`Scorer`].
///
/// This allows to get an estimate of the cost before building the [`Scorer`].
pub trait ScorerSupplier {
    type Scorer: Scorer;
    type BulkScorer: BulkScorer;

    /// Get the [`Scorer`].
    /// This may not return `None` and must be called at most once.
    ///
    /// # Parameters
    ///
    /// - `lead_cost`: Cost of the scorer that will be used in order to lead iteration.
    ///   This can be interpreted as an upper bound of the number of times that
    ///   [`DocIdSetIterator::next_doc`](crate::core::search::doc_id_set_iterator::DocIdSetIterator::next_doc), [`DocIdSetIterator::advance`](crate::core::search::doc_id_set_iterator::DocIdSetIterator::advance), and
    ///   [`TwoPhaseIterator::matches`](crate::core::search::two_phase_iterator::TwoPhaseIterator::matches) will be called.
    ///   If in doubt, pass `i64::MAX`, which will produce a [`Scorer`] that has good iteration capabilities.
    fn get(&self, lead_cost: i64) -> Result<Option<Self::Scorer>>;

    /// Optional: Get a bulk scorer that is optimized for bulk-scoring.
    ///
    /// The default implementation wraps `get(i64::MAX)` in a `DefaultBulkScorer`,
    /// which iterates matches from the scorer. Some queries can have more efficient
    /// approaches for matching all hits.
    fn bulk_scorer(&self) -> Result<Self::BulkScorer> {
        let _scorer = self.get(i64::MAX)?;
        todo!()
    }

    /// Get an estimate of the [`Scorer`] that would be returned by [`ScorerSupplier::get`].
    /// This may be a costly operation, so it should only be called if necessary.
    ///
    /// Corresponds to [`DocIdSetIterator::cost`](crate::core::search::doc_id_set_iterator::DocIdSetIterator::cost).
    fn cost(&mut self) -> i64;

    /// Inform this [`ScorerSupplier`] that its returned scorers produce scores that get passed
    /// to the collector, as opposed to partial scores that then need to get combined (e.g. summed up).
    ///
    /// Note: This method also gets called if scores are not requested, e.g. because the score mode
    /// is [`ScoreMode::COMPLETE_NO_SCORES`](crate::core::search::score_mode::ScoreMode::CompleteNoScores).
    /// Implementations should look at both the score mode and this boolean to know whether to prepare
    /// for reacting to [`Scorer::set_min_competitive_score`] calls.
    fn set_top_level_scoring_clause(&mut self) -> Result<()> {
        Ok(())
    }
}
