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
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::dummy::dummy_scorable::DummyScorable;
use crate::core::search::scorable::Scorable;
use crate::core::search::scorer::Scorer;
use crate::core::search::two_phase_iterator::{
    Either2TwoPhaseIterator, TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::error::lucene_error::Result;

/// A Scorer for queries with a required subscorer and an excluding (prohibited) sub [`Scorer`].
pub struct ReqExclScorer<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    disi: TwoPhaseIteratorAsDocIdSetIterator<TPI<S1, S2>>,
}
impl<S1, S2> ReqExclScorer<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    pub fn new(mut req_scorer: S1, mut excl_scorer: S2) -> Result<Self> {
        let match_cost = match_cost(&mut req_scorer, &mut excl_scorer)?;

        let check_req = match req_scorer.two_phase_iterator_mut() {
            Some(ref mut req_scorer_tpi) => match excl_scorer.two_phase_iterator_mut() {
                Some(excl_scorer_tpi) => {
                    req_scorer_tpi.match_cost() <= excl_scorer_tpi.match_cost()
                },
                None => false,
            },
            None => true,
        };

        let two_phase_iterator = if check_req {
            Either2TwoPhaseIterator::A(TwoPhaseIteratorImpl1::new(
                req_scorer,
                excl_scorer,
                match_cost,
            ))
        } else {
            Either2TwoPhaseIterator::B(TwoPhaseIteratorImpl2::new(
                req_scorer,
                excl_scorer,
                match_cost,
            ))
        };
        let disi = TwoPhaseIteratorAsDocIdSetIterator::new(two_phase_iterator);
        Ok(Self { disi })
    }
}

impl<S1, S2> Scorable for ReqExclScorer<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn score(&mut self) -> Result<f32> {
        match self.disi.two_phase_iterator {
            Either2TwoPhaseIterator::A(ref mut tpi) => Ok(tpi.req_scorer.score()?),
            Either2TwoPhaseIterator::B(ref mut tpi) => Ok(tpi.req_scorer.score()?),
        }
    }

    type Scorable = DummyScorable;
}

impl<S1, S2> Scorer for ReqExclScorer<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    type DocIdSetIterator = TwoPhaseIteratorAsDocIdSetIterator<TPI<S1, S2>>;
    type DocIdSetIteratorRef<'a>
        = &'a TwoPhaseIteratorAsDocIdSetIterator<TPI<S1, S2>>
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = &'a mut TwoPhaseIteratorAsDocIdSetIterator<TPI<S1, S2>>
    where
        Self: 'a;
    type TwoPhaseIter = TPI<S1, S2>;
    type TwoPhaseIterMut<'a>
        = &'a mut TPI<S1, S2>
    where
        Self: 'a;

    fn doc_id(&mut self) -> Result<i32> {
        match self.disi.two_phase_iterator {
            Either2TwoPhaseIterator::A(ref mut tpi) => Ok(tpi.req_scorer.doc_id()?),
            Either2TwoPhaseIterator::B(ref mut tpi) => Ok(tpi.req_scorer.doc_id()?),
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

    fn two_phase_iterator_mut(&mut self) -> Option<Self::TwoPhaseIterMut<'_>> {
        Some(&mut self.disi.two_phase_iterator)
    }

    fn take_two_phase_iterator(self) -> Option<Self::TwoPhaseIter>
    where
        Self: Sized,
    {
        Some(self.disi.two_phase_iterator)
    }

    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        match self.disi.two_phase_iterator {
            Either2TwoPhaseIterator::A(ref mut tpi) => {
                Ok(tpi.req_scorer.advance_shallow(target)?)
            },
            Either2TwoPhaseIterator::B(ref mut tpi) => {
                Ok(tpi.req_scorer.advance_shallow(target)?)
            },
        }
    }

    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        match self.disi.two_phase_iterator {
            Either2TwoPhaseIterator::A(ref mut tpi) => Ok(tpi.req_scorer.get_max_score(up_to)?),
            Either2TwoPhaseIterator::B(ref mut tpi) => Ok(tpi.req_scorer.get_max_score(up_to)?),
        }
    }

    fn has_two_phase_iterator(&self) -> bool {
        true
    }
}

