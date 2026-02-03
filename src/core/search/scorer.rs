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
    DocIdSetIterator, DocIdSetIteratorEnum2, DocIdSetIteratorEnum3, DocIdSetIteratorEnum4,
    DocIdSetIteratorEnum5, DocIdSetIteratorEnum6,
};
use crate::core::search::scorable::{
    ChildScorable, Scorable, ScorableEnum2, ScorableEnum3, ScorableEnum4, ScorableEnum5,
    ScorableEnum6,
};
use crate::core::search::two_phase_iterator::{
    TwoPhaseIterator, TwoPhaseIteratorEnum2, TwoPhaseIteratorEnum3, TwoPhaseIteratorEnum4,
    TwoPhaseIteratorEnum5, TwoPhaseIteratorEnum6,
};
use crate::core::util::error::lucene_error::Result;

/// Expert: Common scoring functionality for different types of queries.
///
/// A `Scorer` exposes an `iterator_mut()` over documents matching a query in
/// increasing order of doc id.
pub trait Scorer: Scorable {
    type DocIdSetIteratorRef<'a>: DocIdSetIterator
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>: DocIdSetIterator
    where
        Self: 'a;

    type TwoPhaseIterRef<'a>: TwoPhaseIterator
    where
        Self: 'a;
    type TwoPhaseIterMut<'a>: TwoPhaseIterator
    where
        Self: 'a;
    /// Returns the doc ID that is currently being scored.
    fn doc_id(&mut self) -> Result<i32>;

    /// Return a [`DocIdSetIterator`] over matching documents.
    ///
    /// The returned iterator will either be positioned on `-1` if no documents
    /// have been scored yet, `NO_MORE_DOCS` if all documents have been scored already,
    /// or the last document id that has been scored otherwise.
    /// # Warning
    /// The returned iterator is a *view*: calling this method several times must
    /// return iterators that share the same state.
    fn iterator(&self) -> Self::DocIdSetIteratorRef<'_>;

    fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_>;

    /// Return a [`DocIdSetIterator`] over matching documents, transferring ownership.
    ///
    /// Unlike [`iterator`](Self::iterator), this method takes ownership of the
    /// underlying iterator rather than returning a view.
    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator>;

    /// Optional: Return a two-phase iterator view of this scorer.
    ///
    /// A return value of `None` indicates that two-phase iteration is not supported.
    ///
    /// Note that the returned [`TwoPhaseIterator`]'s approximation must advance
    /// synchronously with `iterator()`: advancing the approximation must advance
    /// the iterator and vice-versa.
    ///
    /// The default implementation returns `None`.
    /// # Warning
    /// The returned iterator is a *view*: calling this method several times must
    /// return iterators that share the same state.
    fn two_phase_iterator(&self) -> Result<Option<Self::TwoPhaseIterRef<'_>>> {
        Ok(None)
    }

    /// Optional: Return a two-phase iterator view of this scorer.
    ///
    /// A return value of `None` indicates that two-phase iteration is not supported.
    ///
    /// Note that the returned [`TwoPhaseIterator`]'s approximation must advance
    /// synchronously with `iterator()`: advancing the approximation must advance
    /// the iterator and vice-versa.
    ///
    /// The default implementation returns `None`.
    /// # Warning
    /// The returned iterator is a *view*: calling this method several times must
    /// return iterators that share the same state.
    fn two_phase_iterator_mut(&mut self) -> Result<Option<Self::TwoPhaseIterMut<'_>>> {
        Ok(None)
    }

    /// Optional: Return a two-phase iterator for this scorer, transferring ownership.
    ///
    /// By default, this returns `None`.
    fn take_two_phase_iterator(self: Box<Self>) -> Result<Option<Box<dyn TwoPhaseIterator>>>
    where
        Self: Sized,
    {
        Ok(None)
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
    fn default_advance_shallow(&mut self, _target: i32) -> Result<i32> {
        Ok(NO_MORE_DOCS)
    }

    /// Return the maximum score that documents between the last `target` that this
    /// iterator was `advance_shallow`’d to (included) and `up_to` (included) can get.
    fn get_max_score(&mut self, up_to: i32) -> Result<f32>;

    fn default_cost(&mut self) -> Result<i64> {
        self.iterator_mut().cost()
    }
    fn has_two_phase_iterator(&self) -> TwoPhaseState;
}

impl<T> Scorable for Box<T>
where
    T: Scorer + ?Sized,
{
    fn score(&mut self) -> Result<f32> {
        (**self).score()
    }

    type Scorable = T::Scorable;
}

