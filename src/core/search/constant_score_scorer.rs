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

use crate::core::search::approximation_disi::ApproximationBitSetAccess;
use crate::core::search::approximation_disi::ApproximationDISI;
use crate::core::search::doc_id_set_iterator::{
  BitSetIteratorAccess, DocIdSetIterator, DocIdSetIteratorExtensions, EmptyDISI,
};
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::score_mode::ScoreMode;
#[cfg(test)]
use crate::core::search::scorer::ScorerKind;
use crate::core::search::scorer::{Scorer, ScorerEnum2, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
  TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// A constant-scoring scorer whose iteration mode is fixed by its state type.
pub struct ConstantScoreScorer<S: DocIdSetIterator> {
  score: f32,
  state: S,
}

pub type ConstantScoreDisiScorer<D> =
  ScorerEnum2<ConstantScoreScorer<DisiState<D>>, ConstantScoreScorer<TopDisiState<D>>>;
pub type ConstantScoreTwoPhaseScorer<T> =
  ScorerEnum2<ConstantScoreScorer<TpiState<T>>, ConstantScoreScorer<TopTpiState<T>>>;

impl<D: DocIdSetIterator> ConstantScoreScorer<DisiState<D>> {
  /// Creates an ordinary iterator scorer without competitive-score pruning.
  pub(crate) fn from_plain_disi(score: f32, disi: D) -> Self {
    Self {
      score,
      state: DisiState(disi),
    }
  }

  /// Selects the ordinary or competitive iterator mode at construction.
  pub fn from_disi(score: f32, score_mode: ScoreMode, disi: D) -> ConstantScoreDisiScorer<D> {
    match score_mode {
      ScoreMode::TopScores => ScorerEnum2::B(ConstantScoreScorer {
        score,
        state: TopDisiState(DocIdSetIteratorWrapper::new(DisiDelegate::Disi(disi))),
      }),
      _ => ScorerEnum2::A(Self::from_plain_disi(score, disi)),
    }
  }
}
impl<T: TwoPhaseIterator> ConstantScoreScorer<TpiState<T>> {
  /// Selects the ordinary or competitive two-phase mode at construction.
  pub fn from_tpi(
    score: f32,
    score_mode: ScoreMode,
    iterator: T,
  ) -> ConstantScoreTwoPhaseScorer<T> {
    match score_mode {
      ScoreMode::TopScores => ScorerEnum2::B(ConstantScoreScorer {
        score,
        state: TopTpiState(TwoPhaseIteratorAsDocIdSetIterator::new(
          TwoPhaseIteratorImpl::new(DocIdSetIteratorWrapper::new(TwoPhaseDelegate {
            two_phase_iterator: iterator,
            empty: None,
          })),
        )),
      }),
      _ => ScorerEnum2::A(ConstantScoreScorer {
        score,
        state: TpiState(TwoPhaseIteratorAsDocIdSetIterator::new(iterator)),
      }),
    }
  }
}

/// Operations whose implementations depend on the constructor-selected mode.
#[doc(hidden)]
pub trait ConstantScoreState:
  DocIdSetIterator + ApproximationDISI + ApproximationBitSetAccess + 'static
{
  fn match_cost(&self) -> f32;

  fn matches(&mut self) -> Result<bool>;

  fn stop_competitive_iteration(&mut self);

  fn has_two_phase_iterator(&self) -> TwoPhaseState;
}

impl<S: ConstantScoreState> Scorable for ConstantScoreScorer<S> {
  type Child<'a>
    = crate::core::search::dummy::dummy_scorable::DummyScorable
  where
    Self: 'a;
  fn score(&mut self) -> Result<f32> {
    Ok(self.score)
  }
  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    if min_score > self.score {
      self.state.stop_competitive_iteration();
    }
    Ok(())
  }
  fn cost(&self) -> Result<i64> {
    self.state.cost()
  }
}
impl<S: ConstantScoreState> FixedScore for ConstantScoreScorer<S> {}
impl<S: DocIdSetIterator> DocIdSetIteratorExtensions for ConstantScoreScorer<S> {}
impl<S: DocIdSetIterator> BitSetIteratorAccess for ConstantScoreScorer<S> {
  fn is_bit_iter(&self) -> bool {
    self.state.is_bit_iter()
  }
  fn get(&self, index: usize) -> Result<bool> {
    self.state.get(index)
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.state.set_doc_id(doc)
  }
  fn bit_set_length(&self) -> Result<usize> {
    self.state.bit_set_length()
  }
}
impl<S: DocIdSetIterator> DocIdSetIterator for ConstantScoreScorer<S> {
  fn doc_id(&self) -> i32 {
    self.state.doc_id()
  }
  fn next_doc(&mut self) -> Result<i32> {
    self.state.next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.state.advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.state.slow_advance(target)
  }
  fn cost(&self) -> Result<i64> {
    self.state.cost()
  }
}
impl<S: ConstantScoreState> ApproximationDISI for ConstantScoreScorer<S> {
  fn approx_doc_id(&self) -> i32 {
    self.state.approx_doc_id()
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    self.state.approx_next_doc()
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.state.approx_advance(target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.state.approx_slow_advance(target)
  }
  fn approx_cost(&self) -> Result<i64> {
    self.state.approx_cost()
  }
}
impl<S: ConstantScoreState> ApproximationBitSetAccess for ConstantScoreScorer<S> {
  fn approx_is_bit_iter(&self) -> bool {
    self.state.approx_is_bit_iter()
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    self.state.approx_get(index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.state.approx_set_doc_id(doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    self.state.approx_bit_set_length()
  }
}

impl<S: ConstantScoreState> TwoPhaseIterator for ConstantScoreScorer<S> {
  fn matches(&mut self) -> Result<bool> {
    self.state.matches()
  }
  fn match_cost(&self) -> f32 {
    self.state.match_cost()
  }
}

impl<S: ConstantScoreState> Scorer for ConstantScoreScorer<S> {
  fn scoring_doc_id(&mut self) -> Result<i32> {
    Ok(self.state.doc_id())
  }

  fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
    Ok(self.score)
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    self.state.has_two_phase_iterator()
  }

  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    ScorerKind::ConstantScore
  }
}

#[doc(hidden)]
pub struct DisiState<D>(D);
impl<D: DocIdSetIterator> DocIdSetIteratorExtensions for DisiState<D> {}
impl<D: DocIdSetIterator> BitSetIteratorAccess for DisiState<D> {
  fn is_bit_iter(&self) -> bool {
    self.0.is_bit_iter()
  }
  fn get(&self, index: usize) -> Result<bool> {
    self.0.get(index)
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.0.set_doc_id(doc)
  }
  fn bit_set_length(&self) -> Result<usize> {
    self.0.bit_set_length()
  }
}
impl<D: DocIdSetIterator> DocIdSetIterator for DisiState<D> {
  fn doc_id(&self) -> i32 {
    self.0.doc_id()
  }
  fn next_doc(&mut self) -> Result<i32> {
    self.0.next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.0.advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.0.slow_advance(target)
  }
  fn cost(&self) -> Result<i64> {
    self.0.cost()
  }
}
impl<D: DocIdSetIterator> ApproximationDISI for DisiState<D> {
  fn approx_doc_id(&self) -> i32 {
    DocIdSetIterator::doc_id(self)
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    DocIdSetIterator::next_doc(self)
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    DocIdSetIterator::advance(self, target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    DocIdSetIterator::slow_advance(self, target)
  }
  fn approx_cost(&self) -> Result<i64> {
    DocIdSetIterator::cost(self)
  }
}
impl<D: DocIdSetIterator> ApproximationBitSetAccess for DisiState<D> {
  fn approx_is_bit_iter(&self) -> bool {
    BitSetIteratorAccess::is_bit_iter(self)
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    BitSetIteratorAccess::get(self, index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    BitSetIteratorAccess::set_doc_id(self, doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    BitSetIteratorAccess::bit_set_length(self)
  }
}
impl<D: DocIdSetIterator + 'static> ConstantScoreState for DisiState<D> {
  fn match_cost(&self) -> f32 {
    0.0
  }

  fn matches(&mut self) -> Result<bool> {
    Ok(true)
  }

  fn stop_competitive_iteration(&mut self) {}

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }
}

#[doc(hidden)]
pub struct TopDisiState<D>(DocIdSetIteratorWrapper<DisiDelegate<D>>);
impl<D: DocIdSetIterator> DocIdSetIteratorExtensions for TopDisiState<D> {}
impl<D: DocIdSetIterator> BitSetIteratorAccess for TopDisiState<D> {
  fn get(&self, _index: usize) -> Result<bool> {
    Err(LuceneError::unsupported_operation(
      "Wrapped iterator is not a BitSetIterator",
    ))
  }
  fn set_doc_id(&mut self, _doc: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "Wrapped iterator is not a BitSetIterator",
    ))
  }
  fn bit_set_length(&self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(
      "Wrapped iterator is not a BitSetIterator",
    ))
  }
}
impl<D: DocIdSetIterator> DocIdSetIterator for TopDisiState<D> {
  fn doc_id(&self) -> i32 {
    self.0.doc_id()
  }
  fn next_doc(&mut self) -> Result<i32> {
    self.0.next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.0.advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.0.slow_advance(target)
  }
  fn cost(&self) -> Result<i64> {
    self.0.cost()
  }
}
impl<D: DocIdSetIterator> ApproximationDISI for TopDisiState<D> {
  fn approx_doc_id(&self) -> i32 {
    DocIdSetIterator::doc_id(self)
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    DocIdSetIterator::next_doc(self)
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    DocIdSetIterator::advance(self, target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    DocIdSetIterator::slow_advance(self, target)
  }
  fn approx_cost(&self) -> Result<i64> {
    DocIdSetIterator::cost(self)
  }
}
impl<D: DocIdSetIterator> ApproximationBitSetAccess for TopDisiState<D> {
  fn approx_is_bit_iter(&self) -> bool {
    BitSetIteratorAccess::is_bit_iter(self)
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    BitSetIteratorAccess::get(self, index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    BitSetIteratorAccess::set_doc_id(self, doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    BitSetIteratorAccess::bit_set_length(self)
  }
}
impl<D: DocIdSetIterator + 'static> ConstantScoreState for TopDisiState<D> {
  fn match_cost(&self) -> f32 {
    0.0
  }

  fn matches(&mut self) -> Result<bool> {
    Ok(true)
  }

  fn stop_competitive_iteration(&mut self) {
    self.0.delegate = DisiDelegate::Empty(EmptyDISI::new());
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }
}

#[doc(hidden)]
pub struct TpiState<T>(TwoPhaseIteratorAsDocIdSetIterator<T>);
impl<T: TwoPhaseIterator> DocIdSetIteratorExtensions for TpiState<T> {}
impl<T: TwoPhaseIterator> BitSetIteratorAccess for TpiState<T> {
  fn get(&self, _index: usize) -> Result<bool> {
    Err(LuceneError::unsupported_operation(
      "Wrapped iterator is not a BitSetIterator",
    ))
  }
  fn set_doc_id(&mut self, _doc: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "Wrapped iterator is not a BitSetIterator",
    ))
  }
  fn bit_set_length(&self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(
      "Wrapped iterator is not a BitSetIterator",
    ))
  }
}
impl<T: TwoPhaseIterator> DocIdSetIterator for TpiState<T> {
  fn doc_id(&self) -> i32 {
    self.0.doc_id()
  }
  fn next_doc(&mut self) -> Result<i32> {
    self.0.next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.0.advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.0.slow_advance(target)
  }
  fn cost(&self) -> Result<i64> {
    self.0.cost()
  }
}
impl<T: TwoPhaseIterator> ApproximationDISI for TpiState<T> {
  fn approx_doc_id(&self) -> i32 {
    self.0.two_phase_iterator.approx_doc_id()
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    self.0.two_phase_iterator.approx_next_doc()
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.0.two_phase_iterator.approx_advance(target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.0.two_phase_iterator.approx_slow_advance(target)
  }
  fn approx_cost(&self) -> Result<i64> {
    self.0.two_phase_iterator.approx_cost()
  }
}
impl<T: TwoPhaseIterator> ApproximationBitSetAccess for TpiState<T> {
  fn approx_is_bit_iter(&self) -> bool {
    self.0.two_phase_iterator.approx_is_bit_iter()
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    self.0.two_phase_iterator.approx_get(index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.0.two_phase_iterator.approx_set_doc_id(doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    self.0.two_phase_iterator.approx_bit_set_length()
  }
}
impl<T: TwoPhaseIterator + 'static> ConstantScoreState for TpiState<T> {
  fn match_cost(&self) -> f32 {
    self.0.two_phase_iterator.match_cost()
  }

  fn matches(&mut self) -> Result<bool> {
    self.0.two_phase_iterator.matches()
  }

  fn stop_competitive_iteration(&mut self) {}

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::Yes
  }
}

#[doc(hidden)]
pub struct TopTpiState<T>(TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<T>>);
impl<T: TwoPhaseIterator> DocIdSetIteratorExtensions for TopTpiState<T> {}
impl<T: TwoPhaseIterator> BitSetIteratorAccess for TopTpiState<T> {
  fn get(&self, _index: usize) -> Result<bool> {
    Err(LuceneError::unsupported_operation(
      "Wrapped iterator is not a BitSetIterator",
    ))
  }
  fn set_doc_id(&mut self, _doc: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "Wrapped iterator is not a BitSetIterator",
    ))
  }
  fn bit_set_length(&self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(
      "Wrapped iterator is not a BitSetIterator",
    ))
  }
}
impl<T: TwoPhaseIterator> DocIdSetIterator for TopTpiState<T> {
  fn doc_id(&self) -> i32 {
    self.0.doc_id()
  }
  fn next_doc(&mut self) -> Result<i32> {
    self.0.next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.0.advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.0.slow_advance(target)
  }
  fn cost(&self) -> Result<i64> {
    self.0.cost()
  }
}
impl<T: TwoPhaseIterator> ApproximationDISI for TopTpiState<T> {
  fn approx_doc_id(&self) -> i32 {
    self.0.two_phase_iterator.approx_doc_id()
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    self.0.two_phase_iterator.approx_next_doc()
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.0.two_phase_iterator.approx_advance(target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.0.two_phase_iterator.approx_slow_advance(target)
  }
  fn approx_cost(&self) -> Result<i64> {
    self.0.two_phase_iterator.approx_cost()
  }
}
impl<T: TwoPhaseIterator> ApproximationBitSetAccess for TopTpiState<T> {
  fn approx_is_bit_iter(&self) -> bool {
    self.0.two_phase_iterator.approx_is_bit_iter()
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    self.0.two_phase_iterator.approx_get(index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.0.two_phase_iterator.approx_set_doc_id(doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    self.0.two_phase_iterator.approx_bit_set_length()
  }
}
impl<T: TwoPhaseIterator + 'static> ConstantScoreState for TopTpiState<T> {
  fn match_cost(&self) -> f32 {
    self.0.two_phase_iterator.match_cost()
  }

  fn matches(&mut self) -> Result<bool> {
    self.0.two_phase_iterator.matches()
  }

  fn stop_competitive_iteration(&mut self) {
    self.0.two_phase_iterator.approximation.delegate.empty = Some(EmptyDISI::new());
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::Yes
  }
}
pub struct TwoPhaseIteratorImpl<TPI> {
  approximation: DocIdSetIteratorWrapper<TwoPhaseDelegate<TPI>>,
}
impl<TPI> TwoPhaseIteratorImpl<TPI> {
  fn new(approximation: DocIdSetIteratorWrapper<TwoPhaseDelegate<TPI>>) -> Self {
    Self { approximation }
  }
}
impl<TPI> ApproximationDISI for TwoPhaseIteratorImpl<TPI>
where
  TPI: TwoPhaseIterator,
{
  fn approx_doc_id(&self) -> i32 {
    self.approximation.doc_id()
  }

  fn approx_next_doc(&mut self) -> Result<i32> {
    self.approximation.next_doc()
  }

  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.approximation.advance(target)
  }

  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.approximation.slow_advance(target)
  }

