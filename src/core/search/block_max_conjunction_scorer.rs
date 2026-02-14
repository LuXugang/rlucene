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
use crate::core::search::scorable::Scorable;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
    TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub type BlockMaxConjunctionScorerDisi<S> = DocIdSetIteratorEnum2<
    DocIdSetIteratorImpl<S>,
    TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S>>,
>;
/// Scorer for conjunctions that checks the maximum scores of each clause
/// in order to potentially skip over blocks that cannot have competitive matches.
pub struct BlockMaxConjunctionScorer<S>
where
    S: Scorer,
{
    disi: BlockMaxConjunctionScorerDisi<S>,
    two_phase_state: TwoPhaseState,
}
impl<S> BlockMaxConjunctionScorer<S>
where
    S: Scorer,
{
    pub(crate) fn new(scorers_list: Vec<S>) -> Result<Self> {
        let mut scorers_with_cost = Vec::with_capacity(scorers_list.len());
        for mut v in scorers_list {
            let cost = v.iterator_mut().cost()?;
            scorers_with_cost.push((cost, v));
        }
        scorers_with_cost.sort_by(|a, b| b.0.cmp(&a.0));

        let mut scorers = Vec::with_capacity(scorers_with_cost.len());
        for (_, mut v) in scorers_with_cost {
            v.advance_shallow(0)?;
            scorers.push(v);
        }

        let mut match_cost = Vec::with_capacity(scorers.len());
        for (i, s) in scorers.iter_mut().enumerate() {
            if let Some(tpi) = s.two_phase_iterator() {
                match_cost.push((i, tpi.match_cost()));
            }
        }
        match_cost.sort_by(|a, b| b.1.total_cmp(&a.1));
        let approx = DocIdSetIteratorImpl::new(scorers);
        let (disi, two_phase_state) = if match_cost.is_empty() {
            (BlockMaxConjunctionScorerDisi::A(approx), TwoPhaseState::No)
        } else {
            (
                BlockMaxConjunctionScorerDisi::B(TwoPhaseIteratorAsDocIdSetIterator::new(
                    TwoPhaseIteratorImpl::new(approx, match_cost),
                )),
                TwoPhaseState::Yes,
            )
        };
        Ok(Self {
            disi,
            two_phase_state,
        })
    }
    fn do_score(s: &mut [S]) -> Result<f32> {
        let mut sum: f64 = 0.0;
        for scorer in s.iter_mut() {
            sum += scorer.score()? as f64;
        }

        Ok(sum as f32)
    }
}

impl<S> Scorable for BlockMaxConjunctionScorer<S>
where
    S: Scorer,
{
    fn score(&mut self) -> Result<f32> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => Self::do_score(disi.scorers.as_mut()),
            DocIdSetIteratorEnum2::B(ref mut tpi_disi) => {
                Self::do_score(tpi_disi.two_phase_iterator.approx.scorers.as_mut())
            },
        }
    }

    fn set_min_competitive_score(&mut self, score: f32) -> Result<()> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.min_score = score,
            DocIdSetIteratorEnum2::B(ref mut tpi_disi) => {
                tpi_disi.two_phase_iterator.approx.min_score = score
            },
        }
        Ok(())
    }
}

