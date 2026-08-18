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
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, EmptyDISI};
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
#[cfg(test)]
use crate::core::search::scorer::ScorerKind;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
  TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// A constant-scoring Scorer.
pub struct ConstantScoreScorer<DISI, TPI> {
  score: f32,
  score_mode: ScoreMode,
  disi: ConstantScoreIterator<DISI, TPI>,
  tpi_state: TwoPhaseState,
}
impl<DISI> ConstantScoreScorer<DISI, DummyTwoPhaseIterator> {
  /// Creates an instance based on a [`DocIdSetIterator`] used to drive iteration. Two-phase
  /// iteration is not supported.
  ///
  /// # Parameters
  /// - `score`: the score to return on each document.
  /// - `score_mode`: the score mode.
  /// - `disi`: the iterator that defines matching documents.
  pub fn from_disi(score: f32, score_mode: ScoreMode, disi: DISI) -> Self {
    let approximation = match score_mode {
      ScoreMode::TopScores => {
        ConstantScoreIterator::DisiTop(DocIdSetIteratorWrapper::new(DelegateEnum::Disi(disi)))
      },
      _ => ConstantScoreIterator::Disi(disi),
    };
    Self {
      score,
      score_mode,
      disi: approximation,
      tpi_state: TwoPhaseState::No,
    }
  }
}
impl<TPI> ConstantScoreScorer<DummyDISI, TPI> {
  /// Creates an instance based on a [`TwoPhaseIterator`]. In this case the `Scorer` will
  /// support two-phase iteration.
  ///
  /// # Parameters
  /// - `score`: the score to return on each document.
  /// - `score_mode`: the score mode.
  /// - `two_phase_iterator`: the iterator that defines matching documents.
  pub fn from_tpi(score: f32, score_mode: ScoreMode, two_phase_iterator: TPI) -> Self {
    let two_phase_iterator = match score_mode {
      ScoreMode::TopScores => {
        let v: DocIdSetIteratorWrapper<TPI, DummyDISI> =
          DocIdSetIteratorWrapper::new(DelegateEnum::TPI(two_phase_iterator));
        ConstantScoreIterator::TpiTop(TwoPhaseIteratorAsDocIdSetIterator::new(
          TwoPhaseIteratorImpl::new(v),
        ))
      },
      _ => ConstantScoreIterator::Tpi(TwoPhaseIteratorAsDocIdSetIterator::new(two_phase_iterator)),
    };
    Self {
      score,
      score_mode,
      disi: two_phase_iterator,
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
      match &mut self.disi {
        ConstantScoreIterator::DisiTop(iterator) => {
          iterator.delegate = DelegateEnum::Empty(EmptyDISI::new());
        },
        ConstantScoreIterator::TpiTop(iterator) => {
          iterator.two_phase_iterator.approximation.delegate =
            DelegateEnum::Empty(EmptyDISI::new());
        },
        ConstantScoreIterator::Disi(_) | ConstantScoreIterator::Tpi(_) => {
          return Err(LuceneError::illegal_state("TopScores: should not be here"));
        },
      }
    }
    Ok(())
  }

  fn cost(&self) -> Result<i64> {
    self.iterator().cost()
  }
}

impl<DISI, TPI> crate::core::search::scorable::FixedScore for ConstantScoreScorer<DISI, TPI> {}