  fn approx_cost(&self) -> Result<i64> {
    self.approximation.cost()
  }
}
impl<TPI> ApproximationBitSetAccess for TwoPhaseIteratorImpl<TPI>
where
  TPI: TwoPhaseIterator,
{
  fn approx_is_bit_iter(&self) -> bool {
    self.approximation.is_bit_iter()
  }

  fn approx_get(&self, index: usize) -> Result<bool> {
    self.approximation.get(index)
  }

  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.approximation.set_doc_id(doc)
  }

  fn approx_bit_set_length(&self) -> Result<usize> {
    self.approximation.bit_set_length()
  }
}

impl<TPI> TwoPhaseIterator for TwoPhaseIteratorImpl<TPI>
where
  TPI: TwoPhaseIterator,
{
  fn matches(&mut self) -> Result<bool> {
    self.approximation.delegate.two_phase_iterator.matches()
  }

  fn match_cost(&self) -> f32 {
    self.approximation.delegate.two_phase_iterator.match_cost()
  }
}

#[doc(hidden)]
pub enum DisiDelegate<D> {
  Disi(D),
  Empty(EmptyDISI),
}
impl<D> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for DisiDelegate<D> where
  D: DocIdSetIterator
{
}
impl<D> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for DisiDelegate<D> where
  D: DocIdSetIterator
{
}

