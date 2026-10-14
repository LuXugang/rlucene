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

use crate::core::search::conjunction_disi::{
  ConjunctionDISI, ConjunctionDISIEnum, ConjunctionTwoPhaseIterator, ScorerDisi,
};
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::scorable::{ChildScorable, Scorable};
#[cfg(test)]
use crate::core::search::scorer::ScorerKind;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::TwoPhaseIteratorAsDocIdSetIterator;
use crate::core::util::error::lucene_error::Result;

pub type ConjunctionScorerDisi<S> = DocIdSetIteratorEnum2<
  ConjunctionDISIEnum<ScorerDisi<S>>,
  TwoPhaseIteratorAsDocIdSetIterator<ConjunctionTwoPhaseIterator<S>>,
>;
// TODO IMPORTANT: Review conjunction performance; it is worse in some scenarios.
/// Scorer for conjunctions, sets of queries, all of which are required.
pub struct ConjunctionScorer<D: DocIdSetIterator> {
  disi: D,
  scoring_idx: Vec<usize>,
}
impl<S: Scorer> ConjunctionScorer<ConjunctionDISIEnum<ScorerDisi<S>>> {
  /// Select the iterator before entering a document loop.
  pub(crate) fn prepare(required: Vec<S>, scorers: &[usize]) -> Result<ConjunctionScorerDisi<S>> {
    debug_assert!({ scorers.iter().all(|v| *v < required.len()) });
    let mut has_tpi = false;
    for v in required.iter() {
      if v.two_phase_iterator().is_some() {
        has_tpi = true;
        break;
      }
    }
    let v = ConjunctionDISI::from_scorer(required)?;
    if has_tpi {
      Ok(ConjunctionScorerDisi::B(
        TwoPhaseIteratorAsDocIdSetIterator::new(ConjunctionTwoPhaseIterator::new(v)?),
      ))
    } else {
      Ok(ConjunctionScorerDisi::A(v))
    }
  }
  pub(crate) fn from_disi(
    disi: ConjunctionDISIEnum<ScorerDisi<S>>,
    scoring_idx: Vec<usize>,
  ) -> Self {
    Self { disi, scoring_idx }
  }
}
impl<S: Scorer>
  ConjunctionScorer<TwoPhaseIteratorAsDocIdSetIterator<ConjunctionTwoPhaseIterator<S>>>
{
  pub(crate) fn from_tpi(
    disi: TwoPhaseIteratorAsDocIdSetIterator<ConjunctionTwoPhaseIterator<S>>,
    scoring_idx: Vec<usize>,
  ) -> Self {
    Self { disi, scoring_idx }
  }
}

/// Access to the conjunction owned by the selected iterator.
#[doc(hidden)]
pub trait ConjunctionState: DocIdSetIterator + 'static {
  type SubScorer: Scorer + 'static;
  fn conjunction(&self) -> &ConjunctionDISIEnum<ScorerDisi<Self::SubScorer>>;
  fn conjunction_mut(&mut self) -> &mut ConjunctionDISIEnum<ScorerDisi<Self::SubScorer>>;
  fn two_phase_iterator(&self) -> Option<&ConjunctionTwoPhaseIterator<Self::SubScorer>>;
  fn two_phase_iterator_mut(&mut self)
  -> Option<&mut ConjunctionTwoPhaseIterator<Self::SubScorer>>;
  fn take_two_phase_iterator(self) -> Option<ConjunctionTwoPhaseIterator<Self::SubScorer>>;
  fn has_two_phase_iterator(&self) -> TwoPhaseState;
}
impl<S: Scorer + 'static> ConjunctionState for ConjunctionDISIEnum<ScorerDisi<S>> {
  type SubScorer = S;
  fn conjunction(&self) -> &Self {
    self
  }
  fn conjunction_mut(&mut self) -> &mut Self {
    self
  }
  fn two_phase_iterator(&self) -> Option<&ConjunctionTwoPhaseIterator<S>> {
    None
  }
  fn two_phase_iterator_mut(&mut self) -> Option<&mut ConjunctionTwoPhaseIterator<S>> {
    None
  }
  fn take_two_phase_iterator(self) -> Option<ConjunctionTwoPhaseIterator<S>> {
    None
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }
}
impl<S: Scorer + 'static> ConjunctionState
  for TwoPhaseIteratorAsDocIdSetIterator<ConjunctionTwoPhaseIterator<S>>
{
  type SubScorer = S;
  fn conjunction(&self) -> &ConjunctionDISIEnum<ScorerDisi<S>> {
    &self.two_phase_iterator.approximation
  }
  fn conjunction_mut(&mut self) -> &mut ConjunctionDISIEnum<ScorerDisi<S>> {
    &mut self.two_phase_iterator.approximation
  }
  fn two_phase_iterator(&self) -> Option<&ConjunctionTwoPhaseIterator<S>> {
    Some(&self.two_phase_iterator)
  }
  fn two_phase_iterator_mut(&mut self) -> Option<&mut ConjunctionTwoPhaseIterator<S>> {
    Some(&mut self.two_phase_iterator)
  }
  fn take_two_phase_iterator(self) -> Option<ConjunctionTwoPhaseIterator<S>> {
    Some(self.two_phase_iterator)
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::Yes
  }
}

