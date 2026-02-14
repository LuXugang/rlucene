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
use crate::core::search::conjunction_disi::{
    ConjunctionDISI, ConjunctionTwoPhaseIterator, ScorerDisi,
};
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
    TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::error::lucene_error::Result;
#[cfg(test)]
use std::any::Any;

pub type ConjunctionScorerDisi<S> = DocIdSetIteratorEnum2<
    ConjunctionDISI<ScorerDisi<S>>,
    TwoPhaseIteratorAsDocIdSetIterator<ConjunctionTwoPhaseIterator<S>>,
>;
// TODO IMPORTANT This implementation is quite different from the Java version, and performance is worse in some scenarios.
/// Scorer for conjunctions, sets of queries, all of which are required.
pub struct ConjunctionScorer<S>
where
    S: Scorer,
{
    disi: ConjunctionScorerDisi<S>,
    scoring_idx: Vec<usize>,
}
impl<S> ConjunctionScorer<S>
where
    S: Scorer,
{
    /// Create a new [`ConjunctionScorer`], note that `scorers` must be a subset of `required`.
    pub(crate) fn new(required: Vec<S>, scorers: Vec<usize>) -> Result<Self> {
        debug_assert!({ scorers.iter().all(|v| *v < required.len()) });
        let mut has_tpi = false;
        for v in required.iter() {
            if v.two_phase_iterator().is_some() {
                has_tpi = true;
                break;
            }
        }
        let v = ConjunctionDISI::from_scorer(required)?;
        let disi = match has_tpi {
            false => ConjunctionScorerDisi::A(v),
            true => {
                let v =
                    TwoPhaseIteratorAsDocIdSetIterator::new(ConjunctionTwoPhaseIterator::new(v)?);
                ConjunctionScorerDisi::B(v)
            },
        };
        Ok(Self {
            disi,
            scoring_idx: scorers,
        })
    }
}

impl<S> Scorable for ConjunctionScorer<S>
where
    S: Scorer + 'static,
{
    fn score(&mut self) -> Result<f32> {
        let mut sum = 0f64;
        for x in self.scoring_idx.iter() {
            let score = match self.disi {
                DocIdSetIteratorEnum2::A(ref mut v) => v.all_disi[*x].score()?,
                DocIdSetIteratorEnum2::B(ref mut v) => {
                    v.two_phase_iterator.approximation.all_disi[*x].score()?
                },
            };
            sum += score as f64;
        }
        Ok(sum as f32)
    }

    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        // This scorer is only used for TOP_SCORES when there is a single scoring clause
        if self.scoring_idx.len() == 1 {
            let i = self.scoring_idx[0];
            match &mut self.disi {
                DocIdSetIteratorEnum2::A(v) => {
                    v.all_disi[i].set_min_competitive_score(min_score)?
                },
                DocIdSetIteratorEnum2::B(v) => v.two_phase_iterator.approximation.all_disi[i]
                    .set_min_competitive_score(min_score)?,
            }
        }
        Ok(())
    }

    fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
        todo!()
    }

    fn cost(&self) -> Result<i64> {
        self.iterator().cost()
    }
}

impl<S> Scorer for ConjunctionScorer<S>
where
    S: Scorer + 'static,
{
    fn doc_id(&mut self) -> Result<i32> {
        Ok(self.disi.doc_id())
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
        let ConjunctionScorer { disi, .. } = *self;
        match disi {
            DocIdSetIteratorEnum2::A(v) => Box::new(v),
            DocIdSetIteratorEnum2::B(v) => Box::new(v),
        }
    }

    fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
        match self.disi {
            DocIdSetIteratorEnum2::A(_) => None,
            DocIdSetIteratorEnum2::B(ref v) => Some(Box::new(&v.two_phase_iterator)),
        }
    }

    fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
        match self.disi {
            DocIdSetIteratorEnum2::A(_) => None,
            DocIdSetIteratorEnum2::B(ref mut v) => Some(Box::new(&mut v.two_phase_iterator)),
        }
    }

    fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>>
    where
        Self: Sized,
    {
        let ConjunctionScorer { disi, .. } = *self;
        match disi {
            DocIdSetIteratorEnum2::A(_) => None,
            DocIdSetIteratorEnum2::B(v) => Some(Box::new(v.two_phase_iterator)),
        }
    }

    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        if self.scoring_idx.len() == 1 {
            let i = self.scoring_idx[0];
            return match &mut self.disi {
                DocIdSetIteratorEnum2::A(v) => v.all_disi[i].advance_shallow(target),
                DocIdSetIteratorEnum2::B(v) => {
                    v.two_phase_iterator.approximation.all_disi[i].advance_shallow(target)
                },
            };
        }

        match &mut self.disi {
            DocIdSetIteratorEnum2::A(v) => {
                for s in v.all_disi.iter_mut() {
                    s.advance_shallow(target)?;
                }
            },
            DocIdSetIteratorEnum2::B(v) => {
                for s in v.two_phase_iterator.approximation.all_disi.iter_mut() {
                    s.advance_shallow(target)?;
                }
            },
        }

        self.default_advance_shallow(target)
    }

    fn get_max_score(&mut self, upto: i32) -> Result<f32> {
        let mut max_score = 0f64;

        match &mut self.disi {
            DocIdSetIteratorEnum2::A(v) => {
                for s in v.all_disi.iter_mut() {
                    if s.doc_id()? <= upto {
                        max_score += s.get_max_score(upto)? as f64;
                    }
                }
            },
            DocIdSetIteratorEnum2::B(v) => {
                for s in v.two_phase_iterator.approximation.all_disi.iter_mut() {
                    if s.doc_id()? <= upto {
                        max_score += s.get_max_score(upto)? as f64;
                    }
                }
            },
        }

        Ok(max_score as f32)
    }
    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        match self.disi {
            DocIdSetIteratorEnum2::A(_) => TwoPhaseState::No,
            DocIdSetIteratorEnum2::B(_) => TwoPhaseState::Yes,
        }
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
        match self.disi {
            DocIdSetIteratorEnum2::A(_) => self.iterator(),
            DocIdSetIteratorEnum2::B(ref v) => v.two_phase_iterator.approximation(),
        }
    }

    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        match self.disi {
            DocIdSetIteratorEnum2::A(_) => self.iterator_mut(),
            DocIdSetIteratorEnum2::B(ref mut v) => v.two_phase_iterator.approximation_mut(),
        }
    }
    #[cfg(test)]
    fn as_any(&self) -> &dyn Any {
        self
    }
}
