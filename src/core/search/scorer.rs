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
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::scorable::{ChildScorable, FixedScore, Scorable};
#[cfg(test)]
use crate::core::search::scorer::ScorerKind::Other;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::Result;

/// Expert: Common scoring functionality for different types of queries.
///
/// A [`Scorer`] is a [`DocIdSetIterator`] over fully matching documents, in
/// increasing order of doc ID. Its two-phase approximation shares the same cursor.
/// Advancing and scoring borrow the same owner for one operation at a time.
pub trait Scorer: Scorable + DocIdSetIterator {
  /// Returns the document currently being scored, preserving the scorer-specific view.
  /// This is distinct from the full matching iterator cursor.
  fn scoring_doc_id(&mut self) -> Result<i32>;

  /// Return a [`DocIdSetIterator`] over matching documents, transferring ownership.
  ///
  /// This method takes ownership of the
  /// underlying iterator rather than returning a view.
  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator>;

  /// Transfer an owned scorer into its iterator without requiring callers to box it.
  ///
  /// Boxed scorers override this method to transfer their existing allocation.
  /// The default preserves the consuming implementation of concrete scorers.
  fn into_iterator(self) -> Box<dyn DocIdSetIterator>
  where
    Self: Sized,
  {
    Box::new(self).take_iterator()
  }

  /// Optional: Return a two-phase iterator view of this scorer.
  ///
  /// A return value of `None` indicates that two-phase iteration is not supported.
  ///
  /// Note that the returned [`TwoPhaseIterator`]'s approximation must advance
  /// synchronously with this scorer: advancing the approximation must advance
  /// the iterator and vice-versa.
  ///
  /// The default implementation returns `None`.
  /// # Warning
  /// The returned iterator is a *view*: calling this method several times must
  /// return iterators that share the same state.
  fn two_phase_iterator(&self) -> Option<&dyn TwoPhaseIterator> {
    None
  }

  /// Optional: Return a two-phase iterator view of this scorer.
  ///
  /// A return value of `None` indicates that two-phase iteration is not supported.
  ///
  /// Note that the returned [`TwoPhaseIterator`]'s approximation must advance
  /// synchronously with this scorer: advancing the approximation must advance
  /// the iterator and vice-versa.
  ///
  /// The default implementation returns `None`.
  /// # Warning
  /// The returned iterator is a *view*: calling this method several times must
  /// return iterators that share the same state.
  fn two_phase_iterator_mut(&mut self) -> Option<&mut dyn TwoPhaseIterator> {
    None
  }