impl<S> Scorer for BlockMaxConjunctionScorer<S>
where
    S: Scorer + 'static,
{
    fn doc_id(&mut self) -> Result<i32> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut v) => v.scorer_doc_id(),
            DocIdSetIteratorEnum2::B(ref mut v) => v.two_phase_iterator.approx.scorer_doc_id(),
        }
    }

    fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
        match &self.disi {
            DocIdSetIteratorEnum2::A(v) => Box::new(v),
            DocIdSetIteratorEnum2::B(v) => Box::new(v),
        }
    }

    fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        match &mut self.disi {
            DocIdSetIteratorEnum2::A(v) => Box::new(v),
            DocIdSetIteratorEnum2::B(v) => Box::new(v),
        }
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
        let BlockMaxConjunctionScorer { disi, .. } = *self;
        match disi {
            DocIdSetIteratorEnum2::A(v) => Box::new(v),
            DocIdSetIteratorEnum2::B(v) => Box::new(v),
        }
    }

    fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
        match self.two_phase_state {
            TwoPhaseState::No => None,
            _ => match self.disi {
                DocIdSetIteratorEnum2::A(_) => {
                    debug_assert!(false, "should not be here");
                    None
                },
                DocIdSetIteratorEnum2::B(ref v) => Some(Box::new(&v.two_phase_iterator)),
            },
        }
    }

    fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
        match self.two_phase_state {
            TwoPhaseState::No => None,
            _ => match self.disi {
                DocIdSetIteratorEnum2::A(_) => {
                    debug_assert!(false, "should not be here");
                    None
                },
                DocIdSetIteratorEnum2::B(ref mut v) => Some(Box::new(&mut v.two_phase_iterator)),
            },
        }
    }

    fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>>
    where
        Self: Sized,
    {
        let BlockMaxConjunctionScorer {
            disi,
            two_phase_state,
            ..
        } = *self;
        match two_phase_state {
            TwoPhaseState::No => None,
            _ => match disi {
                DocIdSetIteratorEnum2::A(_) => {
                    debug_assert!(false, "should not be here");
                    None
                },
                DocIdSetIteratorEnum2::B(v) => Some(Box::new(v.two_phase_iterator)),
            },
        }
    }

    fn advance_shallow(&mut self, _target: i32) -> Result<i32> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.advance_shallow(_target),
            DocIdSetIteratorEnum2::B(ref mut tpi_disi) => {
                tpi_disi.two_phase_iterator.approx.advance_shallow(_target)
            },
        }
    }

    fn get_max_score(&mut self, upto: i32) -> Result<f32> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.get_max_score(upto),
            DocIdSetIteratorEnum2::B(ref mut tpi_disi) => {
                tpi_disi.two_phase_iterator.approx.get_max_score(upto)
            },
        }
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        self.two_phase_state
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
        match self.two_phase_state {
            TwoPhaseState::No => self.iterator(),
            _ => match self.disi {
                DocIdSetIteratorEnum2::A(_) => self.iterator(),
                DocIdSetIteratorEnum2::B(ref v) => v.two_phase_iterator.approximation(),
            },
        }
    }

    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        match self.two_phase_state {
            TwoPhaseState::No => self.iterator_mut(),
            _ => match self.disi {
                DocIdSetIteratorEnum2::A(_) => self.iterator_mut(),
                DocIdSetIteratorEnum2::B(ref mut v) => v.two_phase_iterator.approximation_mut(),
            },
        }
    }
}

