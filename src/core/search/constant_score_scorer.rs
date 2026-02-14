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
use crate::core::search::doc_id_set_iterator::{
    DocIdSetIterator, DocIdSetIteratorEnum2, EmptyDISI, EmptyEnum,
};
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
    TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator, TwoPhaseIteratorEnum2,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// A constant-scoring Scorer.
pub struct ConstantScoreScorer<DISI, TPI>
where
    DISI: DocIdSetIterator,
    TPI: TwoPhaseIterator,
{
    score: f32,
    score_mode: ScoreMode,
    disi: ConstantDISI_<DISI, TPI>,
    tpi_state: TwoPhaseState,
}
impl<DISI> ConstantScoreScorer<DISI, DummyTwoPhaseIterator>
where
    DISI: DocIdSetIterator,
{
    /// Constructor based on a [`DocIdSetIterator`] used to drive iteration. Two-phase
    /// iteration is not supported.
    ///
    /// # Parameters
    /// - `score`: the score to return on each document.
    /// - `score_mode`: the score mode.
    /// - `disi`: the iterator that defines matching documents.
    pub fn from_disi(score: f32, score_mode: ScoreMode, disi: DISI) -> Self {
        let approximation = match score_mode {
            ScoreMode::TopScores => {
                ConstantDISI::A(DocIdSetIteratorWrapper::new(EmptyEnum::A(disi)))
            },
            _ => ConstantDISI::B(disi),
        };
        Self {
            score,
            score_mode,
            disi: DocIdSetIteratorEnum2::A(approximation),
            tpi_state: TwoPhaseState::No,
        }
    }
}
impl<TPI> ConstantScoreScorer<DummyDISI, TPI>
where
    TPI: TwoPhaseIterator,
{
    /// Constructor based on a [`TwoPhaseIterator`]. In this case the `Scorer` will
    /// support two-phase iteration.
    ///
    /// # Parameters
    /// - `score`: the score to return on each document.
    /// - `score_mode`: the score mode.
    /// - `two_phase_iterator`: the iterator that defines matching documents.
    pub fn from_tpi(score: f32, score_mode: ScoreMode, two_phase_iterator: TPI) -> Self {
        let two_phase_iterator = match score_mode {
            ScoreMode::TopScores => ConstantTPI::A(TwoPhaseIteratorImpl::new(two_phase_iterator)),
            _ => ConstantTPI::B(two_phase_iterator),
        };
        Self {
            score,
            score_mode,
            disi: DocIdSetIteratorEnum2::B(TwoPhaseIteratorAsDocIdSetIterator::new(
                two_phase_iterator,
            )),
            tpi_state: TwoPhaseState::Yes,
        }
    }
}

impl<DISI, TPI> Scorable for ConstantScoreScorer<DISI, TPI>
where
    DISI: DocIdSetIterator + 'static,
    TPI: TwoPhaseIterator + 'static,
{
    fn score(&mut self) -> Result<f32> {
        Ok(self.score)
    }

    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        if min_score > self.score && matches!(self.score_mode, ScoreMode::TopScores) {
            match self.disi {
                ConstantDISI_::A(ref mut v) => match v {
                    DocIdSetIteratorEnum2::A(v) => {
                        v.delegate = EmptyEnum::B(EmptyDISI::new());
                    },
                    DocIdSetIteratorEnum2::B(_) => {
                        return Err(LuceneError::illegal_state("not DocIdSetIteratorWrapper"));
                    },
                },
                ConstantDISI_::B(ref mut v) => v.two_phase_iterator.set_empty()?,
            }
        }
        Ok(())
    }

    fn cost(&self) -> Result<i64> {
        self.iterator().cost()
    }
}