impl<D> DocIdSetIterator for DisiDelegate<D>
where
  D: DocIdSetIterator,
{
  fn doc_id(&self) -> i32 {
    match self {
      Self::Disi(d) => d.doc_id(),
      Self::Empty(e) => e.doc_id(),
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    match self {
      Self::Disi(d) => d.next_doc(),
      Self::Empty(e) => e.next_doc(),
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    match self {
      Self::Disi(d) => d.advance(target),
      Self::Empty(e) => e.advance(target),
    }
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    match self {
      Self::Disi(d) => d.slow_advance(target),
      Self::Empty(e) => e.slow_advance(target),
    }
  }

  fn cost(&self) -> Result<i64> {
    match self {
      Self::Disi(d) => d.cost(),
      Self::Empty(e) => e.cost(),
    }
  }
}

#[doc(hidden)]
pub struct TwoPhaseDelegate<T> {
  two_phase_iterator: T,
  empty: Option<EmptyDISI>,
}
impl<T> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for TwoPhaseDelegate<T> where
  T: TwoPhaseIterator
{
}
impl<T> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for TwoPhaseDelegate<T> where
  T: TwoPhaseIterator
{
}

impl<T> DocIdSetIterator for TwoPhaseDelegate<T>
where
  T: TwoPhaseIterator,
{
  fn doc_id(&self) -> i32 {
    match &self.empty {
      Some(e) => e.doc_id(),
      None => self.two_phase_iterator.approx_doc_id(),
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    match &mut self.empty {
      Some(e) => e.next_doc(),
      None => self.two_phase_iterator.approx_next_doc(),
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    match &mut self.empty {
      Some(e) => e.advance(target),
      None => self.two_phase_iterator.approx_advance(target),
    }
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    match &mut self.empty {
      Some(e) => e.slow_advance(target),
      None => self.two_phase_iterator.approx_slow_advance(target),
    }
  }

  fn cost(&self) -> Result<i64> {
    match &self.empty {
      Some(e) => e.cost(),
      None => self.two_phase_iterator.approx_cost(),
    }
  }
}

#[doc(hidden)]
pub struct DocIdSetIteratorWrapper<D> {
  doc: i32,
  delegate: D,
}

impl<D> DocIdSetIteratorWrapper<D> {
  fn new(delegate: D) -> Self {
    Self { doc: -1, delegate }
  }
}

impl<D> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for DocIdSetIteratorWrapper<D>
where
  D: DocIdSetIterator,
{
}
impl<D> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for DocIdSetIteratorWrapper<D>
where
  D: DocIdSetIterator,
{
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
