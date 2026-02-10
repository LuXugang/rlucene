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
use crate::core::search::disi_wrapper::DisiWrapper;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::scorer::Scorer;
use crate::core::util::error::lucene_error::Result;
/// A helper to propagate block boundaries for disjunctions. Because a disjunction matches if any of
/// its sub clauses matches, it is tempting to return the minimum block boundary across all clauses.
/// The problem is that it might then make the query slow when the minimum competitive score is high
/// and low-scoring clauses don't drive iteration anymore. So this class computes block boundaries
/// only across clauses whose maximum score is greater than or equal to the minimum competitive
/// score, or the maximum scoring clause if there is no such clause.
pub struct DisjunctionScoreBlockBoundaryPropagator {
    cost: Vec<Cost>,
    lead_index: i32,
}
impl DisjunctionScoreBlockBoundaryPropagator {
    pub(crate) fn new<S>(scorers: &mut [S]) -> Result<Self>
    where
        S: Scorer,
    {
        let mut cost = Vec::with_capacity(scorers.len());
        for (i, scorer) in scorers.iter_mut().enumerate() {
            scorer.advance_shallow(0)?;
            let max_score = scorer.get_max_score(NO_MORE_DOCS)?;
            let iter_cost = scorer.iterator_mut().cost()?;
            cost.push(Cost::new(max_score, i, iter_cost));
        }
        cost.sort_by(|a, b| {
            a.max_score
                .total_cmp(&b.max_score)
                .then_with(|| a.iter_cost.cmp(&b.iter_cost))
        });
        Ok(Self {
            cost,
            lead_index: 0,
        })
    }
    /// Equivalent to Lucene's `advanceShallow(int target)`.
    pub(crate) fn advance_shallow<S>(
        &mut self,
        target: i32,
        scorers: &mut [DisiWrapper<S>],
    ) -> Result<i32>
    where
        S: Scorer,
    {
        // For scorers that are below the lead index, just propagate.
        for i in 0..self.lead_index {
            let s = &mut scorers[self.cost[i as usize].idx].scorer;
            if s.doc_id()? < target {
                s.advance_shallow(target)?;
            }
        }

        // For scorers above the lead index, we take the minimum boundary.
        let lead_idx = self.lead_index as usize;
        let lead_scorer = &mut scorers[self.cost[lead_idx].idx];
        let doc_id = lead_scorer.scorer.doc_id()?;
        let mut up_to = lead_scorer
            .scorer
            .advance_shallow(std::cmp::max(doc_id, target))?;

        for i in (lead_idx + 1)..self.cost.len() {
            let scorer = &mut scorers[self.cost[i].idx];
            if scorer.scorer.doc_id()? <= target {
                let v = scorer.scorer.advance_shallow(target)?;
                up_to = std::cmp::min(v, up_to);
            }
        }

        // If the maximum scoring clauses are beyond `target`, then we use their
        // docID as a boundary. It helps not consider them when computing the
        // maximum score and get a lower score upper bound.
        let mut i = self.cost.len() - 1;
        while i > self.lead_index as usize {
            let scorer = &mut scorers[self.cost[i].idx];
            let doc = scorer.scorer.doc_id()?;
            if doc > target {
                up_to = std::cmp::min(up_to, doc - 1);
            } else {
                break;
            }
            i -= 1;
        }

        Ok(up_to)
    }

    /// Set the minimum competitive score to filter out clauses that score less than this threshold.
    pub(crate) fn set_min_competitive_score(&mut self, min_score: f32) {
        // Update the lead index if necessary
        while ((self.lead_index as usize) < self.cost.len() - 1)
            && min_score > self.cost[self.lead_index as usize].max_score
        {
            self.lead_index += 1;
        }
    }
}
struct Cost {
    max_score: f32,
    idx: usize,
    iter_cost: i64,
}
impl Cost {
    fn new(max_score: f32, idx: usize, iter_cost: i64) -> Self {
        Self {
            max_score,
            idx,
            iter_cost,
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::core::search::disjunction_score_block_boundary_propagator::DisjunctionScoreBlockBoundaryPropagator;

    use crate::core::search::disi_wrapper::DisiWrapper;
    use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, EmptyDISI};

    use crate::core::search::scorable::Scorable;
    use crate::core::search::scorer::{Scorer, TwoPhaseState};
    use crate::core::util::error::lucene_error::{LuceneError, Result};
    use crate::test::util::lucene_test_case::lucene_test_case_util::random;
    use rand::prelude::SliceRandom;

    #[allow(dead_code)] // for quick search
    struct TestDisjunctionScoreBlockBoundaryPropagator;
    #[test]
    fn test_basics() -> Result<()> {
        let mut random = random();
        let scorer1 = FakeScorer::new(20, 0.5);
        let scorer2 = FakeScorer::new(50, 1.5);
        let scorer3 = FakeScorer::new(30, 2.0);
        let scorer4 = FakeScorer::new(80, 3.0);
        let mut scorers = vec![scorer1, scorer2, scorer3, scorer4];
        scorers.shuffle(&mut random);

        let mut propagator = DisjunctionScoreBlockBoundaryPropagator::new(scorers.as_mut_slice())?;
        let mut disi_wrapper = Vec::new();
        for s in scorers.into_iter() {
            disi_wrapper.push(DisiWrapper::new(s)?);
        }

        assert_eq!(20, propagator.advance_shallow(0, &mut disi_wrapper)?);

        propagator.set_min_competitive_score(0.2);
        assert_eq!(20, propagator.advance_shallow(0, &mut disi_wrapper)?);

        propagator.set_min_competitive_score(0.7);
        assert_eq!(30, propagator.advance_shallow(0, &mut disi_wrapper)?);

        propagator.set_min_competitive_score(1.2);
        assert_eq!(30, propagator.advance_shallow(0, &mut disi_wrapper)?);

        propagator.set_min_competitive_score(1.7);
        assert_eq!(30, propagator.advance_shallow(0, &mut disi_wrapper)?);

        propagator.set_min_competitive_score(2.2);
        assert_eq!(80, propagator.advance_shallow(0, &mut disi_wrapper)?);

        propagator.set_min_competitive_score(5.0);
        assert_eq!(80, propagator.advance_shallow(0, &mut disi_wrapper)?);

        Ok(())
    }

    struct FakeScorer {
        boundary: i32,
        max_score: f32,
        disi: EmptyDISI,
    }
    impl FakeScorer {
        fn new(boundary: i32, max_score: f32) -> Self {
            Self {
                boundary,
                max_score,
                disi: EmptyDISI::default(),
            }
        }
    }

    impl Scorable for FakeScorer {
        fn score(&mut self) -> Result<f32> {
            Err(LuceneError::unsupported_operation(""))
        }
    }

    impl Scorer for FakeScorer {
        fn doc_id(&mut self) -> Result<i32> {
            Ok(0)
        }

        fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
            Box::new(&self.disi)
        }

        fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
            Box::new(&mut self.disi)
        }

        fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
            let FakeScorer { disi, .. } = *self;
            Box::new(disi)
        }

        fn advance_shallow(&mut self, target: i32) -> Result<i32> {
            assert!(target <= self.boundary);
            Ok(self.boundary)
        }

        fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
            Ok(self.max_score)
        }

        fn has_two_phase_iterator(&self) -> TwoPhaseState {
            TwoPhaseState::No
        }

        fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
            Box::new(&self.disi)
        }

        fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
            Box::new(&mut self.disi)
        }
    }
}