impl<DISI, TPI> Scorer for ConstantScoreScorer<DISI, TPI>
where
    DISI: DocIdSetIterator + 'static,
    TPI: TwoPhaseIterator + 'static,
{
    fn doc_id(&mut self) -> Result<i32> {
        Ok(self.disi.doc_id())
    }

    fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
        match &self.disi {
            ConstantDISI_::A(v) => match v {
                DocIdSetIteratorEnum2::A(wrapper) => Box::new(wrapper),
                DocIdSetIteratorEnum2::B(disi) => Box::new(disi),
            },
            ConstantDISI_::B(v) => Box::new(v),
        }
    }

    fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        match &mut self.disi {
            ConstantDISI_::A(v) => match v {
                DocIdSetIteratorEnum2::A(wrapper) => Box::new(wrapper),
                DocIdSetIteratorEnum2::B(disi) => Box::new(disi),
            },
            ConstantDISI_::B(v) => Box::new(v),
        }
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
        let ConstantScoreScorer { disi, .. } = *self;
        match disi {
            ConstantDISI_::A(v) => match v {
                DocIdSetIteratorEnum2::A(wrapper) => Box::new(wrapper),
                DocIdSetIteratorEnum2::B(disi) => Box::new(disi),
            },
            ConstantDISI_::B(v) => Box::new(v),
        }
    }

    fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
        match self.tpi_state {
            TwoPhaseState::No => None,
            _ => match self.disi {
                ConstantDISI_::A(_) => {
                    debug_assert!(false, "should not be here");
                    None
                },
                ConstantDISI_::B(ref v) => Some(Box::new(&v.two_phase_iterator)),
            },
        }
    }

    fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
        match self.tpi_state {
            TwoPhaseState::No => None,
            _ => match self.disi {
                ConstantDISI_::A(_) => {
                    debug_assert!(false, "should not be here");
                    None
                },
                ConstantDISI_::B(ref mut v) => Some(Box::new(&mut v.two_phase_iterator)),
            },
        }
    }

    fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
        let ConstantScoreScorer {
            disi, tpi_state, ..
        } = *self;
        match tpi_state {
            TwoPhaseState::No => None,
            _ => match disi {
                ConstantDISI_::A(_) => {
                    debug_assert!(false, "should not be here");
                    None
                },
                ConstantDISI_::B(wrapper) => Some(Box::new(wrapper.two_phase_iterator)),
            },
        }
    }

    fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
        Ok(self.score)
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        self.tpi_state
    }

    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        match self.tpi_state {
            TwoPhaseState::No => self.iterator_mut(),
            _ => match self.disi {
                ConstantDISI_::A(_) => self.iterator_mut(),
                ConstantDISI_::B(ref mut v) => v.two_phase_iterator.approximation_mut(),
            },
        }
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
        match self.tpi_state {
            TwoPhaseState::No => self.iterator(),
            _ => match self.disi {
                ConstantDISI_::A(_) => self.iterator(),
                ConstantDISI_::B(ref v) => v.two_phase_iterator.approximation(),
            },
        }
    }
}

pub struct TwoPhaseIteratorImpl<TPI>
where
    TPI: TwoPhaseIterator,
{
    two_phase_iterator: TPI,
}
impl<TPI> TwoPhaseIteratorImpl<TPI>
where
    TPI: TwoPhaseIterator,
{
    pub fn new(two_phase_iterator: TPI) -> Self {
        Self { two_phase_iterator }
    }
}
impl<TPI> TwoPhaseIterator for TwoPhaseIteratorImpl<TPI>
where
    TPI: TwoPhaseIterator,
{
    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        self.two_phase_iterator.approximation_mut()
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
        self.two_phase_iterator.approximation()
    }

    fn set_empty(&mut self) -> Result<()> {
        self.two_phase_iterator.set_empty()
    }

    fn matches(&mut self) -> Result<bool> {
        self.two_phase_iterator.matches()
    }

    fn match_cost(&self) -> f32 {
        self.two_phase_iterator.match_cost()
    }
}

// used for Constructor from DISI
pub type ConstantDISI<DISI> = DocIdSetIteratorEnum2<DocIdSetIteratorWrapper<EmptyEnum<DISI>>, DISI>;
// used Constructor from TwoPhaseIterator
pub type ConstantTPI<TPI> = TwoPhaseIteratorEnum2<TwoPhaseIteratorImpl<TPI>, TPI>;

pub type ConstantDISI_<DISI, TPI> =
    DocIdSetIteratorEnum2<ConstantDISI<DISI>, TwoPhaseIteratorAsDocIdSetIterator<ConstantTPI<TPI>>>;

pub struct DocIdSetIteratorWrapper<D>
where
    D: DocIdSetIterator,
{
    doc: i32,
    delegate: D,
}

impl<D> DocIdSetIteratorWrapper<D>
where
    D: DocIdSetIterator,
{
    pub fn new(delegate: D) -> Self {
        Self { doc: -1, delegate }
    }
}

impl<D> DocIdSetIterator for DocIdSetIteratorWrapper<D>
where
    D: DocIdSetIterator,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.doc = self.delegate.next_doc()?;
        Ok(self.doc)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.doc = self.delegate.advance(target)?;
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        self.delegate.cost()
    }
}