impl<T> Scorer for Box<T>
where
    T: Scorer + ?Sized,
{
    type DocIdSetIteratorRef<'a>
        = T::DocIdSetIteratorRef<'a>
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = T::DocIdSetIteratorMut<'a>
    where
        Self: 'a;
    type TwoPhaseIterRef<'a>
        = T::TwoPhaseIterRef<'a>
    where
        Self: 'a;
    type TwoPhaseIterMut<'a>
        = T::TwoPhaseIterMut<'a>
    where
        Self: 'a;

    fn doc_id(&mut self) -> Result<i32> {
        todo!()
    }

    fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
        todo!()
    }

    fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
        todo!()
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
        todo!()
    }

    fn two_phase_iterator(&self) -> Result<Option<Self::TwoPhaseIterRef<'_>>> {
        todo!()
    }

    fn two_phase_iterator_mut(&mut self) -> Result<Option<Self::TwoPhaseIterMut<'_>>> {
        todo!()
    }

    fn take_two_phase_iterator(self: Box<Self>) -> Result<Option<Box<dyn TwoPhaseIterator>>>
    where
        Self: Sized,
    {
        todo!()
    }

    fn advance_shallow(&mut self, _target: i32) -> Result<i32> {
        todo!()
    }

    fn default_advance_shallow(&mut self, _target: i32) -> Result<i32> {
        todo!()
    }

    fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
        todo!()
    }

    fn default_cost(&mut self) -> Result<i64> {
        todo!()
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        todo!()
    }
}
pub type ScorerDisi = Box<dyn DocIdSetIterator>;
pub type ScorerDisiMut<'a, S> = <S as Scorer>::DocIdSetIteratorMut<'a>;
pub type ScorerDisiRef<'a, S> = <S as Scorer>::DocIdSetIteratorRef<'a>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum TwoPhaseState {
    /// Has two_phase_iterator
    Yes,
    /// no two_phase_iterator
    No,
    /// may or may not present, check with [`Scorer::two_phase_iterator`]
    MayBe,
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
            type DocIdSetIteratorRef<'a> =
                $iter_ty<$( < $T as Scorer >::DocIdSetIteratorRef<'a> ),+>
            where
                Self: 'a;

            type DocIdSetIteratorMut<'a> =
                $iter_ty<$( < $T as Scorer >::DocIdSetIteratorMut<'a> ),+>
            where
                Self: 'a;

            type TwoPhaseIterRef<'a> =
                $two_phase_ty<$( < $T as Scorer >::TwoPhaseIterRef<'a> ),+>
            where
                Self: 'a;
            type TwoPhaseIterMut<'a> =
                $two_phase_ty<$( < $T as Scorer >::TwoPhaseIterMut<'a> ),+>
            where
                Self: 'a;

            #[inline]
            fn doc_id(&mut self) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.doc_id(), )+ }
            }

            #[inline]
            fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
                match self {
                    $( Self::$Variant(inner) => $iter_ty::$Variant(inner.iterator()), )+
                }
            }

            #[inline]
            fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
                match self {
                    $( Self::$Variant(inner) => $iter_ty::$Variant(inner.iterator_mut()), )+
                }
            }

            #[inline]
            fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
                match *self {
                    $( Self::$Variant(inner) => Box::new(inner).take_iterator(), )+
                }
            }

            #[inline]
            fn two_phase_iterator(&self) -> Result<Option<Self::TwoPhaseIterRef<'_>>> {
                match self {
                    $( Self::$Variant(inner) =>
                        inner.two_phase_iterator().map(|res| res.map(|it| $two_phase_ty::$Variant(it))), )+
                }
            }

            #[inline]
            fn two_phase_iterator_mut(&mut self) -> Result<Option<Self::TwoPhaseIterMut<'_>>> {
                match self {
                    $( Self::$Variant(inner) =>
                        inner
                            .two_phase_iterator_mut()
                            .map(|res| res.map(|it| $two_phase_ty::$Variant(it))), )+
                }
            }

            #[inline]
            fn take_two_phase_iterator(self: Box<Self>) -> Result<Option<Box<dyn TwoPhaseIterator>>> {
                match *self {
                    $( Self::$Variant(inner) => Box::new(inner).take_two_phase_iterator(), )+
                }
            }

            #[inline]
            fn advance_shallow(&mut self, target: i32) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.advance_shallow(target), )+ }
            }
            #[inline]
            fn default_advance_shallow(&mut self, target: i32) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.default_advance_shallow(target), )+ }
            }

            #[inline]
            fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
                match self { $( Self::$Variant(inner) => inner.get_max_score(up_to), )+ }
            }

            #[inline]
            fn default_cost(&mut self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => inner.default_cost(), )+ }
            }
             #[inline]
            fn has_two_phase_iterator(&self) -> TwoPhaseState{
                match self { $( Self::$Variant(inner) => inner.has_two_phase_iterator(), )+ }
            }
        }
    };
}
either_scorer!(
    pub ScorerEnum2 {
        iter = DocIdSetIteratorEnum2,
        two_phase = TwoPhaseIteratorEnum2,
        scorable = ScorableEnum2;
        A: A, B: B,
    }
);
either_scorer!(
    pub ScorerEnum3 {
        iter = DocIdSetIteratorEnum3,
        two_phase = TwoPhaseIteratorEnum3,
        scorable = ScorableEnum3;
        A: A, B: B,C: C,
    }
);
either_scorer!(
    pub ScorerEnum4 {
        iter = DocIdSetIteratorEnum4,
        two_phase = TwoPhaseIteratorEnum4,
        scorable = ScorableEnum4;
        A: A, B: B,C: C,D:D
    }
);
either_scorer!(
    pub ScorerEnum5 {
        iter = DocIdSetIteratorEnum5,
        two_phase = TwoPhaseIteratorEnum5,
        scorable = ScorableEnum5;
        A: A, B: B,C: C, D: D,E: E,
    }
);
either_scorer!(
    pub ScorerEnum6 {
        iter = DocIdSetIteratorEnum6,
        two_phase = TwoPhaseIteratorEnum6,
        scorable = ScorableEnum6;
        A: A, B: B,C: C, D: D,E: E,F: F,
    }
);
