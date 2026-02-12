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
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::phrase_matcher::PhraseMatcher;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::Result;

pub struct PhraseScorer<PM, SS, N>
where
    PM: PhraseMatcher,
    SS: SimScorer,
    N: NumericDocValues,
{
    matcher: PM,
    scorer_mode: ScoreMode,
    sim_scorer: SS,
    norms: Option<N>,
    match_cost: f32,
    min_competitive_score: f32,
    freq: f32,
}
impl<PM, SS, N> PhraseScorer<PM, SS, N>
where
    PM: PhraseMatcher,
    SS: SimScorer,
    N: NumericDocValues,
{
}

impl<PM, SS, N> Scorable for PhraseScorer<PM, SS, N>
where
    PM: PhraseMatcher,
    SS: SimScorer,
    N: NumericDocValues,
{
    fn score(&mut self) -> Result<f32> {
        todo!()
    }
}

impl<PM, SS, N> Scorer for PhraseScorer<PM, SS, N>
where
    PM: PhraseMatcher,
    SS: SimScorer,
    N: NumericDocValues,
{
    fn doc_id(&mut self) -> Result<i32> {
        todo!()
    }

    fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
        todo!()
    }

    fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        todo!()
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
        todo!()
    }

    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        todo!()
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        todo!()
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
        todo!()
    }

    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        todo!()
    }
}

pub struct TwoPhaseIteratorImpl<PM, SS, N>
where
    PM: PhraseMatcher,
    SS: SimScorer,
    N: NumericDocValues,
{
    matcher: PM,
    sim_scorer: SS,
    norms: Option<N>,
    match_cost: f32,
}
impl<PM, SS, N> TwoPhaseIteratorImpl<PM, SS, N> where PM: PhraseMatcher, SS:SimScorer,N:NumericDocValues {
}
impl<PM, SS, N> TwoPhaseIterator for TwoPhaseIteratorImpl<PM, SS, N> where PM: PhraseMatcher, SS:SimScorer,N:NumericDocValues {
    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        todo!()
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
        todo!()
    }

    fn matches(&mut self) -> Result<bool> {
        todo!()
    }

    fn match_cost(&self) -> f32 {
        self.match_cost
    }
}
