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
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, Either2DocIdSetIterator};
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::Result;

/// Expert: Common scoring functionality for different types of queries.
///
/// A `Scorer` exposes an `iterator()` over documents matching a query in
/// increasing order of doc id.
pub trait Scorer: Scorable {
    /// Concrete iterator type over matching documents.
    type DocIdSetIterator: DocIdSetIterator;
    type DocIdSetIteratorRef<'a>: DocIdSetIterator
    where
        Self: 'a;

    /// Optional two-phase iterator type (return `None` if unsupported).
    type TwoPhaseIter: TwoPhaseIterator;

    /// Returns the doc ID that is currently being scored.
    fn doc_id(&mut self) -> Result<i32>;

    /// Return a [`DocIdSetIterator`] over matching documents.
    ///
    /// The returned iterator will either be positioned on `-1` if no documents
    /// have been scored yet, `NO_MORE_DOCS` if all documents have been scored already,
    /// or the last document id that has been scored otherwise.
    ///
    /// The returned iterator is a *view*: calling this method several times must
    /// return iterators that share the same state.
    fn iterator(&mut self) -> Self::DocIdSetIteratorRef<'_>;
    // fn iterator_take(&mut self) -> Self::DocIdSetIterator;

    /// Optional: Return a two-phase iterator view of this scorer.
    ///
    /// A return value of `None` indicates that two-phase iteration is not supported.
    ///
    /// Note that the returned [`TwoPhaseIterator`]'s approximation must advance
    /// synchronously with `iterator()`: advancing the approximation must advance
    /// the iterator and vice-versa.
    ///
    /// The default implementation returns `None`.
    fn two_phase_iterator(&mut self) -> Option<&mut Self::TwoPhaseIter> {
        None
    }

    /// Advance to the block of documents that contains `target` in order to get
    /// scoring information about this block.
    ///
    /// This method is implicitly called by `DocIdSetIterator::advance` and
    /// `DocIdSetIterator::next_doc` on the returned doc ID. Calling this method
    /// doesn't modify the current `doc_id()`. It returns a number that is greater
    /// than or equal to all documents contained in the current block, but less than
    /// any doc IDs of the next block. `target` must be `>= doc_id()` as well as all
    /// targets that have been passed to `advance_shallow` so far.
    ///
    /// The default implementation returns `NO_MORE_DOCS`.
    fn advance_shallow(&mut self, _target: i32) -> Result<i32> {
        Ok(NO_MORE_DOCS)
    }

    /// Return the maximum score that documents between the last `target` that this
    /// iterator was `advance_shallow`’d to (included) and `up_to` (included) can get.
    fn get_max_score(&mut self, up_to: i32) -> Result<f32>;
}

fn map_child_scorables<From, To, F>(
    children: Vec<ChildScorable<From>>,
    mapper: F,
) -> Vec<ChildScorable<To>>
where
    From: Scorable,
    To: Scorable,
    F: Fn(From) -> To,
{
    children
        .into_iter()
        .map(
            |ChildScorable {
                 child,
                 relationship,
             }| { ChildScorable::new(mapper(child), relationship) },
        )
        .collect()
}

pub enum Either2ScorerChild<A, B> {
    A(A),
    B(B),
}

impl<A, B> Scorable for Either2ScorerChild<A, B>
where
    A: Scorable,
    B: Scorable,
{
    fn score(&mut self) -> Result<f32> {
        match self {
            Self::A(inner) => inner.score(),
            Self::B(inner) => inner.score(),
        }
    }

    fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
        match self {
            Self::A(inner) => inner.smoothing_score(doc_id),
            Self::B(inner) => inner.smoothing_score(doc_id),
        }
    }

    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        match self {
            Self::A(inner) => inner.set_min_competitive_score(min_score),
            Self::B(inner) => inner.set_min_competitive_score(min_score),
        }
    }

    type Scorable = Either2ScorerChild<A::Scorable, B::Scorable>;

    fn get_children(&self) -> Result<Vec<ChildScorable<Self::Scorable>>> {
        match self {
            Self::A(inner) => inner
                .get_children()
                .map(|children| map_child_scorables(children, Either2ScorerChild::A)),
            Self::B(inner) => inner
                .get_children()
                .map(|children| map_child_scorables(children, Either2ScorerChild::B)),
        }
    }
}

pub enum Either2Scorer<A, B> {
    A(A),
    B(B),
}

impl<A, B> Scorable for Either2Scorer<A, B>
where
    A: Scorer,
    B: Scorer<TwoPhaseIter = A::TwoPhaseIter>,
{
    fn score(&mut self) -> Result<f32> {
        match self {
            Self::A(inner) => inner.score(),
            Self::B(inner) => inner.score(),
        }
    }

    fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
        match self {
            Self::A(inner) => inner.smoothing_score(doc_id),
            Self::B(inner) => inner.smoothing_score(doc_id),
        }
    }

    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        match self {
            Self::A(inner) => inner.set_min_competitive_score(min_score),
            Self::B(inner) => inner.set_min_competitive_score(min_score),
        }
    }

    type Scorable = Either2ScorerChild<A::Scorable, B::Scorable>;

    fn get_children(&self) -> Result<Vec<ChildScorable<Self::Scorable>>> {
        match self {
            Self::A(inner) => inner
                .get_children()
                .map(|children| map_child_scorables(children, Either2ScorerChild::A)),
            Self::B(inner) => inner
                .get_children()
                .map(|children| map_child_scorables(children, Either2ScorerChild::B)),
        }
    }
}

impl<A, B> Scorer for Either2Scorer<A, B>
where
    A: Scorer,
    B: Scorer<TwoPhaseIter = A::TwoPhaseIter>,
{
    type DocIdSetIterator = Either2DocIdSetIterator<A::DocIdSetIterator, B::DocIdSetIterator>;
    type DocIdSetIteratorRef<'a>
        = Either2DocIdSetIterator<A::DocIdSetIteratorRef<'a>, B::DocIdSetIteratorRef<'a>>
    where
        Self: 'a;

    type TwoPhaseIter = A::TwoPhaseIter;

    fn doc_id(&mut self) -> Result<i32> {
        match self {
            Self::A(inner) => inner.doc_id(),
            Self::B(inner) => inner.doc_id(),
        }
    }

    fn iterator(&mut self) -> Self::DocIdSetIteratorRef<'_> {
        match self {
            Self::A(inner) => Either2DocIdSetIterator::A(inner.iterator()),
            Self::B(inner) => Either2DocIdSetIterator::B(inner.iterator()),
        }
    }

    fn two_phase_iterator(&mut self) -> Option<&mut Self::TwoPhaseIter> {
        match self {
            Self::A(inner) => inner.two_phase_iterator(),
            Self::B(inner) => inner.two_phase_iterator(),
        }
    }

    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        match self {
            Self::A(inner) => inner.advance_shallow(target),
            Self::B(inner) => inner.advance_shallow(target),
        }
    }

    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        match self {
            Self::A(inner) => inner.get_max_score(up_to),
            Self::B(inner) => inner.get_max_score(up_to),
        }
    }
}
