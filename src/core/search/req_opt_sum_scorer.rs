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
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::dummy::dummy_scorable::DummyScorable;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::score_mode::ScoreMode::TopScores;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
    TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub type ReqOptSumScorerDisi<S1, S2> = DocIdSetIteratorEnum2<
    DocIdSetIteratorImpl<S1, S2>,
    TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S1, S2>>,
>;
/// A scorer for queries with a required part and an optional part.
/// Delays advance on the optional part until a score is needed.
pub struct ReqOptSumScorer<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    disi: ReqOptSumScorerDisi<S1, S2>,
    tpi_state: TwoPhaseState,
}
impl<S1, S2> ReqOptSumScorer<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    /// Construct a `ReqOptScorer`.
    ///
    /// * `req_scorer` — the required scorer, which must match
    /// * `opt_scorer` — the optional scorer, used only for scoring
    /// * `score_mode` — how the produced scorers will be consumed
    pub(crate) fn new(
        mut req_scorer: S1,
        mut opt_scorer: S2,
        score_mode: ScoreMode,
    ) -> Result<Self> {
        let req_max_score = if score_mode != TopScores {
            f32::MAX
        } else {
            req_scorer.advance_shallow(0)?;
            opt_scorer.advance_shallow(0)?;
            req_scorer.get_max_score(NO_MORE_DOCS)?
        };
        let has_tpi = (req_scorer.has_two_phase_iterator() == TwoPhaseState::Yes
            || req_scorer.two_phase_iterator()?.is_some())
            && (opt_scorer.has_two_phase_iterator() == TwoPhaseState::Yes
                || opt_scorer.two_phase_iterator()?.is_some());
        let approximation = DocIdSetIteratorImpl::new(req_scorer, opt_scorer, req_max_score)?;
        match has_tpi {
            true => Ok(Self {
                disi: DocIdSetIteratorEnum2::B(TwoPhaseIteratorAsDocIdSetIterator::new(
                    TwoPhaseIteratorImpl::new(approximation),
                )),
                tpi_state: TwoPhaseState::Yes,
            }),
            false => Ok(Self {
                disi: DocIdSetIteratorEnum2::A(approximation),
                tpi_state: TwoPhaseState::No,
            }),
        }
    }
}

impl<S1, S2> Scorable for ReqOptSumScorer<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn score(&mut self) -> Result<f32> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.score(),
            DocIdSetIteratorEnum2::B(ref mut wrapper) => wrapper.two_phase_iterator.disi.score(),
        }
    }

    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.set_min_competitive_score(min_score),
            DocIdSetIteratorEnum2::B(ref mut wrapper) => wrapper
                .two_phase_iterator
                .disi
                .set_min_competitive_score(min_score),
        }
    }

    type Scorable = DummyScorable;
}

impl<S1, S2> Scorer for ReqOptSumScorer<S1, S2>
where
    S1: Scorer + 'static,
    S2: Scorer + 'static,
{
    type DocIdSetIteratorRef<'a>
        = &'a ReqOptSumScorerDisi<S1, S2>
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = &'a mut ReqOptSumScorerDisi<S1, S2>
    where
        Self: 'a;
    type TwoPhaseIter = TwoPhaseIteratorImpl<S1, S2>;
    type TwoPhaseIterRef<'a>
        = &'a TwoPhaseIteratorImpl<S1, S2>
    where
        Self: 'a;
    type TwoPhaseIterMut<'a>
        = &'a mut TwoPhaseIteratorImpl<S1, S2>
    where
        Self: 'a;

    fn doc_id(&mut self) -> Result<i32> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.req_scorer.doc_id(),
            DocIdSetIteratorEnum2::B(ref mut wrapper) => {
                wrapper.two_phase_iterator.disi.req_scorer.doc_id()
            },
        }
    }

    fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
        &self.disi
    }

    fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
        &mut self.disi
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
        let ReqOptSumScorer { disi, .. } = *self;
        Box::new(disi)
    }

    fn two_phase_iterator(&self) -> Result<Option<Self::TwoPhaseIterRef<'_>>> {
        match self.tpi_state {
            TwoPhaseState::No => Ok(None),
            _ => match &self.disi {
                DocIdSetIteratorEnum2::A(_) => Err(LuceneError::illegal_state(
                    "No two-phase iterator available",
                )),
                DocIdSetIteratorEnum2::B(wrapper) => Ok(Some(&wrapper.two_phase_iterator)),
            },
        }
    }

    fn two_phase_iterator_mut(&mut self) -> Result<Option<Self::TwoPhaseIterMut<'_>>> {
        match self.tpi_state {
            TwoPhaseState::No => Ok(None),
            _ => match &mut self.disi {
                DocIdSetIteratorEnum2::A(_) => Err(LuceneError::illegal_state(
                    "No two-phase iterator available",
                )),
                DocIdSetIteratorEnum2::B(wrapper) => Ok(Some(&mut wrapper.two_phase_iterator)),
            },
        }
    }

    fn take_two_phase_iterator(self) -> Result<Option<Self::TwoPhaseIter>>
    where
        Self: Sized,
    {
        match self.tpi_state {
            TwoPhaseState::No => Ok(None),
            _ => match self.disi {
                DocIdSetIteratorEnum2::A(_) => Err(LuceneError::illegal_state(
                    "No two-phase iterator available",
                )),
                DocIdSetIteratorEnum2::B(wrapper) => Ok(Some(wrapper.two_phase_iterator)),
            },
        }
    }

    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.advance_shallow(target),
            DocIdSetIteratorEnum2::B(ref mut wrapper) => {
                wrapper.two_phase_iterator.disi.advance_shallow(target)
            },
        }
    }

    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.get_max_score(up_to),
            DocIdSetIteratorEnum2::B(ref mut wrapper) => {
                wrapper.two_phase_iterator.disi.get_max_score(up_to)
            },
        }
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        self.tpi_state
    }
}

