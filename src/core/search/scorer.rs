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
use crate::core::search::doc_id_set_iterator::{
    DocIdSetIterator, Either2DocIdSetIterator, Either3DocIdSetIterator, Either4DocIdSetIterator,
    Either5DocIdSetIterator, Either6DocIdSetIterator, Either7DocIdSetIterator,
    Either8DocIdSetIterator,
};
use crate::core::search::scorable::{
    ChildScorable, Either2Scorable, Either3Scorable, Either4Scorable, Either5Scorable,
    Either6Scorable, Either7Scorable, Either8Scorable, Scorable,
};
use crate::core::search::two_phase_iterator::{
    Either2TwoPhaseIterator, Either3TwoPhaseIterator, Either4TwoPhaseIterator,
    Either5TwoPhaseIterator, Either6TwoPhaseIterator, Either7TwoPhaseIterator,
    Either8TwoPhaseIterator, TwoPhaseIterator,
};
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
    type TwoPhaseIterRef<'a>: TwoPhaseIterator<
        DocIdSetIterator = <Self::TwoPhaseIter as TwoPhaseIterator>::DocIdSetIterator,
    >
    where
        Self: 'a;

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

    /// Return a [`DocIdSetIterator`] over matching documents, transferring ownership.
    ///
    /// Unlike [`iterator`](Self::iterator), this method takes ownership of the
    /// underlying iterator rather than returning a view.
    fn take_iterator(&mut self) -> Self::DocIdSetIterator;

    /// Returns term frequency in the current document.
    fn freq(&mut self) -> Result<i32>;

    /// Optional: Return a two-phase iterator view of this scorer.
    ///
    /// A return value of `None` indicates that two-phase iteration is not supported.
    ///
    /// Note that the returned [`TwoPhaseIterator`]'s approximation must advance
    /// synchronously with `iterator()`: advancing the approximation must advance
    /// the iterator and vice-versa.
    ///
    /// The default implementation returns `None`.
    fn two_phase_iterator(&mut self) -> Option<Self::TwoPhaseIterRef<'_>> {
        None
    }

    /// Optional: Return a two-phase iterator for this scorer, transferring ownership.
    ///
    /// By default, this returns `None`.
    fn take_two_phase_iterator(&mut self) -> Option<Self::TwoPhaseIter> {
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

    fn default_cost(&mut self) -> Result<i64> {
        self.iterator().cost()
    }
}