  /// Optional: Return a two-phase iterator for this scorer, transferring ownership.
  ///
  /// By default, this returns `None`.
  fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
    None
  }

  /// Transfer an owned scorer into its optional two-phase iterator.
  ///
  /// Like [`Self::take_two_phase_iterator`], this consumes the scorer even when
  /// it returns `None`. Boxed scorers reuse their existing allocation.
  fn into_two_phase_iterator(self) -> Option<Box<dyn TwoPhaseIterator>>
  where
    Self: Sized,
  {
    Box::new(self).take_two_phase_iterator()
  }

  /// Advance to the block of documents that contains `target` in order to get
  /// scoring information about this block.
  ///
  /// This method is implicitly called by [`DocIdSetIterator::advance`] and
  /// [`DocIdSetIterator::next_doc`] on the returned doc ID. Calling this method
  /// doesn't modify the current `doc_id()`. It returns a number that is greater
  /// than or equal to all documents contained in the current block, but less than
  /// any doc IDs of the next block. `target` must be `>= doc_id()` as well as all
  /// targets that have been passed to `advance_shallow` so far.
  ///
  /// The default implementation returns [`NO_MORE_DOCS`].
  fn advance_shallow(&mut self, _target: i32) -> Result<i32> {
    Ok(NO_MORE_DOCS)
  }
  fn default_advance_shallow(&mut self, _target: i32) -> Result<i32> {
    Ok(NO_MORE_DOCS)
  }

  /// Return the maximum score that documents between the last `target` that this
  /// iterator was `advance_shallow`’d to (included) and `upto` (included) can get.
  fn get_max_score(&mut self, upto: i32) -> Result<f32>;

  fn default_cost(&mut self) -> Result<i64> {
    DocIdSetIterator::cost(self)
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState;

  /// Return an approximation [`DocIdSetIterator`] for this scorer.
  ///
  /// If this scorer supports two-phase iteration (i.e. [`two_phase_iterator`](TwoPhaseIterator)
  /// returns `Some`), then this method must return the approximation of the
  /// two-phase iterator.
  ///
  /// Otherwise, this method must return the same iterator as this scorer.
  ///
  /// # Warning
  /// The returned iterator is a *view*: calling this method several times must
  /// return iterators that share the same state.
  fn approximation(&self) -> &dyn DocIdSetIterator;

  /// Return a mutable approximation [`DocIdSetIterator`] for this scorer.
  ///
  /// If this scorer supports two-phase iteration (i.e. [`two_phase_iterator_mut`](TwoPhaseIterator)
  /// returns `Some`), then this method must return the mutable approximation of the
  /// two-phase iterator.
  ///
  /// Otherwise, this method must return the same iterator as this scorer.
  ///
  /// # Warning
  /// The returned iterator is a *view*: calling this method several times must
  /// return iterators that share the same state.
  fn approximation_mut(&mut self) -> &mut dyn DocIdSetIterator;

  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    Other
  }
}
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScorerKind {
  Conjunction,
  Disjunction,
  ReqOptSum,
  ReqExcl,
  Boolean,
  ConstantScore,
  Phrase,
  Other,
}

impl<T> Scorable for Box<T>
where
  T: Scorable + ?Sized,
{
  fn score(&mut self) -> Result<f32> {
    (**self).score()
  }

  fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
    (**self).smoothing_score(doc_id)
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    (**self).set_min_competitive_score(min_score)
  }

  fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
    (**self).get_children()
  }

  fn cost(&self) -> Result<i64> {
    (**self).cost()
  }
  #[cfg(test)]
  fn as_any(&mut self) -> &mut dyn std::any::Any {
    (**self).as_any()
  }
}

impl<T> FixedScore for Box<T>
where
  T: FixedScore + ?Sized,
{
  fn set_score(&mut self, score: f32) -> Result<()> {
    (**self).set_score(score)
  }
}

impl<T> Scorer for Box<T>
where
  T: Scorer,
{
  fn scoring_doc_id(&mut self) -> Result<i32> {
    (**self).scoring_doc_id()
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let inner: Box<T> = *self;
    Scorer::take_iterator(inner)
  }

  fn into_iterator(self) -> Box<dyn DocIdSetIterator> {
    Scorer::take_iterator(self)
  }

  fn two_phase_iterator(&self) -> Option<&dyn TwoPhaseIterator> {
    (**self).two_phase_iterator()
  }

  fn two_phase_iterator_mut(&mut self) -> Option<&mut dyn TwoPhaseIterator> {
    (**self).two_phase_iterator_mut()
  }

  fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
    let inner: Box<T> = *self;
    Scorer::take_two_phase_iterator(inner)
  }

  fn into_two_phase_iterator(self) -> Option<Box<dyn TwoPhaseIterator>> {
    Scorer::take_two_phase_iterator(self)
  }

  fn advance_shallow(&mut self, _target: i32) -> Result<i32> {
    (**self).advance_shallow(_target)
  }

  fn default_advance_shallow(&mut self, _target: i32) -> Result<i32> {
    (**self).default_advance_shallow(_target)
  }

  fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
    (**self).get_max_score(_up_to)
  }

  fn default_cost(&mut self) -> Result<i64> {
    (**self).default_cost()
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    (**self).has_two_phase_iterator()
  }

  fn approximation(&self) -> &dyn DocIdSetIterator {
    (**self).approximation()
  }

  fn approximation_mut(&mut self) -> &mut dyn DocIdSetIterator {
    (**self).approximation_mut()
  }
  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    (**self).kind()
  }
}