pub struct TwoPhaseIteratorImpl1<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    req_scorer: S1,
    excl_scorer: S2,
    match_cost: f32,
}
impl<S1, S2> TwoPhaseIteratorImpl1<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn new(req_scorer: S1, excl_scorer: S2, match_cost: f32) -> Self {
        Self {
            req_scorer,
            excl_scorer,
            match_cost,
        }
    }
}
impl<S1, S2> TwoPhaseIterator for TwoPhaseIteratorImpl1<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    type DocIdSetIterator = DummyDISI;
    type DocIdSetIteratorRef<'a>
        = DummyDISI
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = DummyDISI
    where
        Self: 'a;

    fn approximation_mut(&mut self) -> Result<Self::DocIdSetIteratorMut<'_>> {
        unreachable!("should not be called");
    }

    fn approximation(&self) -> Result<Self::DocIdSetIteratorRef<'_>> {
        unreachable!("should not be called");
    }

    fn matches(&mut self) -> Result<bool> {
        let doc = self.req_scorer.iterator().doc_id();
        // check if the doc is not excluded
        {
            let mut excl_iter = self.excl_scorer.iterator_mut();
            let mut excl_doc = excl_iter.doc_id();
            if excl_doc < doc {
                excl_doc = excl_iter.advance(doc)?;
            }
            if excl_doc != doc {
                return match self.req_scorer.two_phase_iterator_mut() {
                    Some(mut req_tpi) => req_tpi.matches(),
                    None => Ok(true),
                };
            }
        }
        let req_match = match self.req_scorer.two_phase_iterator_mut() {
            Some(mut req_tpi) => req_tpi.matches()?,
            None => true,
        };
        match req_match {
            true => {
                let v = match self.excl_scorer.two_phase_iterator_mut() {
                    Some(mut excl_tpi) => excl_tpi.matches()?,
                    None => true,
                };
                Ok(!v)
            },
            false => Ok(false),
        }
    }

    fn match_cost(&self) -> f32 {
        self.match_cost
    }
}

pub struct TwoPhaseIteratorImpl2<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    req_scorer: S1,
    excl_scorer: S2,
    match_cost: f32,
}
impl<S1, S2> TwoPhaseIteratorImpl2<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn new(req_scorer: S1, excl_scorer: S2, match_cost: f32) -> Self {
        Self {
            req_scorer,
            excl_scorer,
            match_cost,
        }
    }
}
impl<S1, S2> TwoPhaseIterator for TwoPhaseIteratorImpl2<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    type DocIdSetIterator = DummyDISI;
    type DocIdSetIteratorRef<'a>
        = DummyDISI
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = DummyDISI
    where
        Self: 'a;

    fn approximation_mut(&mut self) -> Result<Self::DocIdSetIteratorMut<'_>> {
        unreachable!("should not be called");
    }

    fn approximation(&self) -> Result<Self::DocIdSetIteratorRef<'_>> {
        unreachable!("should not be called");
    }

    fn matches(&mut self) -> Result<bool> {
        let doc = self.req_scorer.iterator().doc_id();

        // check if doc is excluded
        {
            let mut excl_iter = self.excl_scorer.iterator_mut();
            let mut excl_doc = excl_iter.doc_id();

            if excl_doc < doc {
                excl_doc = excl_iter.advance(doc)?;
            }

            if excl_doc != doc {
                return match self.req_scorer.two_phase_iterator_mut() {
                    Some(mut req_tpi) => req_tpi.matches(),
                    None => Ok(true),
                };
            }
        }

        let excl_not_match = match self.excl_scorer.two_phase_iterator_mut() {
            Some(mut excl_tpi) => !excl_tpi.matches()?,
            None => false,
        };

        if !excl_not_match {
            return Ok(false);
        }

        let req_match = match self.req_scorer.two_phase_iterator_mut() {
            Some(mut req_tpi) => req_tpi.matches()?,
            None => true,
        };

        Ok(req_match)
    }

    fn match_cost(&self) -> f32 {
        self.match_cost
    }
}
pub type TPI<S1, S2> =
    Either2TwoPhaseIterator<TwoPhaseIteratorImpl1<S1, S2>, TwoPhaseIteratorImpl2<S1, S2>>;
/// Estimation of the number of operations required to call DISI.advance.
/// This is likely completely wrong,
/// especially given that the cost of this method usually depends on how far you want to advance,
/// but it's probably better than nothing.
const ADVANCE_COST: i32 = 10;
fn match_cost<S1, S2>(req_scorer: &mut S1, excl_scorer: &mut S2) -> Result<f32>
where
    S1: Scorer,
    S2: Scorer,
{
    let mut match_cost: f32 = 2.0;

    if let Some(req_tpi) = req_scorer.two_phase_iterator_mut() {
        // this two-phase iterator must always be matched
        match_cost += req_tpi.match_cost();
    }
    // match cost of the prohibited clause: we need to advance the approximation
    // and match the two-phased iterator
    let excl_match_cost = {
        let extra = match excl_scorer.two_phase_iterator_mut() {
            Some(excl_tpi) => excl_tpi.match_cost(),
            None => 0.0,
        };
        (ADVANCE_COST as f32) + extra
    };
    // upper value for the ratio of documents that reqApproximation matches that
    // exclApproximation also matches
    let req_cost = req_scorer.iterator().cost()?;
    let excl_cost = excl_scorer.iterator().cost()?;

    let ratio = if req_cost <= 0 {
        1.0
    } else if excl_cost <= 0 {
        0.0
    } else {
        (req_cost.min(excl_cost) as f32) / (req_cost as f32)
    };

    match_cost += ratio * excl_match_cost;

    Ok(match_cost)
}