impl<DISI, TPI> Scorer for ConstantScoreScorer<DISI, TPI>
where
  DISI: DocIdSetIterator + 'static,
  TPI: TwoPhaseIterator + 'static,
{
  fn doc_id(&mut self) -> Result<i32> {
    Ok(self.disi.doc_id())
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.disi)
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.disi)
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let ConstantScoreScorer { disi, .. } = *self;
    Box::new(disi)
  }

  fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    match self.tpi_state {
      TwoPhaseState::No => None,
      _ => match &self.disi {
        ConstantScoreIterator::DisiTop(_) | ConstantScoreIterator::Disi(_) => {
          debug_assert!(false, "should not be here");
          None
        },
        ConstantScoreIterator::TpiTop(iterator) => Some(Box::new(&iterator.two_phase_iterator)),
        ConstantScoreIterator::Tpi(iterator) => Some(Box::new(&iterator.two_phase_iterator)),
      },
    }
  }

  fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    match self.tpi_state {
      TwoPhaseState::No => None,
      _ => match &mut self.disi {
        ConstantScoreIterator::DisiTop(_) | ConstantScoreIterator::Disi(_) => {
          debug_assert!(false, "should not be here");
          None
        },
        ConstantScoreIterator::TpiTop(iterator) => Some(Box::new(&mut iterator.two_phase_iterator)),
        ConstantScoreIterator::Tpi(iterator) => Some(Box::new(&mut iterator.two_phase_iterator)),
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
        ConstantScoreIterator::DisiTop(_) | ConstantScoreIterator::Disi(_) => {
          debug_assert!(false, "should not be here");
          None
        },
        ConstantScoreIterator::TpiTop(iterator) => Some(Box::new(iterator.two_phase_iterator)),
        ConstantScoreIterator::Tpi(iterator) => Some(Box::new(iterator.two_phase_iterator)),
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
      _ => match &mut self.disi {
        ConstantScoreIterator::DisiTop(iterator) => Box::new(iterator),
        ConstantScoreIterator::Disi(iterator) => Box::new(iterator),
        ConstantScoreIterator::TpiTop(iterator) => iterator.two_phase_iterator.approximation_mut(),
        ConstantScoreIterator::Tpi(iterator) => iterator.two_phase_iterator.approximation_mut(),
      },
    }
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    match self.tpi_state {
      TwoPhaseState::No => self.iterator(),
      _ => match &self.disi {
        ConstantScoreIterator::DisiTop(iterator) => Box::new(iterator),
        ConstantScoreIterator::Disi(iterator) => Box::new(iterator),
        ConstantScoreIterator::TpiTop(iterator) => iterator.two_phase_iterator.approximation(),
        ConstantScoreIterator::Tpi(iterator) => iterator.two_phase_iterator.approximation(),
      },
    }
  }
  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    ScorerKind::ConstantScore
  }
}

pub struct TwoPhaseIteratorImpl<TPI> {
  approximation: DocIdSetIteratorWrapper<TPI, DummyDISI>,
}
impl<TPI> TwoPhaseIteratorImpl<TPI> {
  pub fn new(approximation: DocIdSetIteratorWrapper<TPI, DummyDISI>) -> Self {
    Self { approximation }
  }
}
impl<TPI> TwoPhaseIterator for TwoPhaseIteratorImpl<TPI>
where
  TPI: TwoPhaseIterator,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.approximation)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.approximation)
  }

  fn matches(&mut self) -> Result<bool> {
    match self.approximation.delegate {
      DelegateEnum::TPI(ref mut t) => t.matches(),
      DelegateEnum::Empty(_) => Ok(false),
      _ => Err(LuceneError::illegal_state(
        "two-phase iterator has a doc-id iterator delegate",
      )),
    }
  }

  fn match_cost(&self) -> f32 {
    match self.approximation.delegate {
      DelegateEnum::TPI(ref t) => t.match_cost(),
      DelegateEnum::Empty(_) => 0.0,
      _ => unreachable!("should not be here"),
    }
  }
}

enum ConstantScoreIterator<DISI, TPI> {
  DisiTop(DocIdSetIteratorWrapper<DummyTwoPhaseIterator, DISI>),
  Disi(DISI),
  TpiTop(TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<TPI>>),
  Tpi(TwoPhaseIteratorAsDocIdSetIterator<TPI>),
}

