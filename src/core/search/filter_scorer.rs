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
use crate::core::search::scorable::Scorable;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::Result;
/// # Note
/// See [`JavaIntermediateBaseClass`](crate::migration_notes::JavaIntermediateBaseClass)
#[allow(dead_code)]
pub struct FilterScorer<S> {
    inner: S,
}
impl<S> FilterScorer<S>
where
    S: Scorer,
{
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S> Scorable for FilterScorer<S>
where
    S: Scorer,
{
    fn score(&mut self) -> Result<f32> {
        self.inner.score()
    }
}

impl<S> Scorer for FilterScorer<S>
where
    S: Scorer,
{
    fn doc_id(&mut self) -> Result<i32> {
        self.inner.doc_id()
    }

    fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
        self.inner.iterator()
    }

    fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        self.inner.iterator_mut()
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
        let FilterScorer { inner } = *self;
        Box::new(inner).take_iterator()
    }

    fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
        self.inner.two_phase_iterator()
    }

    fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
        self.inner.two_phase_iterator_mut()
    }

    fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>>
    where
        Self: Sized,
    {
        let FilterScorer { inner } = *self;
        Box::new(inner).take_two_phase_iterator()
    }

    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        self.inner.advance_shallow(target)
    }

    fn default_advance_shallow(&mut self, target: i32) -> Result<i32> {
        self.inner.default_advance_shallow(target)
    }

    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        self.inner.get_max_score(up_to)
    }

    fn default_cost(&mut self) -> Result<i64> {
        self.inner.default_cost()
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        self.inner.has_two_phase_iterator()
    }
}