pub struct DocIdSetIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    upto: i32,
    max_score: f32,
    opt_is_required: bool,
    min_score: f32,
    req_scorer: S1,
    opt_scorer: S2,
    req_max_score: f32,
}
impl<S1, S2> DocIdSetIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn new(mut req_scorer: S1, mut opt_scorer: S2, req_max_score: f32) -> Result<Self> {
        req_scorer.advance_shallow(0)?;
        opt_scorer.advance_shallow(0)?;

        let mut disi = Self {
            upto: -1,
            max_score: 0.0,
            opt_is_required: false,
            min_score: 0.0,
            req_scorer,
            opt_scorer,
            req_max_score,
        };

        disi.move_to_next_block(0)?;

        Ok(disi)
    }

    fn move_to_next_block(&mut self, target: i32) -> Result<()> {
        self.upto = self.advance_shallow(target)?;
        let req_max_score_block = self.req_scorer.get_max_score(self.upto)?;
        self.max_score = self.get_max_score(self.upto)?;
        self.opt_is_required = req_max_score_block < self.min_score;
        Ok(())
    }
    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        let mut up_to = self.req_scorer.advance_shallow(target)?;

        let opt_doc = {
            let it = self.opt_scorer.iterator();
            it.doc_id()
        };

        if opt_doc <= target {
            let v = self.opt_scorer.advance_shallow(target)?;
            up_to = up_to.min(v);
        } else if opt_doc != NO_MORE_DOCS {
            up_to = up_to.min(opt_doc - 1);
        }

        Ok(up_to)
    }
    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        let mut max_score = self.req_scorer.get_max_score(up_to)?;

        let opt_doc = {
            let it = self.opt_scorer.iterator();
            it.doc_id()
        };

        if opt_doc <= up_to {
            max_score += self.opt_scorer.get_max_score(up_to)?;
        }

        Ok(max_score)
    }
    fn advance_impacts(&mut self, mut target: i32) -> Result<i32> {
        if target > self.upto {
            self.move_to_next_block(target)?;
        }

        loop {
            if self.max_score >= self.min_score {
                return Ok(target);
            }

            if self.upto == NO_MORE_DOCS {
                return Ok(NO_MORE_DOCS);
            }

            target = self.upto + 1;

            self.move_to_next_block(target)?;
        }
    }
    fn advance_internal(&mut self, target: i32) -> Result<i32> {
        if target == NO_MORE_DOCS {
            self.req_scorer.iterator_mut().advance(target)?;
            return Ok(NO_MORE_DOCS);
        }

        let mut req_doc = target;

        'advance_head: loop {
            if self.min_score != 0.0 {
                req_doc = self.advance_impacts(req_doc)?;
            }

            {
                let mut req_it = self.req_scorer.iterator_mut();
                if req_it.doc_id() < req_doc {
                    req_doc = req_it.advance(req_doc)?;
                }
            }

            if req_doc == NO_MORE_DOCS || !self.opt_is_required {
                return Ok(req_doc);
            }

            let upper_bound = if self.req_max_score < self.min_score {
                NO_MORE_DOCS
            } else {
                self.upto
            };

            if req_doc > upper_bound {
                continue;
            }
            // Find the next common doc within the current block
            let mut opt_it = self.opt_scorer.iterator_mut();
            let mut req_it = self.req_scorer.iterator_mut();
            loop {
                let mut opt_doc = opt_it.doc_id();

                if opt_doc < req_doc {
                    opt_doc = opt_it.advance(req_doc)?;
                }

                if opt_doc > upper_bound {
                    req_doc = upper_bound + 1;
                    continue 'advance_head;
                }

                if opt_doc != req_doc {
                    req_doc = req_it.advance(opt_doc)?;
                    if req_doc > upper_bound {
                        continue 'advance_head;
                    }
                }

                if req_doc == NO_MORE_DOCS || opt_doc == req_doc {
                    return Ok(req_doc);
                }
            }
        }
    }
    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        self.min_score = min_score;
        // Potentially move to a conjunction
        if self.req_max_score < self.min_score {
            self.opt_is_required = true;
            if self.req_max_score == 0.0 {
                // If the required clause doesn't contribute scores, we can propagate the minimum
                // competitive score to the optional clause. This happens when the required clause is a
                // FILTER clause.
                // In theory we could generalize this and set minScore - reqMaxScore as a minimum
                // competitive score, but it's unlikely to help in practice unless reqMaxScore is much
                // smaller than typical scores of the optional clause.
                self.opt_scorer.set_min_competitive_score(self.min_score)?;
            }
        }
        Ok(())
    }
    fn score(&mut self) -> Result<f32> {
        let (mut opt_doc, cur_doc, mut score) = {
            let cur_doc = {
                let req_it = self.req_scorer.iterator();
                req_it.doc_id()
            };
            let score = self.req_scorer.score()?;

            let opt_it = self.opt_scorer.iterator();
            let opt_doc = opt_it.doc_id();
            (opt_doc, cur_doc, score)
        };

        if opt_doc < cur_doc {
            let mut opt_it = self.opt_scorer.iterator();
            opt_doc = opt_it.advance(cur_doc)?;
            if let Some(mut opt_tpi) = self.opt_scorer.two_phase_iterator()?
                && opt_doc == cur_doc
                && !opt_tpi.matches()?
            {
                opt_doc = opt_it.next_doc()?;
            }
        }

        if opt_doc == cur_doc {
            score += self.opt_scorer.score()?;
        }

        Ok(score)
    }
}
impl<S1, S2> DocIdSetIterator for DocIdSetIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn doc_id(&self) -> i32 {
        self.req_scorer.iterator().doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let next = self.req_scorer.iterator().doc_id() + 1;
        self.advance_internal(next)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.advance_internal(target)
    }

    fn cost(&self) -> Result<i64> {
        self.req_scorer.iterator().cost()
    }
}

