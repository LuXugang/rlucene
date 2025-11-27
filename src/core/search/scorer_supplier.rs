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
use crate::core::search::bulk_scorer::{
    BulkScorer, Either2BulkScorer, Either3BulkScorer, Either4BulkScorer, Either5BulkScorer,
    Either6BulkScorer,
};
use crate::core::search::scorer::{
    Either2Scorer, Either3Scorer, Either4Scorer, Either5Scorer, Either6Scorer, Scorer,
};
use crate::core::search::weight::DefaultBulkScorer;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// A supplier of [`Scorer`].
///
/// This allows to get an estimate of the cost before building the [`Scorer`].
pub trait ScorerSupplier<LR>
where
    LR: LeafReader,
{
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
    /// - `context`: The [`LeafReaderContext`] that this scorer supplier was created for.
    fn get(
        &mut self,
        lead_cost: i64,
        context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::Scorer>>;

    /// Optional: Get a bulk scorer that is optimized for bulk-scoring.
    ///
    /// The default implementation wraps `get(i64::MAX)` in a `DefaultBulkScorer`,
    /// which iterates matches from the scorer. Some queries can have more efficient
    /// approaches for matching all hits.
    fn bulk_scorer(&mut self, context: &LeafReaderContext<LR>) -> Result<Option<Self::BulkScorer>>;
    fn default_bulk_scorer(
        &mut self,
        context: &LeafReaderContext<LR>,
    ) -> Result<DefaultBulkScorer<Self::Scorer>> {
        match self.get(i64::MAX, context)? {
            Some(scorer) => Ok(DefaultBulkScorer::new(scorer)),
            None => Err(LuceneError::illegal_state(
                "ScorerSupplier::get returned None",
            )),
        }
    }

    /// Get an estimate of the [`Scorer`] that would be returned by [`ScorerSupplier::get`].
    /// This may be a costly operation, so it should only be called if necessary.
    ///
    /// Corresponds to [`DocIdSetIterator::cost`](crate::core::search::doc_id_set_iterator::DocIdSetIterator::cost).
    fn cost(&mut self, context: &LeafReaderContext<LR>) -> Result<i64>;

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
macro_rules! either_scorer_supplier {
    (
        $vis:vis $name:ident
        => { bulk: $bulk:ident, scorer: $scorer:ident }
        { $( $Variant:ident : $T:ident ),+ $(,)? }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<LR, $( $T ),+> ScorerSupplier<LR> for $name<$( $T ),+>
        where
            LR: LeafReader,
            $( $T: ScorerSupplier<LR> ),+
        {
            type Scorer = $scorer<$( <$T as ScorerSupplier<LR>>::Scorer ),+>;
            type BulkScorer = $bulk<$( <$T as ScorerSupplier<LR>>::BulkScorer ),+>;

            fn get(
                &mut self,
                lead_cost: i64,
                context: &LeafReaderContext<LR>,
            ) -> Result<Option<Self::Scorer>> {
                match self {
                    $(
                        Self::$Variant(inner) => {
                            let opt = inner.get(lead_cost, context)?;
                            Ok(opt.map($scorer::$Variant))
                        }
                    ),+
                }
            }

            fn bulk_scorer(
                &mut self,
                context: &LeafReaderContext<LR>,
            ) -> Result<Option<Self::BulkScorer>> {
                match self {
                    $(
                        Self::$Variant(inner) => {
                            let opt = inner.bulk_scorer(context)?;
                            Ok(opt.map($bulk::$Variant))
                        }
                    ),+
                }
            }

            fn default_bulk_scorer(
                &mut self,
                context: &LeafReaderContext<LR>,
            ) -> Result<DefaultBulkScorer<Self::Scorer>> {
                match self {
                    $(
                        Self::$Variant(inner) => match inner.get(i64::MAX, context)? {
                            Some(scorer) => Ok(DefaultBulkScorer::new($scorer::$Variant(scorer))),
                            None => Err(LuceneError::illegal_state(
                                "ScorerSupplier::get returned None",
                            )),
                        },
                    )+
                }
            }

            fn cost(&mut self, context: &LeafReaderContext<LR>) -> Result<i64> {
                match self {
                    $( Self::$Variant(inner) => inner.cost(context), )+
                }
            }

            fn set_top_level_scoring_clause(&mut self) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.set_top_level_scoring_clause(), )+
                }
            }
        }
    };
}

either_scorer_supplier!(
    pub Either2ScorerSupplier
    => { bulk: Either2BulkScorer, scorer: Either2Scorer }
    { A: A, B: B }
);

either_scorer_supplier!(
    pub Either3ScorerSupplier
    => { bulk: Either3BulkScorer, scorer: Either3Scorer }
    { A: A, B: B ,C:C}
);

either_scorer_supplier!(
    pub Either4ScorerSupplier
    => { bulk: Either4BulkScorer, scorer: Either4Scorer }
    { A: A, B: B ,C:C,D:D}
);
either_scorer_supplier!(
    pub Either5ScorerSupplier
    => { bulk: Either5BulkScorer, scorer: Either5Scorer }
    { A: A, B: B ,C:C, D:D,E:E }
);
either_scorer_supplier!(
    pub Either6ScorerSupplier
    => { bulk: Either6BulkScorer, scorer: Either6Scorer }
    { A: A, B: B ,C:C, D:D,E:E,F:F }
);
