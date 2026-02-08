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
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// Diff to Java Lucene, Compile-time polymorphism makes it unnecessary to wrap `likelyTermScorer`
/// or `likelyImpactsEnum`.
#[derive(Default)]
pub struct DisiWrapper<S>
where
    S: Scorer,
{
    pub(crate) scorer: S,
    pub(crate) next: Option<usize>,
    pub(crate) doc: i32,
    pub(crate) cost: i64,
    // the match cost for two-phase iterators, 0 otherwise
    pub(crate) match_cost: f32,
    // for MaxScoreBulkScorer
    pub(crate) scaled_max_score: i64,
    // for MaxScoreBulkScorer
    pub(crate) max_window_score: f32,
}
impl<S> DisiWrapper<S>
where
    S: Scorer,
{
    pub fn new(mut scorer: S) -> Result<Self> {
        let cost = scorer.iterator_mut().cost()?;
        let match_cost = match scorer.two_phase_iterator_mut() {
            Some(tpi) => tpi.match_cost(),
            None => 0.0,
        };
        Ok(Self {
            scorer,
            next: None,
            doc: -1,
            cost,
            match_cost,
            scaled_max_score: 0,
            max_window_score: 0.0,
        })
    }

    pub fn matches(&mut self) -> Result<bool> {
        match self.scorer.two_phase_iterator_mut() {
            Some(mut tpi) => tpi.matches(),
            None => Err(LuceneError::illegal_state(
                "this scorer does not support two-phase iteration",
            )),
        }
    }
    pub fn matches_may_none(&mut self) -> Result<bool> {
        match self.scorer.two_phase_iterator_mut() {
            Some(mut tpi) => tpi.matches(),
            None => Ok(true),
        }
    }
}

impl<S> Scorable for DisiWrapper<S>
where
    S: Scorer,
{
    fn score(&mut self) -> Result<f32> {
        self.scorer.score()
    }

    fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
        self.scorer.smoothing_score(doc_id)
    }

    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        self.scorer.set_min_competitive_score(min_score)
    }

    fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
        self.scorer.get_children()
    }

    fn cost(&mut self) -> Result<i64> {
        self.scorer.cost()
    }
}

impl<S> Scorer for DisiWrapper<S>
where
    S: Scorer,
{
    fn doc_id(&mut self) -> Result<i32> {
        self.scorer.doc_id()
    }

    fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
        self.scorer.iterator()
    }

    fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        self.scorer.iterator_mut()
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
        let DisiWrapper { scorer, .. } = *self;
        Box::new(scorer).take_iterator()
    }

    fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
        self.scorer.two_phase_iterator()
    }

    fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
        self.scorer.two_phase_iterator_mut()
    }

    fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
        let DisiWrapper { scorer, .. } = *self;
        Box::new(scorer).take_two_phase_iterator()
    }

    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        self.scorer.advance_shallow(target)
    }

    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        self.scorer.get_max_score(up_to)
    }

    fn default_cost(&mut self) -> Result<i64> {
        self.scorer.default_cost()
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        self.scorer.has_two_phase_iterator()
    }
}