pub struct TwoPhaseIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    disi: DocIdSetIteratorImpl<S1, S2>,
}
impl<S1, S2> TwoPhaseIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn new(disi: DocIdSetIteratorImpl<S1, S2>) -> Self {
        Self { disi }
    }
}
impl<S1, S2> TwoPhaseIterator for TwoPhaseIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    type DocIdSetIteratorRef<'a>
        = &'a DocIdSetIteratorImpl<S1, S2>
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = &'a mut DocIdSetIteratorImpl<S1, S2>
    where
        Self: 'a;

    fn approximation_mut(&mut self) -> Result<Self::DocIdSetIteratorMut<'_>> {
        Ok(&mut self.disi)
    }

    fn approximation(&self) -> Result<Self::DocIdSetIteratorRef<'_>> {
        Ok(&self.disi)
    }

    fn matches(&mut self) -> Result<bool> {
        if let Some(mut req_tpi) = self.disi.req_scorer.two_phase_iterator()?
            && !req_tpi.matches()?
        {
            return Ok(false);
        }

        // optional scorer logic
        if let Some(mut opt_tpi) = self.disi.opt_scorer.two_phase_iterator()? {
            // The below condition is rare and can only happen if we transitioned to
            // optIsRequired=true
            // after the opt approximation was advanced and before it was confirmed.
            let (opt_doc, req_doc) = {
                let opt_disi = opt_tpi.approximation_mut()?;
                let req_it = self.disi.req_scorer.iterator();
                (opt_disi.doc_id(), req_it.doc_id())
            };

            if self.disi.opt_is_required {
                if req_doc != opt_doc {
                    let mut d = opt_doc;
                    if d < req_doc {
                        let mut opt_disi = opt_tpi.approximation_mut()?;
                        d = opt_disi.advance(req_doc)?;
                    }
                    if d != req_doc {
                        return Ok(false);
                    }
                }

                if !opt_tpi.matches()? {
                    let mut opt_disi = opt_tpi.approximation_mut()?;
                    opt_disi.next_doc()?;
                    return Ok(false);
                }
            } else if opt_doc == req_doc && !opt_tpi.matches()? {
                let mut opt_disi = opt_tpi.approximation_mut()?;
                // Advance the iterator to make it clear it doesn't match the current doc id
                opt_disi.next_doc()?;
            }
        }

        Ok(true)
    }

    fn match_cost(&self) -> f32 {
        let mut cost = 1.0;

        if let Ok(Some(req_tpi)) = self.disi.req_scorer.two_phase_iterator() {
            cost += req_tpi.match_cost();
        }

        if let Ok(Some(opt_tpi)) = self.disi.opt_scorer.two_phase_iterator() {
            cost += opt_tpi.match_cost();
        }

        cost
    }
}
#[cfg(test)]
mod tests {
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::search::dummy::dummy_scorable::DummyScorable;