impl<DISI, TPI> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for ConstantScoreIterator<DISI, TPI>
where
  DISI: DocIdSetIterator,
  TPI: TwoPhaseIterator,
{
}
impl<DISI, TPI> DocIdSetIterator for ConstantScoreIterator<DISI, TPI>
where
  DISI: DocIdSetIterator,
  TPI: TwoPhaseIterator,
{
  fn doc_id(&self) -> i32 {
    match self {
      Self::DisiTop(iterator) => iterator.doc_id(),
      Self::Disi(iterator) => iterator.doc_id(),
      Self::TpiTop(iterator) => iterator.doc_id(),
      Self::Tpi(iterator) => iterator.doc_id(),
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    match self {
      Self::DisiTop(iterator) => iterator.next_doc(),
      Self::Disi(iterator) => iterator.next_doc(),
      Self::TpiTop(iterator) => iterator.next_doc(),
      Self::Tpi(iterator) => iterator.next_doc(),
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    match self {
      Self::DisiTop(iterator) => iterator.advance(target),
      Self::Disi(iterator) => iterator.advance(target),
      Self::TpiTop(iterator) => iterator.advance(target),
      Self::Tpi(iterator) => iterator.advance(target),
    }
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    match self {
      Self::DisiTop(iterator) => iterator.slow_advance(target),
      Self::Disi(iterator) => iterator.slow_advance(target),
      Self::TpiTop(iterator) => iterator.slow_advance(target),
      Self::Tpi(iterator) => iterator.slow_advance(target),
    }
  }

  fn cost(&self) -> Result<i64> {
    match self {
      Self::DisiTop(iterator) => iterator.cost(),
      Self::Disi(iterator) => iterator.cost(),
      Self::TpiTop(iterator) => iterator.cost(),
      Self::Tpi(iterator) => iterator.cost(),
    }
  }
}

pub enum DelegateEnum<T, D> {
  TPI(T),
  Disi(D),
  Empty(EmptyDISI),
}
impl<T, D> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for DelegateEnum<T, D>
where
  T: TwoPhaseIterator,
  D: DocIdSetIterator,
{
}
impl<T, D> DocIdSetIterator for DelegateEnum<T, D>
where
  T: TwoPhaseIterator,
  D: DocIdSetIterator,
{
  fn doc_id(&self) -> i32 {
    match self {
      DelegateEnum::TPI(t) => t.approximation().doc_id(),
      DelegateEnum::Disi(d) => d.doc_id(),
      DelegateEnum::Empty(e) => e.doc_id(),
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    match self {
      DelegateEnum::TPI(t) => t.approximation_mut().next_doc(),
      DelegateEnum::Disi(d) => d.next_doc(),
      DelegateEnum::Empty(e) => e.next_doc(),
    }
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    match self {
      DelegateEnum::TPI(t) => t.approximation_mut().advance(_target),
      DelegateEnum::Disi(d) => d.advance(_target),
      DelegateEnum::Empty(e) => e.advance(_target),
    }
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    match self {
      DelegateEnum::TPI(t) => t.approximation_mut().slow_advance(target),
      DelegateEnum::Disi(d) => d.slow_advance(target),
      DelegateEnum::Empty(e) => e.slow_advance(target),
    }
  }

  fn cost(&self) -> Result<i64> {
    match self {
      DelegateEnum::TPI(t) => t.approximation().cost(),
      DelegateEnum::Disi(d) => d.cost(),
      DelegateEnum::Empty(e) => e.cost(),
    }
  }
}

pub struct DocIdSetIteratorWrapper<T, D> {
  doc: i32,
  delegate: DelegateEnum<T, D>,
}

impl<T, D> DocIdSetIteratorWrapper<T, D> {
  pub fn new(delegate: DelegateEnum<T, D>) -> Self {
    Self { doc: -1, delegate }
  }
}

impl<T, D> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for DocIdSetIteratorWrapper<T, D>
where
  T: TwoPhaseIterator,
  D: DocIdSetIterator,
{
}
impl<T, D> DocIdSetIterator for DocIdSetIteratorWrapper<T, D>
where
  T: TwoPhaseIterator,
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
