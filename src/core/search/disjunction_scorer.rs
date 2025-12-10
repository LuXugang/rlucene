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
use crate::core::search::disi_priority_queue::DisiPriorityQueue;
use crate::core::search::disi_wrapper::DisiWrapper;
use crate::core::search::disjunction_disi_approximation::DisjunctionDISIApproximation;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, Either2DocIdSetIterator};
use crate::core::search::dummy::dummy_scorable::DummyScorable;
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
    TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator, as_doc_id_set_iterator,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};

pub type Disi<S> = Either2DocIdSetIterator<
    DisjunctionDISIApproximation<S>,
    TwoPhaseIteratorAsDocIdSetIterator<TwoPhase<S>>,
>;
/// Base class for Scorers that score disjunctions.
pub struct DisjunctionScorer<S, T>
where
    S: Scorer,
    T: DisjunctionScorerBase,
{
    disi: Disi<S>,
    sub: T,
    tpi_state: TwoPhaseState,
}

impl<S, T> DisjunctionScorer<S, T>
where
    S: Scorer,
    T: DisjunctionScorerBase,
{
    pub(crate) fn new(sub_scorers: Vec<S>, score_mode: ScoreMode, sub: T) -> Result<Self> {
        let sub_scorers_len = sub_scorers.len();
        if sub_scorers_len <= 1 {
            return Err(LuceneError::illegal_argument(
                "There must be at least 2 subScorers",
            ));
        }
        let mut dpq = DisiPriorityQueue::new(sub_scorers_len.try_into()?);
        let mut all_scorers = Vec::with_capacity(sub_scorers_len);
        for (i, scorer) in sub_scorers.into_iter().enumerate() {
            let w = DisiWrapper::new(scorer)?;
            all_scorers.push(w);
            dpq.add(i, all_scorers.as_slice());
        }
        let mut approximation = DisjunctionDISIApproximation::new(dpq, all_scorers);
        let needs_scores = score_mode != ScoreMode::CompleteNoScores;
        let mut has_approximation = false;
        let mut sum_match_cost = 0f32;
        let mut sum_approx_cost = 0i64;
        // Compute matchCost as the average over the matchCost of the subScorers.
        // This is weighted by the cost, which is an expected number of matching documents.
        for idx in approximation.sub_iterators.iter() {
            let w = &mut approximation.all_scores[idx];
            let cost_weight = if w.cost <= 1 { 1 } else { w.cost };
            sum_approx_cost += cost_weight;
            if w.has_two_phase_iterator() == TwoPhaseState::Yes || w.two_phase_iterator()?.is_some()
            {
                has_approximation = true;
                sum_match_cost += w.match_cost * cost_weight as f32;
            }
        }
        let (disi, tpi_state) = if !has_approximation {
            (Disi::A(approximation), TwoPhaseState::No)
        } else {
            let match_cost = sum_match_cost / sum_approx_cost as f32;
            let two_phase = TwoPhase::new(approximation, match_cost, needs_scores)?;
            let v = as_doc_id_set_iterator(two_phase);
            (Disi::B(v), TwoPhaseState::Yes)
        };
        Ok(Self {
            disi,
            sub,
            tpi_state,
        })
    }

    fn get_sub_matched(&mut self) -> Result<Option<usize>> {
        match self.disi {
            Disi::A(ref mut v) => Ok(Some(v.sub_iterators.top_list_root(&mut v.all_scores))),
            Disi::B(ref mut v) => {
                let two_phase = &mut v.two_phase_iterator;
                two_phase.get_sub_matches()
            },
        }
    }
}

impl<S, T> Scorable for DisjunctionScorer<S, T>
where
    S: Scorer,
    T: DisjunctionScorerBase,
{
    fn score(&mut self) -> Result<f32> {
        let _v = self.get_sub_matched()?;
        todo!()
    }

    type Scorable = DummyScorable;

    fn get_children(&self) -> Result<Vec<ChildScorable<Self::Scorable>>> {
        todo!()
    }
}

impl<S, T> Scorer for DisjunctionScorer<S, T>
where
    S: Scorer,
    T: DisjunctionScorerBase,
{
    type DocIdSetIterator = Disi<S>;
    type DocIdSetIteratorRef<'a>
        = &'a Disi<S>
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = &'a mut Disi<S>
    where
        Self: 'a;
    type TwoPhaseIter = TwoPhase<S>;
    type TwoPhaseIterRef<'a>
        = &'a TwoPhase<S>
    where
        Self: 'a;
    type TwoPhaseIterMut<'a>
        = &'a mut TwoPhase<S>
    where
        Self: 'a;

    fn doc_id(&mut self) -> Result<i32> {
        match self.disi {
            Disi::A(ref v) => Ok(v.doc_id()),
            Disi::B(ref v) => {
                let approximation = &v
                    .two_phase_iterator
                    .unverified_matches
                    .compare
                    .approximation;
                Ok(approximation.doc_id())
            },
        }
    }

    fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
        &self.disi
    }

    fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
        &mut self.disi
    }

    fn take_iterator(self) -> Self::DocIdSetIterator {
        self.disi
    }

    fn two_phase_iterator(&self) -> Result<Option<Self::TwoPhaseIterRef<'_>>> {
        match self.tpi_state {
            TwoPhaseState::No => Ok(None),
            _ => Ok(match self.disi {
                Disi::B(ref v) => Some(&v.two_phase_iterator),
                _ => {
                    return Err(LuceneError::illegal_state(
                        "No two-phase iterator available",
                    ));
                },
            }),
        }
    }

    fn two_phase_iterator_mut(&mut self) -> Result<Option<Self::TwoPhaseIterMut<'_>>> {
        match self.tpi_state {
            TwoPhaseState::No => Ok(None),
            _ => Ok(match self.disi {
                Disi::B(ref mut v) => Some(&mut v.two_phase_iterator),
                _ => {
                    return Err(LuceneError::illegal_state(
                        "No two-phase iterator available",
                    ));
                },
            }),
        }
    }

    fn take_two_phase_iterator(self) -> Result<Option<Self::TwoPhaseIter>> {
        match self.tpi_state {
            TwoPhaseState::No => Ok(None),
            _ => Ok(match self.disi {
                Disi::B(v) => Some(v.two_phase_iterator),
                _ => {
                    return Err(LuceneError::illegal_state(
                        "No two-phase iterator available",
                    ));
                },
            }),
        }
    }

    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        self.sub.get_max_score(up_to)
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        self.tpi_state
    }
}