impl Scorer for Box<dyn Scorer> {
  fn scoring_doc_id(&mut self) -> Result<i32> {
    (**self).scoring_doc_id()
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let inner: Box<dyn Scorer> = *self;
    Scorer::take_iterator(inner)
  }

  fn into_iterator(self) -> Box<dyn DocIdSetIterator> {
    Scorer::take_iterator(self)
  }

  fn two_phase_iterator(&self) -> Option<&dyn TwoPhaseIterator> {
    (**self).two_phase_iterator()
  }

  fn two_phase_iterator_mut(&mut self) -> Option<&mut dyn TwoPhaseIterator> {
    (**self).two_phase_iterator_mut()
  }

  fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
    let inner: Box<dyn Scorer> = *self;
    Scorer::take_two_phase_iterator(inner)
  }

  fn into_two_phase_iterator(self) -> Option<Box<dyn TwoPhaseIterator>> {
    Scorer::take_two_phase_iterator(self)
  }

  fn advance_shallow(&mut self, _target: i32) -> Result<i32> {
    (**self).advance_shallow(_target)
  }

  fn default_advance_shallow(&mut self, _target: i32) -> Result<i32> {
    (**self).default_advance_shallow(_target)
  }

  fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
    (**self).get_max_score(_up_to)
  }

  fn default_cost(&mut self) -> Result<i64> {
    (**self).default_cost()
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    (**self).has_two_phase_iterator()
  }

  fn approximation(&self) -> &dyn DocIdSetIterator {
    (**self).approximation()
  }

  fn approximation_mut(&mut self) -> &mut dyn DocIdSetIterator {
    (**self).approximation_mut()
  }
  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    (**self).kind()
  }
}
pub type ScorerDisi = Box<dyn DocIdSetIterator>;
pub type ScorerDisiMut<'a> = &'a mut dyn DocIdSetIterator;
pub type ScorerDisiRef<'a> = &'a dyn DocIdSetIterator;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum TwoPhaseState {
  /// Has two_phase_iterator
  Yes,
  /// no two_phase_iterator
  No,
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
            $( $T: Scorable ),+
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


            #[inline]
            fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
                match self {
                    $( Self::$Variant(inner) => inner.get_children(), )+
                }
            }

            #[inline]
            fn cost(&self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => inner.cost(), )+ }
            }
            #[cfg(test)]
            fn as_any(&mut self) -> &mut dyn std::any::Any {
                match self { $( Self::$Variant(inner) => inner.as_any(), )+ }
            }
        }

        impl<$( $T ),+> FixedScore for $name<$( $T ),+>
        where
            $( $T: FixedScore ),+
        {
            #[inline]
            fn set_score(&mut self, score: f32) -> Result<()> {
                match self { $( Self::$Variant(inner) => inner.set_score(score), )+ }
            }
        }

        impl<$( $T ),+> DocIdSetIterator for $name<$( $T ),+>
        where
            $( $T: Scorer ),+
        {
            fn doc_id(&self) -> i32 {
                match self { $( Self::$Variant(inner) => DocIdSetIterator::doc_id(inner), )+ }
            }

            fn next_doc(&mut self) -> Result<i32> {
                match self { $( Self::$Variant(inner) => DocIdSetIterator::next_doc(inner), )+ }
            }

            fn advance(&mut self, target: i32) -> Result<i32> {
                match self { $( Self::$Variant(inner) => DocIdSetIterator::advance(inner, target), )+ }
            }

            fn slow_advance(&mut self, target: i32) -> Result<i32> {
                match self { $( Self::$Variant(inner) => DocIdSetIterator::slow_advance(inner, target), )+ }
            }

            fn cost(&self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => DocIdSetIterator::cost(inner), )+ }
            }

        }

        impl<$( $T ),+> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for $name<$( $T ),+>
        where
            $( $T: Scorer ),+
        {
            fn get_fixed_bit_set(&self) -> Option<&crate::core::util::fixed_bit_set::FixedBitSet> {
                match self { $( Self::$Variant(inner) => crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions::get_fixed_bit_set(inner), )+ }
            }

            fn get_sparse_fixed_bit_set(&self) -> Option<&crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet> {
                match self { $( Self::$Variant(inner) => crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions::get_sparse_fixed_bit_set(inner), )+ }
            }

            fn get_doc_base_fixed_bit_set(&self) -> Option<(usize, &crate::core::util::fixed_bit_set::FixedBitSet)> {
                match self { $( Self::$Variant(inner) => crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions::get_doc_base_fixed_bit_set(inner), )+ }
            }

        }

        impl<$( $T ),+> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for $name<$( $T ),+>
        where
            $( $T: Scorer ),+
        {
            fn is_bit_iter(&self) -> bool {
                match self { $( Self::$Variant(inner) => crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::is_bit_iter(inner), )+ }
            }

            fn get(&self, index: usize) -> Result<bool> {
                match self { $( Self::$Variant(inner) => crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::get(inner, index), )+ }
            }

            fn set_doc_id(&mut self, doc: i32) -> Result<()> {
                match self { $( Self::$Variant(inner) => crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::set_doc_id(inner, doc), )+ }
            }

            fn bit_set_length(&self) -> Result<usize> {
                match self { $( Self::$Variant(inner) => crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::bit_set_length(inner), )+ }
            }

        }

        impl<$( $T ),+> Scorer for $name<$( $T ),+>
        where
            $( $T: Scorer ),+
        {
            fn scoring_doc_id(&mut self) -> Result<i32> {
                match self { $( Self::$Variant(inner) => inner.scoring_doc_id(), )+ }
            }

            #[inline]
            fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
                (*self).into_iterator()
            }

            fn into_iterator(self) -> Box<dyn DocIdSetIterator> {
                match self {
                    $( Self::$Variant(inner) => inner.into_iterator(), )+
                }
            }

            #[inline]
            fn two_phase_iterator(&self) -> Option<&dyn TwoPhaseIterator> {
                match self { $( Self::$Variant(inner) => inner.two_phase_iterator(), )+ }
            }
            #[inline]
            fn two_phase_iterator_mut(&mut self) -> Option<&mut dyn TwoPhaseIterator> {
                match self { $( Self::$Variant(inner) => inner.two_phase_iterator_mut(), )+ }
            }


            #[inline]
            fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
                (*self).into_two_phase_iterator()
            }

            fn into_two_phase_iterator(self) -> Option<Box<dyn TwoPhaseIterator>> {
                match self {
                    $( Self::$Variant(inner) => inner.into_two_phase_iterator(), )+
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
            fn get_max_score(&mut self, upto: i32) -> Result<f32> {
                match self { $( Self::$Variant(inner) => inner.get_max_score(upto), )+ }
            }

            #[inline]
            fn default_cost(&mut self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => inner.default_cost(), )+ }
            }
             #[inline]
            fn has_two_phase_iterator(&self) -> TwoPhaseState{
                match self { $( Self::$Variant(inner) => inner.has_two_phase_iterator(), )+ }
            }
           #[inline]
            fn approximation(&self) -> &dyn DocIdSetIterator {
                match self { $( Self::$Variant(inner) => inner.approximation(), )+ }
            }
            #[inline]
            fn approximation_mut(&mut self) -> &mut dyn DocIdSetIterator {
                match self { $( Self::$Variant(inner) => inner.approximation_mut(), )+ }
            }
            #[cfg(test)]
            fn kind(&self) -> ScorerKind{
                match self { $( Self::$Variant(inner) => inner.kind(), )+ }
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
        iter = DocIdSetIteratorEnum3,
        two_phase = TwoPhaseIteratorEnum3,
        scorable = ScorableEnum3;
        A: A, B: B,C: C, D:D,
    }
);