pub struct DocIdSetIteratorImpl<S>
where
    S: Scorer,
{
    scorers: Vec<S>,
    upto: i32,
    max_score: f32,
    min_score: f32,
}
impl<S> DocIdSetIteratorImpl<S>
where
    S: Scorer,
{
    fn new(scorers: Vec<S>) -> Self {
        Self {
            scorers,
            upto: -1,
            max_score: 0f32,
            min_score: 0f32,
        }
    }

    fn scorer_doc_id(&mut self) -> Result<i32> {
        self.scorers[0].doc_id()
    }

    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        // We use block boundaries of the lead scorer.
        // It is tempting to fold in other clauses as well to have better bounds of
        // the score, but then there is a risk of not progressing fast enough.
        let result = self.scorers[0].advance_shallow(target)?;
        // But we still need to shallow-advance other clauses, in order to have
        // better score upper bounds
        for scorer in self.scorers.iter_mut().skip(1) {
            scorer.advance_shallow(target)?;
        }
        Ok(result)
    }
    fn get_max_score(&mut self, upto: i32) -> Result<f32> {
        let mut sum = 0f64;
        for scorer in self.scorers.iter_mut() {
            sum += scorer.get_max_score(upto)? as f64;
        }
        Ok(sum as f32)
    }
    fn do_next(&mut self, mut doc: i32) -> Result<i32> {
        'advance_head: loop {
            assert_eq!(doc, self.scorers[0].iterator().doc_id());

            if doc == NO_MORE_DOCS {
                return Ok(NO_MORE_DOCS);
            }

            if doc > self.upto {
                // This check is useful when scorers return information about blocks
                // that do not actually have any matches. Otherwise `doc` will always
                // be in the current block already since it is always the result of
                let next_target = self.advance_target(doc)?;
                if next_target != doc {
                    doc = self.scorers[0].iterator_mut().advance(next_target)?;
                    continue;
                }
            }

            assert!(doc <= self.upto);
            let len = self.scorers.len();
            // then find agreement with other iterators
            for i in 1..len {
                let other_doc_id = self.scorers[i].iterator().doc_id();
                // other.doc may already be equal to doc if we "continued advanceHead"
                // on the previous iteration and the advance on the lead scorer exactly matched.
                if other_doc_id < doc {
                    let next = self.scorers[i].iterator_mut().advance(doc)?;
                    if next > doc {
                        // iterator beyond the current doc - advance lead and continue to the new highest
                        // doc.
                        let v = self.advance_target(next)?;
                        doc = self.scorers[0].iterator_mut().advance(v)?;
                        continue 'advance_head;
                    }
                }
                assert_eq!(self.scorers[i].iterator().doc_id(), doc);
            }
            return Ok(doc);
        }
    }
    fn move_to_next_block(&mut self, target: i32) -> Result<()> {
        if self.min_score == 0.0 {
            self.upto = target;
            self.max_score = f32::INFINITY;
        } else {
            self.upto = self.advance_shallow(target)?;
            self.max_score = self.get_max_score(self.upto)?;
        }
        Ok(())
    }
    fn advance_target(&mut self, mut target: i32) -> Result<i32> {
        if target > self.upto {
            self.move_to_next_block(target)?;
        }

        loop {
            assert!(self.upto >= target);

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
}
impl<S> DocIdSetIterator for DocIdSetIteratorImpl<S>
where
    S: Scorer,
{
    fn doc_id(&self) -> i32 {
        self.scorers[0].approximation().doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc_id() + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        let doc = self.advance_target(target)?;
        let v = self.scorers[0].iterator_mut().advance(doc)?;
        self.do_next(v)
    }

    fn cost(&self) -> Result<i64> {
        self.scorers[0].iterator().cost()
    }
}
pub struct TwoPhaseIteratorImpl<S>
where
    S: Scorer,
{
    approx: DocIdSetIteratorImpl<S>,
    match_cost: f32,
    has_tpi_idx: Vec<(usize, f32)>,
}
impl<S> TwoPhaseIteratorImpl<S>
where
    S: Scorer,
{
    fn new(approx: DocIdSetIteratorImpl<S>, has_tpi_idx: Vec<(usize, f32)>) -> Self {
        let match_cost: f32 = has_tpi_idx.iter().map(|&(_, cost)| cost).sum();
        Self {
            approx,
            match_cost,
            has_tpi_idx,
        }
    }
}
impl<S> TwoPhaseIterator for TwoPhaseIteratorImpl<S>
where
    S: Scorer,
{
    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&mut self.approx)
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&self.approx)
    }

    fn matches(&mut self) -> Result<bool> {
        #[cfg(debug_assertions)]
        let doc = self.approx.scorer_doc_id()?;
        for (idx, _) in &self.has_tpi_idx {
            match self.approx.scorers[*idx].two_phase_iterator_mut() {
                Some(ref mut tpi) => {
                    debug_assert!(tpi.approximation().doc_id() == doc);
                    if !tpi.matches()? {
                        return Ok(false);
                    }
                },
                None => {
                    // should not happen
                    return Err(LuceneError::illegal_state(
                        "two_phase_iterator should not be None",
                    ));
                },
            }
        }
        Ok(true)
    }

    fn match_cost(&self) -> f32 {
        self.match_cost
    }
}
