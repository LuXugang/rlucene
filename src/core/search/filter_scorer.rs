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
/// A `FilterScorer` contains another [`Scorer`], which it uses as its basic source of
/// data, possibly transforming the data along the way or providing additional functionality.
///
/// The `FilterScorer` itself simply implements all abstract methods of [`Scorer`] with
/// versions that forward all calls to the wrapped scorer.
///
/// Subclasses of `FilterScorer` may further override some of these methods and may also
/// provide additional methods and fields.
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
    type DocIdSetIteratorRef<'a>
        = S::DocIdSetIteratorRef<'a>
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = S::DocIdSetIteratorMut<'a>
    where
        Self: 'a;
    type TwoPhaseIterRef<'a>
        = S::TwoPhaseIterRef<'a>
    where
        Self: 'a;
    type TwoPhaseIterMut<'a>
        = S::TwoPhaseIterMut<'a>
    where
        Self: 'a;

    fn doc_id(&mut self) -> Result<i32> {
        self.inner.doc_id()
    }

    fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
        self.inner.iterator()
    }

    fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
        self.inner.iterator_mut()
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
        let FilterScorer { inner } = *self;
        Box::new(inner).take_iterator()
    }

    fn two_phase_iterator(&self) -> Result<Option<Self::TwoPhaseIterRef<'_>>> {
        self.inner.two_phase_iterator()
    }

    fn two_phase_iterator_mut(&mut self) -> Result<Option<Self::TwoPhaseIterMut<'_>>> {
        self.inner.two_phase_iterator_mut()
    }

    fn take_two_phase_iterator(self: Box<Self>) -> Result<Option<Box<dyn TwoPhaseIterator>>>
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