    use crate::core::search::req_opt_sum_scorer::{ReqOptSumScorer, ReqOptSumScorerDisi};
    use crate::core::search::scorable::Scorable;
    use crate::core::search::scorer::{Scorer, TwoPhaseState};
    use crate::core::util::error::lucene_error::Result;

    #[allow(dead_code)]
    struct TestReqOptSumScorer;

    // TODO: BooleanQuery未实现
    // TODO: ConstantScoreQuery有bug

    struct ReqOptSumScorerWrapper<S1, S2>
    where
        S1: Scorer,
        S2: Scorer,
    {
        base: ReqOptSumScorer<S1, S2>,
    }
    impl<S1, S2> ReqOptSumScorerWrapper<S1, S2>
    where
        S1: Scorer,
        S2: Scorer,
    {
        fn new(base: ReqOptSumScorer<S1, S2>) -> Self {
            Self { base }
        }
    }

    impl<S1, S2> Scorable for ReqOptSumScorerWrapper<S1, S2>
    where
        S1: Scorer,
        S2: Scorer,
    {
        fn score(&mut self) -> Result<f32> {
            self.base.score()
        }

        type Scorable = DummyScorable;
    }

    impl<S1, S2> Scorer for ReqOptSumScorerWrapper<S1, S2>
    where
        S1: Scorer + 'static,
        S2: Scorer + 'static,
    {
        type DocIdSetIteratorRef<'a>
            = &'a ReqOptSumScorerDisi<S1, S2>
        where
            Self: 'a;
        type DocIdSetIteratorMut<'a>
            = <ReqOptSumScorer<S1, S2> as Scorer>::DocIdSetIteratorMut<'a>
        where
            Self: 'a;
        type TwoPhaseIter = <ReqOptSumScorer<S1, S2> as Scorer>::TwoPhaseIter;
        type TwoPhaseIterRef<'a>
            = <ReqOptSumScorer<S1, S2> as Scorer>::TwoPhaseIterRef<'a>
        where
            Self: 'a;
        type TwoPhaseIterMut<'a>
            = <ReqOptSumScorer<S1, S2> as Scorer>::TwoPhaseIterMut<'a>
        where
            Self: 'a;

        fn doc_id(&mut self) -> Result<i32> {
            self.base.doc_id()
        }

        fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
            self.base.iterator()
        }

        fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
            self.base.iterator_mut()
        }

        fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
            let ReqOptSumScorerWrapper { base } = *self;
            Box::new(base).take_iterator()
        }

        fn two_phase_iterator(&self) -> Result<Option<Self::TwoPhaseIterRef<'_>>> {
            self.base.two_phase_iterator()
        }

        fn two_phase_iterator_mut(&mut self) -> Result<Option<Self::TwoPhaseIterMut<'_>>> {
            self.base.two_phase_iterator_mut()
        }

        fn take_two_phase_iterator(self) -> Result<Option<Self::TwoPhaseIter>>
        where
            Self: Sized,
        {
            self.base.take_two_phase_iterator()
        }

        fn advance_shallow(&mut self, target: i32) -> Result<i32> {
            self.base.advance_shallow(target)
        }

        fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
            Ok(f32::MAX)
        }

        fn default_cost(&mut self) -> Result<i64> {
            self.base.default_cost()
        }

        fn has_two_phase_iterator(&self) -> TwoPhaseState {
            self.base.has_two_phase_iterator()
        }
    }
}