impl<D: ConjunctionState> Scorable for ConjunctionScorer<D> {
  type Child<'a>
    = crate::core::search::dummy::dummy_scorable::DummyScorable
  where
    Self: 'a;
  fn score(&mut self) -> Result<f32> {
    let mut sum = 0f64;
    for x in self.scoring_idx.iter() {
      let score = self.disi.conjunction_mut().iterator_at_mut(*x).score()?;
      sum += score as f64;
    }
    Ok(sum as f32)
  }
  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    // This scorer is only used for TOP_SCORES when there is a single scoring clause
    if self.scoring_idx.len() == 1 {
      let i = self.scoring_idx[0];
      self
        .disi
        .conjunction_mut()
        .iterator_at_mut(i)
        .set_min_competitive_score(min_score)?;
    }
    Ok(())
  }
  fn get_children(&mut self) -> Result<Vec<ChildScorable<Self::Child<'_>>>> {
    todo!()
  }
  fn cost(&self) -> Result<i64> {
    self.disi.cost()
  }
}
impl<D: DocIdSetIterator> crate::core::search::scorable::FixedScore for ConjunctionScorer<D> {}
impl<D: DocIdSetIterator> DocIdSetIterator for ConjunctionScorer<D> {
  fn doc_id(&self) -> i32 {
    self.disi.doc_id()
  }
  fn next_doc(&mut self) -> Result<i32> {
    self.disi.next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.disi.advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.disi.slow_advance(target)
  }
  fn cost(&self) -> Result<i64> {
    self.disi.cost()
  }
}
impl<D: DocIdSetIterator> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for ConjunctionScorer<D>
{
}
impl<D: DocIdSetIterator> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for ConjunctionScorer<D>
{
}
impl<D: ConjunctionState> Scorer for ConjunctionScorer<D> {
  type Iterator = D;
  type OwnedTwoPhase = ConjunctionTwoPhaseIterator<D::SubScorer>;
  type ApproximationRef<'a>
    = &'a ConjunctionDISIEnum<ScorerDisi<D::SubScorer>>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = &'a mut ConjunctionDISIEnum<ScorerDisi<D::SubScorer>>
  where
    Self: 'a;
  type TwoPhaseRef<'a>
    = &'a Self::OwnedTwoPhase
  where
    Self: 'a;
  type TwoPhaseMut<'a>
    = &'a mut Self::OwnedTwoPhase
  where
    Self: 'a;
  fn scoring_doc_id(&mut self) -> Result<i32> {
    Ok(self.disi.doc_id())
  }
  fn take_iterator(self) -> Self::Iterator {
    self.disi
  }
  fn two_phase_iterator(&self) -> Option<Self::TwoPhaseRef<'_>> {
    self.disi.two_phase_iterator()
  }
  fn two_phase_iterator_mut(&mut self) -> Option<Self::TwoPhaseMut<'_>> {
    self.disi.two_phase_iterator_mut()
  }
  fn take_two_phase_iterator(self) -> Option<Self::OwnedTwoPhase> {
    self.disi.take_two_phase_iterator()
  }
  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    if self.scoring_idx.len() == 1 {
      let i = self.scoring_idx[0];
      return self
        .disi
        .conjunction_mut()
        .iterator_at_mut(i)
        .advance_shallow(target);
    }
    for &idx in &self.scoring_idx {
      self
        .disi
        .conjunction_mut()
        .iterator_at_mut(idx)
        .advance_shallow(target)?;
    }
    self.default_advance_shallow(target)
  }
  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    let mut max_score = 0f64;
    for &idx in &self.scoring_idx {
      let s = self.disi.conjunction_mut().iterator_at_mut(idx);
      if s.scoring_doc_id()? <= upto {
        max_score += s.get_max_score(upto)? as f64;
      }
    }
    Ok(max_score as f32)
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    self.disi.has_two_phase_iterator()
  }
  fn approximation(&self) -> Self::ApproximationRef<'_> {
    self.disi.conjunction()
  }
  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    self.disi.conjunction_mut()
  }
  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    ScorerKind::Conjunction
  }
}