macro_rules! either_scorer {
    (
        $vis:vis $name:ident {
            iter = $iter_ty:ident,
            two_phase = $two_phase_ty:ident,
            scorable = $scorable_ty:ident;
            $( $Variant:ident : $T:ident ),+ $(,)?
        }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> Scorable for $name<$( $T ),+>
        where
            $( $T: Scorer ),+
        {
            #[inline]
            fn score(&mut self) -> Result<f32> {
                match self { $( Self::$Variant(inner) => inner.score(), )+ }
            }

            #[inline]
            fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
                match self { $( Self::$Variant(inner) => inner.smoothing_score(doc_id), )+ }
            }

            #[inline]
            fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
                match self { $( Self::$Variant(inner) => inner.set_min_competitive_score(min_score), )+ }
            }

            type Scorable = $scorable_ty<$( < $T as Scorable >::Scorable ),+>;

            #[inline]
            fn get_children(&self) -> Result<Vec<ChildScorable<Self::Scorable>>> {
                match self {
                    $(
                        Self::$Variant(inner) => {
                            let children = inner.get_children()?;
                            let mapped = children
                                .into_iter()
                                .map(|child| ChildScorable {
                                    child: Self::Scorable::$Variant(child.child),
                                    relationship: child.relationship,
                                })
                                .collect();
                            Ok(mapped)
                        }
                    ),+
                }
            }

            #[inline]
            fn cost(&mut self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => inner.default_cost(), )+ }
            }
        }

        impl<$( $T ),+> Scorer for $name<$( $T ),+>
        where
            $( $T: Scorer ),+
        {
            type DocIdSetIterator =
                $iter_ty<$( < $T as Scorer >::DocIdSetIterator ),+>;

            type DocIdSetIteratorRef<'a> =
                $iter_ty<$( < $T as Scorer >::DocIdSetIteratorRef<'a> ),+>
            where
                Self: 'a;

            type TwoPhaseIter =
                $two_phase_ty<$( < $T as Scorer >::TwoPhaseIter ),+>;
            type TwoPhaseIterRef<'a> =
                $two_phase_ty<$( < $T as Scorer >::TwoPhaseIterRef<'a> ),+>
            where
                Self: 'a;

            #[inline]
            fn doc_id(&mut self) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.doc_id(), )+ }
            }

            #[inline]
            fn iterator(&mut self) -> Self::DocIdSetIteratorRef<'_> {
                match self {
                    $( Self::$Variant(inner) => $iter_ty::$Variant(inner.iterator()), )+
                }
            }

            #[inline]
            fn take_iterator(&mut self) -> Self::DocIdSetIterator {
                match self {
                    $( Self::$Variant(inner) => $iter_ty::$Variant(inner.take_iterator()), )+
                }
            }

            #[inline]
            fn freq(&mut self) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.freq(), )+ }
            }

            #[inline]
            fn two_phase_iterator(&mut self) -> Option<Self::TwoPhaseIterRef<'_>> {
                match self {
                    $( Self::$Variant(inner) =>
                        inner.two_phase_iterator().map(|it| $two_phase_ty::$Variant(it)), )+
                }
            }

            #[inline]
            fn take_two_phase_iterator(&mut self) -> Option<Self::TwoPhaseIter> {
                match self {
                    $( Self::$Variant(inner) =>
                        inner.take_two_phase_iterator().map(|it| $two_phase_ty::$Variant(it)), )+
                }
            }

            #[inline]
            fn advance_shallow(&mut self, target: i32) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.advance_shallow(target), )+ }
            }

            #[inline]
            fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
                match self { $( Self::$Variant(inner) => inner.get_max_score(up_to), )+ }
            }

            #[inline]
            fn default_cost(&mut self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => inner.default_cost(), )+ }
            }
        }
    };
}
either_scorer!(
    pub Either2Scorer {
        iter = Either2DocIdSetIterator,
        two_phase = Either2TwoPhaseIterator,
        scorable = Either2Scorable;
        A: A, B: B,
    }
);
either_scorer!(
    pub Either3Scorer {
        iter = Either3DocIdSetIterator,
        two_phase = Either3TwoPhaseIterator,
        scorable = Either3Scorable;
        A: A, B: B,C: C,
    }
);
either_scorer!(
    pub Either4Scorer {
        iter = Either4DocIdSetIterator,
        two_phase = Either4TwoPhaseIterator,
        scorable = Either4Scorable;
        A: A, B: B,C: C,D:D
    }
);
either_scorer!(
    pub Either5Scorer {
        iter = Either5DocIdSetIterator,
        two_phase = Either5TwoPhaseIterator,
        scorable = Either5Scorable;
        A: A, B: B,C: C, D: D,E: E,
    }
);
either_scorer!(
    pub Either6Scorer {
        iter = Either6DocIdSetIterator,
        two_phase = Either6TwoPhaseIterator,
        scorable = Either6Scorable;
        A: A, B: B,C: C, D: D,E: E,F: F,
    }
);
either_scorer!(
    pub Either7Scorer {
        iter = Either7DocIdSetIterator,
        two_phase = Either7TwoPhaseIterator,
        scorable = Either7Scorable;
        A: A, B: B,C: C, D: D,E: E,F: F,G: G,
    }
);
either_scorer!(
    pub Either8Scorer {
        iter = Either8DocIdSetIterator,
        two_phase = Either8TwoPhaseIterator,
        scorable = Either8Scorable;
        A: A, B: B,C: C, D: D,E: E,F: F,G: G,H: H,
    }
);