pub struct TwoPhase<S>
where
    S: Scorer,
{
    match_cost: f32,
    // list of verified matches on the current doc
    verified_matches: Option<usize>,
    // priority queue of approximations on the current doc that have not been verified yet
    unverified_matches: PriorityQueue<usize, DisiWrapperCmp<S>>,
    needs_scores: bool,
}
impl<S> TwoPhase<S>
where
    S: Scorer,
{
    fn new(
        approximation: DisjunctionDISIApproximation<S>,
        match_cost: f32,
        needs_scores: bool,
    ) -> Result<Self> {
        let cmp = DisiWrapperCmp { approximation };
        let size: i32 = cmp.approximation.all_scores.len().try_into()?;
        let unverified_matches = PriorityQueue::new(size, cmp)?;
        Ok(Self {
            match_cost,
            verified_matches: None,
            unverified_matches,
            needs_scores,
        })
    }
    pub(crate) fn get_sub_matches(&mut self) -> Result<Option<usize>> {
        // iteration order does not matter
        // TODO IMPORTANT Due to borrow checker, we have to collect first, could we avoid it?
        let v: Vec<usize> = self.unverified_matches.iter_ref().cloned().collect();
        for i in v {
            let w = &mut self.unverified_matches.compare.approximation.all_scores[i];

            if w.matches()? {
                w.next = self.verified_matches;

                self.verified_matches = Some(i);
            }
        }
        self.unverified_matches.clear();
        Ok(self.verified_matches)
    }
    fn all_scores_ref(&mut self) -> &[DisiWrapper<S>] {
        &self.unverified_matches.compare.approximation.all_scores
    }
    fn all_scores_mut(&mut self) -> &mut [DisiWrapper<S>] {
        &mut self.unverified_matches.compare.approximation.all_scores
    }
}
impl<S> TwoPhaseIterator for TwoPhase<S>
where
    S: Scorer,
{
    type DocIdSetIterator = DisjunctionDISIApproximation<S>;
    type DocIdSetIteratorRef<'a>
        = &'a DisjunctionDISIApproximation<S>
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = &'a mut DisjunctionDISIApproximation<S>
    where
        Self: 'a;

    fn approximation_mut(&mut self) -> Result<Self::DocIdSetIteratorMut<'_>> {
        Ok(&mut self.unverified_matches.compare.approximation)
    }

    fn approximation(&self) -> Result<Self::DocIdSetIteratorRef<'_>> {
        Ok(&self.unverified_matches.compare.approximation)
    }

    fn matches(&mut self) -> Result<bool> {
        self.verified_matches = None;
        self.unverified_matches.clear();

        let root_idx = {
            self.unverified_matches
                .compare
                .approximation
                .sub_iterators
                .top_list_root(&mut self.unverified_matches.compare.approximation.all_scores)
        };
        let mut w_idx_opt = Some(root_idx);
        while w_idx_opt.is_some() {
            let w_idx = w_idx_opt.unwrap();
            let w = &mut self.unverified_matches.compare.approximation.all_scores[w_idx];
            let next = w.next;
            let has_no_two_phase_view = (w.has_two_phase_iterator() == TwoPhaseState::No)
                || w.two_phase_iterator()?.is_some();
            if has_no_two_phase_view {
                // implicitly verified, move it to verifiedMatches
                w.next = self.verified_matches;
                self.verified_matches = Some(w_idx);

                if !self.needs_scores {
                    // we can stop here
                    return Ok(true);
                }
            } else {
                self.unverified_matches.add(w_idx)?;
            }
            w_idx_opt = next;
        }

        if self.verified_matches.is_some() {
            return Ok(true);
        }

        // verify subs that have an two-phase iterator
        // least-costly ones first
        while let Some(w_idx) = self.unverified_matches.pop()? {
            let w = &mut self.all_scores_mut()[w_idx];

            if w.matches()? {
                w.next = None;
                self.verified_matches = Some(w_idx);
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn match_cost(&self) -> f32 {
        self.match_cost
    }
}

struct DisiWrapperCmp<S>
where
    S: Scorer,
{
    approximation: DisjunctionDISIApproximation<S>,
}
impl<S> Compare<usize> for DisiWrapperCmp<S>
where
    S: Scorer,
{
    fn less_than(&self, a: &usize, b: &usize) -> Result<bool> {
        Ok(self.approximation.all_scores[*a].match_cost
            < self.approximation.all_scores[*b].match_cost)
    }
}

pub trait DisjunctionScorerBase {
    fn score<S>(&self, top_list: DisiWrapper<S>) -> Result<f32>
    where
        S: Scorer;
    fn advance_shallow(&mut self, target: i32) -> Result<i32>;
    fn get_max_score(&mut self, up_to: i32) -> Result<f32>;
}
